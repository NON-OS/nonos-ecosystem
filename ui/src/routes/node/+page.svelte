<script lang="ts">
	import { onMount } from 'svelte';

	let nodeRunning = false;
	let nodeStatus = { running: false, connected_nodes: 0, quality: 0, total_requests: 0 };
	let stakingStatus = {
		staked_amount: '0',
		tier: 'None',
		tier_multiplier: '0x',
		pending_rewards: '0',
		current_epoch: 0,
		next_tier_threshold: '1,000 NOX',
		estimated_apy: '0%'
	};
	let stakeAmount = '';
	let isLoading = false;
	let startTime = Date.now();

	const tiers = [
		{ name: 'Bronze', min: '1,000', lock: '0', apy: '5-8%' },
		{ name: 'Silver', min: '10,000', lock: '30', apy: '8-12%' },
		{ name: 'Gold', min: '50,000', lock: '90', apy: '12-18%' },
		{ name: 'Platinum', min: '200,000', lock: '180', apy: '18-25%' },
		{ name: 'Diamond', min: '1,000,000', lock: '365', apy: '25-35%' }
	];

	onMount(() => {
		updateStatus();
		const interval = setInterval(updateStatus, 5000);
		return () => clearInterval(interval);
	});

	async function updateStatus() {
		if (!window.nonos) return;
		try {
			// Get node status
			nodeStatus = await window.nonos.node.getStatus();
			nodeRunning = nodeStatus.running;

			// Get staking status
			try {
				stakingStatus = await window.nonos.staking.getStatus();
			} catch (e) {
				// Wallet may not be initialized yet
			}
		} catch (e) {
			console.error('Failed to get status:', e);
		}
	}

	async function toggleNode() {
		if (!window.nonos) return;
		isLoading = true;
		try {
			if (nodeRunning) {
				await window.nonos.node.stopEmbedded();
			} else {
				await window.nonos.node.startEmbedded();
				startTime = Date.now();
			}
			await updateStatus();
		} catch (e) {
			console.error('Failed to toggle node:', e);
		} finally {
			isLoading = false;
		}
	}

	async function stake() {
		if (!window.nonos || !stakeAmount) return;
		isLoading = true;
		try {
			await window.nonos.staking.stake(stakeAmount);
			stakeAmount = '';
			await updateStatus();
		} catch (e) {
			console.error('Failed to stake:', e);
		} finally {
			isLoading = false;
		}
	}

	async function claimRewards() {
		if (!window.nonos) return;
		isLoading = true;
		try {
			await window.nonos.staking.claimRewards();
			await updateStatus();
		} catch (e) {
			console.error('Failed to claim rewards:', e);
		} finally {
			isLoading = false;
		}
	}

	function formatUptime(): string {
		if (!nodeRunning) return '0h 0m';
		const seconds = Math.floor((Date.now() - startTime) / 1000);
		const hours = Math.floor(seconds / 3600);
		const mins = Math.floor((seconds % 3600) / 60);
		return `${hours}h ${mins}m`;
	}
</script>

<div class="node-page">
	<h1>Node & Staking</h1>

	<div class="node-status">
		<div class="status-header">
			<h2>Node Status</h2>
			<div class="status-indicator" class:running={nodeRunning}>
				<span class="dot"></span>
				<span>{nodeRunning ? 'Running' : 'Stopped'}</span>
			</div>
		</div>

		<div class="metrics-grid">
			<div class="metric-card">
				<div class="metric-label">Uptime</div>
				<div class="metric-value">{formatUptime()}</div>
			</div>
			<div class="metric-card">
				<div class="metric-label">Requests Served</div>
				<div class="metric-value">{nodeStatus.total_requests.toLocaleString()}</div>
			</div>
			<div class="metric-card">
				<div class="metric-label">Staked NOX</div>
				<div class="metric-value">{stakingStatus.staked_amount} NOX</div>
			</div>
			<div class="metric-card">
				<div class="metric-label">Current Tier</div>
				<div class="metric-value tier">{stakingStatus.tier}</div>
			</div>
			<div class="metric-card">
				<div class="metric-label">Pending Rewards</div>
				<div class="metric-value reward">{stakingStatus.pending_rewards} NOX</div>
			</div>
			<div class="metric-card">
				<div class="metric-label">Est. APY</div>
				<div class="metric-value apy">{stakingStatus.estimated_apy}</div>
			</div>
		</div>

		<button class="btn toggle" class:stop={nodeRunning} on:click={toggleNode} disabled={isLoading}>
			{isLoading ? 'Processing...' : nodeRunning ? 'Stop Node' : 'Start Node'}
		</button>
	</div>

	<div class="staking-section">
		<h2>Staking Tiers</h2>
		<p class="section-desc">
			Stake NOX tokens to run a community node and earn rewards. Higher tiers unlock better APY rates with stake-weighted distribution (sqrt formula to prevent whale dominance).
		</p>

		<div class="tiers-grid">
			{#each tiers as tier}
				<div class="tier-card">
					<div class="tier-name">{tier.name}</div>
					<div class="tier-details">
						<div class="tier-row">
							<span class="label">Minimum Stake</span>
							<span class="value">{tier.min} NOX</span>
						</div>
						<div class="tier-row">
							<span class="label">Lock Period</span>
							<span class="value">{tier.lock === '0' ? 'None' : `${tier.lock} days`}</span>
						</div>
						<div class="tier-row">
							<span class="label">APY Range</span>
							<span class="value apy">{tier.apy}</span>
						</div>
					</div>
				</div>
			{/each}
		</div>
	</div>

	<div class="stake-form">
		<h2>Stake NOX</h2>
		<div class="form-row">
			<input
				type="text"
				bind:value={stakeAmount}
				placeholder="Amount to stake..."
				class="input"
			/>
			<button class="btn primary" on:click={stake} disabled={isLoading || !stakeAmount}>
				Stake
			</button>
		</div>
		<button class="btn secondary" on:click={claimRewards} disabled={isLoading}>
			Claim Pending Rewards
		</button>
	</div>

	<div class="info-section">
		<h3>How Rewards Work</h3>
		<ul>
			<li><strong>Epoch Duration:</strong> 24 hours</li>
			<li><strong>Total Staking Pool:</strong> 32,000,000 NOX (4% of supply)</li>
			<li><strong>Distribution Formula:</strong> sqrt(stake) × tier_multiplier × quality_score</li>
			<li><strong>Yearly Decay:</strong> 15% emission reduction per year</li>
			<li><strong>Quality Score:</strong> Based on uptime, success rate, latency, and reliability</li>
		</ul>
	</div>
</div>

<style>
	.node-page {
		max-width: 900px;
		margin: 0 auto;
	}

	h1 {
		font-size: 28px;
		font-weight: 700;
		margin-bottom: var(--nox-space-lg);
	}

	h2 {
		font-size: 18px;
		font-weight: 600;
		margin-bottom: var(--nox-space-md);
	}

	.section-desc {
		color: var(--nox-text-secondary);
		font-size: 14px;
		margin-bottom: var(--nox-space-lg);
	}

	.node-status {
		background: var(--nox-bg-secondary);
		border: 1px solid var(--nox-border);
		border-radius: var(--nox-radius-lg);
		padding: var(--nox-space-lg);
		margin-bottom: var(--nox-space-xl);
	}

	.status-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: var(--nox-space-lg);
	}

	.status-header h2 {
		margin-bottom: 0;
	}

	.status-indicator {
		display: flex;
		align-items: center;
		gap: var(--nox-space-sm);
		padding: var(--nox-space-xs) var(--nox-space-md);
		background: rgba(239, 68, 68, 0.1);
		border-radius: var(--nox-radius-full);
		font-size: 13px;
		color: var(--nox-error);
	}

	.status-indicator.running {
		background: rgba(34, 197, 94, 0.1);
		color: var(--nox-success);
	}

	.status-indicator .dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: currentColor;
	}

	.metrics-grid {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: var(--nox-space-md);
		margin-bottom: var(--nox-space-lg);
	}

	.metric-value.tier {
		color: var(--nox-accent-primary);
	}

	.metric-value.reward {
		color: var(--nox-success);
	}

	.metric-value.apy {
		color: var(--nox-warning);
	}

	.metric-card {
		background: var(--nox-bg-tertiary);
		padding: var(--nox-space-md);
		border-radius: var(--nox-radius-md);
	}

	.metric-label {
		font-size: 12px;
		color: var(--nox-text-muted);
		margin-bottom: var(--nox-space-xs);
	}

	.metric-value {
		font-size: 20px;
		font-weight: 600;
		font-family: var(--nox-font-mono);
	}

	.btn.toggle {
		width: 100%;
		padding: var(--nox-space-md);
		background: var(--nox-success);
		color: white;
		border-radius: var(--nox-radius-md);
		font-weight: 600;
		transition: all var(--nox-transition-fast);
	}

	.btn.toggle:hover:not(:disabled) {
		filter: brightness(1.1);
	}

	.btn.toggle.stop {
		background: var(--nox-error);
	}

	.staking-section {
		margin-bottom: var(--nox-space-xl);
	}

	.tiers-grid {
		display: grid;
		grid-template-columns: repeat(5, 1fr);
		gap: var(--nox-space-md);
	}

	.tier-card {
		background: var(--nox-bg-secondary);
		border: 1px solid var(--nox-border);
		border-radius: var(--nox-radius-lg);
		padding: var(--nox-space-md);
		transition: all var(--nox-transition-fast);
	}

	.tier-card:hover {
		border-color: var(--nox-accent-primary);
	}

	.tier-name {
		font-weight: 600;
		font-size: 14px;
		margin-bottom: var(--nox-space-md);
		text-align: center;
	}

	.tier-details {
		display: flex;
		flex-direction: column;
		gap: var(--nox-space-xs);
	}

	.tier-row {
		display: flex;
		justify-content: space-between;
		font-size: 11px;
	}

	.tier-row .label {
		color: var(--nox-text-muted);
	}

	.tier-row .value {
		font-family: var(--nox-font-mono);
	}

	.tier-row .value.apy {
		color: var(--nox-success);
	}

	.stake-form {
		background: var(--nox-bg-secondary);
		border: 1px solid var(--nox-border);
		border-radius: var(--nox-radius-lg);
		padding: var(--nox-space-lg);
		margin-bottom: var(--nox-space-xl);
	}

	.form-row {
		display: flex;
		gap: var(--nox-space-md);
		margin-bottom: var(--nox-space-md);
	}

	.form-row .input {
		flex: 1;
	}

	.btn {
		padding: var(--nox-space-sm) var(--nox-space-lg);
		border-radius: var(--nox-radius-md);
		font-weight: 500;
		transition: all var(--nox-transition-fast);
	}

	.btn.primary {
		background: var(--nox-accent-gradient);
		color: white;
	}

	.btn.secondary {
		background: var(--nox-bg-tertiary);
		border: 1px solid var(--nox-border);
		width: 100%;
	}

	.btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.info-section {
		background: var(--nox-bg-secondary);
		border: 1px solid var(--nox-border);
		border-radius: var(--nox-radius-lg);
		padding: var(--nox-space-lg);
	}

	.info-section h3 {
		font-size: 16px;
		margin-bottom: var(--nox-space-md);
	}

	.info-section ul {
		list-style: none;
	}

	.info-section li {
		padding: var(--nox-space-xs) 0;
		font-size: 13px;
		color: var(--nox-text-secondary);
	}

	.info-section li strong {
		color: var(--nox-text-primary);
	}
</style>
