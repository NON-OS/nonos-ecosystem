use nonos_crypto::{StealthKeyPair, StealthScanner};
use nonos_types::{NodeId, NonosResult};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{debug, info};

pub struct StealthScannerService {
    _node_id: NodeId,
    scanner: Arc<RwLock<Option<StealthScanner>>>,
    detected_payments: AtomicU64,
    announcements_scanned: AtomicU64,
}

impl StealthScannerService {
    pub fn new(node_id: NodeId) -> Self {
        Self {
            _node_id: node_id,
            scanner: Arc::new(RwLock::new(None)),
            detected_payments: AtomicU64::new(0),
            announcements_scanned: AtomicU64::new(0),
        }
    }

    pub async fn initialize(&self, keypair: StealthKeyPair) -> NonosResult<String> {
        let meta_address = keypair.meta_address();
        let encoded = meta_address.encode();
        *self.scanner.write().await = Some(StealthScanner::new(keypair));
        info!("Stealth scanner initialized");
        Ok(encoded)
    }

    pub async fn is_initialized(&self) -> bool {
        self.scanner.read().await.is_some()
    }

    pub fn stats(&self) -> (u64, u64) {
        (
            self.detected_payments.load(Ordering::Relaxed),
            self.announcements_scanned.load(Ordering::Relaxed),
        )
    }

    pub async fn run(self: Arc<Self>, shutdown: Arc<AtomicBool>) -> NonosResult<()> {
        info!("Stealth Scanner started");
        let mut ticker = interval(Duration::from_secs(30));

        while !shutdown.load(Ordering::Relaxed) {
            ticker.tick().await;
            let (detected, scanned) = self.stats();
            debug!("Stealth scanner: {} detected, {} scanned", detected, scanned);
        }

        info!("Stealth Scanner stopped");
        Ok(())
    }
}
