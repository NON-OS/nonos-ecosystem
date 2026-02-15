<script lang="ts">
	import { invoke } from '@tauri-apps/api/tauri';
	import { browserStore } from '$lib/stores/browser';

	export let networkStatus: { connected: boolean; bootstrap_progress: number; circuits: number };
	export let walletAddress: string;

	let url = '';

	function formatAddress(addr: string): string {
		if (!addr) return '';
		return `${addr.slice(0, 6)}...${addr.slice(-4)}`;
	}

	/**
	 * Navigate to a URL using native Tauri webview windows.
	 * NONOS NODES POWER THE BROWSER - all traffic routed through Anyone Network!
	 */
	async function navigate() {
		if (!url) return;

		// Determine final URL
		let targetUrl: string;
		if (url.startsWith('http://') || url.startsWith('https://')) {
			targetUrl = url;
		} else if (url.includes('.') && !url.includes(' ')) {
			targetUrl = 'https://' + url;
		} else {
			// Search query
			targetUrl = `https://html.duckduckgo.com/html/?q=${encodeURIComponent(url)}`;
		}

		console.log('NONOS TopBar: Opening URL via NONOS Node Network:', targetUrl);

		browserStore.setLoading(true);

		try {
			// Use Tauri's browser_navigate to open in a new native window
			// Traffic is routed through NONOS Nodes via Anyone Network SOCKS5 proxy
			const result = await invoke('browser_navigate', { url: targetUrl }) as string;
			console.log('NONOS TopBar: Browser navigate result:', result);

			// Parse tab ID from result
			const tabMatch = result.match(/tab (\d+)/);
			const tabId = tabMatch ? parseInt(tabMatch[1]) : Date.now();

			// Update store
			browserStore.setPageContent({
				url: targetUrl,
				content: 'native-window',
				viaProxy: networkStatus.connected,
				circuitId: 'nonos-node-' + tabId
			});

			browserStore.setLoading(false);
		} catch (e) {
			console.error('NONOS TopBar: Navigation failed:', e);
			browserStore.setError(`Navigation failed: ${e}`);
			browserStore.setLoading(false);
		}
	}

	function handleBack() {
		// Back/forward with native windows - just show info
		console.log('NONOS: Back navigation - use browser window controls');
	}

	function handleForward() {
		// Back/forward with native windows - just show info
		console.log('NONOS: Forward navigation - use browser window controls');
	}

	function handleReload() {
		if (url) {
			navigate();
		}
	}

	async function handleNewCircuit() {
		try {
			if (window.nonos) {
				await window.nonos.network.newIdentity();
			}
		} catch (e) {
			console.error('Failed to create new identity:', e);
		}
	}
</script>

<header class="topbar">
	<div class="nav-controls">
		<button class="nav-btn" on:click={handleBack} title="Go Back" aria-label="Go Back">
			<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
				<path d="M19 12H5M12 19l-7-7 7-7" />
			</svg>
		</button>
		<button class="nav-btn" on:click={handleForward} title="Go Forward" aria-label="Go Forward">
			<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
				<path d="M5 12h14M12 5l7 7-7 7" />
			</svg>
		</button>
		<button class="nav-btn" on:click={handleReload} title="Reload Page" aria-label="Reload" class:loading={$browserStore.isLoading}>
			<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
				<path d="M23 4v6h-6M1 20v-6h6" />
				<path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15" />
			</svg>
		</button>
	</div>

	<form class="url-bar" on:submit|preventDefault={navigate}>
		<div class="security-badge" class:secure={networkStatus.connected}>
			{#if networkStatus.connected}
				<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
					<path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
					<path d="M9 12l2 2 4-4" />
				</svg>
				<span class="security-text">Secure</span>
			{:else}
				<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
					<circle cx="12" cy="12" r="10" />
					<path d="M12 8v4M12 16h.01" />
				</svg>
				<span class="security-text">Not Connected</span>
			{/if}
		</div>
		<input
			type="text"
			bind:value={url}
			placeholder="Search or enter URL..."
			class="url-input"
			autocomplete="off"
			spellcheck="false"
		/>
		<button type="submit" class="go-btn" disabled={!url} aria-label="Navigate">
			<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
				<path d="M5 12h14M12 5l7 7-7 7" />
			</svg>
		</button>
	</form>

	<div class="toolbar-actions">
		<button class="action-btn identity-btn" on:click={handleNewCircuit} title="New Identity (New Circuit)">
			<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
				<path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
			</svg>
			{#if networkStatus.circuits > 0}
				<span class="circuit-badge">{networkStatus.circuits}</span>
			{/if}
		</button>

		{#if walletAddress}
			<a href="/wallet" class="wallet-pill">
				<span class="wallet-dot"></span>
				<span class="wallet-address">{formatAddress(walletAddress)}</span>
			</a>
		{:else}
			<a href="/wallet" class="wallet-pill empty">
				<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
					<rect x="2" y="6" width="20" height="14" rx="2"/>
					<path d="M2 10h20"/>
				</svg>
				<span>Connect</span>
			</a>
		{/if}
	</div>
</header>

<style>
	.topbar {
		display: flex;
		align-items: center;
		gap: var(--nox-space-md);
		padding: var(--nox-space-sm) var(--nox-space-lg);
		background: var(--nox-bg-secondary);
		border-bottom: 1px solid var(--nox-border);
		-webkit-app-region: drag;
		height: 52px;
	}

	.nav-controls {
		display: flex;
		gap: var(--nox-space-xs);
		-webkit-app-region: no-drag;
	}

	.nav-btn {
		width: 32px;
		height: 32px;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: var(--nox-radius-md);
		color: var(--nox-text-muted);
		background: transparent;
		transition: all var(--nox-transition-fast);
	}

	.nav-btn svg {
		width: 16px;
		height: 16px;
	}

	.nav-btn:hover {
		background: var(--nox-bg-hover);
		color: var(--nox-text-primary);
	}

	.nav-btn:active {
		background: var(--nox-bg-active);
	}

	.nav-btn.loading svg {
		animation: spin 0.8s linear infinite;
	}

	.url-bar {
		flex: 1;
		display: flex;
		align-items: center;
		gap: var(--nox-space-sm);
		background: var(--nox-bg-tertiary);
		border: 1px solid var(--nox-border);
		border-radius: var(--nox-radius-lg);
		padding: var(--nox-space-xs) var(--nox-space-xs) var(--nox-space-xs) var(--nox-space-sm);
		-webkit-app-region: no-drag;
		transition: all var(--nox-transition-fast);
	}

	.url-bar:focus-within {
		border-color: var(--nox-accent-primary);
		box-shadow: 0 0 0 3px var(--nox-accent-glow);
	}

	.security-badge {
		display: flex;
		align-items: center;
		gap: var(--nox-space-xs);
		padding: var(--nox-space-2xs) var(--nox-space-sm);
		background: var(--nox-warning-bg);
		border-radius: var(--nox-radius-md);
		color: var(--nox-warning);
		font-size: var(--nox-text-xs);
		font-weight: var(--nox-font-medium);
		white-space: nowrap;
	}

	.security-badge svg {
		width: 14px;
		height: 14px;
	}

	.security-badge.secure {
		background: var(--nox-success-bg);
		color: var(--nox-success);
	}

	.security-text {
		display: none;
	}

	@media (min-width: 800px) {
		.security-text {
			display: inline;
		}
	}

	.url-input {
		flex: 1;
		background: transparent;
		border: none;
		padding: var(--nox-space-xs) var(--nox-space-sm);
		font-size: var(--nox-text-sm);
		color: var(--nox-text-primary);
		min-width: 0;
	}

	.url-input:focus {
		outline: none;
	}

	.url-input::placeholder {
		color: var(--nox-text-muted);
	}

	.go-btn {
		width: 32px;
		height: 32px;
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--nox-accent-gradient);
		border-radius: var(--nox-radius-md);
		color: var(--nox-bg-primary);
		transition: all var(--nox-transition-fast);
	}

	.go-btn svg {
		width: 16px;
		height: 16px;
	}

	.go-btn:hover:not(:disabled) {
		box-shadow: var(--nox-shadow-glow);
		transform: translateX(2px);
	}

	.go-btn:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}

	.toolbar-actions {
		display: flex;
		align-items: center;
		gap: var(--nox-space-sm);
		-webkit-app-region: no-drag;
	}

	.action-btn {
		width: 36px;
		height: 36px;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: var(--nox-radius-md);
		color: var(--nox-text-muted);
		background: var(--nox-bg-tertiary);
		border: 1px solid var(--nox-border);
		transition: all var(--nox-transition-fast);
	}

	.action-btn svg {
		width: 18px;
		height: 18px;
	}

	.action-btn:hover {
		border-color: var(--nox-accent-primary);
		color: var(--nox-accent-primary);
	}

	.identity-btn {
		position: relative;
	}

	.circuit-badge {
		position: absolute;
		top: -4px;
		right: -4px;
		min-width: 18px;
		height: 18px;
		padding: 0 5px;
		background: var(--nox-accent-primary);
		border-radius: var(--nox-radius-full);
		font-size: 10px;
		font-weight: var(--nox-font-semibold);
		color: var(--nox-bg-primary);
		display: flex;
		align-items: center;
		justify-content: center;
		font-family: var(--nox-font-mono);
	}

	.wallet-pill {
		display: flex;
		align-items: center;
		gap: var(--nox-space-sm);
		padding: var(--nox-space-xs) var(--nox-space-md);
		background: var(--nox-bg-tertiary);
		border: 1px solid var(--nox-border);
		border-radius: var(--nox-radius-full);
		font-size: var(--nox-text-sm);
		color: var(--nox-text-secondary);
		text-decoration: none;
		transition: all var(--nox-transition-fast);
	}

	.wallet-pill:hover {
		border-color: var(--nox-accent-primary);
		color: var(--nox-text-primary);
	}

	.wallet-pill svg {
		width: 16px;
		height: 16px;
	}

	.wallet-pill.empty {
		background: var(--nox-accent-gradient);
		border: none;
		color: var(--nox-bg-primary);
		font-weight: var(--nox-font-medium);
	}

	.wallet-pill.empty:hover {
		box-shadow: var(--nox-shadow-glow);
	}

	.wallet-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: var(--nox-success);
		box-shadow: 0 0 6px var(--nox-success);
	}

	.wallet-address {
		font-family: var(--nox-font-mono);
		font-size: var(--nox-text-xs);
	}

	@keyframes spin {
		100% { transform: rotate(360deg); }
	}
</style>
