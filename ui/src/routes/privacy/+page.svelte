<script lang="ts">
	import { onMount } from 'svelte';

	// Privacy stats from NONOS network
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

	// LEGENDARY FEATURES STATE
	let legendaryStatus: any = null;
	let stealthSession: any = null;
	let fingerprintStatus: any = null;
	let mixnetStatus: any = null;
	let domainScoreCheck = '';
	let domainScore: any = null;
	let zkCredentialType = 'age_over_18';

	// ZK Identity state
	let identities: Array<{ id: string; commitment: string; created: string }> = [];
	let newIdentityName = '';
	let identityRoot = '';
	let isCreatingIdentity = false;

	// Cache mixer state
	let cacheContent = '';
	let isCaching = false;

	// Tracking blocker state
	let domainToBlock = '';
	let blockedDomains: string[] = [];

	// Loading states
	let isLoading = true;
	let error = '';
	let networkConnected = true; // Now using remote nodes, always available

	// Bootstrap nodes for display
	const bootstrapNodes = [
		{ name: 'boot1-nl', ip: '5.255.99.170', location: 'Netherlands' },
		{ name: 'boot2-bg', ip: '45.9.156.24', location: 'Bulgaria' },
		{ name: 'boot3-za', ip: '102.211.56.24', location: 'South Africa' },
		{ name: 'boot4-hu', ip: '45.9.168.18', location: 'Hungary' }
	];

	onMount(() => {
		loadPrivacyStats();
		loadLegendaryStatus();
		const interval = setInterval(loadPrivacyStats, 10000);
		return () => clearInterval(interval);
	});

	async function loadLegendaryStatus() {
		if (!window.nonos?.legendary) return;
		try {
			legendaryStatus = await window.nonos.legendary.getSummary();
			fingerprintStatus = await window.nonos.legendary.getFingerprintStatus();
			mixnetStatus = await window.nonos.legendary.getMixnetStatus();
		} catch (e) {
			console.error('Failed to load legendary status:', e);
		}
	}

	async function createStealthSession() {
		if (!window.nonos?.legendary) return;
		try {
			stealthSession = await window.nonos.legendary.createStealthSession();
		} catch (e: any) {
			error = e.toString();
		}
	}

	async function checkDomainPrivacy() {
		if (!window.nonos?.legendary || !domainScoreCheck) return;
		try {
			domainScore = await window.nonos.legendary.getDomainScore(domainScoreCheck);
		} catch (e: any) {
			error = e.toString();
		}
	}

	async function generateCredentialProof() {
		if (!window.nonos?.legendary) return;
		try {
			const proof = await window.nonos.legendary.proveCredential(zkCredentialType);
			alert(`ZK Credential Proof Generated!\n\nType: ${proof.credential_type}\nCommitment: ${proof.commitment}\n\n${proof.note}`);
		} catch (e: any) {
			error = e.toString();
		}
	}

	async function loadPrivacyStats() {
		if (!window.nonos) return;

		try {
			const result = await window.nonos.privacy.getStats();
			stats = result;
			networkConnected = true;
			error = '';

			// Get identity root
			try {
				identityRoot = await window.nonos.privacy.getIdentityRoot();
			} catch {}
		} catch (e: any) {
			// Network may be temporarily unavailable, but services still work locally
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
		<h1>🔮 Legendary Privacy</h1>
		<p class="subtitle">World-first privacy features powered by NONOS nodes</p>
	</div>

	<!-- LEGENDARY FEATURES HERO -->
	{#if legendaryStatus}
		<div class="legendary-hero">
			<div class="legendary-badge">
				<span class="badge-icon">⚡</span>
				<span class="badge-text">{legendaryStatus.status}</span>
			</div>
			<div class="legendary-grid">
				{#each Object.entries(legendaryStatus.legendary_features || {}) as [key, feature]}
					<div class="legendary-card" class:active={feature.enabled}>
						<div class="card-status">{feature.enabled ? '✓' : '○'}</div>
						<div class="card-title">{key.replace(/_/g, ' ').replace(/\b\w/g, l => l.toUpperCase())}</div>
						<div class="card-desc">{feature.description}</div>
					</div>
				{/each}
			</div>
		</div>
	{/if}

	<!-- STEALTH SESSIONS -->
	<section class="section legendary-section">
		<div class="section-header">
			<h2>🥷 Stealth Sessions</h2>
			<p>Every browsing session uses a unique stealth address - completely unlinkable</p>
		</div>
		<div class="card">
			{#if stealthSession}
				<div class="stealth-active">
					<div class="stealth-indicator">
						<span class="pulse"></span>
						<span>STEALTH SESSION ACTIVE</span>
					</div>
					<div class="stealth-details">
						<div class="detail">
							<span class="label">Session ID:</span>
							<code>{stealthSession.session_id.slice(0, 16)}...</code>
						</div>
						<div class="detail">
							<span class="label">Stealth Address:</span>
							<code>{stealthSession.stealth_address}</code>
						</div>
					</div>
				</div>
			{:else}
				<p class="info-text">Create a stealth session for unlinkable browsing</p>
			{/if}
			<button class="btn primary" on:click={createStealthSession}>
				{stealthSession ? 'New Stealth Session' : 'Create Stealth Session'}
			</button>
		</div>
	</section>

	<!-- PRIVACY ORACLE -->
	<section class="section legendary-section">
		<div class="section-header">
			<h2>🔍 Privacy Oracle</h2>
			<p>Real-time domain privacy scoring powered by NONOS node network</p>
		</div>
		<div class="card">
			<div class="domain-check">
				<input
					type="text"
					bind:value={domainScoreCheck}
					placeholder="Enter domain to analyze (e.g., google.com)"
					class="input"
				/>
				<button class="btn secondary" on:click={checkDomainPrivacy}>Analyze</button>
			</div>
			{#if domainScore}
				<div class="domain-result" class:good={domainScore.score >= 70} class:bad={domainScore.score < 50}>
					<div class="score-circle">
						<span class="grade">{domainScore.grade}</span>
						<span class="score">{domainScore.score}/100</span>
					</div>
					<div class="score-details">
						<div class="domain-name">{domainScore.domain}</div>
						<div class="recommendation">{domainScore.recommendation}</div>
						{#if domainScore.trackers_detected?.length > 0}
							<div class="trackers-found">
								<strong>Trackers:</strong> {domainScore.trackers_detected.join(', ')}
							</div>
						{/if}
					</div>
				</div>
			{/if}
		</div>
	</section>

	<!-- ZK CREDENTIALS -->
	<section class="section legendary-section">
		<div class="section-header">
			<h2>🎭 ZK Credentials</h2>
			<p>Prove attributes without revealing your identity</p>
		</div>
		<div class="card">
			<div class="credential-options">
				<label class="credential-option">
					<input type="radio" bind:group={zkCredentialType} value="age_over_18" />
					<span>Prove Age ≥ 18</span>
				</label>
				<label class="credential-option">
					<input type="radio" bind:group={zkCredentialType} value="stake_over_1000" />
					<span>Prove Stake ≥ 1,000 NOX</span>
				</label>
				<label class="credential-option">
					<input type="radio" bind:group={zkCredentialType} value="account_age_30" />
					<span>Prove Account ≥ 30 days</span>
				</label>
				<label class="credential-option">
					<input type="radio" bind:group={zkCredentialType} value="human_verified" />
					<span>Prove Human (not bot)</span>
				</label>
			</div>
			<button class="btn primary" on:click={generateCredentialProof}>
				Generate ZK Proof
			</button>
			<p class="info-text">
				Zero-knowledge proofs verify attributes without revealing identity.
				Websites see "✓ Verified" but never learn WHO you are.
			</p>
		</div>
	</section>

	<!-- FINGERPRINT PROTECTION -->
	{#if fingerprintStatus}
		<section class="section legendary-section">
			<div class="section-header">
				<h2>👤 Fingerprint Normalization</h2>
				<p>All NONOS users appear identical to websites - {fingerprintStatus.entropy_reduction} entropy reduction</p>
			</div>
			<div class="card">
				<div class="fingerprint-grid">
					{#each Object.entries(fingerprintStatus.protections || {}) as [key, value]}
						<div class="fingerprint-item">
							<span class="fp-key">{key.replace(/_/g, ' ')}</span>
							<span class="fp-value">{value}</span>
						</div>
					{/each}
				</div>
			</div>
		</section>
	{/if}

	<!-- MIXNET STATUS -->
	{#if mixnetStatus}
		<section class="section legendary-section">
			<div class="section-header">
				<h2>🌐 Multi-Node Mixnet</h2>
				<p>Requests mixed through {mixnetStatus.nodes_in_path} NONOS nodes before Anyone Network</p>
			</div>
			<div class="card">
				<div class="mixnet-visual">
					<div class="mix-node you">YOU</div>
					<div class="mix-arrow">→</div>
					<div class="mix-node">NODE 1</div>
					<div class="mix-arrow">→</div>
					<div class="mix-node">NODE 2</div>
					<div class="mix-arrow">→</div>
					<div class="mix-node">NODE 3</div>
					<div class="mix-arrow">→</div>
					<div class="mix-node anyone">ANYONE</div>
					<div class="mix-arrow">→</div>
					<div class="mix-node destination">SITE</div>
				</div>
				<div class="mixnet-stats">
					<div class="mix-stat">
						<span class="value">{mixnetStatus.mixing_pool_size}</span>
						<span class="label">Pool Size</span>
					</div>
					<div class="mix-stat">
						<span class="value">{mixnetStatus.max_delay_ms}ms</span>
						<span class="label">Max Delay</span>
					</div>
					<div class="mix-stat">
						<span class="value">{mixnetStatus.requests_mixed}</span>
						<span class="label">Mixed</span>
					</div>
				</div>
			</div>
		</section>
	{/if}

	{#if error}
		<div class="error-banner">
			<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
				<circle cx="12" cy="12" r="10"/>
				<path d="M12 8v4M12 16h.01"/>
			</svg>
			<span>{error}</span>
		</div>
	{/if}

	<!-- Network Status Banner -->
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

	{#if true}
		<!-- Stats Overview -->
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

		<!-- ZK Identity Section -->
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

		<!-- Cache Mixer Section -->
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

		<!-- Tracking Blocker Section -->
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

		<!-- Stealth Payments -->
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
	{/if}
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

	.legendary-hero {
		background: linear-gradient(135deg, rgba(139, 92, 246, 0.1) 0%, rgba(59, 130, 246, 0.1) 100%);
		border: 1px solid rgba(139, 92, 246, 0.3);
		border-radius: var(--nox-radius-xl);
		padding: var(--nox-space-xl);
		margin-bottom: var(--nox-space-xl);
	}

	.legendary-badge {
		display: inline-flex;
		align-items: center;
		gap: var(--nox-space-sm);
		background: linear-gradient(135deg, #8b5cf6, #3b82f6);
		padding: var(--nox-space-sm) var(--nox-space-lg);
		border-radius: var(--nox-radius-full);
		margin-bottom: var(--nox-space-lg);
	}

	.badge-icon {
		font-size: 18px;
	}

	.badge-text {
		font-weight: var(--nox-font-semibold);
		font-size: var(--nox-text-sm);
		color: white;
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.legendary-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
		gap: var(--nox-space-md);
	}

	.legendary-card {
		background: var(--nox-bg-secondary);
		border: 1px solid var(--nox-border);
		border-radius: var(--nox-radius-lg);
		padding: var(--nox-space-md);
		transition: all var(--nox-transition-fast);
	}

	.legendary-card.active {
		border-color: var(--nox-success);
		background: rgba(34, 197, 94, 0.05);
	}

	.legendary-card .card-status {
		font-size: 18px;
		margin-bottom: var(--nox-space-xs);
	}

	.legendary-card.active .card-status {
		color: var(--nox-success);
	}

	.legendary-card .card-title {
		font-weight: var(--nox-font-medium);
		font-size: var(--nox-text-sm);
		margin-bottom: var(--nox-space-2xs);
		text-transform: capitalize;
	}

	.legendary-card .card-desc {
		font-size: var(--nox-text-xs);
		color: var(--nox-text-muted);
		line-height: 1.4;
	}

	.legendary-section {
		border-left: 3px solid transparent;
		border-image: linear-gradient(180deg, #8b5cf6, #3b82f6) 1;
		padding-left: var(--nox-space-lg);
	}

	.stealth-active {
		background: linear-gradient(135deg, rgba(34, 197, 94, 0.1) 0%, rgba(16, 185, 129, 0.05) 100%);
		border: 1px solid var(--nox-success);
		border-radius: var(--nox-radius-lg);
		padding: var(--nox-space-lg);
		margin-bottom: var(--nox-space-md);
	}

	.stealth-indicator {
		display: flex;
		align-items: center;
		gap: var(--nox-space-sm);
		font-weight: var(--nox-font-semibold);
		color: var(--nox-success);
		margin-bottom: var(--nox-space-md);
	}

	.stealth-indicator .pulse {
		width: 10px;
		height: 10px;
		background: var(--nox-success);
		border-radius: 50%;
		animation: stealth-pulse 1.5s ease-in-out infinite;
	}

	@keyframes stealth-pulse {
		0%, 100% {
			opacity: 1;
			box-shadow: 0 0 0 0 rgba(34, 197, 94, 0.4);
		}
		50% {
			opacity: 0.8;
			box-shadow: 0 0 0 8px rgba(34, 197, 94, 0);
		}
	}

	.stealth-details {
		display: flex;
		flex-direction: column;
		gap: var(--nox-space-sm);
	}

	.stealth-details .detail {
		display: flex;
		align-items: center;
		gap: var(--nox-space-md);
	}

	.stealth-details .label {
		font-size: var(--nox-text-sm);
		color: var(--nox-text-muted);
		min-width: 120px;
	}

	.stealth-details code {
		font-family: var(--nox-font-mono);
		font-size: var(--nox-text-xs);
		color: var(--nox-accent-primary);
		background: var(--nox-bg-tertiary);
		padding: var(--nox-space-2xs) var(--nox-space-sm);
		border-radius: var(--nox-radius-sm);
	}

	.domain-check {
		display: flex;
		gap: var(--nox-space-md);
		margin-bottom: var(--nox-space-md);
	}

	.domain-result {
		display: flex;
		align-items: center;
		gap: var(--nox-space-lg);
		padding: var(--nox-space-lg);
		background: var(--nox-bg-tertiary);
		border-radius: var(--nox-radius-lg);
		margin-top: var(--nox-space-md);
	}

	.domain-result.good {
		border: 1px solid var(--nox-success);
		background: rgba(34, 197, 94, 0.05);
	}

	.domain-result.bad {
		border: 1px solid var(--nox-error);
		background: rgba(239, 68, 68, 0.05);
	}

	.score-circle {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		width: 80px;
		height: 80px;
		border-radius: 50%;
		background: var(--nox-bg-secondary);
		border: 3px solid currentColor;
		flex-shrink: 0;
	}

	.domain-result.good .score-circle {
		color: var(--nox-success);
	}

	.domain-result.bad .score-circle {
		color: var(--nox-error);
	}

	.score-circle .grade {
		font-size: var(--nox-text-xl);
		font-weight: var(--nox-font-bold);
	}

	.score-circle .score {
		font-size: var(--nox-text-xs);
		color: var(--nox-text-muted);
	}

	.score-details {
		flex: 1;
	}

	.score-details .domain-name {
		font-weight: var(--nox-font-semibold);
		font-size: var(--nox-text-lg);
		margin-bottom: var(--nox-space-xs);
	}

	.score-details .recommendation {
		font-size: var(--nox-text-sm);
		color: var(--nox-text-secondary);
		margin-bottom: var(--nox-space-sm);
	}

	.score-details .trackers-found {
		font-size: var(--nox-text-xs);
		color: var(--nox-error);
	}

	.credential-options {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: var(--nox-space-md);
		margin-bottom: var(--nox-space-lg);
	}

	@media (max-width: 600px) {
		.credential-options {
			grid-template-columns: 1fr;
		}
	}

	.credential-option {
		display: flex;
		align-items: center;
		gap: var(--nox-space-sm);
		padding: var(--nox-space-md);
		background: var(--nox-bg-tertiary);
		border: 1px solid var(--nox-border);
		border-radius: var(--nox-radius-md);
		cursor: pointer;
		transition: all var(--nox-transition-fast);
	}

	.credential-option:hover {
		border-color: var(--nox-accent-primary);
	}

	.credential-option:has(input:checked) {
		border-color: var(--nox-accent-primary);
		background: var(--nox-accent-glow);
	}

	.credential-option input {
		accent-color: var(--nox-accent-primary);
	}

	.credential-option span {
		font-size: var(--nox-text-sm);
		font-weight: var(--nox-font-medium);
	}

	.fingerprint-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
		gap: var(--nox-space-sm);
	}

	.fingerprint-item {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: var(--nox-space-sm) var(--nox-space-md);
		background: var(--nox-bg-tertiary);
		border-radius: var(--nox-radius-md);
	}

	.fingerprint-item .fp-key {
		font-size: var(--nox-text-xs);
		color: var(--nox-text-muted);
		text-transform: capitalize;
	}

	.fingerprint-item .fp-value {
		font-size: var(--nox-text-xs);
		font-family: var(--nox-font-mono);
		color: var(--nox-accent-primary);
	}

	.mixnet-visual {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: var(--nox-space-sm);
		padding: var(--nox-space-lg);
		background: var(--nox-bg-tertiary);
		border-radius: var(--nox-radius-lg);
		margin-bottom: var(--nox-space-lg);
		overflow-x: auto;
	}

	.mix-node {
		display: flex;
		align-items: center;
		justify-content: center;
		min-width: 60px;
		height: 40px;
		padding: 0 var(--nox-space-sm);
		background: var(--nox-bg-secondary);
		border: 1px solid var(--nox-border);
		border-radius: var(--nox-radius-md);
		font-size: var(--nox-text-xs);
		font-weight: var(--nox-font-medium);
		white-space: nowrap;
	}

	.mix-node.you {
		background: linear-gradient(135deg, #8b5cf6, #3b82f6);
		border: none;
		color: white;
	}

	.mix-node.anyone {
		background: var(--nox-accent-primary);
		border: none;
		color: var(--nox-bg-primary);
	}

	.mix-node.destination {
		background: var(--nox-success);
		border: none;
		color: white;
	}

	.mix-arrow {
		color: var(--nox-text-muted);
		font-size: 16px;
	}

	.mixnet-stats {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: var(--nox-space-md);
	}

	.mix-stat {
		display: flex;
		flex-direction: column;
		align-items: center;
		padding: var(--nox-space-md);
		background: var(--nox-bg-tertiary);
		border-radius: var(--nox-radius-md);
	}

	.mix-stat .value {
		font-size: var(--nox-text-xl);
		font-weight: var(--nox-font-semibold);
		font-family: var(--nox-font-mono);
		color: var(--nox-accent-primary);
	}

	.mix-stat .label {
		font-size: var(--nox-text-xs);
		color: var(--nox-text-muted);
		margin-top: var(--nox-space-2xs);
	}
</style>
