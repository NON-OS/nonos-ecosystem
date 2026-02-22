<script lang="ts">
	import { page } from '$app/stores';

	export let networkStatus: { connected: boolean; bootstrap_progress: number; circuits: number };

	const navItems = [
		{ path: '/', label: 'Browser', icon: 'globe' },
		{ path: '/wallet', label: 'Wallet', icon: 'wallet' },
		{ path: '/staking', label: 'Staking', icon: 'staking' },
		{ path: '/network', label: 'Network', icon: 'network' },
		{ path: '/node', label: 'Node', icon: 'server' },
		{ path: '/privacy', label: 'Privacy', icon: 'shield' },
		{ path: '/settings', label: 'Settings', icon: 'settings' }
	];

	$: currentPath = $page.url.pathname;
</script>

<aside class="sidebar">
	<div class="sidebar-header">
		<div class="logo">
			<!-- Official NONOS Logo Icon - Embedded SVG -->
			<svg class="logo-icon" viewBox="0 0 450 500" fill="none" xmlns="http://www.w3.org/2000/svg">
				<path d="M280.574 205.148C281.041 206.156 281.49 207.184 281.918 208.232C286.849 220.104 289.315 233.94 289.315 249.739C289.315 265.447 286.849 279.283 281.918 291.246C277.078 303.21 269.726 312.571 259.863 319.329C250.092 325.996 237.854 329.283 223.15 329.191C208.447 329.1 196.165 325.721 186.302 319.055C184.37 317.719 182.534 316.281 180.791 314.745L280.574 205.148Z" fill="#66FFFF"/>
				<path d="M223.15 170.288C237.854 170.379 250.092 173.758 259.863 180.425C262.495 182.203 264.945 184.169 267.219 186.318L166.722 296.461C165.838 294.739 165.013 292.955 164.246 291.109C159.315 279.237 156.895 265.447 156.986 249.739C157.078 233.94 159.589 220.059 164.521 208.096C169.452 196.132 176.758 186.817 186.438 180.15C196.21 173.484 208.447 170.197 223.15 170.288Z" fill="#66FFFF"/>
				<path fill-rule="evenodd" clip-rule="evenodd" d="M370 0C414.183 0 450 35.8172 450 80V420C450 464.183 414.183 500 370 500H80C35.8172 500 0 464.183 0 420V80C2.3583e-05 35.8173 35.8173 2.57867e-05 80 0H370ZM223.15 147C203.516 147 186.849 151.338 173.15 160.014C159.452 168.689 148.996 180.744 141.781 196.178C134.658 211.611 131.096 229.465 131.096 249.739C131.096 270.013 134.658 287.868 141.781 303.302C143.839 307.704 146.162 311.83 148.747 315.683L130 336.315L147.397 351.932L164.499 333.18C167.223 335.441 170.106 337.538 173.15 339.466C186.849 348.142 203.516 352.479 223.15 352.479C242.876 352.479 259.589 348.142 273.288 339.466C286.986 330.79 297.397 318.735 304.521 303.302C311.644 287.868 315.205 270.013 315.205 249.739C315.205 229.465 311.644 211.611 304.521 196.178C302.836 192.528 300.965 189.068 298.912 185.796L319.041 163.575L301.644 147.959L283.659 167.779C280.441 164.945 276.985 162.355 273.288 160.014C259.589 151.338 242.876 147 223.15 147Z" fill="#66FFFF"/>
			</svg>
			<span class="logo-name">NONOS</span>
		</div>
		<div class="logo-subtitle">
			<span class="logo-tagline">Nodes Power The Browser</span>
		</div>
	</div>

	<nav class="sidebar-nav">
		{#each navItems as item}
			<a
				href={item.path}
				class="nav-item"
				class:active={currentPath === item.path}
				aria-current={currentPath === item.path ? 'page' : undefined}
			>
				<span class="nav-icon">
					{#if item.icon === 'globe'}
						<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
							<circle cx="12" cy="12" r="10"/>
							<path d="M2 12h20M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/>
						</svg>
					{:else if item.icon === 'wallet'}
						<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
							<rect x="2" y="6" width="20" height="14" rx="2"/>
							<path d="M2 10h20"/>
							<circle cx="16" cy="14" r="2"/>
						</svg>
					{:else if item.icon === 'network'}
						<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
							<circle cx="12" cy="5" r="3"/>
							<circle cx="5" cy="19" r="3"/>
							<circle cx="19" cy="19" r="3"/>
							<path d="M12 8v4M8.5 14.5L5.5 17M15.5 14.5l3 2.5"/>
							<circle cx="12" cy="14" r="2"/>
						</svg>
					{:else if item.icon === 'server'}
						<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
							<rect x="3" y="4" width="18" height="6" rx="1"/>
							<rect x="3" y="14" width="18" height="6" rx="1"/>
							<circle cx="7" cy="7" r="1" fill="currentColor"/>
							<circle cx="7" cy="17" r="1" fill="currentColor"/>
						</svg>
					{:else if item.icon === 'staking'}
						<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
							<path d="M12 2v20M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6"/>
						</svg>
					{:else if item.icon === 'shield'}
						<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
							<path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>
						</svg>
					{:else if item.icon === 'settings'}
						<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
							<circle cx="12" cy="12" r="3"/>
							<path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"/>
						</svg>
					{/if}
				</span>
				<span class="nav-label">{item.label}</span>
				{#if currentPath === item.path}
					<span class="nav-indicator"></span>
				{/if}
			</a>
		{/each}
	</nav>

	<div class="status-panel">
		<div class="status-header">
			<span class="status-title">Network Status</span>
		</div>
		<div class="status-row">
			<span class="status-dot" class:connected={networkStatus.connected}></span>
			<span class="status-label">{networkStatus.connected ? 'Connected' : 'Disconnected'}</span>
		</div>
		{#if !networkStatus.connected && networkStatus.bootstrap_progress > 0}
			<div class="bootstrap-progress">
				<div class="progress-track">
					<div class="progress-fill" style="width: {networkStatus.bootstrap_progress}%"></div>
				</div>
				<span class="progress-label">{networkStatus.bootstrap_progress}%</span>
			</div>
		{/if}
		<div class="status-row">
			<span class="status-label-muted">Active Circuits</span>
			<span class="status-value">{networkStatus.circuits}</span>
		</div>
	</div>

	<div class="sidebar-footer">
		<div class="footer-row">
			<span class="version">v1.0.5</span>
			<a href="https://nonos.systems" target="_blank" rel="noopener" class="footer-link">
				nonos.systems
			</a>
		</div>
	</div>
</aside>

<style>
	.sidebar {
		width: 240px;
		min-width: 240px;
		height: 100%;
		display: flex;
		flex-direction: column;
		background: var(--nox-bg-secondary);
		border-right: 1px solid var(--nox-border);
	}

	.sidebar-header {
		padding: var(--nox-space-lg);
		border-bottom: 1px solid var(--nox-border-subtle);
	}

	.logo {
		display: flex;
		align-items: center;
		gap: var(--nox-space-md);
	}

	.logo-icon {
		width: 36px;
		height: 36px;
		flex-shrink: 0;
	}

	.logo-name {
		font-size: var(--nox-text-lg);
		font-weight: var(--nox-font-semibold);
		letter-spacing: 0.08em;
		color: var(--nox-text-primary);
	}

	.logo-tagline {
		font-size: var(--nox-text-xs);
		font-weight: var(--nox-font-light);
		color: var(--nox-text-muted);
		letter-spacing: 0.02em;
	}

	.sidebar-nav {
		flex: 1;
		padding: var(--nox-space-md);
		display: flex;
		flex-direction: column;
		gap: var(--nox-space-2xs);
	}

	.nav-item {
		display: flex;
		align-items: center;
		gap: var(--nox-space-md);
		padding: var(--nox-space-sm) var(--nox-space-md);
		border-radius: var(--nox-radius-md);
		color: var(--nox-text-secondary);
		text-decoration: none;
		transition: all var(--nox-transition-fast);
		position: relative;
	}

	.nav-item:hover {
		background: var(--nox-bg-hover);
		color: var(--nox-text-primary);
	}

	.nav-item.active {
		background: var(--nox-accent-glow);
		color: var(--nox-accent-primary);
	}

	.nav-icon {
		width: 20px;
		height: 20px;
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
	}

	.nav-icon svg {
		width: 18px;
		height: 18px;
	}

	.nav-label {
		font-size: var(--nox-text-sm);
		font-weight: var(--nox-font-medium);
	}

	.nav-indicator {
		position: absolute;
		left: 0;
		top: 50%;
		transform: translateY(-50%);
		width: 3px;
		height: 20px;
		background: var(--nox-accent-primary);
		border-radius: 0 var(--nox-radius-sm) var(--nox-radius-sm) 0;
		box-shadow: 0 0 8px var(--nox-accent-primary);
	}

	.status-panel {
		margin: var(--nox-space-md);
		padding: var(--nox-space-md);
		background: var(--nox-bg-tertiary);
		border: 1px solid var(--nox-border-subtle);
		border-radius: var(--nox-radius-lg);
	}

	.status-header {
		margin-bottom: var(--nox-space-sm);
	}

	.status-title {
		font-size: var(--nox-text-xs);
		font-weight: var(--nox-font-medium);
		color: var(--nox-text-muted);
		text-transform: uppercase;
		letter-spacing: 0.08em;
	}

	.status-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: var(--nox-space-xs) 0;
	}

	.status-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: var(--nox-error);
		margin-right: var(--nox-space-sm);
	}

	.status-dot.connected {
		background: var(--nox-success);
		box-shadow: 0 0 8px var(--nox-success);
		animation: pulse-glow 2s infinite;
	}

	.status-label {
		font-size: var(--nox-text-sm);
		color: var(--nox-text-secondary);
		display: flex;
		align-items: center;
	}

	.status-label-muted {
		font-size: var(--nox-text-xs);
		color: var(--nox-text-muted);
	}

	.status-value {
		font-size: var(--nox-text-sm);
		font-family: var(--nox-font-mono);
		color: var(--nox-accent-primary);
		font-weight: var(--nox-font-medium);
	}

	.bootstrap-progress {
		display: flex;
		align-items: center;
		gap: var(--nox-space-sm);
		padding: var(--nox-space-xs) 0;
	}

	.progress-track {
		flex: 1;
		height: 4px;
		background: var(--nox-bg-primary);
		border-radius: var(--nox-radius-full);
		overflow: hidden;
	}

	.progress-fill {
		height: 100%;
		background: var(--nox-accent-gradient);
		border-radius: var(--nox-radius-full);
		transition: width var(--nox-transition-normal);
	}

	.progress-label {
		font-size: var(--nox-text-xs);
		font-family: var(--nox-font-mono);
		color: var(--nox-text-muted);
		min-width: 32px;
		text-align: right;
	}

	.sidebar-footer {
		padding: var(--nox-space-md) var(--nox-space-lg);
		border-top: 1px solid var(--nox-border-subtle);
	}

	.footer-row {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.version {
		font-size: var(--nox-text-xs);
		font-family: var(--nox-font-mono);
		color: var(--nox-text-disabled);
	}

	.footer-link {
		font-size: var(--nox-text-xs);
		color: var(--nox-text-muted);
		text-decoration: none;
		transition: color var(--nox-transition-fast);
	}

	.footer-link:hover {
		color: var(--nox-accent-primary);
	}

	@keyframes pulse-glow {
		0%, 100% {
			opacity: 1;
			box-shadow: 0 0 8px var(--nox-success);
		}
		50% {
			opacity: 0.7;
			box-shadow: 0 0 4px var(--nox-success);
		}
	}
</style>
