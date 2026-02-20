<script lang="ts">
	import { onMount } from 'svelte';

	let stats = {
		zk_proofs_issued: 0,
		zk_verifications: 0,
		cache_hits: 0,
		cache_misses: 0,
		cache_hit_rate: 0,
		tracking_blocked: 0,
		tracking_total: 0,
		block_rate: 0,
		stealth_payments: 0,
		stealth_scanned: 0
	};

	let identities: Array<{ id: string; commitment: string; created: string }> = [];
	let newIdentityName = '';
	let identityRoot = '';
	let isCreatingIdentity = false;

	let cacheContent = '';
	let isCaching = false;

	let domainToBlock = '';
	let blockedDomains: string[] = [];

	let isLoading = true;
	let error = '';
	let networkConnected = true;

	const bootstrapNodes = [
		{ name: 'boot1-nl', ip: '5.255.99.170', location: 'Netherlands' },
		{ name: 'boot2-bg', ip: '45.9.156.24', location: 'Bulgaria' },
		{ name: 'boot3-za', ip: '102.211.56.24', location: 'South Africa' },
		{ name: 'boot4-hu', ip: '45.9.168.18', location: 'Hungary' }
	];

	onMount(() => {
		loadPrivacyStats();
		const interval = setInterval(loadPrivacyStats, 10000);
		return () => clearInterval(interval);
	});

	async function loadPrivacyStats() {
		if (!window.nonos) return;

		try {
			const result = await window.nonos.privacy.getStats();
			stats = result;
			networkConnected = true;
			error = '';

			try {
				identityRoot = await window.nonos.privacy.getIdentityRoot();
			} catch {}
		} catch (e: any) {
			networkConnected = false;
			error = 'Connecting to NONOS network...';
		} finally {
			isLoading = false;
		}
	}

	async function createZkIdentity() {
		if (!window.nonos || !newIdentityName.trim()) return;

		isCreatingIdentity = true;
		try {
			const result = await window.nonos.privacy.generateIdentity(newIdentityName.trim());
			identities = [...identities, {
				id: result.identity_id,
				commitment: result.commitment,
				created: new Date().toISOString()
			}];
			identityRoot = result.merkle_root;
			newIdentityName = '';
			await loadPrivacyStats();
		} catch (e: any) {
			error = e.toString();
		} finally {
			isCreatingIdentity = false;
		}
	}

	async function storeInCacheMixer() {
		if (!window.nonos || !cacheContent.trim()) return;

		isCaching = true;
		try {
			await window.nonos.privacy.cacheStore(cacheContent.trim());
			cacheContent = '';
			await loadPrivacyStats();
		} catch (e: any) {
			error = e.toString();
		} finally {
			isCaching = false;
		}
	}

	async function blockDomain() {
		if (!window.nonos || !domainToBlock.trim()) return;

		try {
			await window.nonos.privacy.blockDomain(domainToBlock.trim());
			blockedDomains = [...blockedDomains, domainToBlock.trim()];
			domainToBlock = '';
			await loadPrivacyStats();
		} catch (e: any) {
			error = e.toString();
		}
	}

	async function checkTracking(domain: string) {
		if (!window.nonos) return;

		try {
			const result = await window.nonos.privacy.checkTracking(domain);
			alert(`Domain: ${result.domain}\nBlocked: ${result.blocked}\nReason: ${result.reason || 'N/A'}`);
		} catch (e: any) {
			error = e.toString();
		}
	}
</script>

<div class="privacy-page">
	<div class="page-header">
		<h1>Privacy Center</h1>
		<p class="subtitle">Zero-knowledge identity and tracking protection powered by NONOS nodes</p>
	</div>

	{#if error}
		<div class="error-banner">
			<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
				<circle cx="12" cy="12" r="10"/>
				<path d="M12 8v4M12 16h.01"/>
			</svg>
			<span>{error}</span>
		</div>
	{/if}

	<div class="network-status" class:connected={networkConnected}>
		<div class="status-indicator">
			<span class="dot"></span>
			<span class="label">{networkConnected ? 'Connected to NONOS Network' : 'Connecting...'}</span>
		</div>
		<div class="nodes-info">
			{#each bootstrapNodes as node}
				<span class="node-badge" title="{node.ip}">
					<span class="flag">{node.location === 'Netherlands' ? '🇳🇱' : node.location === 'Bulgaria' ? '🇧🇬' : node.location === 'South Africa' ? '🇿🇦' : '🇭🇺'}</span>
					{node.name}
				</span>
			{/each}
		</div>
	</div>

	<div class="stats-grid">
		<div class="stat-card">
			<div class="stat-icon zk">
				<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
					<path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>
					<path d="M9 12l2 2 4-4"/>
				</svg>
			</div>
			<div class="stat-info">
				<span class="stat-value">{stats.zk_proofs_issued}</span>
				<span class="stat-label">ZK Proofs Issued</span>
			</div>
		</div>

		<div class="stat-card">
			<div class="stat-icon verify">
				<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
					<polyline points="20 6 9 17 4 12"/>
				</svg>
			</div>
			<div class="stat-info">
				<span class="stat-value">{stats.zk_verifications}</span>
				<span class="stat-label">Verifications</span>
			</div>
		</div>

		<div class="stat-card">
			<div class="stat-icon cache">
				<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
					<rect x="2" y="2" width="20" height="8" rx="2"/>
					<rect x="2" y="14" width="20" height="8" rx="2"/>
					<line x1="6" y1="6" x2="6" y2="6"/>
					<line x1="6" y1="18" x2="6" y2="18"/>
				</svg>
			</div>
			<div class="stat-info">
				<span class="stat-value">{stats.cache_hits}</span>
				<span class="stat-label">Cache Hits ({(stats.cache_hit_rate * 100).toFixed(1)}%)</span>
			</div>
		</div>

		<div class="stat-card">
			<div class="stat-icon blocked">
				<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
					<circle cx="12" cy="12" r="10"/>
					<path d="M4.93 4.93l14.14 14.14"/>
				</svg>
			</div>
			<div class="stat-info">
				<span class="stat-value">{stats.tracking_blocked}</span>
				<span class="stat-label">Trackers Blocked ({(stats.block_rate * 100).toFixed(1)}%)</span>
			</div>
		</div>
	</div>

	<section class="section">
		<div class="section-header">
			<h2>ZK Identity</h2>
			<p>Create zero-knowledge identities for anonymous authentication</p>
		</div>

		<div class="card">
			<div class="identity-root">
				<span class="label">Identity Merkle Root:</span>
				<code>{identityRoot || 'No identities yet'}</code>
			</div>

			<div class="create-identity">
				<input
					type="text"
					bind:value={newIdentityName}
					placeholder="Identity name (e.g., 'Shopping', 'Social')"
					class="input"
				/>
				<button
					class="btn primary"
					on:click={createZkIdentity}
					disabled={isCreatingIdentity || !newIdentityName.trim()}
				>
					{isCreatingIdentity ? 'Creating...' : 'Create ZK Identity'}
				</button>
			</div>

			{#if identities.length > 0}
				<div class="identities-list">
					<h4>Your Identities</h4>
					{#each identities as identity}
						<div class="identity-item">
							<div class="identity-name">{identity.id}</div>
							<code class="identity-commitment">{identity.commitment.slice(0, 16)}...</code>
						</div>
					{/each}
				</div>
			{/if}
		</div>
	</section>

	<section class="section">
		<div class="section-header">
			<h2>Cache Mixer</h2>
			<p>Obfuscate content patterns with encrypted cache mixing</p>
		</div>

		<div class="card">
			<div class="cache-stats">
				<div class="cache-stat">
					<span class="value">{stats.cache_hits + stats.cache_misses}</span>
					<span class="label">Total Operations</span>
				</div>
				<div class="cache-stat">
					<span class="value">{stats.cache_misses}</span>
					<span class="label">Mix Operations</span>
				</div>
			</div>

			<div class="cache-input">
				<textarea
					bind:value={cacheContent}
					placeholder="Enter content to store in the cache mixer..."
					rows="3"
					class="textarea"
				></textarea>
				<button
					class="btn secondary"
					on:click={storeInCacheMixer}
					disabled={isCaching || !cacheContent.trim()}
				>
					{isCaching ? 'Storing...' : 'Store in Cache Mixer'}
				</button>
			</div>

			<p class="info-text">
				Content is encrypted with a unique commitment and stored in a Merkle tree.
				Access patterns are obfuscated through mixing operations.
			</p>
		</div>
	</section>

	<section class="section">
		<div class="section-header">
			<h2>Tracking Blocker</h2>
			<p>Block trackers, fingerprinting, and surveillance domains</p>
		</div>

		<div class="card">
			<div class="tracking-stats">
				<div class="tracking-stat blocked">
					<span class="value">{stats.tracking_blocked}</span>
					<span class="label">Blocked</span>
				</div>
				<div class="tracking-stat total">
					<span class="value">{stats.tracking_total}</span>
					<span class="label">Total Checked</span>
				</div>
				<div class="tracking-stat rate">
					<span class="value">{(stats.block_rate * 100).toFixed(1)}%</span>
					<span class="label">Block Rate</span>
				</div>
			</div>

			<div class="block-domain">
				<input
					type="text"
					bind:value={domainToBlock}
					placeholder="Domain to block (e.g., tracker.example.com)"
					class="input"
				/>
				<button
					class="btn danger"
					on:click={blockDomain}
					disabled={!domainToBlock.trim()}
				>
					Block Domain
				</button>
			</div>

			{#if blockedDomains.length > 0}
				<div class="blocked-list">
					<h4>Custom Blocked Domains</h4>
					{#each blockedDomains as domain}
						<div class="blocked-item">
							<span class="domain">{domain}</span>
							<button class="check-btn" on:click={() => checkTracking(domain)}>Check</button>
						</div>
					{/each}
				</div>
			{/if}
		</div>
	</section>

	<section class="section">
		<div class="section-header">
			<h2>Stealth Payments</h2>
			<p>Private transactions using stealth addresses</p>
		</div>

		<div class="card">
			<div class="stealth-stats">
				<div class="stealth-stat">
					<span class="value">{stats.stealth_payments}</span>
					<span class="label">Stealth Payments Sent</span>
				</div>
				<div class="stealth-stat">
					<span class="value">{stats.stealth_scanned}</span>
					<span class="label">Addresses Scanned</span>
				</div>
			</div>
			<p class="info-text">
				Stealth addresses provide unlinkable payment destinations.
				Each payment generates a unique one-time address.
			</p>
		</div>
	</section>
</div>

<style>
	.privacy-page {
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

	.error-banner {
		display: flex;
		align-items: center;
		gap: var(--nox-space-md);
		background: var(--nox-error-bg);
		border: 1px solid var(--nox-error);
		color: var(--nox-error);
		padding: var(--nox-space-md);
		border-radius: var(--nox-radius-lg);
		margin-bottom: var(--nox-space-lg);
	}

	.error-banner svg {
		width: 20px;
		height: 20px;
		flex-shrink: 0;
	}

	.network-status {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: var(--nox-space-md) var(--nox-space-lg);
		background: var(--nox-bg-secondary);
		border: 1px solid var(--nox-border);
		border-radius: var(--nox-radius-lg);
		margin-bottom: var(--nox-space-lg);
	}

	.network-status.connected {
		border-color: var(--nox-success);
		background: rgba(34, 197, 94, 0.05);
	}

	.status-indicator {
		display: flex;
		align-items: center;
		gap: var(--nox-space-sm);
	}

	.status-indicator .dot {
		width: 10px;
		height: 10px;
		border-radius: 50%;
		background: var(--nox-text-muted);
		animation: pulse 2s ease-in-out infinite;
	}

	.network-status.connected .dot {
		background: var(--nox-success);
	}

	@keyframes pulse {
		0%, 100% { opacity: 1; }
		50% { opacity: 0.5; }
	}

	.status-indicator .label {
		font-size: var(--nox-text-sm);
		font-weight: var(--nox-font-medium);
	}

	.nodes-info {
		display: flex;
		gap: var(--nox-space-sm);
		flex-wrap: wrap;
	}

	.node-badge {
		display: flex;
		align-items: center;
		gap: var(--nox-space-2xs);
		padding: var(--nox-space-2xs) var(--nox-space-sm);
		background: var(--nox-bg-tertiary);
		border-radius: var(--nox-radius-sm);
		font-size: var(--nox-text-xs);
		color: var(--nox-text-secondary);
	}

	.node-badge .flag {
		font-size: 12px;
	}

	@media (max-width: 600px) {
		.network-status {
			flex-direction: column;
			gap: var(--nox-space-md);
			align-items: flex-start;
		}
	}

	.stats-grid {
		display: grid;
		grid-template-columns: repeat(4, 1fr);
		gap: var(--nox-space-md);
		margin-bottom: var(--nox-space-xl);
	}

	@media (max-width: 800px) {
		.stats-grid {
			grid-template-columns: repeat(2, 1fr);
		}
	}

	.stat-card {
		display: flex;
		align-items: center;
		gap: var(--nox-space-md);
		background: var(--nox-bg-secondary);
		border: 1px solid var(--nox-border);
		border-radius: var(--nox-radius-lg);
		padding: var(--nox-space-md);
	}

	.stat-icon {
		width: 48px;
		height: 48px;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: var(--nox-radius-md);
	}

	.stat-icon svg {
		width: 24px;
		height: 24px;
	}

	.stat-icon.zk {
		background: var(--nox-accent-glow);
		color: var(--nox-accent-primary);
	}

	.stat-icon.verify {
		background: rgba(34, 197, 94, 0.1);
		color: var(--nox-success);
	}

	.stat-icon.cache {
		background: rgba(59, 130, 246, 0.1);
		color: #3b82f6;
	}

	.stat-icon.blocked {
		background: rgba(239, 68, 68, 0.1);
		color: var(--nox-error);
	}

	.stat-info {
		display: flex;
		flex-direction: column;
	}

	.stat-value {
		font-size: var(--nox-text-xl);
		font-weight: var(--nox-font-semibold);
		font-family: var(--nox-font-mono);
	}

	.stat-label {
		font-size: var(--nox-text-xs);
		color: var(--nox-text-muted);
	}

	.section {
		margin-bottom: var(--nox-space-xl);
	}

	.section-header {
		margin-bottom: var(--nox-space-md);
	}

	.section-header h2 {
		font-size: var(--nox-text-lg);
		font-weight: var(--nox-font-semibold);
		margin-bottom: var(--nox-space-2xs);
	}

	.section-header p {
		color: var(--nox-text-muted);
		font-size: var(--nox-text-sm);
	}

	.card {
		background: var(--nox-bg-secondary);
		border: 1px solid var(--nox-border);
		border-radius: var(--nox-radius-xl);
		padding: var(--nox-space-lg);
	}

	.identity-root {
		display: flex;
		align-items: center;
		gap: var(--nox-space-md);
		margin-bottom: var(--nox-space-lg);
		padding: var(--nox-space-md);
		background: var(--nox-bg-tertiary);
		border-radius: var(--nox-radius-md);
	}

	.identity-root .label {
		font-size: var(--nox-text-sm);
		color: var(--nox-text-muted);
	}

	.identity-root code {
		font-family: var(--nox-font-mono);
		font-size: var(--nox-text-xs);
		color: var(--nox-accent-primary);
		word-break: break-all;
	}

	.create-identity {
		display: flex;
		gap: var(--nox-space-md);
		margin-bottom: var(--nox-space-lg);
	}

	.input, .textarea {
		flex: 1;
		padding: var(--nox-space-sm) var(--nox-space-md);
		background: var(--nox-bg-tertiary);
		border: 1px solid var(--nox-border);
		border-radius: var(--nox-radius-md);
		color: var(--nox-text-primary);
		font-size: var(--nox-text-sm);
	}

	.input:focus, .textarea:focus {
		outline: none;
		border-color: var(--nox-accent-primary);
	}

	.textarea {
		resize: vertical;
		width: 100%;
		margin-bottom: var(--nox-space-md);
	}

	.btn {
		padding: var(--nox-space-sm) var(--nox-space-lg);
		border-radius: var(--nox-radius-md);
		font-weight: var(--nox-font-medium);
		font-size: var(--nox-text-sm);
		transition: all var(--nox-transition-fast);
		white-space: nowrap;
		border: none;
		cursor: pointer;
	}

	.btn.primary {
		background: var(--nox-accent-gradient);
		color: var(--nox-bg-primary);
	}

	.btn.secondary {
		background: var(--nox-bg-tertiary);
		border: 1px solid var(--nox-border);
		color: var(--nox-text-primary);
	}

	.btn.danger {
		background: var(--nox-error);
		color: white;
	}

	.btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.identities-list, .blocked-list {
		margin-top: var(--nox-space-lg);
		border-top: 1px solid var(--nox-border);
		padding-top: var(--nox-space-lg);
	}

	.identities-list h4, .blocked-list h4 {
		font-size: var(--nox-text-sm);
		font-weight: var(--nox-font-medium);
		margin-bottom: var(--nox-space-md);
	}

	.identity-item, .blocked-item {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: var(--nox-space-sm) var(--nox-space-md);
		background: var(--nox-bg-tertiary);
		border-radius: var(--nox-radius-md);
		margin-bottom: var(--nox-space-sm);
	}

	.identity-name, .domain {
		font-weight: var(--nox-font-medium);
	}

	.identity-commitment {
		font-family: var(--nox-font-mono);
		font-size: var(--nox-text-xs);
		color: var(--nox-text-muted);
	}

	.check-btn {
		font-size: var(--nox-text-xs);
		padding: var(--nox-space-2xs) var(--nox-space-sm);
		background: var(--nox-bg-secondary);
		border: 1px solid var(--nox-border);
		border-radius: var(--nox-radius-sm);
		color: var(--nox-text-secondary);
		cursor: pointer;
	}

	.cache-stats, .tracking-stats, .stealth-stats {
		display: flex;
		gap: var(--nox-space-lg);
		margin-bottom: var(--nox-space-lg);
	}

	.cache-stat, .tracking-stat, .stealth-stat {
		display: flex;
		flex-direction: column;
		padding: var(--nox-space-md);
		background: var(--nox-bg-tertiary);
		border-radius: var(--nox-radius-md);
		flex: 1;
		text-align: center;
	}

	.cache-stat .value, .tracking-stat .value, .stealth-stat .value {
		font-size: var(--nox-text-2xl);
		font-weight: var(--nox-font-semibold);
		font-family: var(--nox-font-mono);
	}

	.tracking-stat.blocked .value {
		color: var(--nox-error);
	}

	.tracking-stat.rate .value {
		color: var(--nox-success);
	}

	.cache-stat .label, .tracking-stat .label, .stealth-stat .label {
		font-size: var(--nox-text-xs);
		color: var(--nox-text-muted);
		margin-top: var(--nox-space-2xs);
	}

	.cache-input {
		margin-bottom: var(--nox-space-md);
	}

	.block-domain {
		display: flex;
		gap: var(--nox-space-md);
	}

	.info-text {
		font-size: var(--nox-text-sm);
		color: var(--nox-text-muted);
		line-height: 1.5;
	}
</style>
