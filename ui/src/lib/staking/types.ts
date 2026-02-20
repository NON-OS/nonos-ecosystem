export type LockTier = {
	id: number;
	duration_days: number;
	multiplier: number;
	multiplier_display: string;
};

export type LPLockInfo = {
	lock_id: number;
	amount: string;
	tier: number;
	tier_name: string;
	multiplier: string;
	lock_start: number;
	lock_end: number;
	is_locked: boolean;
	pending_rewards: string;
};

export type LPStatus = {
	total_locked: string;
	weighted_total: string;
	total_pending_rewards: string;
	current_epoch: number;
	epoch_lp_pool: string;
};

export type WorkCategory = {
	name: string;
	weight: number;
	score: number;
	raw_value: number;
};

export type TrafficRelayMetrics = {
	bytes_relayed: number;
	relay_sessions: number;
	successful_relays: number;
	failed_relays: number;
	avg_latency_ms: number;
};

export type ZkProofMetrics = {
	proofs_generated: number;
	proofs_verified: number;
	avg_generation_time_ms: number;
	verification_failures: number;
};

export type MixerOpsMetrics = {
	deposits_processed: number;
	spends_processed: number;
	total_value_mixed: number;
	pool_participations: number;
};

export type EntropyMetrics = {
	entropy_bytes_contributed: number;
	entropy_requests_served: number;
	quality_score: number;
};

export type RegistryOpsMetrics = {
	registrations_processed: number;
	lookups_served: number;
	sync_operations: number;
	failed_operations: number;
};

export type EpochInfo = {
	current_epoch: number;
	epoch_start_timestamp: number;
	epoch_end_timestamp: number;
	submitted_to_oracle: boolean;
};

export type WorkMetrics = {
	traffic_relay: TrafficRelayMetrics;
	zk_proofs: ZkProofMetrics;
	mixer_ops: MixerOpsMetrics;
	entropy: EntropyMetrics;
	registry_ops: RegistryOpsMetrics;
	epoch: EpochInfo;
	total_work_score: number;
};

export function formatBytes(bytes: number): string {
	if (bytes >= 1e9) return `${(bytes / 1e9).toFixed(2)} GB`;
	if (bytes >= 1e6) return `${(bytes / 1e6).toFixed(2)} MB`;
	if (bytes >= 1e3) return `${(bytes / 1e3).toFixed(2)} KB`;
	return `${bytes} B`;
}

export function formatNumber(n: number): string {
	return n.toLocaleString();
}
