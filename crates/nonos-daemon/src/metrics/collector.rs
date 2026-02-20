use nonos_types::QualityScore;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use sysinfo::{Pid, System};
use tracing::debug;

use super::types::{MetricsSummary, P2pMetricsSummary, ServiceMetrics, WorkMetrics};
use super::work_state::WorkState;

const LATENCY_BUCKETS: &[u64] = &[1, 5, 10, 25, 50, 100, 250, 500, 1000, 2500, 5000, 10000];
const MAX_WINDOW_SAMPLES: usize = 1000;

pub struct NodeMetricsCollector {
    total_requests: AtomicU64,
    successful_requests: AtomicU64,
    failed_requests: AtomicU64,
    total_latency_us: AtomicU64,
    latency_count: AtomicU64,
    latency_histogram: Vec<AtomicU64>,
    bytes_sent: AtomicU64,
    bytes_received: AtomicU64,
    active_connections: AtomicU64,
    total_connections: AtomicU64,
    service_metrics: RwLock<HashMap<String, ServiceMetrics>>,
    uptime_samples: RwLock<Vec<bool>>,
    start_time: Instant,
    pub(crate) node_id: RwLock<Option<String>>,
    pub(crate) node_role: RwLock<Option<String>>,
    system: RwLock<System>,
    pid: Pid,
    last_system_update: RwLock<Instant>,
    cached_cpu: AtomicU64,
    cached_memory: AtomicU64,
    p2p_peers_connected: AtomicU64,
    p2p_peers_total: AtomicU64,
    p2p_messages_published: AtomicU64,
    p2p_messages_received: AtomicU64,
    p2p_messages_dropped: AtomicU64,
    p2p_peer_bans: AtomicU64,
    p2p_rate_limit_hits: AtomicU64,
    pub(crate) api_requests: AtomicU64,
    pub(crate) api_errors: AtomicU64,
    work: WorkState,
}

impl NodeMetricsCollector {
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
            p2p_peers_connected: AtomicU64::new(0),
            p2p_peers_total: AtomicU64::new(0),
            p2p_messages_published: AtomicU64::new(0),
            p2p_messages_received: AtomicU64::new(0),
            p2p_messages_dropped: AtomicU64::new(0),
            p2p_peer_bans: AtomicU64::new(0),
            p2p_rate_limit_hits: AtomicU64::new(0),
            api_requests: AtomicU64::new(0),
            api_errors: AtomicU64::new(0),
            work: WorkState::new(),
        }
    }

    pub fn set_node_role(&self, role: &str) {
        *self.node_role.write() = Some(role.to_string());
    }

    pub fn set_node_id(&self, id: String) {
        *self.node_id.write() = Some(id);
    }

    pub fn set_p2p_peers_connected(&self, count: u64) {
        self.p2p_peers_connected.store(count, Ordering::Relaxed);
    }

    pub fn set_p2p_peers_total(&self, count: u64) {
        self.p2p_peers_total.store(count, Ordering::Relaxed);
    }

    pub fn record_p2p_message_published(&self) {
        self.p2p_messages_published.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_p2p_message_received(&self) {
        self.p2p_messages_received.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_p2p_message_dropped(&self) {
        self.p2p_messages_dropped.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_p2p_peer_ban(&self) {
        self.p2p_peer_bans.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_p2p_rate_limit_hit(&self) {
        self.p2p_rate_limit_hits.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_api_request(&self, success: bool) {
        self.api_requests.fetch_add(1, Ordering::Relaxed);
        if !success {
            self.api_errors.fetch_add(1, Ordering::Relaxed);
        }
    }

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

    pub fn record_bytes_sent(&self, bytes: u64) {
        self.bytes_sent.fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn record_bytes_received(&self, bytes: u64) {
        self.bytes_received.fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn connection_opened(&self) {
        self.active_connections.fetch_add(1, Ordering::Relaxed);
        self.total_connections.fetch_add(1, Ordering::Relaxed);
    }

    pub fn connection_closed(&self) {
        self.active_connections.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn record_uptime_sample(&self, is_up: bool) {
        let mut samples = self.uptime_samples.write();
        samples.push(is_up);
        if samples.len() > MAX_WINDOW_SAMPLES {
            samples.remove(0);
        }
    }

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

    pub fn total_requests(&self) -> u64 {
        self.total_requests.load(Ordering::Relaxed)
    }

    pub fn successful_requests(&self) -> u64 {
        self.successful_requests.load(Ordering::Relaxed)
    }

    pub fn failed_requests(&self) -> u64 {
        self.failed_requests.load(Ordering::Relaxed)
    }

    pub fn active_connections(&self) -> u64 {
        self.active_connections.load(Ordering::Relaxed)
    }

    pub fn bytes_sent(&self) -> u64 {
        self.bytes_sent.load(Ordering::Relaxed)
    }

    pub fn bytes_received(&self) -> u64 {
        self.bytes_received.load(Ordering::Relaxed)
    }

    pub fn uptime_secs(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    pub fn average_latency_ms(&self) -> f64 {
        let count = self.latency_count.load(Ordering::Relaxed);
        if count == 0 {
            return 0.0;
        }
        let total_us = self.total_latency_us.load(Ordering::Relaxed);
        (total_us as f64 / count as f64) / 1000.0
    }

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

        QualityScore {
            uptime,
            success_rate,
            latency_score,
            reliability: success_rate,
        }
    }

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

    pub fn cpu_usage(&self) -> f64 {
        self.cached_cpu.load(Ordering::Relaxed) as f64 / 100.0
    }

    pub fn memory_usage(&self) -> u64 {
        self.cached_memory.load(Ordering::Relaxed)
    }

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

    pub fn flush(&self) {
        debug!(
            "Flushing metrics: total={}, successful={}, uptime={}s",
            self.total_requests(),
            self.successful_requests(),
            self.uptime_secs()
        );
    }

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

    pub fn service_metrics(&self) -> HashMap<String, ServiceMetrics> {
        self.service_metrics.read().clone()
    }

    pub fn latency_histogram(&self) -> Vec<(u64, u64)> {
        LATENCY_BUCKETS
            .iter()
            .zip(self.latency_histogram.iter())
            .map(|(&bucket, count)| (bucket, count.load(Ordering::Relaxed)))
            .collect()
    }

    pub fn total_connections(&self) -> u64 {
        self.total_connections.load(Ordering::Relaxed)
    }

    pub fn latency_count(&self) -> u64 {
        self.latency_count.load(Ordering::Relaxed)
    }

    pub fn total_latency_us(&self) -> u64 {
        self.total_latency_us.load(Ordering::Relaxed)
    }

    pub fn record_relay(&self, bytes: u64, success: bool, latency_ms: u64) {
        self.work.record_relay(bytes, success, latency_ms);
    }

    pub fn record_zk_proof_generated(&self, generation_time_ms: u64) {
        self.work.record_zk_proof_generated(generation_time_ms);
    }

    pub fn record_zk_proof_verified(&self, success: bool) {
        self.work.record_zk_proof_verified(success);
    }

    pub fn record_mixer_deposit(&self, value: u128) {
        self.work.record_mixer_deposit(value);
    }

    pub fn record_mixer_spend(&self, value: u128) {
        self.work.record_mixer_spend(value);
    }

    pub fn record_mixer_pool_participation(&self) {
        self.work.record_mixer_pool_participation();
    }

    pub fn record_entropy_contribution(&self, bytes: u64, quality_score: f64) {
        self.work.record_entropy_contribution(bytes, quality_score);
    }

    pub fn record_entropy_request_served(&self) {
        self.work.record_entropy_request_served();
    }

    pub fn record_registry_registration(&self) {
        self.work.record_registry_registration();
    }

    pub fn record_registry_lookup(&self) {
        self.work.record_registry_lookup();
    }

    pub fn record_registry_sync(&self) {
        self.work.record_registry_sync();
    }

    pub fn record_registry_failure(&self) {
        self.work.record_registry_failure();
    }

    pub fn epoch_info(&self) -> super::types::EpochInfo {
        self.work.epoch_info()
    }

    pub fn check_epoch_advance(&self) -> bool {
        self.work.check_epoch_advance()
    }

    pub fn mark_epoch_submitted(&self) {
        self.work.mark_epoch_submitted();
    }

    pub fn reset_work_metrics(&self) {
        self.work.reset();
    }

    pub fn work_summary(&self) -> WorkMetrics {
        self.work.summary()
    }
}

impl Default for NodeMetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}
