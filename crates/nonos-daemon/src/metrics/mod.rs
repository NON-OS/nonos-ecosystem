mod collector;
mod prometheus;
mod timer;
mod types;

pub use collector::NodeMetricsCollector;
pub use prometheus::PrometheusExporter;
pub use timer::RequestTimer;
pub use types::{MetricsSummary, P2pMetricsSummary, ServiceMetrics};

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::time::Duration;

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
