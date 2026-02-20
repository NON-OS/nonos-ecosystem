use parking_lot::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};

use super::types::{
    WorkMetrics, TrafficRelayMetrics, ZkProofMetrics, MixerOpsMetrics,
    EntropyMetrics, RegistryOpsMetrics, EpochInfo,
    WORK_WEIGHT_TRAFFIC_RELAY, WORK_WEIGHT_ZK_PROOFS, WORK_WEIGHT_MIXER_OPS,
    WORK_WEIGHT_ENTROPY, WORK_WEIGHT_REGISTRY_OPS, EPOCH_DURATION_SECS,
};

const BASELINE_BYTES_RELAYED: u64 = 1_000_000_000;
const BASELINE_ZK_PROOFS: u64 = 1000;
const BASELINE_MIXER_OPS: u64 = 100;
const BASELINE_ENTROPY_BYTES: u64 = 10_000_000;
const BASELINE_REGISTRY_OPS: u64 = 500;

pub struct WorkState {
    pub(crate) bytes_relayed: AtomicU64,
    pub(crate) relay_sessions: AtomicU64,
    pub(crate) successful_relays: AtomicU64,
    pub(crate) failed_relays: AtomicU64,
    pub(crate) relay_latency_total_ms: AtomicU64,
    pub(crate) relay_latency_count: AtomicU64,
    pub(crate) zk_proofs_generated: AtomicU64,
    pub(crate) zk_proofs_verified: AtomicU64,
    pub(crate) zk_gen_time_total_ms: AtomicU64,
    pub(crate) zk_gen_count: AtomicU64,
    pub(crate) zk_verification_failures: AtomicU64,
    pub(crate) mixer_deposits: AtomicU64,
    pub(crate) mixer_spends: AtomicU64,
    pub(crate) mixer_value_total: RwLock<u128>,
    pub(crate) mixer_pool_participations: AtomicU64,
    pub(crate) entropy_bytes: AtomicU64,
    pub(crate) entropy_requests: AtomicU64,
    pub(crate) entropy_quality_sum: AtomicU64,
    pub(crate) entropy_quality_count: AtomicU64,
    pub(crate) registry_registrations: AtomicU64,
    pub(crate) registry_lookups: AtomicU64,
    pub(crate) registry_syncs: AtomicU64,
    pub(crate) registry_failures: AtomicU64,
    pub(crate) epoch_start: RwLock<u64>,
    pub(crate) epoch_number: AtomicU64,
    pub(crate) epoch_submitted: RwLock<bool>,
}

impl WorkState {
    pub fn new() -> Self {
        Self {
            bytes_relayed: AtomicU64::new(0),
            relay_sessions: AtomicU64::new(0),
            successful_relays: AtomicU64::new(0),
            failed_relays: AtomicU64::new(0),
            relay_latency_total_ms: AtomicU64::new(0),
            relay_latency_count: AtomicU64::new(0),
            zk_proofs_generated: AtomicU64::new(0),
            zk_proofs_verified: AtomicU64::new(0),
            zk_gen_time_total_ms: AtomicU64::new(0),
            zk_gen_count: AtomicU64::new(0),
            zk_verification_failures: AtomicU64::new(0),
            mixer_deposits: AtomicU64::new(0),
            mixer_spends: AtomicU64::new(0),
            mixer_value_total: RwLock::new(0),
            mixer_pool_participations: AtomicU64::new(0),
            entropy_bytes: AtomicU64::new(0),
            entropy_requests: AtomicU64::new(0),
            entropy_quality_sum: AtomicU64::new(0),
            entropy_quality_count: AtomicU64::new(0),
            registry_registrations: AtomicU64::new(0),
            registry_lookups: AtomicU64::new(0),
            registry_syncs: AtomicU64::new(0),
            registry_failures: AtomicU64::new(0),
            epoch_start: RwLock::new(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            ),
            epoch_number: AtomicU64::new(0),
            epoch_submitted: RwLock::new(false),
        }
    }

    pub fn record_relay(&self, bytes: u64, success: bool, latency_ms: u64) {
        self.bytes_relayed.fetch_add(bytes, Ordering::Relaxed);
        self.relay_sessions.fetch_add(1, Ordering::Relaxed);
        if success {
            self.successful_relays.fetch_add(1, Ordering::Relaxed);
        } else {
            self.failed_relays.fetch_add(1, Ordering::Relaxed);
        }
        self.relay_latency_total_ms.fetch_add(latency_ms, Ordering::Relaxed);
        self.relay_latency_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_zk_proof_generated(&self, generation_time_ms: u64) {
        self.zk_proofs_generated.fetch_add(1, Ordering::Relaxed);
        self.zk_gen_time_total_ms.fetch_add(generation_time_ms, Ordering::Relaxed);
        self.zk_gen_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_zk_proof_verified(&self, success: bool) {
        self.zk_proofs_verified.fetch_add(1, Ordering::Relaxed);
        if !success {
            self.zk_verification_failures.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn record_mixer_deposit(&self, value: u128) {
        self.mixer_deposits.fetch_add(1, Ordering::Relaxed);
        *self.mixer_value_total.write() += value;
    }

    pub fn record_mixer_spend(&self, value: u128) {
        self.mixer_spends.fetch_add(1, Ordering::Relaxed);
        *self.mixer_value_total.write() += value;
    }

    pub fn record_mixer_pool_participation(&self) {
        self.mixer_pool_participations.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_entropy_contribution(&self, bytes: u64, quality_score: f64) {
        self.entropy_bytes.fetch_add(bytes, Ordering::Relaxed);
        self.entropy_quality_sum.fetch_add((quality_score * 100.0) as u64, Ordering::Relaxed);
        self.entropy_quality_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_entropy_request_served(&self) {
        self.entropy_requests.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_registry_registration(&self) {
        self.registry_registrations.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_registry_lookup(&self) {
        self.registry_lookups.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_registry_sync(&self) {
        self.registry_syncs.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_registry_failure(&self) {
        self.registry_failures.fetch_add(1, Ordering::Relaxed);
    }

    pub fn epoch_info(&self) -> EpochInfo {
        let epoch_start = *self.epoch_start.read();
        EpochInfo {
            current_epoch: self.epoch_number.load(Ordering::Relaxed),
            epoch_start_timestamp: epoch_start,
            epoch_end_timestamp: epoch_start + EPOCH_DURATION_SECS,
            submitted_to_oracle: *self.epoch_submitted.read(),
        }
    }

    pub fn check_epoch_advance(&self) -> bool {
        let epoch_start = *self.epoch_start.read();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if now >= epoch_start + EPOCH_DURATION_SECS {
            *self.epoch_start.write() = now;
            self.epoch_number.fetch_add(1, Ordering::Relaxed);
            *self.epoch_submitted.write() = false;
            self.reset();
            true
        } else {
            false
        }
    }

    pub fn mark_epoch_submitted(&self) {
        *self.epoch_submitted.write() = true;
    }

    pub fn reset(&self) {
        self.bytes_relayed.store(0, Ordering::Relaxed);
        self.relay_sessions.store(0, Ordering::Relaxed);
        self.successful_relays.store(0, Ordering::Relaxed);
        self.failed_relays.store(0, Ordering::Relaxed);
        self.relay_latency_total_ms.store(0, Ordering::Relaxed);
        self.relay_latency_count.store(0, Ordering::Relaxed);
        self.zk_proofs_generated.store(0, Ordering::Relaxed);
        self.zk_proofs_verified.store(0, Ordering::Relaxed);
        self.zk_gen_time_total_ms.store(0, Ordering::Relaxed);
        self.zk_gen_count.store(0, Ordering::Relaxed);
        self.zk_verification_failures.store(0, Ordering::Relaxed);
        self.mixer_deposits.store(0, Ordering::Relaxed);
        self.mixer_spends.store(0, Ordering::Relaxed);
        *self.mixer_value_total.write() = 0;
        self.mixer_pool_participations.store(0, Ordering::Relaxed);
        self.entropy_bytes.store(0, Ordering::Relaxed);
        self.entropy_requests.store(0, Ordering::Relaxed);
        self.entropy_quality_sum.store(0, Ordering::Relaxed);
        self.entropy_quality_count.store(0, Ordering::Relaxed);
        self.registry_registrations.store(0, Ordering::Relaxed);
        self.registry_lookups.store(0, Ordering::Relaxed);
        self.registry_syncs.store(0, Ordering::Relaxed);
        self.registry_failures.store(0, Ordering::Relaxed);
    }

    fn calculate_category_score(raw_value: u64, baseline: u64) -> f64 {
        if baseline == 0 {
            return 0.0;
        }
        let ratio = raw_value as f64 / baseline as f64;
        (ratio * 100.0).min(100.0)
    }

    pub fn summary(&self) -> WorkMetrics {
        let relay_count = self.relay_latency_count.load(Ordering::Relaxed);
        let avg_relay_latency = if relay_count > 0 {
            self.relay_latency_total_ms.load(Ordering::Relaxed) as f64 / relay_count as f64
        } else {
            0.0
        };

        let traffic_relay = TrafficRelayMetrics {
            bytes_relayed: self.bytes_relayed.load(Ordering::Relaxed),
            relay_sessions: self.relay_sessions.load(Ordering::Relaxed),
            successful_relays: self.successful_relays.load(Ordering::Relaxed),
            failed_relays: self.failed_relays.load(Ordering::Relaxed),
            avg_latency_ms: avg_relay_latency,
        };

        let zk_gen_count = self.zk_gen_count.load(Ordering::Relaxed);
        let avg_zk_gen_time = if zk_gen_count > 0 {
            self.zk_gen_time_total_ms.load(Ordering::Relaxed) as f64 / zk_gen_count as f64
        } else {
            0.0
        };

        let zk_proofs = ZkProofMetrics {
            proofs_generated: self.zk_proofs_generated.load(Ordering::Relaxed),
            proofs_verified: self.zk_proofs_verified.load(Ordering::Relaxed),
            avg_generation_time_ms: avg_zk_gen_time,
            verification_failures: self.zk_verification_failures.load(Ordering::Relaxed),
        };

        let mixer_ops = MixerOpsMetrics {
            deposits_processed: self.mixer_deposits.load(Ordering::Relaxed),
            spends_processed: self.mixer_spends.load(Ordering::Relaxed),
            total_value_mixed: *self.mixer_value_total.read(),
            pool_participations: self.mixer_pool_participations.load(Ordering::Relaxed),
        };

        let entropy_count = self.entropy_quality_count.load(Ordering::Relaxed);
        let entropy_quality = if entropy_count > 0 {
            (self.entropy_quality_sum.load(Ordering::Relaxed) as f64 / entropy_count as f64) / 100.0
        } else {
            0.0
        };

        let entropy = EntropyMetrics {
            entropy_bytes_contributed: self.entropy_bytes.load(Ordering::Relaxed),
            entropy_requests_served: self.entropy_requests.load(Ordering::Relaxed),
            quality_score: entropy_quality,
        };

        let registry_ops = RegistryOpsMetrics {
            registrations_processed: self.registry_registrations.load(Ordering::Relaxed),
            lookups_served: self.registry_lookups.load(Ordering::Relaxed),
            sync_operations: self.registry_syncs.load(Ordering::Relaxed),
            failed_operations: self.registry_failures.load(Ordering::Relaxed),
        };

        let traffic_score = Self::calculate_category_score(traffic_relay.bytes_relayed, BASELINE_BYTES_RELAYED);
        let zk_score = Self::calculate_category_score(
            zk_proofs.proofs_generated + zk_proofs.proofs_verified,
            BASELINE_ZK_PROOFS,
        );
        let mixer_score = Self::calculate_category_score(
            mixer_ops.deposits_processed + mixer_ops.spends_processed,
            BASELINE_MIXER_OPS,
        );
        let entropy_score = Self::calculate_category_score(entropy.entropy_bytes_contributed, BASELINE_ENTROPY_BYTES);
        let registry_score = Self::calculate_category_score(
            registry_ops.registrations_processed + registry_ops.lookups_served,
            BASELINE_REGISTRY_OPS,
        );

        let total_work_score =
            traffic_score * WORK_WEIGHT_TRAFFIC_RELAY +
            zk_score * WORK_WEIGHT_ZK_PROOFS +
            mixer_score * WORK_WEIGHT_MIXER_OPS +
            entropy_score * WORK_WEIGHT_ENTROPY +
            registry_score * WORK_WEIGHT_REGISTRY_OPS;

        WorkMetrics {
            traffic_relay,
            zk_proofs,
            mixer_ops,
            entropy,
            registry_ops,
            epoch: self.epoch_info(),
            total_work_score,
        }
    }
}

impl Default for WorkState {
    fn default() -> Self {
        Self::new()
    }
}
