use std::sync::atomic::Ordering;
use std::sync::Arc;

use super::collector::NodeMetricsCollector;

pub struct PrometheusExporter {
    collector: Arc<NodeMetricsCollector>,
}

impl PrometheusExporter {
    pub fn new(collector: Arc<NodeMetricsCollector>) -> Self {
        Self { collector }
    }

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

        self.write_info_metrics(&mut output, &node_id, &node_role, &summary);
        self.write_request_metrics(&mut output, &summary, &histogram);
        self.write_connection_metrics(&mut output, &summary);
        self.write_quality_metrics(&mut output, &summary, &quality);
        self.write_system_metrics(&mut output, &summary);
        self.write_p2p_metrics(&mut output, &p2p);
        self.write_api_metrics(&mut output);
        self.write_service_metrics(&mut output, &services);

        output
    }

    fn write_info_metrics(&self, output: &mut String, node_id: &str, node_role: &str, summary: &super::types::MetricsSummary) {
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
    }

    fn write_request_metrics(&self, output: &mut String, summary: &super::types::MetricsSummary, histogram: &[(u64, u64)]) {
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
        for (bucket_ms, count) in histogram {
            cumulative += count;
            let bucket_secs = *bucket_ms as f64 / 1000.0;
            output.push_str(&format!("nonos_request_duration_seconds_bucket{{le=\"{:.3}\"}} {}\n",
                bucket_secs, cumulative));
        }
        output.push_str(&format!("nonos_request_duration_seconds_bucket{{le=\"+Inf\"}} {}\n",
            self.collector.latency_count()));
        output.push_str(&format!("nonos_request_duration_seconds_sum {:.6}\n",
            self.collector.total_latency_us() as f64 / 1_000_000.0));
        output.push_str(&format!("nonos_request_duration_seconds_count {}\n\n",
            self.collector.latency_count()));
    }

    fn write_connection_metrics(&self, output: &mut String, summary: &super::types::MetricsSummary) {
        output.push_str("# HELP nonos_connections_active Current active connections\n");
        output.push_str("# TYPE nonos_connections_active gauge\n");
        output.push_str(&format!("nonos_connections_active {}\n\n", summary.active_connections));

        output.push_str("# HELP nonos_connections_total Total connections ever established\n");
        output.push_str("# TYPE nonos_connections_total counter\n");
        output.push_str(&format!("nonos_connections_total {}\n\n", self.collector.total_connections()));

        output.push_str("# HELP nonos_bytes_sent_total Total bytes sent\n");
        output.push_str("# TYPE nonos_bytes_sent_total counter\n");
        output.push_str(&format!("nonos_bytes_sent_total {}\n\n", summary.bytes_sent));

        output.push_str("# HELP nonos_bytes_received_total Total bytes received\n");
        output.push_str("# TYPE nonos_bytes_received_total counter\n");
        output.push_str(&format!("nonos_bytes_received_total {}\n\n", summary.bytes_received));

        output.push_str("# HELP nonos_uptime_seconds Node uptime in seconds\n");
        output.push_str("# TYPE nonos_uptime_seconds gauge\n");
        output.push_str(&format!("nonos_uptime_seconds {}\n\n", summary.uptime_secs));
    }

    fn write_quality_metrics(&self, output: &mut String, summary: &super::types::MetricsSummary, quality: &nonos_types::QualityScore) {
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
    }

    fn write_system_metrics(&self, output: &mut String, summary: &super::types::MetricsSummary) {
        output.push_str("# HELP nonos_cpu_usage_percent CPU usage percentage\n");
        output.push_str("# TYPE nonos_cpu_usage_percent gauge\n");
        output.push_str(&format!("nonos_cpu_usage_percent {:.2}\n\n", summary.cpu_usage));

        output.push_str("# HELP nonos_memory_bytes Memory usage in bytes\n");
        output.push_str("# TYPE nonos_memory_bytes gauge\n");
        output.push_str(&format!("nonos_memory_bytes {}\n\n", summary.memory_bytes));
    }

    fn write_p2p_metrics(&self, output: &mut String, p2p: &super::types::P2pMetricsSummary) {
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
    }

    fn write_api_metrics(&self, output: &mut String) {
        output.push_str("# HELP nonos_api_requests_total Total API requests\n");
        output.push_str("# TYPE nonos_api_requests_total counter\n");
        output.push_str(&format!("nonos_api_requests_total {}\n\n", self.collector.api_requests.load(Ordering::Relaxed)));

        output.push_str("# HELP nonos_api_requests_errors_total Total API errors\n");
        output.push_str("# TYPE nonos_api_requests_errors_total counter\n");
        output.push_str(&format!("nonos_api_requests_errors_total {}\n\n", self.collector.api_errors.load(Ordering::Relaxed)));
    }

    fn write_service_metrics(&self, output: &mut String, services: &std::collections::HashMap<String, super::types::ServiceMetrics>) {
        if services.is_empty() {
            return;
        }

        output.push_str("# HELP nonos_service_requests_total Requests per service\n");
        output.push_str("# TYPE nonos_service_requests_total counter\n");
        for (name, metrics) in services {
            output.push_str(&format!("nonos_service_requests_total{{service=\"{}\"}} {}\n",
                name, metrics.requests));
        }
        output.push('\n');

        output.push_str("# HELP nonos_service_errors_total Errors per service\n");
        output.push_str("# TYPE nonos_service_errors_total counter\n");
        for (name, metrics) in services {
            output.push_str(&format!("nonos_service_errors_total{{service=\"{}\"}} {}\n",
                name, metrics.errors));
        }
        output.push('\n');

        output.push_str("# HELP nonos_service_running Service running status (1=running, 0=stopped)\n");
        output.push_str("# TYPE nonos_service_running gauge\n");
        for (name, metrics) in services {
            output.push_str(&format!("nonos_service_running{{service=\"{}\"}} {}\n",
                name, if metrics.running { 1 } else { 0 }));
        }
        output.push('\n');

        output.push_str("# HELP nonos_service_restarts_total Total restarts per service\n");
        output.push_str("# TYPE nonos_service_restarts_total counter\n");
        for (name, metrics) in services {
            output.push_str(&format!("nonos_service_restarts_total{{service=\"{}\"}} {}\n",
                name, metrics.restarts));
        }
        output.push('\n');

        output.push_str("# HELP nonos_service_uptime_seconds Uptime per service\n");
        output.push_str("# TYPE nonos_service_uptime_seconds gauge\n");
        for (name, metrics) in services {
            output.push_str(&format!("nonos_service_uptime_seconds{{service=\"{}\"}} {}\n",
                name, metrics.uptime_secs));
        }
        output.push('\n');
    }
}
