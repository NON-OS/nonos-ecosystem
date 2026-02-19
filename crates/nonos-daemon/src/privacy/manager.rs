use super::{
    ZkIdentityService, CacheMixingService, TrackingBlockerService, StealthScannerService,
    ZkIdentityRegistry, NoteMixer,
};
use nonos_types::{NodeId, NonosResult};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::{error, info};

pub struct PrivacyServiceManager {
    pub zk_identity: Arc<ZkIdentityService>,
    pub cache_mixing: Arc<CacheMixingService>,
    pub tracking_blocker: Arc<TrackingBlockerService>,
    pub stealth_scanner: Arc<StealthScannerService>,
    pub identity_registry: Arc<ZkIdentityRegistry>,
    pub note_mixer: Arc<NoteMixer>,
    shutdown: Arc<AtomicBool>,
}

impl PrivacyServiceManager {
    pub fn new(node_id: NodeId) -> Self {
        Self {
            zk_identity: Arc::new(ZkIdentityService::new(node_id)),
            cache_mixing: Arc::new(CacheMixingService::new(node_id, 10000)),
            tracking_blocker: Arc::new(TrackingBlockerService::new(node_id)),
            stealth_scanner: Arc::new(StealthScannerService::new(node_id)),
            identity_registry: Arc::new(ZkIdentityRegistry::new()),
            note_mixer: Arc::new(NoteMixer::new()),
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    pub async fn start_all(&self) -> NonosResult<()> {
        info!("Starting NONOS privacy services");
        let shutdown = self.shutdown.clone();

        let zk = self.zk_identity.clone();
        let s1 = shutdown.clone();
        tokio::spawn(async move {
            if let Err(e) = zk.run(s1).await { error!("ZK Identity error: {}", e); }
        });

        let cache = self.cache_mixing.clone();
        let s2 = shutdown.clone();
        tokio::spawn(async move {
            if let Err(e) = cache.run(s2).await { error!("Cache Mixing error: {}", e); }
        });

        let blocker = self.tracking_blocker.clone();
        let s3 = shutdown.clone();
        tokio::spawn(async move {
            if let Err(e) = blocker.run(s3).await { error!("Tracking Blocker error: {}", e); }
        });

        let stealth = self.stealth_scanner.clone();
        let s4 = shutdown.clone();
        tokio::spawn(async move {
            if let Err(e) = stealth.run(s4).await { error!("Stealth Scanner error: {}", e); }
        });

        info!("ZK Identity Registry initialized");
        info!("Note Mixer initialized");

        info!("All privacy services started");
        Ok(())
    }

    pub fn stop_all(&self) {
        self.shutdown.store(true, Ordering::SeqCst);
        info!("Privacy services stopped");
    }

    pub fn stats(&self) -> PrivacyStats {
        let (zk_issued, zk_verified) = self.zk_identity.stats();
        let (cache_hits, cache_misses, mix_ops) = self.cache_mixing.stats();
        let (blocked, total, fingerprint) = self.tracking_blocker.stats();
        let (payments, scanned) = self.stealth_scanner.stats();
        let (id_registrations, id_passed, id_failed) = self.identity_registry.stats();
        let (note_deposits, note_spends, note_failed) = self.note_mixer.stats();

        PrivacyStats {
            zk_proofs_issued: zk_issued,
            zk_verifications: zk_verified,
            cache_hits,
            cache_misses,
            cache_mix_ops: mix_ops,
            tracking_blocked: blocked,
            tracking_total: total,
            fingerprint_blocked: fingerprint,
            stealth_payments: payments,
            stealth_scanned: scanned,
            identity_registrations: id_registrations,
            identity_verifications_passed: id_passed,
            identity_verifications_failed: id_failed,
            note_deposits,
            note_spends,
            note_failed_spends: note_failed,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PrivacyStats {
    pub zk_proofs_issued: u64,
    pub zk_verifications: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub cache_mix_ops: u64,
    pub tracking_blocked: u64,
    pub tracking_total: u64,
    pub fingerprint_blocked: u64,
    pub stealth_payments: u64,
    pub stealth_scanned: u64,
    pub identity_registrations: u64,
    pub identity_verifications_passed: u64,
    pub identity_verifications_failed: u64,
    pub note_deposits: u64,
    pub note_spends: u64,
    pub note_failed_spends: u64,
}

impl std::fmt::Display for PrivacyStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "NONOS Privacy Statistics")?;
        writeln!(f, "========================")?;
        writeln!(f, "ZK Identity: {} issued, {} verified", self.zk_proofs_issued, self.zk_verifications)?;
        writeln!(f, "Identity Registry: {} registered, {} passed, {} failed",
            self.identity_registrations, self.identity_verifications_passed, self.identity_verifications_failed)?;
        writeln!(f, "Note Mixer: {} deposits, {} spends, {} failed",
            self.note_deposits, self.note_spends, self.note_failed_spends)?;
        writeln!(f, "Cache: {} hits, {} misses, {} mix ops", self.cache_hits, self.cache_misses, self.cache_mix_ops)?;
        writeln!(f, "Tracker: {} blocked/{} total, {} fingerprint", self.tracking_blocked, self.tracking_total, self.fingerprint_blocked)?;
        writeln!(f, "Stealth: {} payments, {} scanned", self.stealth_payments, self.stealth_scanned)?;
        Ok(())
    }
}
