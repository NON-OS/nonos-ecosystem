use crate::{NodeMetricsCollector, NodeStorage};
use nonos_types::{NodeId, NonosResult};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use tracing::{debug, info, warn};

pub struct QualityOracle {
    node_id: NodeId,
    metrics: Arc<NodeMetricsCollector>,
    storage: Arc<NodeStorage>,
}

impl QualityOracle {
    pub fn new(
        node_id: NodeId,
        metrics: Arc<NodeMetricsCollector>,
        storage: Arc<NodeStorage>,
    ) -> Self {
        Self { node_id, metrics, storage }
    }

    pub async fn run(&self, shutdown: Arc<AtomicBool>) -> NonosResult<()> {
        let mut ticker = interval(Duration::from_secs(300));
        info!("Quality oracle running for node {}", self.node_id);

        loop {
            if shutdown.load(Ordering::SeqCst) {
                info!("Quality oracle shutting down");
                break;
            }

            ticker.tick().await;

            if let Err(e) = self.record_quality().await {
                warn!("Failed to record quality: {}", e);
            }
        }

        Ok(())
    }

    async fn record_quality(&self) -> NonosResult<()> {
        let quality = self.metrics.quality_score();
        let summary = self.metrics.summary();

        self.storage.record_metrics(
            summary.total_requests,
            summary.successful_requests,
            quality.total(),
        ).await?;

        debug!("Recorded quality: score={:.4}, requests={}", quality.total(), summary.total_requests);
        Ok(())
    }
}
