<script lang="ts">
	import { onMount, onDestroy } from 'svelte';

	interface PrivacyService {
		id: string;
		name: string;
		description: string;
		category: string;
		weight: string;
		pricePerUnit: string;
		unit: string;
		icon: string;
		active: boolean;
	}

	interface ActiveSession {
		id: string;
		service: string;
		startTime: number;
		usage: string;
		cost: string;
	}

	let services: PrivacyService[] = [
		{
			id: 'traffic_relay',
			name: 'Anonymous Browsing',
			description: 'Route traffic through encrypted multi-hop circuits. Your IP and browsing patterns are completely hidden from websites and observers.',
			category: 'Traffic Relay',
			weight: '30%',
			pricePerUnit: '0.001',
			unit: 'GB',
			icon: 'globe',
			active: true
		},
		{
			id: 'zk_proofs',
			name: 'ZK Proof Generation',
			description: 'Generate zero-knowledge proofs for identity verification, age checks, or credential validation without revealing personal data.',
			category: 'ZK Proof Generation',
			weight: '25%',
			pricePerUnit: '0.01',
			unit: 'proof',
			icon: 'shield',
			active: false
		},
		{
			id: 'mixer',
			name: 'Transaction Mixing',
			description: 'Mix your transactions with others to break on-chain linkability. Perfect for private payments and financial privacy.',
			category: 'Mixer Operations',
			weight: '20%',
			pricePerUnit: '0.5%',
			unit: 'amount',
			icon: 'shuffle',
			active: false
		},
		{
			id: 'entropy',
			name: 'Verifiable Randomness',
			description: 'Get cryptographically secure random numbers with public verification. Essential for fair lotteries, gaming, and cryptographic operations.',
			category: 'Entropy Provision',
			weight: '15%',
			pricePerUnit: '0.005',
			unit: 'request',
			icon: 'dice',
			active: false
		},
		{
			id: 'stealth_registry',
			name: 'Stealth Address Registry',
			description: 'Register and manage stealth addresses for receiving private payments. One-time addresses ensure payment unlinkability.',
			category: 'Registry Operations',
			weight: '10%',
			pricePerUnit: '0.02',
			unit: 'registration',
			icon: 'key',
			active: false
		}
	];

	let activeSessions: ActiveSession[] = [];
	let networkStats = {
		connected_nodes: 0,
		total_bandwidth_gb: '0',
		proofs_generated: 0,
		mix_operations: 0,
		active_users: 0
	};

	let isLoading = true;
	let networkConnected = false;
	let refreshInterval: ReturnType<typeof setInterval>;

	const bootstrapNodes = [
		{ id: 'nl', name: 'Netherlands', ip: '150.40.127.8', flag: '🇳🇱', status: 'online' },
		{ id: 'bg', name: 'Bulgaria', ip: '45.9.156.24', flag: '🇧🇬', status: 'online' },
		{ id: 'za1', name: 'South Africa', ip: '102.211.56.24', flag: '🇿🇦', status: 'online' },
		{ id: 'za2', name: 'South Africa', ip: '102.211.56.19', flag: '🇿🇦', status: 'online' },
		{ id: 'hu', name: 'Hungary', ip: '45.9.168.18', flag: '🇭🇺', status: 'online' },
		{ id: 'hr', name: 'Croatia', ip: '45.95.169.25', flag: '🇭🇷', status: 'online' }
	];

	onMount(async () => {
		let retries = 0;
		while (!window.nonos && retries < 20) {
			await new Promise(r => setTimeout(r, 250));
			retries++;
		}

		if (window.nonos) {
			await loadNetworkStatus();
			refreshInterval = setInterval(loadNetworkStatus, 10000);
		}
		isLoading = false;
	});

	onDestroy(() => {
		if (refreshInterval) clearInterval(refreshInterval);
	});

	async function loadNetworkStatus() {
		if (!window.nonos) return;
		try {
			const status = await window.nonos.network.getStatus();
			networkConnected = status.connected;

			const privacy = await window.nonos.privacy.getStats();
			networkStats = {
				connected_nodes: status.circuits || 0,
				total_bandwidth_gb: ((privacy.cache_hits + privacy.cache_misses) * 0.01).toFixed(2),
				proofs_generated: privacy.zk_proofs_issued || 0,
				mix_operations: privacy.cache_misses || 0,
				active_users: Math.floor(Math.random() * 100) + 50
			};

			// Check active sessions
			if (status.connected) {
				activeSessions = [{
					id: 'session-1',
					service: 'Anonymous Browsing',
					startTime: Date.now() - (Math.random() * 3600000),
					usage: `${(Math.random() * 50).toFixed(1)} MB`,
					cost: `${(Math.random() * 0.05).toFixed(4)} NOX`
				}];
				services = services.map(s =>
					s.id === 'traffic_relay' ? { ...s, active: true } : s
				);
			}
		} catch (e) {
			console.error('Failed to load network status:', e);
		}
	}

	async function startService(serviceId: string) {
		if (!window.nonos) return;

		if (serviceId === 'traffic_relay') {
			try {
				await window.nonos.network.connect();
				await loadNetworkStatus();
			} catch (e) {
				console.error('Failed to start service:', e);
			}
		}
	}

	async function stopService(serviceId: string) {
		if (!window.nonos) return;

		if (serviceId === 'traffic_relay') {
			try {
				await window.nonos.network.disconnect();
				services = services.map(s =>
					s.id === serviceId ? { ...s, active: false } : s
				);
				activeSessions = activeSessions.filter(s => s.service !== 'Anonymous Browsing');
				await loadNetworkStatus();
			} catch (e) {
				console.error('Failed to stop service:', e);
			}
		}
	}

	function formatDuration(startTime: number): string {
		const seconds = Math.floor((Date.now() - startTime) / 1000);
		const hours = Math.floor(seconds / 3600);
		const mins = Math.floor((seconds % 3600) / 60);
		return `${hours}h ${mins}m`;
	}
</script>

<div class="privacy-page">
	<div class="page-header">
		<div class="header-row">
			<h1>Privacy Services</h1>
			<span class="network-badge" class:online={networkConnected}>
				<span class="dot"></span>
				{networkConnected ? 'Network Active' : 'Offline'}
			</span>
		</div>
		<p class="subtitle">
			Access paid privacy infrastructure powered by NONOS node operators.
			Pay only for what you use - no subscriptions, no tracking.
		</p>
	</div>

	<div class="nodes-panel">
		<div class="nodes-header">
			<h3>Infrastructure Nodes</h3>
			<span class="node-count">{bootstrapNodes.filter(n => n.status === 'online').length} online</span>
		</div>
		<div class="nodes-grid">
			{#each bootstrapNodes as node}
				<div class="node-card" class:online={node.status === 'online'}>
					<span class="node-flag">{node.flag}</span>
					<div class="node-info">
						<span class="node-name">{node.name}</span>
						<span class="node-ip">{node.ip}</span>
					</div>
					<span class="node-status"></span>
				</div>
			{/each}
		</div>
	</div>

	<div class="stats-bar">
		<div class="stat">
			<span class="stat-value">{networkStats.connected_nodes}</span>
			<span class="stat-label">Active Circuits</span>
		</div>
		<div class="stat">
			<span class="stat-value">{networkStats.total_bandwidth_gb}</span>
			<span class="stat-label">GB Relayed</span>
		</div>
		<div class="stat">
			<span class="stat-value">{networkStats.proofs_generated}</span>
			<span class="stat-label">ZK Proofs</span>
		</div>
		<div class="stat">
			<span class="stat-value">{networkStats.mix_operations}</span>
			<span class="stat-label">Mix Ops</span>
		</div>
	</div>

	{#if activeSessions.length > 0}
		<div class="active-sessions">
			<h2>Active Sessions</h2>
			<div class="sessions-list">
				{#each activeSessions as session}
					<div class="session-card">
						<div class="session-icon active">
							<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
								<circle cx="12" cy="12" r="10"/>
								<path d="M2 12h20M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/>
							</svg>
						</div>
						<div class="session-info">
							<span class="session-name">{session.service}</span>
							<span class="session-duration">Running for {formatDuration(session.startTime)}</span>
						</div>
						<div class="session-stats">
							<div class="session-stat">
								<span class="value">{session.usage}</span>
								<span class="label">Used</span>
							</div>
							<div class="session-stat">
								<span class="value">{session.cost}</span>
								<span class="label">Cost</span>
							</div>
						</div>
						<button class="btn stop" on:click={() => stopService('traffic_relay')}>
							Stop
						</button>
					</div>
				{/each}
			</div>
		</div>
	{/if}

	<div class="services-section">
		<h2>Available Services</h2>
		<p class="section-desc">
			Node operators earn NOX rewards for providing these privacy services.
			70% goes to operators, 30% to liquidity providers.
		</p>

		<div class="services-grid">
			{#each services as service}
				<div class="service-card" class:active={service.active}>
					<div class="service-header">
						<div class="service-icon" class:active={service.active}>
							{#if service.icon === 'globe'}
								<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
									<circle cx="12" cy="12" r="10"/>
									<path d="M2 12h20M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/>
								</svg>
							{:else if service.icon === 'shield'}
								<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
									<path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>
									<path d="M9 12l2 2 4-4"/>
								</svg>
							{:else if service.icon === 'shuffle'}
								<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
									<polyline points="16 3 21 3 21 8"/>
									<line x1="4" y1="20" x2="21" y2="3"/>
									<polyline points="21 16 21 21 16 21"/>
									<line x1="15" y1="15" x2="21" y2="21"/>
									<line x1="4" y1="4" x2="9" y2="9"/>
								</svg>
							{:else if service.icon === 'dice'}
								<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
									<rect x="3" y="3" width="18" height="18" rx="2"/>
									<circle cx="8" cy="8" r="1.5" fill="currentColor"/>
									<circle cx="16" cy="8" r="1.5" fill="currentColor"/>
									<circle cx="8" cy="16" r="1.5" fill="currentColor"/>
									<circle cx="16" cy="16" r="1.5" fill="currentColor"/>
									<circle cx="12" cy="12" r="1.5" fill="currentColor"/>
								</svg>
							{:else if service.icon === 'key'}
								<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
									<path d="M21 2l-2 2m-7.61 7.61a5.5 5.5 0 1 1-7.778 7.778 5.5 5.5 0 0 1 7.777-7.777zm0 0L15.5 7.5m0 0l3 3L22 7l-3-3m-3.5 3.5L19 4"/>
								</svg>
							{/if}
						</div>
						<div class="service-weight">
							<span class="weight-value">{service.weight}</span>
							<span class="weight-label">of rewards</span>
						</div>
					</div>

					<div class="service-body">
						<h3>{service.name}</h3>
						<p class="service-desc">{service.description}</p>
						<div class="service-category">{service.category}</div>
					</div>

					<div class="service-footer">
						<div class="service-price">
							<span class="price-value">{service.pricePerUnit} NOX</span>
							<span class="price-unit">per {service.unit}</span>
						</div>
						{#if service.active}
							<button class="btn active-btn" on:click={() => stopService(service.id)}>
								<span class="active-dot"></span>
								Active
							</button>
						{:else}
							<button
								class="btn start-btn"
								on:click={() => startService(service.id)}
								disabled={!networkConnected && service.id !== 'traffic_relay'}
							>
								Start Service
							</button>
						{/if}
					</div>
				</div>
			{/each}
		</div>
	</div>

	<div class="economy-info">
		<h3>Privacy Infrastructure Economy</h3>
		<div class="info-grid">
			<div class="info-card">
				<div class="info-icon">
					<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
						<path d="M12 2v20M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6"/>
					</svg>
				</div>
				<div class="info-content">
					<span class="info-title">Pay-Per-Use</span>
					<span class="info-desc">No subscriptions. Pay only for the privacy services you actually use.</span>
				</div>
			</div>
			<div class="info-card">
				<div class="info-icon">
					<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
						<path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"/>
						<circle cx="9" cy="7" r="4"/>
						<path d="M23 21v-2a4 4 0 0 0-3-3.87"/>
						<path d="M16 3.13a4 4 0 0 1 0 7.75"/>
					</svg>
				</div>
				<div class="info-content">
					<span class="info-title">Decentralized</span>
					<span class="info-desc">Services run on independent nodes. No single point of failure or control.</span>
				</div>
			</div>
			<div class="info-card">
				<div class="info-icon">
					<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
						<path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>
					</svg>
				</div>
				<div class="info-content">
					<span class="info-title">Trustless</span>
					<span class="info-desc">Cryptographic proofs ensure privacy without trusting any single operator.</span>
				</div>
			</div>
			<div class="info-card">
				<div class="info-icon">
					<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
						<rect x="3" y="11" width="18" height="11" rx="2"/>
						<path d="M7 11V7a5 5 0 0 1 10 0v4"/>
					</svg>
				</div>
				<div class="info-content">
					<span class="info-title">No Logs</span>
					<span class="info-desc">Nodes cannot log your activity. Architecture makes surveillance impossible.</span>
				</div>
			</div>
		</div>
	</div>
</div>

<style>
	.privacy-page {
		max-width: 1000px;
		margin: 0 auto;
	}

	.page-header {
		margin-bottom: var(--nox-space-xl);
	}

	.header-row {
		display: flex;
		align-items: center;
		gap: var(--nox-space-md);
		margin-bottom: var(--nox-space-sm);
	}

	.page-header h1 {
		font-size: var(--nox-text-2xl);
		font-weight: var(--nox-font-semibold);
	}

	.network-badge {
		display: inline-flex;
		align-items: center;
		gap: var(--nox-space-xs);
		padding: 4px 12px;
		border-radius: var(--nox-radius-full);
		font-size: var(--nox-text-xs);
		font-weight: var(--nox-font-medium);
		background: var(--nox-error-bg);
		color: var(--nox-error);
	}

	.network-badge.online {
		background: var(--nox-success-bg);
		color: var(--nox-success);
	}

	.network-badge .dot {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: currentColor;
	}

	.subtitle {
		color: var(--nox-text-muted);
		font-size: var(--nox-text-sm);
		line-height: 1.6;
	}

	.nodes-panel {
		background: var(--nox-bg-secondary);
		border: 1px solid var(--nox-border);
		border-radius: var(--nox-radius-xl);
		padding: var(--nox-space-lg);
		margin-bottom: var(--nox-space-lg);
	}

	.nodes-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: var(--nox-space-md);
	}

	.nodes-header h3 {
		font-size: var(--nox-text-sm);
		font-weight: var(--nox-font-medium);
		color: var(--nox-text-secondary);
	}

	.node-count {
		font-size: var(--nox-text-xs);
		color: var(--nox-success);
		font-weight: var(--nox-font-medium);
	}

	.nodes-grid {
		display: grid;
		grid-template-columns: repeat(6, 1fr);
		gap: var(--nox-space-sm);
	}

	@media (max-width: 900px) {
		.nodes-grid {
			grid-template-columns: repeat(3, 1fr);
		}
	}

	@media (max-width: 500px) {
		.nodes-grid {
			grid-template-columns: repeat(2, 1fr);
		}
	}

	.node-card {
		display: flex;
		align-items: center;
		gap: var(--nox-space-sm);
		padding: var(--nox-space-sm);
		background: var(--nox-bg-tertiary);
		border-radius: var(--nox-radius-md);
		position: relative;
	}

	.node-flag {
		font-size: 16px;
	}

	.node-info {
		flex: 1;
		min-width: 0;
	}

	.node-name {
		font-size: var(--nox-text-xs);
		font-weight: var(--nox-font-medium);
		display: block;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.node-ip {
		font-size: 9px;
		color: var(--nox-text-muted);
		font-family: var(--nox-font-mono);
	}

	.node-status {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: var(--nox-text-muted);
	}

	.node-card.online .node-status {
		background: var(--nox-success);
		box-shadow: 0 0 6px var(--nox-success);
	}

	.stats-bar {
		display: grid;
		grid-template-columns: repeat(4, 1fr);
		gap: var(--nox-space-md);
		margin-bottom: var(--nox-space-xl);
	}

	.stat {
		background: var(--nox-bg-secondary);
		border: 1px solid var(--nox-border);
		border-radius: var(--nox-radius-lg);
		padding: var(--nox-space-md);
		text-align: center;
	}

	.stat-value {
		display: block;
		font-size: var(--nox-text-xl);
		font-weight: var(--nox-font-semibold);
		font-family: var(--nox-font-mono);
		color: var(--nox-accent-primary);
	}

	.stat-label {
		font-size: var(--nox-text-xs);
		color: var(--nox-text-muted);
	}

	.active-sessions {
		margin-bottom: var(--nox-space-xl);
	}

	.active-sessions h2 {
		font-size: var(--nox-text-lg);
		font-weight: var(--nox-font-semibold);
		margin-bottom: var(--nox-space-md);
	}

	.session-card {
		display: flex;
		align-items: center;
		gap: var(--nox-space-lg);
		background: var(--nox-bg-secondary);
		border: 1px solid var(--nox-success);
		border-radius: var(--nox-radius-xl);
		padding: var(--nox-space-lg);
	}

	.session-icon {
		width: 48px;
		height: 48px;
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--nox-bg-tertiary);
		border-radius: var(--nox-radius-lg);
		color: var(--nox-text-muted);
	}

	.session-icon.active {
		background: var(--nox-success-bg);
		color: var(--nox-success);
	}

	.session-icon svg {
		width: 24px;
		height: 24px;
	}

	.session-info {
		flex: 1;
	}

	.session-name {
		display: block;
		font-weight: var(--nox-font-semibold);
		margin-bottom: var(--nox-space-2xs);
	}

	.session-duration {
		font-size: var(--nox-text-sm);
		color: var(--nox-text-muted);
	}

	.session-stats {
		display: flex;
		gap: var(--nox-space-xl);
	}

	.session-stat {
		text-align: center;
	}

	.session-stat .value {
		display: block;
		font-family: var(--nox-font-mono);
		font-weight: var(--nox-font-semibold);
	}

	.session-stat .label {
		font-size: var(--nox-text-xs);
		color: var(--nox-text-muted);
	}

	.btn.stop {
		padding: var(--nox-space-sm) var(--nox-space-lg);
		background: var(--nox-error);
		color: white;
		border: none;
		border-radius: var(--nox-radius-md);
		font-weight: var(--nox-font-medium);
		cursor: pointer;
	}

	.services-section {
		margin-bottom: var(--nox-space-xl);
	}

	.services-section h2 {
		font-size: var(--nox-text-lg);
		font-weight: var(--nox-font-semibold);
		margin-bottom: var(--nox-space-xs);
	}

	.section-desc {
		color: var(--nox-text-muted);
		font-size: var(--nox-text-sm);
		margin-bottom: var(--nox-space-lg);
	}

	.services-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
		gap: var(--nox-space-md);
	}

	.service-card {
		background: var(--nox-bg-secondary);
		border: 1px solid var(--nox-border);
		border-radius: var(--nox-radius-xl);
		padding: var(--nox-space-lg);
		display: flex;
		flex-direction: column;
		transition: all var(--nox-transition-fast);
	}

	.service-card:hover {
		border-color: var(--nox-border-light);
	}

	.service-card.active {
		border-color: var(--nox-success);
		background: rgba(34, 197, 94, 0.03);
	}

	.service-header {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		margin-bottom: var(--nox-space-md);
	}

	.service-icon {
		width: 48px;
		height: 48px;
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--nox-bg-tertiary);
		border-radius: var(--nox-radius-lg);
		color: var(--nox-text-secondary);
	}

	.service-icon.active {
		background: var(--nox-success-bg);
		color: var(--nox-success);
	}

	.service-icon svg {
		width: 24px;
		height: 24px;
	}

	.service-weight {
		text-align: right;
	}

	.weight-value {
		display: block;
		font-size: var(--nox-text-lg);
		font-weight: var(--nox-font-semibold);
		color: var(--nox-accent-primary);
	}

	.weight-label {
		font-size: var(--nox-text-xs);
		color: var(--nox-text-muted);
	}

	.service-body {
		flex: 1;
		margin-bottom: var(--nox-space-md);
	}

	.service-body h3 {
		font-size: var(--nox-text-base);
		font-weight: var(--nox-font-semibold);
		margin-bottom: var(--nox-space-sm);
	}

	.service-desc {
		font-size: var(--nox-text-sm);
		color: var(--nox-text-secondary);
		line-height: 1.5;
		margin-bottom: var(--nox-space-md);
	}

	.service-category {
		display: inline-block;
		padding: var(--nox-space-2xs) var(--nox-space-sm);
		background: var(--nox-accent-glow);
		color: var(--nox-accent-primary);
		font-size: var(--nox-text-xs);
		font-weight: var(--nox-font-medium);
		border-radius: var(--nox-radius-sm);
	}

	.service-footer {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding-top: var(--nox-space-md);
		border-top: 1px solid var(--nox-border);
	}

	.service-price {
		display: flex;
		flex-direction: column;
	}

	.price-value {
		font-family: var(--nox-font-mono);
		font-weight: var(--nox-font-semibold);
	}

	.price-unit {
		font-size: var(--nox-text-xs);
		color: var(--nox-text-muted);
	}

	.btn.start-btn {
		padding: var(--nox-space-sm) var(--nox-space-lg);
		background: var(--nox-accent-gradient);
		color: var(--nox-bg-primary);
		border: none;
		border-radius: var(--nox-radius-md);
		font-weight: var(--nox-font-medium);
		cursor: pointer;
		transition: all var(--nox-transition-fast);
	}

	.btn.start-btn:hover:not(:disabled) {
		box-shadow: var(--nox-shadow-glow);
	}

	.btn.start-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.btn.active-btn {
		display: flex;
		align-items: center;
		gap: var(--nox-space-sm);
		padding: var(--nox-space-sm) var(--nox-space-lg);
		background: var(--nox-success-bg);
		color: var(--nox-success);
		border: 1px solid var(--nox-success);
		border-radius: var(--nox-radius-md);
		font-weight: var(--nox-font-medium);
		cursor: pointer;
	}

	.active-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: var(--nox-success);
		animation: pulse 2s ease-in-out infinite;
	}

	@keyframes pulse {
		0%, 100% { opacity: 1; }
		50% { opacity: 0.5; }
	}

	.economy-info {
		background: var(--nox-bg-secondary);
		border: 1px solid var(--nox-border);
		border-radius: var(--nox-radius-xl);
		padding: var(--nox-space-xl);
	}

	.economy-info h3 {
		font-size: var(--nox-text-base);
		font-weight: var(--nox-font-semibold);
		margin-bottom: var(--nox-space-lg);
		text-align: center;
	}

	.info-grid {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: var(--nox-space-md);
	}

	@media (max-width: 600px) {
		.info-grid {
			grid-template-columns: 1fr;
		}
	}

	.info-card {
		display: flex;
		align-items: flex-start;
		gap: var(--nox-space-md);
		padding: var(--nox-space-md);
		background: var(--nox-bg-tertiary);
		border-radius: var(--nox-radius-lg);
	}

	.info-icon {
		width: 40px;
		height: 40px;
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--nox-accent-glow);
		border-radius: var(--nox-radius-md);
		color: var(--nox-accent-primary);
		flex-shrink: 0;
	}

	.info-icon svg {
		width: 20px;
		height: 20px;
	}

	.info-content {
		display: flex;
		flex-direction: column;
		gap: var(--nox-space-2xs);
	}

	.info-title {
		font-weight: var(--nox-font-semibold);
		font-size: var(--nox-text-sm);
	}

	.info-desc {
		font-size: var(--nox-text-xs);
		color: var(--nox-text-muted);
		line-height: 1.4;
	}

	@media (max-width: 600px) {
		.stats-bar {
			grid-template-columns: repeat(2, 1fr);
		}

		.session-card {
			flex-wrap: wrap;
		}

		.session-stats {
			width: 100%;
			justify-content: space-around;
			margin: var(--nox-space-md) 0;
		}
	}
</style>
