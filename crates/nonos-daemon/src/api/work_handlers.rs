use super::handlers::send_response;
use crate::metrics::{NodeMetricsCollector, WorkMetrics};
use nonos_types::NonosResult;
use serde::Serialize;
use std::sync::Arc;
use tokio::net::TcpStream;

#[derive(Serialize)]
pub struct WorkMetricsResponse {
    pub success: bool,
    pub data: WorkMetrics,
}

#[derive(Serialize)]
pub struct EpochResponse {
    pub success: bool,
    pub epoch: u64,
    pub epoch_start: u64,
    pub epoch_end: u64,
    pub submitted: bool,
}

pub async fn serve_work_metrics(
    stream: &mut TcpStream,
    metrics: &Arc<NodeMetricsCollector>,
) -> NonosResult<()> {
    let work = metrics.work_summary();
    let response = WorkMetricsResponse {
        success: true,
        data: work,
    };
    let json = serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
    send_response(stream, 200, "application/json", &json).await
}

pub async fn serve_epoch_info(
    stream: &mut TcpStream,
    metrics: &Arc<NodeMetricsCollector>,
) -> NonosResult<()> {
    let epoch = metrics.epoch_info();
    let response = EpochResponse {
        success: true,
        epoch: epoch.current_epoch,
        epoch_start: epoch.epoch_start_timestamp,
        epoch_end: epoch.epoch_end_timestamp,
        submitted: epoch.submitted_to_oracle,
    };
    let json = serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
    send_response(stream, 200, "application/json", &json).await
}

pub async fn check_epoch_advance(
    stream: &mut TcpStream,
    metrics: &Arc<NodeMetricsCollector>,
) -> NonosResult<()> {
    let advanced = metrics.check_epoch_advance();
    let epoch = metrics.epoch_info();
    let response = serde_json::json!({
        "success": true,
        "advanced": advanced,
        "current_epoch": epoch.current_epoch
    });
    let json = response.to_string();
    send_response(stream, 200, "application/json", &json).await
}

pub async fn mark_epoch_submitted(
    stream: &mut TcpStream,
    metrics: &Arc<NodeMetricsCollector>,
) -> NonosResult<()> {
    metrics.mark_epoch_submitted();
    let response = serde_json::json!({
        "success": true,
        "message": "Epoch marked as submitted"
    });
    let json = response.to_string();
    send_response(stream, 200, "application/json", &json).await
}
