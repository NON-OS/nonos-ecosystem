// NONOS GNU AFFERO GENERAL PUBLIC LICENSE
// Version 3, 19 November 2007
// Copyright (C) 2025 NON-OS <team@nonos.systems>

//! Metrics Module
//!
//! Production-grade metrics collection with:
//! - Lock-free atomic counters for high performance
//! - Histogram support for latency tracking
//! - Full Prometheus exposition format
//! - Per-service metrics tracking
//! - System resource monitoring (CPU, memory, network)
//! - Sliding window health scoring

use nonos_types::QualityScore;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use sysinfo::{System, Pid};
use tracing::debug;

/// Histogram bucket boundaries for latency tracking (in milliseconds)
const LATENCY_BUCKETS: &[u64] = &[1, 5, 10, 25, 50, 100, 250, 500, 1000, 2500, 5000, 10000];

/// Maximum samples for sliding window calculations
const MAX_WINDOW_SAMPLES: usize = 1000;

/// Metrics collector using lock-free atomic counters
pub struct NodeMetricsCollector {
    /// Total requests handled
    total_requests: AtomicU64,
    /// Successful requests
    successful_requests: AtomicU64,
    /// Failed requests
    failed_requests: AtomicU64,
    /// Total latency in microseconds (for averaging)
    total_latency_us: AtomicU64,
    /// Request count for latency calculations
    latency_count: AtomicU64,
    /// Latency histogram buckets
    latency_histogram: Vec<AtomicU64>,
    /// Bytes sent
    bytes_sent: AtomicU64,
    /// Bytes received
    bytes_received: AtomicU64,
    /// Active connections
    active_connections: AtomicU64,
    /// Total connections ever established
    total_connections: AtomicU64,
    /// Per-service metrics
    service_metrics: RwLock<HashMap<String, ServiceMetrics>>,
    /// Uptime samples (sliding window)
    uptime_samples: RwLock<Vec<bool>>,
    /// Start time for uptime calculation
    start_time: Instant,
    /// Node ID for labeling
    node_id: RwLock<Option<String>>,
    /// Node role for labeling
    node_role: RwLock<Option<String>>,
    /// System info collector
    system: RwLock<System>,
    /// Current process ID
    pid: Pid,
    /// Last system metrics update
    last_system_update: RwLock<Instant>,
    /// Cached CPU usage
    cached_cpu: AtomicU64,
    /// Cached memory usage (bytes)
    cached_memory: AtomicU64,
    // P2P-specific metrics
    /// P2P peers connected (gauge)
    p2p_peers_connected: AtomicU64,
    /// P2P total known peers
    p2p_peers_total: AtomicU64,
    /// P2P messages published
    p2p_messages_published: AtomicU64,
    /// P2P messages received
    p2p_messages_received: AtomicU64,
    /// P2P messages dropped
    p2p_messages_dropped: AtomicU64,
    /// P2P peer bans total
    p2p_peer_bans: AtomicU64,
    /// P2P rate limit hits
    p2p_rate_limit_hits: AtomicU64,
    /// API requests total
    api_requests: AtomicU64,
    /// API request errors
    api_errors: AtomicU64,
}

/// Per-service metrics
#[derive(Clone, Debug, Default)]
pub struct ServiceMetrics {
    /// Service name
    pub name: String,
    /// Requests handled
    pub requests: u64,
    /// Errors encountered
    pub errors: u64,
    /// Total latency in microseconds
    pub total_latency_us: u64,
    /// Is service running
    pub running: bool,
    /// Restart count
    pub restarts: u32,
    /// Service uptime in seconds
    pub uptime_secs: u64,
}

impl NodeMetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> Self {
        let latency_histogram: Vec<AtomicU64> = LATENCY_BUCKETS
            .iter()
            .map(|_| AtomicU64::new(0))
            .collect();

        Self {
            total_requests: AtomicU64::new(0),
            successful_requests: AtomicU64::new(0),
            failed_requests: AtomicU64::new(0),
            total_latency_us: AtomicU64::new(0),
            latency_count: AtomicU64::new(0),
            latency_histogram,
            bytes_sent: AtomicU64::new(0),
            bytes_received: AtomicU64::new(0),
            active_connections: AtomicU64::new(0),
            total_connections: AtomicU64::new(0),
            service_metrics: RwLock::new(HashMap::new()),
            uptime_samples: RwLock::new(Vec::with_capacity(MAX_WINDOW_SAMPLES)),
            start_time: Instant::now(),
            node_id: RwLock::new(None),
            node_role: RwLock::new(None),
            system: RwLock::new(System::new_all()),
            pid: Pid::from_u32(std::process::id()),
            last_system_update: RwLock::new(Instant::now()),
            cached_cpu: AtomicU64::new(0),
            cached_memory: AtomicU64::new(0),
            // P2P metrics
            p2p_peers_connected: AtomicU64::new(0),
            p2p_peers_total: AtomicU64::new(0),
            p2p_messages_published: AtomicU64::new(0),
            p2p_messages_received: AtomicU64::new(0),
            p2p_messages_dropped: AtomicU64::new(0),
            p2p_peer_bans: AtomicU64::new(0),
            p2p_rate_limit_hits: AtomicU64::new(0),
            api_requests: AtomicU64::new(0),
            api_errors: AtomicU64::new(0),
        }
    }

    /// Set node role for metric labeling
    pub fn set_node_role(&self, role: &str) {
        *self.node_role.write() = Some(role.to_string());
    }

    // P2P metrics methods

    /// Set P2P connected peers count (gauge)
    pub fn set_p2p_peers_connected(&self, count: u64) {
        self.p2p_peers_connected.store(count, Ordering::Relaxed);
    }

    /// Set P2P total known peers (gauge)
    pub fn set_p2p_peers_total(&self, count: u64) {
        self.p2p_peers_total.store(count, Ordering::Relaxed);
    }

    /// Increment P2P messages published
    pub fn record_p2p_message_published(&self) {
        self.p2p_messages_published.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment P2P messages received
    pub fn record_p2p_message_received(&self) {
        self.p2p_messages_received.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment P2P messages dropped
    pub fn record_p2p_message_dropped(&self) {
        self.p2p_messages_dropped.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment P2P peer bans
    pub fn record_p2p_peer_ban(&self) {
        self.p2p_peer_bans.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment P2P rate limit hits
    pub fn record_p2p_rate_limit_hit(&self) {
        self.p2p_rate_limit_hits.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment API requests
    pub fn record_api_request(&self, success: bool) {
        self.api_requests.fetch_add(1, Ordering::Relaxed);
        if !success {
            self.api_errors.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Get P2P metrics summary
    pub fn p2p_summary(&self) -> P2pMetricsSummary {
        P2pMetricsSummary {
            peers_connected: self.p2p_peers_connected.load(Ordering::Relaxed),
            peers_total: self.p2p_peers_total.load(Ordering::Relaxed),
            messages_published: self.p2p_messages_published.load(Ordering::Relaxed),
            messages_received: self.p2p_messages_received.load(Ordering::Relaxed),
            messages_dropped: self.p2p_messages_dropped.load(Ordering::Relaxed),
            peer_bans: self.p2p_peer_bans.load(Ordering::Relaxed),
            rate_limit_hits: self.p2p_rate_limit_hits.load(Ordering::Relaxed),
        }
    }

    /// Set node ID for metric labeling
    pub fn set_node_id(&self, id: String) {
        *self.node_id.write() = Some(id);
    }

    /// Record a request with timing
    pub fn record_request(&self, success: bool, latency: Duration) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);

        if success {
            self.successful_requests.fetch_add(1, Ordering::Relaxed);
        } else {
            self.failed_requests.fetch_add(1, Ordering::Relaxed);
        }

        let latency_us = latency.as_micros() as u64;
        self.total_latency_us.fetch_add(latency_us, Ordering::Relaxed);
        self.latency_count.fetch_add(1, Ordering::Relaxed);

        let latency_ms = latency.as_millis() as u64;
        for (i, &bucket) in LATENCY_BUCKETS.iter().enumerate() {
            if latency_ms <= bucket {
                self.latency_histogram[i].fetch_add(1, Ordering::Relaxed);
                break;
            }
        }
    }

    /// Record bytes sent
    pub fn record_bytes_sent(&self, bytes: u64) {
        self.bytes_sent.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Record bytes received
    pub fn record_bytes_received(&self, bytes: u64) {
        self.bytes_received.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Increment active connections
    pub fn connection_opened(&self) {
        self.active_connections.fetch_add(1, Ordering::Relaxed);
        self.total_connections.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrement active connections
    pub fn connection_closed(&self) {
        self.active_connections.fetch_sub(1, Ordering::Relaxed);
    }

    /// Record uptime sample
    pub fn record_uptime_sample(&self, is_up: bool) {
        let mut samples = self.uptime_samples.write();
        samples.push(is_up);
        if samples.len() > MAX_WINDOW_SAMPLES {
            samples.remove(0);
        }
    }

    /// Record service metrics
    pub fn record_service_request(&self, service: &str, success: bool, latency: Duration) {
        let mut services = self.service_metrics.write();
        let metrics = services.entry(service.to_string()).or_insert_with(|| {
            ServiceMetrics {
                name: service.to_string(),
                ..Default::default()
            }
        });
        metrics.requests += 1;
        if !success {
            metrics.errors += 1;
        }
        metrics.total_latency_us += latency.as_micros() as u64;
    }

    /// Update service status
    pub fn update_service_status(&self, service: &str, running: bool, restarts: u32, uptime_secs: u64) {
        let mut services = self.service_metrics.write();
        let metrics = services.entry(service.to_string()).or_insert_with(|| {
            ServiceMetrics {
                name: service.to_string(),
                ..Default::default()
            }
        });
        metrics.running = running;
        metrics.restarts = restarts;
        metrics.uptime_secs = uptime_secs;
    }

    /// Get total requests
    pub fn total_requests(&self) -> u64 {
        self.total_requests.load(Ordering::Relaxed)
    }

    /// Get successful requests
    pub fn successful_requests(&self) -> u64 {
        self.successful_requests.load(Ordering::Relaxed)
    }

    /// Get failed requests
    pub fn failed_requests(&self) -> u64 {
        self.failed_requests.load(Ordering::Relaxed)
    }

    /// Get active connections
    pub fn active_connections(&self) -> u64 {
        self.active_connections.load(Ordering::Relaxed)
    }

    /// Get bytes sent
    pub fn bytes_sent(&self) -> u64 {
        self.bytes_sent.load(Ordering::Relaxed)
    }

    /// Get bytes received
    pub fn bytes_received(&self) -> u64 {
        self.bytes_received.load(Ordering::Relaxed)
    }

    /// Get uptime in seconds
    pub fn uptime_secs(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    /// Get average latency in milliseconds
    pub fn average_latency_ms(&self) -> f64 {
        let count = self.latency_count.load(Ordering::Relaxed);
        if count == 0 {
            return 0.0;
        }
        let total_us = self.total_latency_us.load(Ordering::Relaxed);
        (total_us as f64 / count as f64) / 1000.0
    }

    /// Calculate quality score
    pub fn quality_score(&self) -> QualityScore {
        let total = self.total_requests.load(Ordering::Relaxed);
        let successful = self.successful_requests.load(Ordering::Relaxed);

        let success_rate = if total > 0 {
            successful as f64 / total as f64
        } else {
            1.0
        };

        let avg_latency = self.average_latency_ms();
        let latency_score = if avg_latency < 100.0 {
            1.0
        } else if avg_latency > 1000.0 {
            0.0
        } else {
            1.0 - ((avg_latency - 100.0) / 900.0)
        };

        let samples = self.uptime_samples.read();
        let uptime = if samples.is_empty() {
            1.0
        } else {
            let up_count = samples.iter().filter(|&&x| x).count();
            up_count as f64 / samples.len() as f64
        };

        let reliability = success_rate;

        QualityScore {
            uptime,
            success_rate,
            latency_score,
            reliability,
        }
    }

    /// Update system metrics (CPU, memory)
    pub fn update_system_metrics(&self) {
        let mut last_update = self.last_system_update.write();
        if last_update.elapsed() < Duration::from_secs(5) {
            return;
        }
        *last_update = Instant::now();

        let mut system = self.system.write();
        system.refresh_process(self.pid);

        if let Some(process) = system.process(self.pid) {
            let cpu = (process.cpu_usage() * 100.0) as u64;
            self.cached_cpu.store(cpu, Ordering::Relaxed);
            self.cached_memory.store(process.memory(), Ordering::Relaxed);
        }
    }

    /// Get CPU usage percentage (0-10000 = 0-100.00%)
    pub fn cpu_usage(&self) -> f64 {
        self.cached_cpu.load(Ordering::Relaxed) as f64 / 100.0
    }

    /// Get memory usage in bytes
    pub fn memory_usage(&self) -> u64 {
        self.cached_memory.load(Ordering::Relaxed)
    }

    /// Reset all metrics
    pub fn reset(&self) {
        self.total_requests.store(0, Ordering::Relaxed);
        self.successful_requests.store(0, Ordering::Relaxed);
        self.failed_requests.store(0, Ordering::Relaxed);
        self.total_latency_us.store(0, Ordering::Relaxed);
        self.latency_count.store(0, Ordering::Relaxed);
        self.bytes_sent.store(0, Ordering::Relaxed);
        self.bytes_received.store(0, Ordering::Relaxed);

        for bucket in &self.latency_histogram {
            bucket.store(0, Ordering::Relaxed);
        }

        self.service_metrics.write().clear();
        self.uptime_samples.write().clear();
    }

    /// Flush metrics (for shutdown)
    pub fn flush(&self) {
        debug!(
            "Flushing metrics: total={}, successful={}, uptime={}s",
            self.total_requests(),
            self.successful_requests(),
            self.uptime_secs()
        );
    }

    /// Get metrics summary
    pub fn summary(&self) -> MetricsSummary {
        let quality = self.quality_score();

        MetricsSummary {
            total_requests: self.total_requests(),
            successful_requests: self.successful_requests(),
            failed_requests: self.failed_requests(),
            average_latency_ms: self.average_latency_ms(),
            active_connections: self.active_connections(),
            bytes_sent: self.bytes_sent(),
            bytes_received: self.bytes_received(),
            uptime_secs: self.uptime_secs(),
            quality_score: quality.total(),
            cpu_usage: self.cpu_usage(),
            memory_bytes: self.memory_usage(),
        }
    }

    /// Get service metrics
    pub fn service_metrics(&self) -> HashMap<String, ServiceMetrics> {
        self.service_metrics.read().clone()
    }

    /// Get latency histogram buckets
    pub fn latency_histogram(&self) -> Vec<(u64, u64)> {
        LATENCY_BUCKETS
            .iter()
            .zip(self.latency_histogram.iter())
            .map(|(&bucket, count)| (bucket, count.load(Ordering::Relaxed)))
            .collect()
    }
}

impl Default for NodeMetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Metrics summary for API responses
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct MetricsSummary {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub average_latency_ms: f64,
    pub active_connections: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub uptime_secs: u64,
    pub quality_score: f64,
    pub cpu_usage: f64,
    pub memory_bytes: u64,
}

/// P2P metrics summary
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct P2pMetricsSummary {
    pub peers_connected: u64,
    pub peers_total: u64,
    pub messages_published: u64,
    pub messages_received: u64,
    pub messages_dropped: u64,
    pub peer_bans: u64,
    pub rate_limit_hits: u64,
}

/// Prometheus-compatible metrics exporter
pub struct PrometheusExporter {
    collector: Arc<NodeMetricsCollector>,
}

impl PrometheusExporter {
    /// Create new exporter
    pub fn new(collector: Arc<NodeMetricsCollector>) -> Self {
        Self { collector }
    }

    /// Export metrics in Prometheus format
    pub fn export(&self) -> String {
        self.collector.update_system_metrics();

        let summary = self.collector.summary();
        let quality = self.collector.quality_score();
        let p2p = self.collector.p2p_summary();
        let node_id = self.collector.node_id.read().clone().unwrap_or_else(|| "unknown".to_string());
        let node_role = self.collector.node_role.read().clone().unwrap_or_else(|| "local".to_string());
        let services = self.collector.service_metrics();
        let histogram = self.collector.latency_histogram();

        let mut output = String::with_capacity(12288);

        output.push_str("# HELP nonos_info Node information\n");
        output.push_str("# TYPE nonos_info gauge\n");
        output.push_str(&format!("nonos_info{{node_id=\"{}\",version=\"{}\",role=\"{}\"}} 1\n\n",
            node_id, env!("CARGO_PKG_VERSION"), node_role));

        output.push_str("# HELP nonos_node_uptime_seconds Node uptime in seconds\n");
        output.push_str("# TYPE nonos_node_uptime_seconds gauge\n");
        output.push_str(&format!("nonos_node_uptime_seconds {}\n\n", summary.uptime_secs));

        output.push_str("# HELP nonos_node_role Node role indicator\n");
        output.push_str("# TYPE nonos_node_role gauge\n");
        output.push_str(&format!("nonos_node_role{{role=\"{}\"}} 1\n\n", node_role));

        output.push_str("# HELP nonos_requests_total Total requests handled\n");
        output.push_str("# TYPE nonos_requests_total counter\n");
        output.push_str(&format!("nonos_requests_total {}\n\n", summary.total_requests));

        output.push_str("# HELP nonos_requests_success_total Successful requests\n");
        output.push_str("# TYPE nonos_requests_success_total counter\n");
        output.push_str(&format!("nonos_requests_success_total {}\n\n", summary.successful_requests));

        output.push_str("# HELP nonos_requests_failed_total Failed requests\n");
        output.push_str("# TYPE nonos_requests_failed_total counter\n");
        output.push_str(&format!("nonos_requests_failed_total {}\n\n", summary.failed_requests));

        output.push_str("# HELP nonos_request_duration_seconds Request latency histogram\n");
        output.push_str("# TYPE nonos_request_duration_seconds histogram\n");
        let mut cumulative: u64 = 0;
        for (bucket_ms, count) in &histogram {
            cumulative += count;
            let bucket_secs = *bucket_ms as f64 / 1000.0;
            output.push_str(&format!("nonos_request_duration_seconds_bucket{{le=\"{:.3}\"}} {}\n",
                bucket_secs, cumulative));
        }
        output.push_str(&format!("nonos_request_duration_seconds_bucket{{le=\"+Inf\"}} {}\n",
            self.collector.latency_count.load(Ordering::Relaxed)));
        output.push_str(&format!("nonos_request_duration_seconds_sum {:.6}\n",
            self.collector.total_latency_us.load(Ordering::Relaxed) as f64 / 1_000_000.0));
        output.push_str(&format!("nonos_request_duration_seconds_count {}\n\n",
            self.collector.latency_count.load(Ordering::Relaxed)));

        output.push_str("# HELP nonos_connections_active Current active connections\n");
        output.push_str("# TYPE nonos_connections_active gauge\n");
        output.push_str(&format!("nonos_connections_active {}\n\n", summary.active_connections));

        output.push_str("# HELP nonos_connections_total Total connections ever established\n");
        output.push_str("# TYPE nonos_connections_total counter\n");
        output.push_str(&format!("nonos_connections_total {}\n\n",
            self.collector.total_connections.load(Ordering::Relaxed)));

        output.push_str("# HELP nonos_bytes_sent_total Total bytes sent\n");
        output.push_str("# TYPE nonos_bytes_sent_total counter\n");
        output.push_str(&format!("nonos_bytes_sent_total {}\n\n", summary.bytes_sent));

        output.push_str("# HELP nonos_bytes_received_total Total bytes received\n");
        output.push_str("# TYPE nonos_bytes_received_total counter\n");
        output.push_str(&format!("nonos_bytes_received_total {}\n\n", summary.bytes_received));

        output.push_str("# HELP nonos_uptime_seconds Node uptime in seconds\n");
        output.push_str("# TYPE nonos_uptime_seconds gauge\n");
        output.push_str(&format!("nonos_uptime_seconds {}\n\n", summary.uptime_secs));

        output.push_str("# HELP nonos_quality_score Overall quality score (0-1)\n");
        output.push_str("# TYPE nonos_quality_score gauge\n");
        output.push_str(&format!("nonos_quality_score {:.4}\n\n", summary.quality_score));

        output.push_str("# HELP nonos_quality_uptime Uptime component of quality score\n");
        output.push_str("# TYPE nonos_quality_uptime gauge\n");
        output.push_str(&format!("nonos_quality_uptime {:.4}\n\n", quality.uptime));

        output.push_str("# HELP nonos_quality_success_rate Success rate component\n");
        output.push_str("# TYPE nonos_quality_success_rate gauge\n");
        output.push_str(&format!("nonos_quality_success_rate {:.4}\n\n", quality.success_rate));

        output.push_str("# HELP nonos_quality_latency Latency score component\n");
        output.push_str("# TYPE nonos_quality_latency gauge\n");
        output.push_str(&format!("nonos_quality_latency {:.4}\n\n", quality.latency_score));

        output.push_str("# HELP nonos_quality_reliability Reliability component\n");
        output.push_str("# TYPE nonos_quality_reliability gauge\n");
        output.push_str(&format!("nonos_quality_reliability {:.4}\n\n", quality.reliability));

        output.push_str("# HELP nonos_cpu_usage_percent CPU usage percentage\n");
        output.push_str("# TYPE nonos_cpu_usage_percent gauge\n");
        output.push_str(&format!("nonos_cpu_usage_percent {:.2}\n\n", summary.cpu_usage));

        output.push_str("# HELP nonos_memory_bytes Memory usage in bytes\n");
        output.push_str("# TYPE nonos_memory_bytes gauge\n");
        output.push_str(&format!("nonos_memory_bytes {}\n\n", summary.memory_bytes));

        // P2P metrics
        output.push_str("# HELP nonos_p2p_peers_connected Currently connected peers\n");
        output.push_str("# TYPE nonos_p2p_peers_connected gauge\n");
        output.push_str(&format!("nonos_p2p_peers_connected {}\n\n", p2p.peers_connected));

        output.push_str("# HELP nonos_p2p_peers_total Total known peers\n");
        output.push_str("# TYPE nonos_p2p_peers_total gauge\n");
        output.push_str(&format!("nonos_p2p_peers_total {}\n\n", p2p.peers_total));

        output.push_str("# HELP nonos_p2p_messages_total Total P2P messages (published + received)\n");
        output.push_str("# TYPE nonos_p2p_messages_total counter\n");
        output.push_str(&format!("nonos_p2p_messages_total{{direction=\"published\"}} {}\n", p2p.messages_published));
        output.push_str(&format!("nonos_p2p_messages_total{{direction=\"received\"}} {}\n\n", p2p.messages_received));

        output.push_str("# HELP nonos_p2p_messages_dropped_total P2P messages dropped\n");
        output.push_str("# TYPE nonos_p2p_messages_dropped_total counter\n");
        output.push_str(&format!("nonos_p2p_messages_dropped_total {}\n\n", p2p.messages_dropped));

        output.push_str("# HELP nonos_p2p_peer_bans_total Total peer bans\n");
        output.push_str("# TYPE nonos_p2p_peer_bans_total counter\n");
        output.push_str(&format!("nonos_p2p_peer_bans_total {}\n\n", p2p.peer_bans));

        output.push_str("# HELP nonos_p2p_rate_limit_hits_total Rate limit hits\n");
        output.push_str("# TYPE nonos_p2p_rate_limit_hits_total counter\n");
        output.push_str(&format!("nonos_p2p_rate_limit_hits_total {}\n\n", p2p.rate_limit_hits));

        // API metrics
        output.push_str("# HELP nonos_api_requests_total Total API requests\n");
        output.push_str("# TYPE nonos_api_requests_total counter\n");
        output.push_str(&format!("nonos_api_requests_total {}\n\n", self.collector.api_requests.load(Ordering::Relaxed)));

        output.push_str("# HELP nonos_api_requests_errors_total Total API errors\n");
        output.push_str("# TYPE nonos_api_requests_errors_total counter\n");
        output.push_str(&format!("nonos_api_requests_errors_total {}\n\n", self.collector.api_errors.load(Ordering::Relaxed)));

        if !services.is_empty() {
            output.push_str("# HELP nonos_service_requests_total Requests per service\n");
            output.push_str("# TYPE nonos_service_requests_total counter\n");
            for (name, metrics) in &services {
                output.push_str(&format!("nonos_service_requests_total{{service=\"{}\"}} {}\n",
                    name, metrics.requests));
            }
            output.push('\n');

            output.push_str("# HELP nonos_service_errors_total Errors per service\n");
            output.push_str("# TYPE nonos_service_errors_total counter\n");
            for (name, metrics) in &services {
                output.push_str(&format!("nonos_service_errors_total{{service=\"{}\"}} {}\n",
                    name, metrics.errors));
            }
            output.push('\n');

            output.push_str("# HELP nonos_service_running Service running status (1=running, 0=stopped)\n");
            output.push_str("# TYPE nonos_service_running gauge\n");
            for (name, metrics) in &services {
                output.push_str(&format!("nonos_service_running{{service=\"{}\"}} {}\n",
                    name, if metrics.running { 1 } else { 0 }));
            }
            output.push('\n');

            output.push_str("# HELP nonos_service_restarts_total Total restarts per service\n");
            output.push_str("# TYPE nonos_service_restarts_total counter\n");
            for (name, metrics) in &services {
                output.push_str(&format!("nonos_service_restarts_total{{service=\"{}\"}} {}\n",
                    name, metrics.restarts));
            }
            output.push('\n');

            output.push_str("# HELP nonos_service_uptime_seconds Uptime per service\n");
            output.push_str("# TYPE nonos_service_uptime_seconds gauge\n");
            for (name, metrics) in &services {
                output.push_str(&format!("nonos_service_uptime_seconds{{service=\"{}\"}} {}\n",
                    name, metrics.uptime_secs));
            }
            output.push('\n');
        }

        output
    }
}

/// Request timer for measuring latency
pub struct RequestTimer {
    start: Instant,
    collector: Arc<NodeMetricsCollector>,
    service: Option<String>,
    recorded: AtomicBool,
}

impl RequestTimer {
    /// Start a new request timer
    pub fn new(collector: Arc<NodeMetricsCollector>) -> Self {
        Self {
            start: Instant::now(),
            collector,
            service: None,
            recorded: AtomicBool::new(false),
        }
    }

    /// Start a timer for a specific service
    pub fn for_service(collector: Arc<NodeMetricsCollector>, service: &str) -> Self {
        Self {
            start: Instant::now(),
            collector,
            service: Some(service.to_string()),
            recorded: AtomicBool::new(false),
        }
    }

    /// Record the request as successful
    pub fn success(self) {
        if self.recorded.swap(true, Ordering::SeqCst) {
            return;
        }
        let duration = self.start.elapsed();
        self.collector.record_request(true, duration);
        if let Some(ref service) = self.service {
            self.collector.record_service_request(service, true, duration);
        }
    }

    /// Record the request as failed
    pub fn failure(self) {
        if self.recorded.swap(true, Ordering::SeqCst) {
            return;
        }
        let duration = self.start.elapsed();
        self.collector.record_request(false, duration);
        if let Some(ref service) = self.service {
            self.collector.record_service_request(service, false, duration);
        }
    }

    /// Get elapsed time without recording
    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }
}

impl Drop for RequestTimer {
    fn drop(&mut self) {
        if !self.recorded.load(Ordering::SeqCst) {
            let duration = self.start.elapsed();
            self.collector.record_request(false, duration);
            if let Some(ref service) = self.service {
                self.collector.record_service_request(service, false, duration);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atomic_metrics_collection() {
        let collector = NodeMetricsCollector::new();

        collector.record_request(true, Duration::from_millis(50));
        collector.record_request(true, Duration::from_millis(100));
        collector.record_request(false, Duration::from_millis(500));

        assert_eq!(collector.total_requests(), 3);
        assert_eq!(collector.successful_requests(), 2);
        assert_eq!(collector.failed_requests(), 1);
    }

    #[test]
    fn test_quality_score_calculation() {
        let collector = NodeMetricsCollector::new();

        for _ in 0..9 {
            collector.record_request(true, Duration::from_millis(50));
        }
        collector.record_request(false, Duration::from_millis(50));

        let quality = collector.quality_score();
        assert!((quality.success_rate - 0.9).abs() < 0.01);
        assert!(quality.latency_score > 0.9);
    }

    #[test]
    fn test_uptime_tracking() {
        let collector = NodeMetricsCollector::new();

        for _ in 0..9 {
            collector.record_uptime_sample(true);
        }
        collector.record_uptime_sample(false);

        let quality = collector.quality_score();
        assert!((quality.uptime - 0.9).abs() < 0.01);
    }

    #[test]
    fn test_prometheus_export() {
        let collector = Arc::new(NodeMetricsCollector::new());
        collector.set_node_id("test-node-123".to_string());
        collector.record_request(true, Duration::from_millis(100));

        let exporter = PrometheusExporter::new(collector);
        let output = exporter.export();

        assert!(output.contains("nonos_requests_total 1"));
        assert!(output.contains("nonos_info"));
        assert!(output.contains("test-node-123"));
    }

    #[test]
    fn test_latency_histogram() {
        let collector = NodeMetricsCollector::new();

        collector.record_request(true, Duration::from_millis(5));
        collector.record_request(true, Duration::from_millis(50));
        collector.record_request(true, Duration::from_millis(500));
        collector.record_request(true, Duration::from_millis(5000));

        let histogram = collector.latency_histogram();
        assert!(!histogram.is_empty());
    }

    #[test]
    fn test_service_metrics() {
        let collector = NodeMetricsCollector::new();

        collector.record_service_request("health_beacon", true, Duration::from_millis(10));
        collector.record_service_request("health_beacon", true, Duration::from_millis(15));
        collector.record_service_request("quality_oracle", false, Duration::from_millis(100));

        let services = collector.service_metrics();
        assert_eq!(services.get("health_beacon").unwrap().requests, 2);
        assert_eq!(services.get("quality_oracle").unwrap().errors, 1);
    }

    #[test]
    fn test_request_timer() {
        let collector = Arc::new(NodeMetricsCollector::new());

        {
            let timer = RequestTimer::new(collector.clone());
            std::thread::sleep(Duration::from_millis(10));
            timer.success();
        }

        assert_eq!(collector.total_requests(), 1);
        assert_eq!(collector.successful_requests(), 1);
    }

    #[test]
    fn test_bytes_tracking() {
        let collector = NodeMetricsCollector::new();

        collector.record_bytes_sent(1000);
        collector.record_bytes_received(2000);

        assert_eq!(collector.bytes_sent(), 1000);
        assert_eq!(collector.bytes_received(), 2000);
    }

    #[test]
    fn test_connection_tracking() {
        let collector = NodeMetricsCollector::new();

        collector.connection_opened();
        collector.connection_opened();
        assert_eq!(collector.active_connections(), 2);

        collector.connection_closed();
        assert_eq!(collector.active_connections(), 1);
    }
}
