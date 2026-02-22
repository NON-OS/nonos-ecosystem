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

	// LP Lock Duration Tiers (from NONOS Privacy Infrastructure Economy whitepaper)
	const lpTiers = [
		{ duration: '14 days', multiplier: '1.00x', boost: 'Baseline' },
		{ duration: '30 days', multiplier: '1.25x', boost: '+25%' },
		{ duration: '90 days', multiplier: '1.60x', boost: '+60%' },
		{ duration: '180 days', multiplier: '2.00x', boost: '+100%' },
		{ duration: '365 days', multiplier: '2.50x', boost: '+150%' }
	];

	// Work Categories (from whitepaper)
	const workCategories = [
		{ name: 'Traffic Relay', weight: '30%', desc: 'Bandwidth relayed through circuits' },
		{ name: 'ZK Proof Generation', weight: '25%', desc: 'Zero-knowledge proofs computed' },
		{ name: 'Mixer Operations', weight: '20%', desc: 'Participation in mixing rounds' },
		{ name: 'Entropy Provision', weight: '15%', desc: 'Verifiable randomness contributions' },
		{ name: 'Registry Operations', weight: '10%', desc: 'Stealth address registrations' }
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
		<h2>Liquidity Pool Lock Tiers</h2>
		<p class="section-desc">
			Lock NOX in the Privacy Liquidity Pool to earn 30% of protocol rewards. Longer lock durations receive higher reward multipliers. Node operators earn the other 70% based on work performed.
		</p>

		<div class="tiers-grid">
			{#each lpTiers as tier}
				<div class="tier-card">
					<div class="tier-name">{tier.duration}</div>
					<div class="tier-details">
						<div class="tier-row">
							<span class="label">Multiplier</span>
							<span class="value multiplier">{tier.multiplier}</span>
						</div>
						<div class="tier-row">
							<span class="label">Effective Boost</span>
							<span class="value boost">{tier.boost}</span>
						</div>
					</div>
				</div>
			{/each}
		</div>
	</div>

	<div class="work-section">
		<h2>Work Categories</h2>
		<p class="section-desc">
			Node operators earn rewards based on measurable privacy work performed, not tokens staked. Work is tracked across five categories:
		</p>

		<div class="work-grid">
			{#each workCategories as category}
				<div class="work-card">
					<div class="work-header">
						<span class="work-name">{category.name}</span>
						<span class="work-weight">{category.weight}</span>
					</div>
					<div class="work-desc">{category.desc}</div>
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
		<h3>Privacy Infrastructure Economy</h3>
		<ul>
			<li><strong>Epoch Duration:</strong> 7 days</li>
			<li><strong>Bootstrap Pool:</strong> 40,000,000 NOX (5% of 800M supply)</li>
			<li><strong>Daily Emissions:</strong> 54,794 NOX for 730 days (2 years)</li>
			<li><strong>Reward Split:</strong> 70% to node operators, 30% to liquidity providers</li>
			<li><strong>Node Distribution:</strong> Proportional to work score (work performed, not tokens held)</li>
			<li><strong>LP Distribution:</strong> Proportional to weighted lock amount (duration × multiplier)</li>
			<li><strong>Core Principle:</strong> Work creates value. Value creates rewards.</li>
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

	.tier-row .value.multiplier {
		color: var(--nox-accent-primary);
		font-weight: 600;
	}

	.tier-row .value.boost {
		color: var(--nox-success);
	}

	/* Work Section */
	.work-section {
		margin-bottom: var(--nox-space-xl);
	}

	.work-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
		gap: var(--nox-space-md);
	}

	.work-card {
		background: var(--nox-bg-secondary);
		border: 1px solid var(--nox-border);
		border-radius: var(--nox-radius-lg);
		padding: var(--nox-space-md);
		transition: all var(--nox-transition-fast);
	}

	.work-card:hover {
		border-color: var(--nox-accent-primary);
	}

	.work-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: var(--nox-space-sm);
	}

	.work-name {
		font-weight: 600;
		font-size: 13px;
	}

	.work-weight {
		font-family: var(--nox-font-mono);
		font-size: 12px;
		font-weight: 600;
		color: var(--nox-accent-primary);
		background: var(--nox-accent-glow);
		padding: 2px 8px;
		border-radius: var(--nox-radius-sm);
	}

	.work-desc {
		font-size: 11px;
		color: var(--nox-text-muted);
		line-height: 1.4;
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
