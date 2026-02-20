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

#[test]
fn test_work_metrics_traffic_relay() {
    let collector = NodeMetricsCollector::new();

    collector.record_relay(1000, true, 50);
    collector.record_relay(2000, true, 100);
    collector.record_relay(500, false, 200);

    let work = collector.work_summary();
    assert_eq!(work.traffic_relay.bytes_relayed, 3500);
    assert_eq!(work.traffic_relay.relay_sessions, 3);
    assert_eq!(work.traffic_relay.successful_relays, 2);
    assert_eq!(work.traffic_relay.failed_relays, 1);
}

#[test]
fn test_work_metrics_zk_proofs() {
    let collector = NodeMetricsCollector::new();

    collector.record_zk_proof_generated(100);
    collector.record_zk_proof_generated(150);
    collector.record_zk_proof_verified(true);
    collector.record_zk_proof_verified(false);

    let work = collector.work_summary();
    assert_eq!(work.zk_proofs.proofs_generated, 2);
    assert_eq!(work.zk_proofs.proofs_verified, 2);
    assert_eq!(work.zk_proofs.verification_failures, 1);
    assert!((work.zk_proofs.avg_generation_time_ms - 125.0).abs() < 0.01);
}

#[test]
fn test_work_metrics_mixer() {
    let collector = NodeMetricsCollector::new();

    collector.record_mixer_deposit(1000000);
    collector.record_mixer_deposit(2000000);
    collector.record_mixer_spend(500000);
    collector.record_mixer_pool_participation();

    let work = collector.work_summary();
    assert_eq!(work.mixer_ops.deposits_processed, 2);
    assert_eq!(work.mixer_ops.spends_processed, 1);
    assert_eq!(work.mixer_ops.total_value_mixed, 3500000);
    assert_eq!(work.mixer_ops.pool_participations, 1);
}

#[test]
fn test_work_metrics_entropy() {
    let collector = NodeMetricsCollector::new();

    collector.record_entropy_contribution(1000, 95.0);
    collector.record_entropy_contribution(500, 85.0);
    collector.record_entropy_request_served();

    let work = collector.work_summary();
    assert_eq!(work.entropy.entropy_bytes_contributed, 1500);
    assert_eq!(work.entropy.entropy_requests_served, 1);
    assert!((work.entropy.quality_score - 90.0).abs() < 1.0);
}

#[test]
fn test_work_metrics_registry() {
    let collector = NodeMetricsCollector::new();

    collector.record_registry_registration();
    collector.record_registry_registration();
    collector.record_registry_lookup();
    collector.record_registry_sync();
    collector.record_registry_failure();

    let work = collector.work_summary();
    assert_eq!(work.registry_ops.registrations_processed, 2);
    assert_eq!(work.registry_ops.lookups_served, 1);
    assert_eq!(work.registry_ops.sync_operations, 1);
    assert_eq!(work.registry_ops.failed_operations, 1);
}

#[test]
fn test_work_score_calculation() {
    let collector = NodeMetricsCollector::new();

    collector.record_relay(100_000_000, true, 50);
    collector.record_zk_proof_generated(100);
    collector.record_mixer_deposit(0);
    collector.record_entropy_contribution(1_000_000, 100.0);
    collector.record_registry_registration();

    let work = collector.work_summary();
    assert!(work.total_work_score > 0.0);
    assert!(work.total_work_score <= 100.0);
}

#[test]
fn test_epoch_info() {
    let collector = NodeMetricsCollector::new();

    let epoch = collector.epoch_info();
    assert_eq!(epoch.current_epoch, 0);
    assert!(!epoch.submitted_to_oracle);
    assert!(epoch.epoch_end_timestamp > epoch.epoch_start_timestamp);
}

#[test]
fn test_work_metrics_reset() {
    let collector = NodeMetricsCollector::new();

    collector.record_relay(1000, true, 50);
    collector.record_zk_proof_generated(100);

    let work_before = collector.work_summary();
    assert_eq!(work_before.traffic_relay.bytes_relayed, 1000);

    collector.reset_work_metrics();

    let work_after = collector.work_summary();
    assert_eq!(work_after.traffic_relay.bytes_relayed, 0);
    assert_eq!(work_after.zk_proofs.proofs_generated, 0);
}
