<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { LPStaking, WorkMetricsPanel, RewardsPanel } from '$lib/staking';
	import type { LockTier, LPLockInfo, LPStatus, WorkMetrics, WorkCategory } from '$lib/staking';

	let activeTab: 'lp' | 'work' | 'rewards' = 'lp';
	let error = '';
	let success = '';

	let lockTiers: LockTier[] = [];
	let myLocks: LPLockInfo[] = [];
	let lpStatus: LPStatus = {
		total_locked: '0',
		weighted_total: '0',
		total_pending_rewards: '0',
		current_epoch: 0,
		epoch_lp_pool: '0'
	};
	let lockLoading = false;

	let workMetrics: WorkMetrics | null = null;
	let workCategories: WorkCategory[] = [];
	let estimatedReward = '0';
	let epochProgress = 0;
	let epochTimeRemaining = '';

	let refreshInterval: ReturnType<typeof setInterval>;

	onMount(async () => {
		let retries = 0;
		while (!window.nonos && retries < 20) {
			await new Promise(r => setTimeout(r, 250));
			retries++;
		}

		if (!window.nonos) {
			error = 'NONOS bridge not available';
			return;
		}

		await loadLPData();
		await loadWorkMetrics();

		refreshInterval = setInterval(async () => {
			if (activeTab === 'work') {
				await loadWorkMetrics();
			}
		}, 30000);
	});

	onDestroy(() => {
		if (refreshInterval) clearInterval(refreshInterval);
	});

	async function loadLPData() {
		if (!window.nonos) return;
		try {
			lockTiers = await window.nonos.lpStaking.getTiers();
			const status = await window.nonos.lpStaking.getStatus();
			lpStatus = {
				total_locked: status.total_locked,
				weighted_total: status.weighted_total,
				total_pending_rewards: status.total_pending_rewards,
				current_epoch: status.current_epoch,
				epoch_lp_pool: status.epoch_lp_pool
			};
			myLocks = status.locks || [];
		} catch (e) {
			console.error('Failed to load LP data:', e);
		}
	}

	async function loadWorkMetrics() {
		if (!window.nonos) return;
		try {
			const dashboard = await window.nonos.work.getDashboard();
			workMetrics = dashboard.metrics;
			workCategories = dashboard.categories;
			estimatedReward = dashboard.estimated_epoch_reward;

			if (workMetrics?.epoch) {
				const now = Math.floor(Date.now() / 1000);
				const start = workMetrics.epoch.epoch_start_timestamp;
				const end = workMetrics.epoch.epoch_end_timestamp;
				const total = end - start;
				const elapsed = now - start;
				epochProgress = Math.min(100, Math.max(0, (elapsed / total) * 100));

				const remaining = Math.max(0, end - now);
				const days = Math.floor(remaining / 86400);
				const hours = Math.floor((remaining % 86400) / 3600);
				epochTimeRemaining = `${days}d ${hours}h`;
			}
		} catch (e) {
			console.error('Failed to load work metrics:', e);
		}
	}

	async function handleLock(event: CustomEvent<{ amount: string; tier: number }>) {
		if (!window.nonos) return;
		lockLoading = true;
		error = '';
		try {
			const result = await window.nonos.lpStaking.lock(event.detail.amount, event.detail.tier);
			success = result;
			await loadLPData();
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			lockLoading = false;
		}
	}

	async function handleUnlock(event: CustomEvent<{ lockId: number }>) {
		if (!window.nonos) return;
		lockLoading = true;
		error = '';
		try {
			const result = await window.nonos.lpStaking.unlock(event.detail.lockId);
			success = result;
			await loadLPData();
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			lockLoading = false;
		}
	}

	async function handleClaimRewards(event: CustomEvent<{ lockId: number }>) {
		if (!window.nonos) return;
		lockLoading = true;
		error = '';
		try {
			const result = await window.nonos.lpStaking.claimRewards(event.detail.lockId);
			success = result;
			await loadLPData();
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			lockLoading = false;
		}
	}

	async function handleClaimAll() {
		if (!window.nonos) return;
		lockLoading = true;
		error = '';
		try {
			const result = await window.nonos.lpStaking.claimAllRewards();
			success = result;
			await loadLPData();
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			lockLoading = false;
		}
	}
</script>

<div class="staking-page">
	<div class="page-header">
		<h1>Privacy Infrastructure Economy</h1>
		<p class="subtitle">Stake NOX, provide liquidity, and earn rewards for privacy work</p>
	</div>

	{#if error}
		<div class="error-banner">
			<span>{error}</span>
			<button class="dismiss" on:click={() => error = ''}>×</button>
		</div>
	{/if}

	{#if success}
		<div class="success-banner">
			<span>{success}</span>
			<button class="dismiss" on:click={() => success = ''}>×</button>
		</div>
	{/if}

	<div class="tabs">
		<button class="tab" class:active={activeTab === 'lp'} on:click={() => activeTab = 'lp'}>
			LP Staking
		</button>
		<button class="tab" class:active={activeTab === 'work'} on:click={() => { activeTab = 'work'; loadWorkMetrics(); }}>
			Work Metrics
		</button>
		<button class="tab" class:active={activeTab === 'rewards'} on:click={() => activeTab = 'rewards'}>
			Rewards
		</button>
	</div>

	{#if activeTab === 'lp'}
		<LPStaking
			{lockTiers}
			{myLocks}
			{lpStatus}
			{lockLoading}
			on:lock={handleLock}
			on:unlock={handleUnlock}
			on:claim={handleClaimRewards}
		/>
	{/if}

	{#if activeTab === 'work'}
		<WorkMetricsPanel
			{workMetrics}
			{workCategories}
			{estimatedReward}
			{epochProgress}
			{epochTimeRemaining}
		/>
	{/if}

	{#if activeTab === 'rewards'}
		<RewardsPanel
			{lpStatus}
			{estimatedReward}
			{lockLoading}
			on:claimAll={handleClaimAll}
		/>
	{/if}
</div>

<style>
	.staking-page {
		max-width: 900px;
		margin: 0 auto;
	}

	.page-header {
		margin-bottom: var(--nox-space-xl);
	}

	.page-header h1 {
		font-size: var(--nox-text-2xl);
		font-weight: var(--nox-font-semibold);
		margin-bottom: var(--nox-space-xs);
	}

	.subtitle {
		color: var(--nox-text-muted);
		font-size: var(--nox-text-sm);
	}

	.error-banner, .success-banner {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: var(--nox-space-md);
		border-radius: var(--nox-radius-lg);
		margin-bottom: var(--nox-space-lg);
	}

	.error-banner {
		background: var(--nox-error-bg);
		border: 1px solid var(--nox-error);
		color: var(--nox-error);
	}

	.success-banner {
		background: var(--nox-success-bg);
		border: 1px solid var(--nox-success);
		color: var(--nox-success);
	}

	.dismiss {
		background: none;
		border: none;
		font-size: 1.5rem;
		cursor: pointer;
		color: inherit;
		padding: 0 var(--nox-space-sm);
	}

	.tabs {
		display: flex;
		gap: var(--nox-space-sm);
		margin-bottom: var(--nox-space-xl);
		border-bottom: 1px solid var(--nox-border);
		padding-bottom: var(--nox-space-sm);
	}

	.tab {
		padding: var(--nox-space-sm) var(--nox-space-lg);
		background: none;
		border: none;
		color: var(--nox-text-muted);
		font-size: var(--nox-text-sm);
		font-weight: var(--nox-font-medium);
		cursor: pointer;
		border-radius: var(--nox-radius-md);
		transition: all var(--nox-transition-fast);
	}

	.tab:hover {
		color: var(--nox-text-primary);
		background: var(--nox-bg-hover);
	}

	.tab.active {
		color: var(--nox-accent-primary);
		background: var(--nox-accent-glow);
	}
</style>
