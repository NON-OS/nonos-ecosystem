<script lang="ts">
	import type { WorkMetrics, WorkCategory } from './types';
	import { formatBytes, formatNumber } from './types';

	export let workMetrics: WorkMetrics | null = null;
	export let workCategories: WorkCategory[] = [];
	export let estimatedReward = '0';
	export let epochProgress = 0;
	export let epochTimeRemaining = '';
</script>

<div class="section">
	<div class="section-header">
		<h2>Work Metrics</h2>
		<p>Your node's privacy work score determines 70% of your epoch rewards</p>
	</div>

	{#if workMetrics}
		<div class="epoch-progress">
			<div class="epoch-header">
				<span>Epoch {workMetrics.epoch.current_epoch}</span>
				<span class="epoch-time">{epochTimeRemaining} remaining</span>
			</div>
			<div class="progress-bar">
				<div class="progress-fill" style="width: {epochProgress}%"></div>
			</div>
		</div>

		<div class="work-score-card">
			<div class="score-circle">
				<svg viewBox="0 0 100 100">
					<circle cx="50" cy="50" r="45" fill="none" stroke="var(--nox-border)" stroke-width="8" />
					<circle
						cx="50" cy="50" r="45" fill="none"
						stroke="url(#scoreGradient)" stroke-width="8"
						stroke-dasharray="{workMetrics.total_work_score * 2.83} 283"
						stroke-linecap="round"
						transform="rotate(-90 50 50)"
					/>
					<defs>
						<linearGradient id="scoreGradient">
							<stop offset="0%" stop-color="var(--nox-accent-primary)" />
							<stop offset="100%" stop-color="var(--nox-success)" />
						</linearGradient>
					</defs>
				</svg>
				<div class="score-text">
					<span class="score-value">{workMetrics.total_work_score.toFixed(1)}</span>
					<span class="score-label">Work Score</span>
				</div>
			</div>
			<div class="estimated-reward">
				<span class="reward-label">Estimated Epoch Reward</span>
				<span class="reward-value">{estimatedReward} NOX</span>
			</div>
		</div>

		<div class="work-categories">
			{#each workCategories as cat}
				<div class="category-card">
					<div class="category-header">
						<span class="category-name">{cat.name}</span>
						<span class="category-weight">{cat.weight}%</span>
					</div>
					<div class="category-bar">
						<div class="category-fill" style="width: {cat.score}%"></div>
					</div>
					<div class="category-stats">
						<span>Score: {cat.score.toFixed(1)}</span>
						<span>Raw: {formatNumber(cat.raw_value)}</span>
					</div>
				</div>
			{/each}
		</div>

		<div class="work-details">
			<div class="detail-card">
				<h4>Traffic Relay</h4>
				<div class="detail-row">
					<span>Bytes Relayed</span>
					<span>{formatBytes(workMetrics.traffic_relay.bytes_relayed)}</span>
				</div>
				<div class="detail-row">
					<span>Sessions</span>
					<span>{formatNumber(workMetrics.traffic_relay.relay_sessions)}</span>
				</div>
				<div class="detail-row">
					<span>Success Rate</span>
					<span>{workMetrics.traffic_relay.relay_sessions > 0 ? ((workMetrics.traffic_relay.successful_relays / workMetrics.traffic_relay.relay_sessions) * 100).toFixed(1) : 0}%</span>
				</div>
			</div>

			<div class="detail-card">
				<h4>ZK Proofs</h4>
				<div class="detail-row">
					<span>Generated</span>
					<span>{formatNumber(workMetrics.zk_proofs.proofs_generated)}</span>
				</div>
				<div class="detail-row">
					<span>Verified</span>
					<span>{formatNumber(workMetrics.zk_proofs.proofs_verified)}</span>
				</div>
				<div class="detail-row">
					<span>Avg Gen Time</span>
					<span>{workMetrics.zk_proofs.avg_generation_time_ms.toFixed(0)}ms</span>
				</div>
			</div>

			<div class="detail-card">
				<h4>Mixer Operations</h4>
				<div class="detail-row">
					<span>Deposits</span>
					<span>{formatNumber(workMetrics.mixer_ops.deposits_processed)}</span>
				</div>
				<div class="detail-row">
					<span>Spends</span>
					<span>{formatNumber(workMetrics.mixer_ops.spends_processed)}</span>
				</div>
				<div class="detail-row">
					<span>Pool Participations</span>
					<span>{formatNumber(workMetrics.mixer_ops.pool_participations)}</span>
				</div>
			</div>

			<div class="detail-card">
				<h4>Entropy</h4>
				<div class="detail-row">
					<span>Bytes Contributed</span>
					<span>{formatBytes(workMetrics.entropy.entropy_bytes_contributed)}</span>
				</div>
				<div class="detail-row">
					<span>Requests Served</span>
					<span>{formatNumber(workMetrics.entropy.entropy_requests_served)}</span>
				</div>
				<div class="detail-row">
					<span>Quality Score</span>
					<span>{workMetrics.entropy.quality_score.toFixed(1)}</span>
				</div>
			</div>
		</div>
	{:else}
		<div class="loading-state">
			<p>Loading work metrics...</p>
		</div>
	{/if}
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

	.epoch-progress {
		margin-bottom: var(--nox-space-xl);
	}

	.epoch-header {
		display: flex;
		justify-content: space-between;
		margin-bottom: var(--nox-space-sm);
		font-size: var(--nox-text-sm);
	}

	.epoch-time {
		color: var(--nox-text-muted);
	}

	.progress-bar {
		height: 8px;
		background: var(--nox-bg-tertiary);
		border-radius: var(--nox-radius-full);
		overflow: hidden;
	}

	.progress-fill {
		height: 100%;
		background: var(--nox-accent-gradient);
		border-radius: var(--nox-radius-full);
		transition: width 0.3s ease;
	}

	.work-score-card {
		display: flex;
		align-items: center;
		gap: var(--nox-space-xl);
		background: var(--nox-bg-tertiary);
		border-radius: var(--nox-radius-xl);
		padding: var(--nox-space-xl);
		margin-bottom: var(--nox-space-xl);
	}

	.score-circle {
		position: relative;
		width: 160px;
		height: 160px;
	}

	.score-circle svg {
		width: 100%;
		height: 100%;
	}

	.score-text {
		position: absolute;
		inset: 0;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
	}

	.score-value {
		font-size: var(--nox-text-3xl);
		font-weight: var(--nox-font-bold);
		color: var(--nox-accent-primary);
	}

	.score-label {
		font-size: var(--nox-text-xs);
		color: var(--nox-text-muted);
	}

	.estimated-reward {
		flex: 1;
		text-align: center;
	}

	.reward-label {
		display: block;
		font-size: var(--nox-text-sm);
		color: var(--nox-text-muted);
		margin-bottom: var(--nox-space-xs);
	}

	.reward-value {
		font-size: var(--nox-text-2xl);
		font-weight: var(--nox-font-bold);
		color: var(--nox-success);
	}

	.work-categories {
		display: grid;
		grid-template-columns: repeat(5, 1fr);
		gap: var(--nox-space-md);
		margin-bottom: var(--nox-space-xl);
	}

	.category-card {
		background: var(--nox-bg-tertiary);
		border-radius: var(--nox-radius-lg);
		padding: var(--nox-space-md);
	}

	.category-header {
		display: flex;
		justify-content: space-between;
		margin-bottom: var(--nox-space-sm);
	}

	.category-name {
		font-size: var(--nox-text-xs);
		font-weight: var(--nox-font-medium);
	}

	.category-weight {
		font-size: var(--nox-text-xs);
		color: var(--nox-text-muted);
	}

	.category-bar {
		height: 6px;
		background: var(--nox-bg-primary);
		border-radius: var(--nox-radius-full);
		overflow: hidden;
		margin-bottom: var(--nox-space-sm);
	}

	.category-fill {
		height: 100%;
		background: var(--nox-accent-primary);
		border-radius: var(--nox-radius-full);
	}

	.category-stats {
		display: flex;
		justify-content: space-between;
		font-size: var(--nox-text-xs);
		color: var(--nox-text-muted);
	}

	.work-details {
		display: grid;
		grid-template-columns: repeat(4, 1fr);
		gap: var(--nox-space-md);
	}

	.detail-card {
		background: var(--nox-bg-tertiary);
		border-radius: var(--nox-radius-lg);
		padding: var(--nox-space-md);
	}

	.detail-card h4 {
		font-size: var(--nox-text-sm);
		margin-bottom: var(--nox-space-md);
	}

	.detail-row {
		display: flex;
		justify-content: space-between;
		font-size: var(--nox-text-xs);
		padding: var(--nox-space-xs) 0;
		border-bottom: 1px solid var(--nox-border);
	}

	.detail-row:last-child {
		border-bottom: none;
	}

	.detail-row span:first-child {
		color: var(--nox-text-muted);
	}

	.loading-state {
		text-align: center;
		padding: var(--nox-space-xl);
		color: var(--nox-text-muted);
	}

	@media (max-width: 800px) {
		.work-categories {
			grid-template-columns: repeat(2, 1fr);
		}

		.work-details {
			grid-template-columns: repeat(2, 1fr);
		}
	}

	@media (max-width: 500px) {
		.work-categories {
			grid-template-columns: 1fr;
		}

		.work-details {
			grid-template-columns: 1fr;
		}
	}
</style>
