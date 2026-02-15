// See https://kit.svelte.dev/docs/types#app
// for information about these interfaces
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
			create(password: string): Promise<string>; // Returns mnemonic
			import(mnemonic: string, password: string): Promise<void>;
			lock(): Promise<void>;
			unlock(password: string): Promise<void>;
			getAddress(): Promise<string | null>;
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
