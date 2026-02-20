<script lang="ts">
	import type { LPStatus } from './types';
	import { createEventDispatcher } from 'svelte';

	export let lpStatus: LPStatus;
	export let estimatedReward = '0';
	export let lockLoading = false;

	const dispatch = createEventDispatcher<{
		claimAll: void;
	}>();

	function handleClaimAll() {
		dispatch('claimAll');
	}
</script>

<div class="section">
	<div class="section-header">
		<h2>Rewards</h2>
		<p>Claim your earned NOX rewards from node operations and LP staking</p>
	</div>

	<div class="rewards-overview">
		<div class="reward-card total">
			<div class="reward-icon">
				<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
					<path d="M12 2L22 12L12 22L2 12L12 2Z"/>
				</svg>
			</div>
			<div class="reward-info">
				<div class="reward-title">Total Pending Rewards</div>
				<div class="reward-amount">{lpStatus.total_pending_rewards} NOX</div>
			</div>
			<button class="btn primary" on:click={handleClaimAll} disabled={lockLoading}>
				{lockLoading ? 'Claiming...' : 'Claim All'}
			</button>
		</div>
	</div>

	<div class="rewards-breakdown">
		<div class="breakdown-card">
			<h4>LP Staking Rewards</h4>
			<p class="breakdown-desc">30% of daily emissions distributed to LP lockers</p>
			<div class="breakdown-stat">
				<span>Your Share</span>
				<span>{lpStatus.total_pending_rewards} NOX</span>
			</div>
		</div>

		<div class="breakdown-card">
			<h4>Node Work Rewards</h4>
			<p class="breakdown-desc">70% of daily emissions distributed by work score</p>
			<div class="breakdown-stat">
				<span>Estimated This Epoch</span>
				<span>{estimatedReward} NOX</span>
			</div>
		</div>
	</div>

	<div class="emission-info">
		<h4>Daily Emission Schedule</h4>
		<div class="emission-stats">
			<div class="emission-stat">
				<span class="stat-label">Total Daily</span>
				<span class="stat-value">54,794.52 NOX</span>
			</div>
			<div class="emission-stat">
				<span class="stat-label">To Nodes (70%)</span>
				<span class="stat-value">38,356.16 NOX</span>
			</div>
			<div class="emission-stat">
				<span class="stat-label">To LPs (30%)</span>
				<span class="stat-value">16,438.36 NOX</span>
			</div>
			<div class="emission-stat">
				<span class="stat-label">Per Epoch (7 days)</span>
				<span class="stat-value">383,561.64 NOX</span>
			</div>
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

	.btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.rewards-overview {
		margin-bottom: var(--nox-space-xl);
	}

	.reward-card {
		display: flex;
		align-items: center;
		gap: var(--nox-space-lg);
		background: var(--nox-bg-tertiary);
		border-radius: var(--nox-radius-xl);
		padding: var(--nox-space-xl);
	}

	.reward-card.total {
		background: linear-gradient(135deg, var(--nox-accent-glow) 0%, var(--nox-bg-tertiary) 100%);
		border: 1px solid var(--nox-accent-primary);
	}

	.reward-icon {
		width: 56px;
		height: 56px;
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--nox-accent-gradient);
		border-radius: var(--nox-radius-full);
		color: var(--nox-bg-primary);
	}

	.reward-icon svg {
		width: 28px;
		height: 28px;
	}

	.reward-info {
		flex: 1;
	}

	.reward-title {
		font-size: var(--nox-text-sm);
		color: var(--nox-text-muted);
		margin-bottom: var(--nox-space-xs);
	}

	.reward-amount {
		font-size: var(--nox-text-2xl);
		font-weight: var(--nox-font-bold);
	}

	.rewards-breakdown {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: var(--nox-space-md);
		margin-bottom: var(--nox-space-xl);
	}

	.breakdown-card {
		background: var(--nox-bg-tertiary);
		border-radius: var(--nox-radius-lg);
		padding: var(--nox-space-lg);
	}

	.breakdown-card h4 {
		font-size: var(--nox-text-base);
		margin-bottom: var(--nox-space-xs);
	}

	.breakdown-desc {
		font-size: var(--nox-text-xs);
		color: var(--nox-text-muted);
		margin-bottom: var(--nox-space-md);
	}

	.breakdown-stat {
		display: flex;
		justify-content: space-between;
		font-size: var(--nox-text-sm);
	}

	.breakdown-stat span:last-child {
		font-weight: var(--nox-font-semibold);
		color: var(--nox-accent-primary);
	}

	.emission-info {
		background: var(--nox-bg-tertiary);
		border-radius: var(--nox-radius-lg);
		padding: var(--nox-space-lg);
	}

	.emission-info h4 {
		font-size: var(--nox-text-base);
		margin-bottom: var(--nox-space-md);
	}

	.emission-stats {
		display: grid;
		grid-template-columns: repeat(4, 1fr);
		gap: var(--nox-space-md);
	}

	.emission-stat {
		text-align: center;
	}

	.stat-label {
		display: block;
		font-size: var(--nox-text-xs);
		color: var(--nox-text-muted);
		margin-bottom: var(--nox-space-xs);
	}

	.stat-value {
		font-size: var(--nox-text-sm);
		font-weight: var(--nox-font-semibold);
		font-family: var(--nox-font-mono);
	}

	@media (max-width: 800px) {
		.emission-stats {
			grid-template-columns: repeat(2, 1fr);
		}
	}

	@media (max-width: 500px) {
		.rewards-breakdown {
			grid-template-columns: 1fr;
		}
	}
</style>
