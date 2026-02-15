use crate::{NodeMetricsCollector, P2pNetwork};
use nonos_types::{NodeId, NonosError, NonosResult};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{debug, info, warn};

pub struct HealthBeacon {
    node_id: NodeId,
    network: Arc<RwLock<P2pNetwork>>,
    metrics: Arc<NodeMetricsCollector>,
}

impl HealthBeacon {
    pub fn new(
        node_id: NodeId,
        network: Arc<RwLock<P2pNetwork>>,
        metrics: Arc<NodeMetricsCollector>,
    ) -> Self {
        Self { node_id, network, metrics }
    }

    pub async fn run(&self, shutdown: Arc<AtomicBool>) -> NonosResult<()> {
        let mut ticker = interval(Duration::from_secs(60));
        info!("Health beacon running for node {}", self.node_id);

        loop {
            if shutdown.load(Ordering::SeqCst) {
                info!("Health beacon shutting down");
                break;
            }

            ticker.tick().await;

            if let Err(e) = self.broadcast_health().await {
                warn!("Failed to broadcast health: {}", e);
            }
        }

        Ok(())
    }

    async fn broadcast_health(&self) -> NonosResult<()> {
        let quality = self.metrics.quality_score();
        let summary = self.metrics.summary();

        let health = HealthStatus {
            node_id: self.node_id,
            timestamp: chrono::Utc::now(),
            quality_score: quality.total(),
            uptime: quality.uptime,
            success_rate: quality.success_rate,
            total_requests: summary.total_requests,
            peer_count: self.network.read().await.peer_count(),
        };

        let message = serde_json::to_vec(&health)
            .map_err(|e| NonosError::Internal(format!("Serialize error: {}", e)))?;

        self.network.write().await.publish("nonos/health/v1", &message).await?;

        debug!("Broadcast health: quality={:.2}, peers={}", health.quality_score, health.peer_count);
        Ok(())
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct HealthStatus {
    pub node_id: NodeId,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub quality_score: f64,
    pub uptime: f64,
    pub success_rate: f64,
    pub total_requests: u64,
    pub peer_count: usize,
}
