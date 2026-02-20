// See https://kit.svelte.dev/docs/types#app
// for information about these interfaces

declare module 'qrcode' {
	export function toDataURL(text: string, options?: {
		width?: number;
		margin?: number;
		color?: { dark?: string; light?: string };
	}): Promise<string>;
}

declare global {
	namespace App {
		// interface Error {}
		// interface Locals {}
		// interface PageData {}
		// interface PageState {}
		// interface Platform {}
	}

	// Native bridge to Rust backend
	interface NonosBridge {
		version: string;

		// Wallet operations
		wallet: {
			getStatus(): Promise<{
				initialized: boolean;
				locked: boolean;
				address: string | null;
				nox_balance: string;
				eth_balance: string;
				pending_rewards: string;
			}>;
			create(password: string): Promise<string>;
			import(mnemonic: string, password: string): Promise<void>;
			lock(): Promise<void>;
			unlock(password: string): Promise<void>;
			getAddress(): Promise<string | null>;
			sendEth(to: string, amount: string): Promise<string>;
			sendNox(to: string, amount: string): Promise<string>;
			getTransactions(): Promise<Array<{
				hash: string;
				from: string;
				to: string;
				value: string;
				token: string;
				timestamp: number;
				status: string;
			}>>;
			exists(): Promise<boolean>;
			getStealthAddress(): Promise<string>;
			changePassword(oldPassword: string, newPassword: string): Promise<void>;
		};

		// Staking operations
		staking: {
			getStatus(): Promise<{
				staked_amount: string;
				tier: string;
				tier_multiplier: string;
				pending_rewards: string;
				current_epoch: number;
				next_tier_threshold: string;
				estimated_apy: string;
			}>;
			stake(amount: string): Promise<void>;
			unstake(amount: string): Promise<void>;
			claimRewards(): Promise<void>;
		};

		// LP Staking operations
		lpStaking: {
			getStatus(): Promise<{
				total_locked: string;
				weighted_total: string;
				locks: Array<{
					lock_id: number;
					amount: string;
					tier: number;
					tier_name: string;
					multiplier: string;
					lock_start: number;
					lock_end: number;
					is_locked: boolean;
					pending_rewards: string;
				}>;
				total_pending_rewards: string;
				available_tiers: Array<{
					id: number;
					duration_days: number;
					multiplier: number;
					multiplier_display: string;
				}>;
				current_epoch: number;
				epoch_lp_pool: string;
			}>;
			getTiers(): Promise<Array<{
				id: number;
				duration_days: number;
				multiplier: number;
				multiplier_display: string;
			}>>;
			lock(amount: string, tier: number): Promise<string>;
			unlock(lockId: number): Promise<string>;
			earlyUnlock(lockId: number): Promise<string>;
			extendLock(lockId: number, newTier: number): Promise<string>;
			claimRewards(lockId: number): Promise<string>;
			claimAllRewards(): Promise<string>;
			compoundRewards(lockId: number): Promise<string>;
		};

		// Work metrics
		work: {
			getMetrics(): Promise<{
				traffic_relay: { bytes_relayed: number; relay_sessions: number; successful_relays: number; failed_relays: number; avg_latency_ms: number };
				zk_proofs: { proofs_generated: number; proofs_verified: number; avg_generation_time_ms: number; verification_failures: number };
				mixer_ops: { deposits_processed: number; spends_processed: number; total_value_mixed: number; pool_participations: number };
				entropy: { entropy_bytes_contributed: number; entropy_requests_served: number; quality_score: number };
				registry_ops: { registrations_processed: number; lookups_served: number; sync_operations: number; failed_operations: number };
				epoch: { current_epoch: number; epoch_start_timestamp: number; epoch_end_timestamp: number; submitted_to_oracle: boolean };
				total_work_score: number;
			}>;
			getDashboard(): Promise<{
				metrics: {
					traffic_relay: { bytes_relayed: number; relay_sessions: number; successful_relays: number; failed_relays: number; avg_latency_ms: number };
					zk_proofs: { proofs_generated: number; proofs_verified: number; avg_generation_time_ms: number; verification_failures: number };
					mixer_ops: { deposits_processed: number; spends_processed: number; total_value_mixed: number; pool_participations: number };
					entropy: { entropy_bytes_contributed: number; entropy_requests_served: number; quality_score: number };
					registry_ops: { registrations_processed: number; lookups_served: number; sync_operations: number; failed_operations: number };
					epoch: { current_epoch: number; epoch_start_timestamp: number; epoch_end_timestamp: number; submitted_to_oracle: boolean };
					total_work_score: number;
				};
				categories: Array<{ name: string; weight: number; score: number; raw_value: number }>;
				estimated_epoch_reward: string;
				network_rank: number | null;
				network_total_nodes: number;
			}>;
			getEpoch(): Promise<{
				current_epoch: number;
				epoch_start_timestamp: number;
				epoch_end_timestamp: number;
				submitted_to_oracle: boolean;
			}>;
		};

		// Network operations
		network: {
			connect(): Promise<void>;
			disconnect(): Promise<void>;
			getStatus(): Promise<{
				connected: boolean;
				status: string;
				bootstrap_progress: number;
				circuits: number;
				socks_port: number;
				error: string | null;
			}>;
			newIdentity(): Promise<void>;
		};

		// Browser operations
		browser: {
			navigate(url: string): Promise<string>;
			getSocksProxy(): Promise<{ host: string; port: number }>;
			proxyFetch(url: string, options?: {
				method?: string;
				headers?: Record<string, string>;
				body?: string;
			}): Promise<{ status: number; body: string; headers: Record<string, string> }>;
		};

		// Node operations
		node: {
			getStatus(): Promise<{
				running: boolean;
				connected_nodes: number;
				quality: number;
				total_requests: number;
			}>;
			startEmbedded(): Promise<void>;
			stopEmbedded(): Promise<void>;
			getConnected(): Promise<Array<{
				id: string;
				address: string;
				quality_score: number;
				latency_ms: number;
				connected: boolean;
			}>>;
		};

		// Privacy services
		privacy: {
			getStats(): Promise<{
				zk_proofs_issued: number;
				zk_verifications: number;
				cache_hits: number;
				cache_misses: number;
				cache_hit_rate: number;
				tracking_blocked: number;
				tracking_total: number;
				block_rate: number;
				stealth_payments: number;
				stealth_scanned: number;
			}>;
			checkTracking(domain: string): Promise<{ domain: string; blocked: boolean; reason: string | null }>;
			blockDomain(domain: string): Promise<void>;
			generateIdentity(name: string): Promise<{ identity_id: string; commitment: string; merkle_root: string }>;
			getIdentityRoot(): Promise<string>;
			cacheStore(content: string): Promise<string>;
		};

		// App info
		getAppInfo(): Promise<{
			name: string;
			version: string;
			platform: string;
			arch: string;
			build: string;
		}>;

		// Event listeners
		onNetworkStatus(callback: (status: {
			connected: boolean;
			status: string;
			bootstrap_progress: number;
			circuits: number;
			error: string | null;
		}) => void): Promise<() => void>;
		onIdentityChanged(callback: () => void): Promise<() => void>;
		onNodeStarted(callback: () => void): Promise<() => void>;
		onNodeStopped(callback: () => void): Promise<() => void>;
	}

	interface Window {
		nonos: NonosBridge;
	}
}

export {};
