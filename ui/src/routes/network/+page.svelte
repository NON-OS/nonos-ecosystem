<script lang="ts">
	import { onMount } from 'svelte';

	let networkStatus = { connected: false, bootstrap_progress: 0, circuits: 0, status: 'Disconnected', error: null as string | null };
	let circuits: Array<{ id: string; path: string[] }> = [];
	let isConnecting = false;

	onMount(() => {
		updateStatus();
		const interval = setInterval(updateStatus, 3000);

		// Listen for real-time network status updates
		if (window.nonos?.onNetworkStatus) {
			window.nonos.onNetworkStatus((status: typeof networkStatus) => {
				networkStatus = status;
				isConnecting = status.status === 'Connecting' || status.status === 'Bootstrapping';
			});
		}

		return () => clearInterval(interval);
	});

	async function updateStatus() {
		if (!window.nonos) return;
		try {
			const status = await window.nonos.network.getStatus();
			networkStatus = status;
			isConnecting = status.status === 'Connecting' || status.status === 'Bootstrapping';
		} catch (e) {
			console.error('Failed to get status:', e);
		}
	}

	async function connect() {
		if (!window.nonos) return;
		isConnecting = true;
		try {
			await window.nonos.network.connect();
			await updateStatus();
		} catch (e) {
			console.error('Failed to connect:', e);
		} finally {
			isConnecting = false;
		}
	}

	async function disconnect() {
		if (!window.nonos) return;
		try {
			await window.nonos.network.disconnect();
			await updateStatus();
		} catch (e) {
			console.error('Failed to disconnect:', e);
		}
	}

	async function newIdentity() {
		if (!window.nonos) return;
		try {
			await window.nonos.network.newIdentity();
			await updateStatus();
			// Add a visual indicator that identity changed
			circuits = [];
		} catch (e) {
			console.error('Failed to create new identity:', e);
		}
	}
</script>

<div class="network-page">
	<h1>Network</h1>

	<div class="connection-card">
		<div class="connection-header">
			<div class="status-badge" class:connected={networkStatus.connected}>
				<span class="dot"></span>
				{networkStatus.connected ? 'Connected to Anyone Network' : 'Disconnected'}
			</div>
		</div>

		{#if !networkStatus.connected && networkStatus.bootstrap_progress > 0}
			<div class="bootstrap-section">
				<div class="bootstrap-label">Bootstrapping... {networkStatus.status}</div>
				<div class="progress-bar">
					<div class="progress-fill" style="width: {networkStatus.bootstrap_progress}%"></div>
				</div>
				<div class="progress-text">{networkStatus.bootstrap_progress}%</div>
			</div>
		{/if}

		{#if networkStatus.error}
			<div class="error-section">
				<span class="error-text">{networkStatus.error}</span>
			</div>
		{/if}

		<div class="connection-stats">
			<div class="stat">
				<div class="stat-label">Active Circuits</div>
				<div class="stat-value">{networkStatus.circuits}</div>
			</div>
		</div>

		<div class="connection-actions">
			{#if networkStatus.connected}
				<button class="btn danger" on:click={disconnect}>Disconnect</button>
				<button class="btn secondary" on:click={newIdentity}>New Identity</button>
			{:else}
				<button class="btn primary" on:click={connect} disabled={isConnecting}>
					{isConnecting ? 'Connecting...' : 'Connect to Anyone Network'}
				</button>
			{/if}
		</div>
	</div>

	<div class="circuits-section">
		<h2>Active Circuits</h2>
		<p class="section-desc">
			Circuits route your traffic through multiple relays for privacy. Each circuit provides a unique identity.
		</p>

		{#if circuits.length === 0}
			<div class="empty-state">
				<p>No active circuits. Connect to the network to start browsing privately.</p>
			</div>
		{:else}
			<div class="circuits-list">
				{#each circuits as circuit}
					<div class="circuit-card">
						<div class="circuit-id">{circuit.id}</div>
						<div class="circuit-path">
							{#each circuit.path as relay, i}
								<span class="relay">{relay}</span>
								{#if i < circuit.path.length - 1}
									<span class="arrow">→</span>
								{/if}
							{/each}
						</div>
					</div>
				{/each}
			</div>
		{/if}
	</div>

	<div class="info-card">
		<h3>About Anyone Network</h3>
		<p>
			Anyone Network provides anonymous routing through a decentralized network of relays.
			Your traffic is encrypted and routed through multiple hops, preventing surveillance
			and protecting your privacy.
		</p>
		<ul>
			<li>Multi-hop encryption (3+ relays per circuit)</li>
			<li>Automatic circuit rotation every 10 minutes</li>
			<li>No connection logs or traffic analysis</li>
			<li>Exit node diversity for maximum privacy</li>
		</ul>
	</div>
</div>

<style>
	.network-page {
		max-width: 800px;
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
		margin-bottom: var(--nox-space-sm);
	}

	.section-desc {
		color: var(--nox-text-secondary);
		font-size: 14px;
		margin-bottom: var(--nox-space-lg);
	}

	.connection-card {
		background: var(--nox-bg-secondary);
		border: 1px solid var(--nox-border);
		border-radius: var(--nox-radius-lg);
		padding: var(--nox-space-lg);
		margin-bottom: var(--nox-space-xl);
	}

	.connection-header {
		margin-bottom: var(--nox-space-lg);
	}

	.status-badge {
		display: inline-flex;
		align-items: center;
		gap: var(--nox-space-sm);
		padding: var(--nox-space-sm) var(--nox-space-md);
		background: rgba(239, 68, 68, 0.1);
		border-radius: var(--nox-radius-full);
		font-size: 14px;
		color: var(--nox-error);
	}

	.status-badge.connected {
		background: rgba(34, 197, 94, 0.1);
		color: var(--nox-success);
	}

	.status-badge .dot {
		width: 10px;
		height: 10px;
		border-radius: 50%;
		background: currentColor;
	}

	.bootstrap-section {
		margin-bottom: var(--nox-space-lg);
	}

	.error-section {
		background: rgba(239, 68, 68, 0.1);
		border: 1px solid var(--nox-error);
		border-radius: var(--nox-radius-md);
		padding: var(--nox-space-md);
		margin-bottom: var(--nox-space-lg);
	}

	.error-text {
		color: var(--nox-error);
		font-size: 13px;
		font-family: var(--nox-font-mono);
	}

	.bootstrap-label {
		font-size: 13px;
		color: var(--nox-text-secondary);
		margin-bottom: var(--nox-space-xs);
	}

	.progress-bar {
		height: 8px;
		background: var(--nox-bg-tertiary);
		border-radius: var(--nox-radius-full);
		overflow: hidden;
		margin-bottom: var(--nox-space-xs);
	}

	.progress-fill {
		height: 100%;
		background: var(--nox-accent-gradient);
		transition: width var(--nox-transition-normal);
	}

	.progress-text {
		font-size: 12px;
		color: var(--nox-text-muted);
		font-family: var(--nox-font-mono);
	}

	.connection-stats {
		display: flex;
		gap: var(--nox-space-lg);
		margin-bottom: var(--nox-space-lg);
	}

	.stat {
		background: var(--nox-bg-tertiary);
		padding: var(--nox-space-md);
		border-radius: var(--nox-radius-md);
		flex: 1;
	}

	.stat-label {
		font-size: 12px;
		color: var(--nox-text-muted);
		margin-bottom: var(--nox-space-xs);
	}

	.stat-value {
		font-size: 24px;
		font-weight: 600;
		font-family: var(--nox-font-mono);
	}

	.connection-actions {
		display: flex;
		gap: var(--nox-space-md);
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
		flex: 1;
	}

	.btn.secondary {
		background: var(--nox-bg-tertiary);
		border: 1px solid var(--nox-border);
	}

	.btn.danger {
		background: var(--nox-error);
		color: white;
	}

	.btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.circuits-section {
		margin-bottom: var(--nox-space-xl);
	}

	.empty-state {
		background: var(--nox-bg-secondary);
		border: 1px dashed var(--nox-border);
		border-radius: var(--nox-radius-lg);
		padding: var(--nox-space-xl);
		text-align: center;
		color: var(--nox-text-muted);
	}

	.circuits-list {
		display: flex;
		flex-direction: column;
		gap: var(--nox-space-sm);
	}

	.circuit-card {
		background: var(--nox-bg-secondary);
		border: 1px solid var(--nox-border);
		border-radius: var(--nox-radius-md);
		padding: var(--nox-space-md);
	}

	.circuit-id {
		font-size: 12px;
		font-family: var(--nox-font-mono);
		color: var(--nox-text-muted);
		margin-bottom: var(--nox-space-xs);
	}

	.circuit-path {
		display: flex;
		align-items: center;
		gap: var(--nox-space-sm);
		flex-wrap: wrap;
	}

	.relay {
		background: var(--nox-bg-tertiary);
		padding: var(--nox-space-xs) var(--nox-space-sm);
		border-radius: var(--nox-radius-sm);
		font-size: 12px;
		font-family: var(--nox-font-mono);
	}

	.arrow {
		color: var(--nox-text-muted);
	}

	.info-card {
		background: var(--nox-bg-secondary);
		border: 1px solid var(--nox-border);
		border-radius: var(--nox-radius-lg);
		padding: var(--nox-space-lg);
	}

	.info-card h3 {
		font-size: 16px;
		margin-bottom: var(--nox-space-md);
	}

	.info-card p {
		color: var(--nox-text-secondary);
		font-size: 14px;
		margin-bottom: var(--nox-space-md);
	}

	.info-card ul {
		list-style: none;
	}

	.info-card li {
		padding: var(--nox-space-xs) 0;
		font-size: 13px;
		color: var(--nox-text-secondary);
	}

	.info-card li {
		display: flex;
		align-items: center;
		gap: var(--nox-space-sm);
	}

	.info-card li::before {
		content: '';
		width: 16px;
		height: 16px;
		background: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='%2300ff88' stroke-width='2'%3E%3Cpolyline points='20 6 9 17 4 12'/%3E%3C/svg%3E") center/contain no-repeat;
		flex-shrink: 0;
	}
</style>
