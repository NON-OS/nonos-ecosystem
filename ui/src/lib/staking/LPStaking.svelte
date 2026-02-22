<script lang="ts">
	import type { LockTier, LPLockInfo, LPStatus } from './types';
	import { createEventDispatcher } from 'svelte';

	export let lockTiers: LockTier[] = [];
	export let myLocks: LPLockInfo[] = [];
	export let lpStatus: LPStatus;
	export let lockLoading = false;

	const dispatch = createEventDispatcher<{
		lock: { amount: string; tier: number };
		unlock: { lockId: number };
		claim: { lockId: number };
	}>();

	let lockAmount = '';
	let selectedTier = 0;

	function handleLock() {
		if (!lockAmount) return;
		dispatch('lock', { amount: String(lockAmount), tier: selectedTier });
		lockAmount = '';
	}

	function handleUnlock(lockId: number) {
		dispatch('unlock', { lockId });
	}

	function handleClaim(lockId: number) {
		dispatch('claim', { lockId });
	}
</script>

<div class="section">
	<div class="section-header">
		<h2>Lock NOX Tokens</h2>
		<p>Lock your NOX tokens to earn a share of the LP pool rewards (30% of emissions)</p>
	</div>

	<div class="lock-form">
		<div class="form-row">
			<div class="form-group">
				<label>Amount to Lock</label>
				<input type="number" bind:value={lockAmount} placeholder="0.00" class="input" />
			</div>
			<div class="form-group">
				<label>Lock Duration</label>
				<select bind:value={selectedTier} class="input">
					{#each lockTiers as tier}
						<option value={tier.id}>{tier.duration_days} days ({tier.multiplier_display})</option>
					{/each}
				</select>
			</div>
		</div>
		<button class="btn primary full-width" on:click={handleLock} disabled={lockLoading || !lockAmount}>
			{lockLoading ? 'Processing...' : 'Lock NOX'}
		</button>
	</div>

	<div class="tier-grid">
		{#each lockTiers as tier}
			<button class="tier-card" class:selected={selectedTier === tier.id} on:click={() => selectedTier = tier.id}>
				<div class="tier-duration">{tier.duration_days} Days</div>
				<div class="tier-multiplier">{tier.multiplier_display}</div>
				<div class="tier-label">Reward Multiplier</div>
			</button>
		{/each}
	</div>

	{#if myLocks.length > 0}
		<div class="my-locks">
			<h3>Your Active Locks</h3>
			<div class="locks-list">
				{#each myLocks as lock}
					<div class="lock-card">
						<div class="lock-info">
							<div class="lock-amount">{lock.amount} NOX</div>
							<div class="lock-tier">{lock.tier_name} • {lock.multiplier}</div>
						</div>
						<div class="lock-rewards">
							<div class="rewards-label">Pending</div>
							<div class="rewards-value">{lock.pending_rewards} NOX</div>
						</div>
						<div class="lock-actions">
							{#if lock.is_locked}
								<span class="locked-badge">Locked</span>
							{:else}
								<button class="btn small" on:click={() => handleUnlock(lock.lock_id)}>Unlock</button>
							{/if}
							<button class="btn small secondary" on:click={() => handleClaim(lock.lock_id)}>Claim</button>
						</div>
					</div>
				{/each}
			</div>
		</div>
	{/if}

	<div class="lp-summary">
		<div class="summary-card">
			<div class="summary-label">Total Locked</div>
			<div class="summary-value">{lpStatus.total_locked} NOX</div>
		</div>
		<div class="summary-card">
			<div class="summary-label">Weighted Total</div>
			<div class="summary-value">{lpStatus.weighted_total}</div>
		</div>
		<div class="summary-card">
			<div class="summary-label">Pending Rewards</div>
			<div class="summary-value accent">{lpStatus.total_pending_rewards} NOX</div>
		</div>
		<div class="summary-card">
			<div class="summary-label">Current Epoch</div>
			<div class="summary-value">{lpStatus.current_epoch}</div>
		</div>
	</div>
</div>

<style>
	.section {
		background: var(--nox-bg-secondary);
		border: 1px solid var(--nox-border);
		border-radius: var(--nox-radius-xl);
		padding: var(--nox-space-xl);
	}

	.section-header {
		margin-bottom: var(--nox-space-lg);
	}

	.section-header h2 {
		font-size: var(--nox-text-lg);
		font-weight: var(--nox-font-semibold);
		margin-bottom: var(--nox-space-xs);
	}

	.section-header p {
		color: var(--nox-text-muted);
		font-size: var(--nox-text-sm);
	}

	.lock-form {
		background: var(--nox-bg-tertiary);
		border-radius: var(--nox-radius-lg);
		padding: var(--nox-space-lg);
		margin-bottom: var(--nox-space-lg);
	}

	.form-row {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: var(--nox-space-md);
		margin-bottom: var(--nox-space-md);
	}

	.form-group label {
		display: block;
		font-size: var(--nox-text-sm);
		color: var(--nox-text-secondary);
		margin-bottom: var(--nox-space-xs);
	}

	.input {
		width: 100%;
		padding: var(--nox-space-sm) var(--nox-space-md);
		background: var(--nox-bg-primary);
		border: 1px solid var(--nox-border);
		border-radius: var(--nox-radius-md);
		color: var(--nox-text-primary);
		font-size: var(--nox-text-sm);
	}

	.input:focus {
		outline: none;
		border-color: var(--nox-accent-primary);
	}

	select.input {
		cursor: pointer;
	}

	.btn {
		padding: var(--nox-space-sm) var(--nox-space-lg);
		border-radius: var(--nox-radius-md);
		font-weight: var(--nox-font-medium);
		font-size: var(--nox-text-sm);
		cursor: pointer;
		transition: all var(--nox-transition-fast);
		border: none;
	}

	.btn.primary {
		background: var(--nox-accent-gradient);
		color: var(--nox-bg-primary);
	}

	.btn.primary:hover:not(:disabled) {
		box-shadow: var(--nox-shadow-glow);
	}

	.btn.secondary {
		background: var(--nox-bg-tertiary);
		border: 1px solid var(--nox-border);
		color: var(--nox-text-primary);
	}

	.btn.small {
		padding: var(--nox-space-xs) var(--nox-space-sm);
		font-size: var(--nox-text-xs);
	}

	.btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.btn.full-width {
		width: 100%;
	}

	.tier-grid {
		display: grid;
		grid-template-columns: repeat(5, 1fr);
		gap: var(--nox-space-md);
		margin-bottom: var(--nox-space-xl);
	}

	.tier-card {
		background: var(--nox-bg-tertiary);
		border: 2px solid var(--nox-border);
		border-radius: var(--nox-radius-lg);
		padding: var(--nox-space-md);
		text-align: center;
		cursor: pointer;
		transition: all var(--nox-transition-fast);
	}

	.tier-card:hover {
		border-color: var(--nox-border-light);
	}

	.tier-card.selected {
		border-color: var(--nox-accent-primary);
		background: var(--nox-accent-glow);
	}

	.tier-duration {
		font-size: var(--nox-text-lg);
		font-weight: var(--nox-font-semibold);
		margin-bottom: var(--nox-space-xs);
	}

	.tier-multiplier {
		font-size: var(--nox-text-xl);
		font-weight: var(--nox-font-bold);
		color: var(--nox-accent-primary);
	}

	.tier-label {
		font-size: var(--nox-text-xs);
		color: var(--nox-text-muted);
		margin-top: var(--nox-space-xs);
	}

	.lp-summary {
		display: grid;
		grid-template-columns: repeat(4, 1fr);
		gap: var(--nox-space-md);
	}

	.summary-card {
		background: var(--nox-bg-tertiary);
		border-radius: var(--nox-radius-lg);
		padding: var(--nox-space-md);
		text-align: center;
	}

	.summary-label {
		font-size: var(--nox-text-xs);
		color: var(--nox-text-muted);
		margin-bottom: var(--nox-space-xs);
	}

	.summary-value {
		font-size: var(--nox-text-lg);
		font-weight: var(--nox-font-semibold);
		font-family: var(--nox-font-mono);
	}

	.summary-value.accent {
		color: var(--nox-accent-primary);
	}

	.my-locks {
		margin-bottom: var(--nox-space-xl);
	}

	.my-locks h3 {
		font-size: var(--nox-text-base);
		margin-bottom: var(--nox-space-md);
	}

	.locks-list {
		display: flex;
		flex-direction: column;
		gap: var(--nox-space-sm);
	}

	.lock-card {
		display: flex;
		align-items: center;
		gap: var(--nox-space-lg);
		background: var(--nox-bg-tertiary);
		border-radius: var(--nox-radius-lg);
		padding: var(--nox-space-md);
	}

	.lock-info {
		flex: 1;
	}

	.lock-amount {
		font-size: var(--nox-text-base);
		font-weight: var(--nox-font-semibold);
	}

	.lock-tier {
		font-size: var(--nox-text-xs);
		color: var(--nox-text-muted);
	}

	.lock-rewards {
		text-align: right;
	}

	.rewards-label {
		font-size: var(--nox-text-xs);
		color: var(--nox-text-muted);
	}

	.rewards-value {
		font-size: var(--nox-text-sm);
		font-weight: var(--nox-font-semibold);
		color: var(--nox-accent-primary);
	}

	.lock-actions {
		display: flex;
		gap: var(--nox-space-sm);
	}

	.locked-badge {
		padding: var(--nox-space-xs) var(--nox-space-sm);
		background: var(--nox-warning-bg);
		color: var(--nox-warning);
		border-radius: var(--nox-radius-sm);
		font-size: var(--nox-text-xs);
	}

	@media (max-width: 800px) {
		.tier-grid {
			grid-template-columns: repeat(3, 1fr);
		}

		.lp-summary {
			grid-template-columns: repeat(2, 1fr);
		}

		.form-row {
			grid-template-columns: 1fr;
		}
	}

	@media (max-width: 500px) {
		.tier-grid {
			grid-template-columns: repeat(2, 1fr);
		}
	}
</style>
