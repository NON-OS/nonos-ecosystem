<script lang="ts">
	import { invoke } from '@tauri-apps/api/tauri';
	import { browserStore } from '$lib/stores/browser';

	// Track open browser tabs
	let openTabs: { id: number; url: string }[] = [];

	/**
	 * Navigate to a URL by opening it in a new Tauri browser window.
	 * NONOS NODES POWER THE BROWSER - all traffic routed through Anyone Network!
	 * This provides FULL page rendering with CSS, JS, images - everything works!
	 */
	async function navigateToUrl(url: string) {
		if (!url) return;

		console.log('NONOS: Opening URL via NONOS Node Network:', url);

		browserStore.setLoading(true);

		try {
			// Use Tauri's browser_navigate command to open in a new window
			// Traffic is routed through NONOS Nodes via Anyone Network SOCKS5 proxy
			const result = await invoke('browser_navigate', { url }) as string;
			console.log('NONOS: Browser navigate result:', result);

			// Parse tab ID from result (format: "Opened URL in tab X ...")
			const tabMatch = result.match(/tab (\d+)/);
			const tabId = tabMatch ? parseInt(tabMatch[1]) : Date.now();

			// Track the opened tab
			openTabs = [...openTabs, { id: tabId, url }];

			// Update store to show we're browsing
			browserStore.setPageContent({
				url: url,
				content: 'native-window', // Mark as native window
				viaProxy: true,
				circuitId: 'nonos-node-' + tabId
			});

			browserStore.setLoading(false);
		} catch (e) {
			console.error('Navigation failed:', e);
			browserStore.setError(`Navigation failed: ${e}`);
			browserStore.setLoading(false);
		}
	}

	async function handleQuickLink(event: MouseEvent, url: string) {
		event.preventDefault();
		await navigateToUrl(url);
	}

	// Clear browsing state to return to home
	function returnToHome() {
		browserStore.clearPage();
	}

	// Close a browser tab
	async function closeTab(tabId: number) {
		try {
			await invoke('browser_close_tab', { tabId });
			openTabs = openTabs.filter(t => t.id !== tabId);
			if (openTabs.length === 0) {
				browserStore.clearPage();
			}
		} catch (e) {
			console.error('Failed to close tab:', e);
		}
	}
</script>

<div class="browser-page">
	{#if $browserStore.pageContent}
		<!-- Browsing mode - show fetched content -->
		<!-- Page opened in native window - show tab manager -->
		<div class="native-browser-view">
			<div class="open-tabs-header">
				<h2>
					<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
						<path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
						<path d="M9 12l2 2 4-4" />
					</svg>
					NONOS Nodes Power This Browser
				</h2>
				<p class="privacy-note">Network Traffic: <strong>Anyone Protocol</strong> (Onion Routing) | Infrastructure: <strong>NONOS Community Nodes</strong></p>
			</div>

			{#if $browserStore.isLoading}
				<div class="loading-indicator">
					<div class="spinner"></div>
					<p>Opening page via NONOS Node Network...</p>
				</div>
			{/if}

			{#if $browserStore.errorMessage}
				<div class="error-message">
					<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
						<circle cx="12" cy="12" r="10" />
						<path d="M12 8v4M12 16h.01" />
					</svg>
					<p>{$browserStore.errorMessage}</p>
					<button class="retry-btn" on:click={() => navigateToUrl($browserStore.currentUrl)}>Retry</button>
				</div>
			{:else}
				<div class="open-tabs-list">
					{#each openTabs as tab}
						<div class="tab-card">
							<div class="tab-icon">
								<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
									<path d="M3 12a9 9 0 1 0 18 0 9 9 0 0 0-18 0"/>
									<path d="M3.6 9h16.8M3.6 15h16.8"/>
									<path d="M12 3a15 15 0 0 1 0 18 15 15 0 0 1 0-18"/>
								</svg>
							</div>
							<div class="tab-info">
								<span class="tab-url">{tab.url}</span>
								<span class="tab-status">
									<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
										<path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
									</svg>
									Routed via NONOS Nodes
								</span>
							</div>
							<button class="close-tab-btn" on:click={() => closeTab(tab.id)} title="Close tab">
								<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
									<path d="M18 6L6 18M6 6l12 12" />
								</svg>
							</button>
						</div>
					{/each}

					{#if openTabs.length === 0 && $browserStore.currentUrl}
						<div class="tab-card">
							<div class="tab-icon">
								<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
									<path d="M3 12a9 9 0 1 0 18 0 9 9 0 0 0-18 0"/>
									<path d="M3.6 9h16.8M3.6 15h16.8"/>
									<path d="M12 3a15 15 0 0 1 0 18 15 15 0 0 1 0-18"/>
								</svg>
							</div>
							<div class="tab-info">
								<span class="tab-url">{$browserStore.currentUrl}</span>
								<span class="tab-status">
									<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
										<path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
									</svg>
									Opened in browser window - NONOS Node powered
								</span>
							</div>
						</div>
					{/if}
				</div>

				<div class="node-power-info">
					<div class="info-card">
						<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
							<circle cx="12" cy="5" r="3"/>
							<circle cx="5" cy="19" r="3"/>
							<circle cx="19" cy="19" r="3"/>
							<path d="M12 8v4M8.5 14.5L5.5 17M15.5 14.5l3 2.5"/>
							<circle cx="12" cy="14" r="2"/>
						</svg>
						<div>
							<h4>NONOS Nodes = Browser Infrastructure</h4>
							<p>NONOS community nodes ARE the browser's backbone. Every relay, every circuit - powered by node operators running NONOS software.</p>
						</div>
					</div>
					<div class="info-card">
						<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
							<path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>
						</svg>
						<div>
							<h4>Anyone Protocol = Network Layer</h4>
							<p>Traffic encrypted via Anyone's onion routing (multi-hop encryption). Your data bounces through 3+ NONOS nodes before reaching destination.</p>
						</div>
					</div>
					<div class="info-card">
						<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
							<path d="M13 2L3 14h9l-1 8 10-12h-9l1-8z"/>
						</svg>
						<div>
							<h4>Run a Node, Earn NOX</h4>
							<p>Node operators stake NOX and earn rewards for routing browser traffic. More traffic = more rewards. Fair 3-year emissions.</p>
						</div>
					</div>
					<div class="info-card">
						<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
							<rect x="2" y="6" width="20" height="14" rx="2"/>
							<path d="M2 10h20"/>
							<circle cx="16" cy="14" r="2"/>
						</svg>
						<div>
							<h4>Zero-Trust By Design</h4>
							<p>No single node sees your full request. Entry node knows you, exit node sees destination - neither knows both. True privacy.</p>
						</div>
					</div>
				</div>

				<button class="home-btn" on:click={returnToHome}>
					<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
						<path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/>
						<path d="M9 22V12h6v10"/>
					</svg>
					Return to Home
				</button>
			{/if}
		</div>
	{:else}
		<!-- Welcome/Home screen -->
		<div class="webview-container">
			<div class="webview-placeholder">
				<div class="welcome">
					<div class="hero-logo">
						<!-- Official NONOS Logo - Embedded SVG -->
						<svg class="nonos-logo" viewBox="0 0 1428 500" fill="none" xmlns="http://www.w3.org/2000/svg">
							<path d="M1361.48 155.388C1379.33 155.388 1393.96 159.853 1405.31 168.822L1405.84 169.244C1417.04 178.166 1423.37 189.937 1424.81 204.527L1424.86 205.076H1396.67L1396.62 204.638C1395.73 197.536 1392 191.299 1385.37 185.924L1385.37 185.919C1378.77 180.392 1370.01 177.604 1359.03 177.604C1348.79 177.604 1340.48 180.298 1334.06 185.648L1334.05 185.654C1327.66 190.803 1324.44 198.084 1324.44 207.568C1324.44 214.357 1326.32 219.851 1330.03 224.094L1330.77 224.888C1334.56 228.8 1339.05 231.847 1344.26 234.031C1350.04 236.199 1358.09 238.732 1368.42 241.63H1368.42C1380.93 245.078 1391.02 248.53 1398.67 251.989L1399.39 252.303C1406.78 255.616 1413.13 260.733 1418.45 267.642L1418.96 268.307C1424.15 275.248 1426.72 284.576 1426.72 296.24C1426.72 305.581 1424.24 314.374 1419.3 322.61C1414.35 330.858 1407.03 337.536 1397.35 342.65C1387.65 347.772 1376.23 350.323 1363.11 350.323C1350.54 350.323 1339.23 348.137 1329.18 343.754L1329.17 343.75V343.749C1319.31 339.184 1311.54 332.967 1305.86 325.092L1305.86 325.088C1300.17 317.023 1297.24 307.767 1297.06 297.337L1297.05 296.828H1324.4L1324.44 297.278C1325.34 306.046 1328.91 313.46 1335.17 319.542C1341.57 325.409 1350.86 328.38 1363.11 328.38C1374.82 328.38 1383.94 325.499 1390.53 319.798C1397.31 313.911 1400.69 306.434 1400.69 297.328C1400.69 290.171 1398.73 284.402 1394.83 279.978C1390.9 275.508 1385.98 272.108 1380.07 269.778C1374.11 267.43 1366.06 264.896 1355.91 262.179C1343.39 258.912 1333.3 255.641 1325.65 252.364L1325.65 252.362C1318.13 249.062 1311.63 243.931 1306.14 236.982L1306.14 236.976L1306.14 236.97C1300.79 229.787 1298.15 220.242 1298.15 208.384C1298.15 197.959 1300.8 188.704 1306.12 180.637C1311.43 172.574 1318.85 166.35 1328.36 161.962L1328.36 161.96C1338.05 157.574 1349.09 155.388 1361.48 155.388Z" fill="white" stroke="white"/>
							<path d="M1185.11 148C1204.69 148 1221.29 152.307 1234.89 160.92C1248.49 169.533 1258.82 181.502 1265.89 196.824C1272.97 212.147 1276.5 229.872 1276.5 250C1276.5 270.128 1272.97 287.853 1265.89 303.176C1258.82 318.498 1248.49 330.467 1234.89 339.08C1221.29 347.693 1204.69 352 1185.11 352C1165.62 352 1149.07 347.693 1135.47 339.08C1121.87 330.467 1111.49 318.498 1104.33 303.176C1097.26 287.853 1093.72 270.128 1093.72 250C1093.72 229.872 1097.26 212.147 1104.33 196.824C1111.49 181.502 1121.87 169.533 1135.47 160.92C1149.07 152.307 1165.62 148 1185.11 148ZM1185.11 171.12C1170.51 171.03 1158.36 174.294 1148.66 180.912C1139.05 187.531 1131.8 196.779 1126.9 208.656C1122.01 220.534 1119.51 234.315 1119.42 250C1119.33 265.595 1121.73 279.286 1126.63 291.072C1131.53 302.859 1138.83 312.107 1148.53 318.816C1158.32 325.435 1170.51 328.789 1185.11 328.88C1199.71 328.971 1211.86 325.707 1221.56 319.088C1231.35 312.379 1238.65 303.085 1243.46 291.208C1248.35 279.331 1250.8 265.595 1250.8 250C1250.8 234.315 1248.35 220.579 1243.46 208.792C1238.65 197.005 1231.35 187.802 1221.56 181.184C1211.86 174.565 1199.71 171.211 1185.11 171.12Z" fill="white"/>
							<path d="M557.964 157.564L558.112 157.789L656.747 307.087V157.564H682.499V348.42H656.979L656.83 348.195L558.195 198.625V348.42H532.443V157.564H557.964Z" fill="white" stroke="white"/>
							<path d="M724.874 351.456L707.602 335.952L731.946 309.16L735.754 305.488L850.538 179.688L852.714 176.832L878.01 148.952L895.282 164.456L867.81 194.784L864.954 197.096L750.17 323.168L747.45 326.704L724.874 351.456ZM800.082 352C780.588 352 764.042 347.693 750.442 339.08C736.842 330.467 726.46 318.499 719.298 303.176C712.226 287.853 708.69 270.128 708.69 250C708.69 229.872 712.226 212.147 719.298 196.824C726.46 181.501 736.842 169.533 750.442 160.92C764.042 152.307 780.588 148 800.082 148C819.666 148 836.258 152.307 849.858 160.92C863.458 169.533 873.794 181.501 880.866 196.824C887.938 212.147 891.474 229.872 891.474 250C891.474 270.128 887.938 287.853 880.866 303.176C873.794 318.499 863.458 330.467 849.858 339.08C836.258 347.693 819.666 352 800.082 352ZM800.082 328.88C814.679 328.971 826.828 325.707 836.53 319.088C846.322 312.379 853.62 303.085 858.426 291.208C863.322 279.331 865.77 265.595 865.77 250C865.77 234.315 863.322 220.579 858.426 208.792C853.62 197.005 846.322 187.803 836.53 181.184C826.828 174.565 814.679 171.211 800.082 171.12C785.484 171.029 773.335 174.293 763.634 180.912C754.023 187.531 746.77 196.779 741.874 208.656C736.978 220.533 734.484 234.315 734.394 250C734.303 265.595 736.706 279.285 741.602 291.072C746.498 302.859 753.796 312.107 763.498 318.816C773.29 325.435 785.484 328.789 800.082 328.88Z" fill="white"/>
							<path d="M943.021 157.564L943.169 157.789L1041.8 307.087V157.564H1067.56V348.42H1042.04L1041.89 348.195L943.252 198.625V348.42H917.5V157.564H943.021Z" fill="white" stroke="white"/>
							<path d="M280.574 205.148C281.041 206.156 281.49 207.184 281.918 208.232C286.849 220.104 289.315 233.94 289.315 249.739C289.315 265.447 286.849 279.283 281.918 291.246C277.078 303.21 269.726 312.571 259.863 319.329C250.092 325.996 237.854 329.283 223.15 329.191C208.447 329.1 196.165 325.721 186.302 319.055C184.37 317.719 182.534 316.281 180.791 314.745L280.574 205.148Z" fill="#66FFFF"/>
							<path d="M223.15 170.288C237.854 170.379 250.092 173.758 259.863 180.425C262.495 182.203 264.945 184.169 267.219 186.318L166.722 296.461C165.838 294.739 165.013 292.955 164.246 291.109C159.315 279.237 156.895 265.447 156.986 249.739C157.078 233.94 159.589 220.059 164.521 208.096C169.452 196.132 176.758 186.817 186.438 180.15C196.21 173.484 208.447 170.197 223.15 170.288Z" fill="#66FFFF"/>
							<path fill-rule="evenodd" clip-rule="evenodd" d="M370 0C414.183 0 450 35.8172 450 80V420C450 464.183 414.183 500 370 500H80C35.8172 500 0 464.183 0 420V80C2.3583e-05 35.8173 35.8173 2.57867e-05 80 0H370ZM223.15 147C203.516 147 186.849 151.338 173.15 160.014C159.452 168.689 148.996 180.744 141.781 196.178C134.658 211.611 131.096 229.465 131.096 249.739C131.096 270.013 134.658 287.868 141.781 303.302C143.839 307.704 146.162 311.83 148.747 315.683L130 336.315L147.397 351.932L164.499 333.18C167.223 335.441 170.106 337.538 173.15 339.466C186.849 348.142 203.516 352.479 223.15 352.479C242.876 352.479 259.589 348.142 273.288 339.466C286.986 330.79 297.397 318.735 304.521 303.302C311.644 287.868 315.205 270.013 315.205 249.739C315.205 229.465 311.644 211.611 304.521 196.178C302.836 192.528 300.965 189.068 298.912 185.796L319.041 163.575L301.644 147.959L283.659 167.779C280.441 164.945 276.985 162.355 273.288 160.014C259.589 151.338 242.876 147 223.15 147Z" fill="#66FFFF"/>
						</svg>
					</div>
					<h1>Welcome to <span class="brand">NONOS Ecosystem</span></h1>
					<p class="tagline">Zero-Trust OS By Design</p>
					<p class="powered-by">NONOS Nodes = Infrastructure | Anyone Protocol = Network Traffic</p>

					<div class="features">
						<div class="feature">
							<div class="feature-icon">
								<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
									<circle cx="12" cy="5" r="3"/>
									<circle cx="5" cy="19" r="3"/>
									<circle cx="19" cy="19" r="3"/>
									<path d="M12 8v4M8.5 14.5L5.5 17M15.5 14.5l3 2.5"/>
									<circle cx="12" cy="14" r="2"/>
								</svg>
							</div>
							<h3>NONOS Nodes = Infrastructure</h3>
							<p>Community nodes ARE the browser. Every relay running your traffic is a NONOS node.</p>
						</div>
						<div class="feature">
							<div class="feature-icon">
								<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
									<path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>
									<path d="M9 12l2 2 4-4"/>
								</svg>
							</div>
							<h3>Anyone Protocol = Network</h3>
							<p>Onion routing via Anyone Network. Multi-hop encryption through 3+ NONOS nodes.</p>
						</div>
						<div class="feature">
							<div class="feature-icon">
								<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
									<rect x="2" y="6" width="20" height="14" rx="2"/>
									<path d="M2 10h20"/>
									<circle cx="16" cy="14" r="2"/>
								</svg>
							</div>
							<h3>Built-in Wallet</h3>
							<p>Native NOX token & ETH support with stealth addresses</p>
						</div>
						<div class="feature">
							<div class="feature-icon">
								<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
									<path d="M13 2L3 14h9l-1 8 10-12h-9l1-8z"/>
								</svg>
							</div>
							<h3>Run a Node, Earn NOX</h3>
							<p>Stake NOX, route browser traffic, earn rewards. Fair 3-year emissions.</p>
						</div>
					</div>

					<div class="quick-links">
						<h4>Quick Links</h4>
						<div class="link-grid">
							<button class="quick-link" on:click={(e) => handleQuickLink(e, 'https://nonos.systems')}>
								<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
									<path d="M3 12a9 9 0 1 0 18 0 9 9 0 0 0-18 0"/>
									<path d="M3.6 9h16.8M3.6 15h16.8"/>
									<path d="M12 3a15 15 0 0 1 0 18 15 15 0 0 1 0-18"/>
								</svg>
								<span>NONOS Home</span>
							</button>
							<button class="quick-link" on:click={(e) => handleQuickLink(e, 'https://github.com/NON-OS')}>
								<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
									<path d="M9 19c-5 1.5-5-2.5-7-3m14 6v-3.87a3.37 3.37 0 0 0-.94-2.61c3.14-.35 6.44-1.54 6.44-7A5.44 5.44 0 0 0 20 4.77 5.07 5.07 0 0 0 19.91 1S18.73.65 16 2.48a13.38 13.38 0 0 0-7 0C6.27.65 5.09 1 5.09 1A5.07 5.07 0 0 0 5 4.77a5.44 5.44 0 0 0-1.5 3.78c0 5.42 3.3 6.61 6.44 7A3.37 3.37 0 0 0 9 18.13V22"/>
								</svg>
								<span>GitHub</span>
							</button>
							<button class="quick-link" on:click={(e) => handleQuickLink(e, 'https://etherscan.io/token/0x0a26c80Be4E060e688d7C23aDdB92cBb5D2C9eCA')}>
								<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
									<polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"/>
								</svg>
								<span>NOX Token</span>
							</button>
							<button class="quick-link" on:click={(e) => handleQuickLink(e, 'https://anyone.io')}>
								<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
									<path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>
								</svg>
								<span>Anyone Network</span>
							</button>
						</div>
					</div>

					<div class="security-notice">
						<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
							<circle cx="12" cy="12" r="10"/>
							<path d="M12 16v-4M12 8h.01"/>
						</svg>
						<p>
							<strong>How it works:</strong> Enter a URL above. Your request is encrypted using <strong>Anyone Protocol</strong> (onion routing) and relayed through <strong>NONOS community nodes</strong>. Each hop only knows the previous and next node - true zero-knowledge privacy. Node operators earn NOX rewards for powering your browser.
						</p>
					</div>
				</div>
			</div>
		</div>
	{/if}
</div>

<style>
	.browser-page {
		height: 100%;
		display: flex;
		flex-direction: column;
	}

	/* Native browser view - when pages open in new Tauri windows */
	.native-browser-view {
		flex: 1;
		display: flex;
		flex-direction: column;
		padding: var(--nox-space-xl);
		overflow: auto;
		background: var(--nox-bg-primary);
	}

	.open-tabs-header {
		text-align: center;
		margin-bottom: var(--nox-space-xl);
	}

	.open-tabs-header h2 {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: var(--nox-space-sm);
		font-size: var(--nox-text-xl);
		font-weight: var(--nox-font-semibold);
		color: var(--nox-accent-primary);
		margin-bottom: var(--nox-space-sm);
	}

	.open-tabs-header h2 svg {
		width: 28px;
		height: 28px;
	}

	.privacy-note {
		font-size: var(--nox-text-sm);
		color: var(--nox-text-secondary);
	}

	.loading-indicator {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--nox-space-md);
		padding: var(--nox-space-xl);
	}

	.loading-indicator p {
		color: var(--nox-text-secondary);
		font-size: var(--nox-text-sm);
	}

	.spinner {
		width: 40px;
		height: 40px;
		border: 3px solid var(--nox-border);
		border-top-color: var(--nox-accent-primary);
		border-radius: 50%;
		animation: spin 1s linear infinite;
	}

	@keyframes spin {
		100% { transform: rotate(360deg); }
	}

	.error-message {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--nox-space-md);
		padding: var(--nox-space-xl);
		color: var(--nox-error);
	}

	.error-message svg {
		width: 48px;
		height: 48px;
	}

	.error-message p {
		font-size: var(--nox-text-sm);
		max-width: 400px;
		text-align: center;
	}

	.retry-btn {
		padding: var(--nox-space-sm) var(--nox-space-lg);
		background: var(--nox-accent-gradient);
		color: white;
		border-radius: var(--nox-radius-md);
		font-weight: var(--nox-font-medium);
	}

	.open-tabs-list {
		display: flex;
		flex-direction: column;
		gap: var(--nox-space-md);
		margin-bottom: var(--nox-space-xl);
	}

	.tab-card {
		display: flex;
		align-items: center;
		gap: var(--nox-space-md);
		padding: var(--nox-space-md) var(--nox-space-lg);
		background: var(--nox-bg-secondary);
		border: 1px solid var(--nox-border);
		border-radius: var(--nox-radius-lg);
		transition: all var(--nox-transition-fast);
	}

	.tab-card:hover {
		border-color: var(--nox-accent-primary);
	}

	.tab-icon {
		width: 40px;
		height: 40px;
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--nox-accent-glow);
		border-radius: var(--nox-radius-md);
	}

	.tab-icon svg {
		width: 20px;
		height: 20px;
		color: var(--nox-accent-primary);
	}

	.tab-info {
		flex: 1;
		min-width: 0;
	}

	.tab-url {
		display: block;
		font-size: var(--nox-text-sm);
		color: var(--nox-text-primary);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		margin-bottom: var(--nox-space-2xs);
	}

	.tab-status {
		display: flex;
		align-items: center;
		gap: var(--nox-space-xs);
		font-size: var(--nox-text-xs);
		color: var(--nox-success);
	}

	.tab-status svg {
		width: 12px;
		height: 12px;
	}

	.close-tab-btn {
		width: 32px;
		height: 32px;
		display: flex;
		align-items: center;
		justify-content: center;
		background: transparent;
		border-radius: var(--nox-radius-md);
		color: var(--nox-text-muted);
		transition: all var(--nox-transition-fast);
	}

	.close-tab-btn:hover {
		background: var(--nox-error);
		color: white;
	}

	.close-tab-btn svg {
		width: 16px;
		height: 16px;
	}

	.node-power-info {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: var(--nox-space-md);
		margin-bottom: var(--nox-space-xl);
	}

	@media (max-width: 700px) {
		.node-power-info {
			grid-template-columns: 1fr;
		}
	}

	.info-card {
		display: flex;
		gap: var(--nox-space-md);
		padding: var(--nox-space-lg);
		background: var(--nox-bg-secondary);
		border: 1px solid var(--nox-border);
		border-radius: var(--nox-radius-lg);
	}

	.info-card svg {
		width: 32px;
		height: 32px;
		color: var(--nox-accent-primary);
		flex-shrink: 0;
	}

	.info-card h4 {
		font-size: var(--nox-text-sm);
		font-weight: var(--nox-font-semibold);
		color: var(--nox-text-primary);
		margin-bottom: var(--nox-space-xs);
	}

	.info-card p {
		font-size: var(--nox-text-xs);
		color: var(--nox-text-secondary);
		line-height: 1.5;
	}

	.home-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: var(--nox-space-sm);
		padding: var(--nox-space-md) var(--nox-space-xl);
		background: var(--nox-bg-secondary);
		border: 1px solid var(--nox-border);
		border-radius: var(--nox-radius-lg);
		color: var(--nox-text-secondary);
		font-size: var(--nox-text-sm);
		margin: 0 auto;
		transition: all var(--nox-transition-fast);
	}

	.home-btn:hover {
		border-color: var(--nox-accent-primary);
		color: var(--nox-text-primary);
	}

	.home-btn svg {
		width: 18px;
		height: 18px;
	}

	/* Welcome screen styles */
	.webview-container {
		flex: 1;
		background: var(--nox-bg-primary);
	}

	.webview-placeholder {
		height: 100%;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: var(--nox-space-xl);
		overflow: auto;
	}

	.welcome {
		max-width: 900px;
		text-align: center;
	}

	.hero-logo {
		margin-bottom: var(--nox-space-lg);
	}

	.hero-logo svg {
		width: 80px;
		height: 80px;
		filter: drop-shadow(0 0 30px rgba(102, 255, 255, 0.3));
	}

	.nonos-logo {
		width: 200px;
		height: auto;
		filter: drop-shadow(0 0 30px rgba(102, 255, 255, 0.3));
	}

	.welcome h1 {
		font-size: var(--nox-text-3xl);
		font-weight: var(--nox-font-semibold);
		margin-bottom: var(--nox-space-sm);
		color: var(--nox-text-primary);
	}

	.welcome h1 .brand {
		background: var(--nox-accent-gradient);
		-webkit-background-clip: text;
		-webkit-text-fill-color: transparent;
		background-clip: text;
	}

	.tagline {
		font-size: var(--nox-text-lg);
		color: var(--nox-text-secondary);
		margin-bottom: var(--nox-space-xs);
		font-weight: var(--nox-font-light);
		letter-spacing: 0.02em;
	}

	.powered-by {
		font-size: var(--nox-text-sm);
		color: var(--nox-accent-primary);
		margin-bottom: var(--nox-space-2xl);
		font-weight: var(--nox-font-medium);
	}

	.features {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: var(--nox-space-lg);
		margin-bottom: var(--nox-space-2xl);
	}

	@media (max-width: 700px) {
		.features {
			grid-template-columns: 1fr;
		}
	}

	.feature {
		background: var(--nox-bg-secondary);
		border: 1px solid var(--nox-border);
		border-radius: var(--nox-radius-xl);
		padding: var(--nox-space-lg);
		text-align: left;
		transition: all var(--nox-transition-fast);
	}

	.feature:hover {
		border-color: var(--nox-accent-primary);
		transform: translateY(-2px);
		box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3);
	}

	.feature-icon {
		width: 48px;
		height: 48px;
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--nox-accent-glow);
		border-radius: var(--nox-radius-lg);
		margin-bottom: var(--nox-space-md);
	}

	.feature-icon svg {
		width: 24px;
		height: 24px;
		color: var(--nox-accent-primary);
	}

	.feature h3 {
		font-size: var(--nox-text-base);
		font-weight: var(--nox-font-semibold);
		margin-bottom: var(--nox-space-xs);
		color: var(--nox-text-primary);
	}

	.feature p {
		font-size: var(--nox-text-sm);
		color: var(--nox-text-secondary);
		line-height: 1.5;
	}

	.quick-links {
		text-align: left;
		margin-bottom: var(--nox-space-xl);
	}

	.quick-links h4 {
		font-size: var(--nox-text-xs);
		font-weight: var(--nox-font-medium);
		color: var(--nox-text-muted);
		text-transform: uppercase;
		letter-spacing: 0.08em;
		margin-bottom: var(--nox-space-md);
	}

	.link-grid {
		display: grid;
		grid-template-columns: repeat(4, 1fr);
		gap: var(--nox-space-sm);
	}

	@media (max-width: 600px) {
		.link-grid {
			grid-template-columns: repeat(2, 1fr);
		}
	}

	.quick-link {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--nox-space-sm);
		background: var(--nox-bg-secondary);
		border: 1px solid var(--nox-border);
		border-radius: var(--nox-radius-lg);
		padding: var(--nox-space-md);
		font-size: var(--nox-text-sm);
		color: var(--nox-text-secondary);
		text-decoration: none;
		transition: all var(--nox-transition-fast);
		cursor: pointer;
		font-family: inherit;
	}

	.quick-link svg {
		width: 24px;
		height: 24px;
		color: var(--nox-text-muted);
		transition: color var(--nox-transition-fast);
	}

	.quick-link:hover {
		border-color: var(--nox-accent-primary);
		background: var(--nox-bg-hover);
		color: var(--nox-text-primary);
	}

	.quick-link:hover svg {
		color: var(--nox-accent-primary);
	}

	.security-notice {
		display: flex;
		align-items: flex-start;
		gap: var(--nox-space-md);
		background: var(--nox-bg-tertiary);
		border: 1px solid var(--nox-border-subtle);
		border-radius: var(--nox-radius-lg);
		padding: var(--nox-space-md) var(--nox-space-lg);
	}

	.security-notice svg {
		width: 20px;
		height: 20px;
		color: var(--nox-accent-primary);
		flex-shrink: 0;
		margin-top: 2px;
	}

	.security-notice p {
		font-size: var(--nox-text-sm);
		color: var(--nox-text-secondary);
		text-align: left;
		line-height: 1.5;
	}

	.security-notice strong {
		color: var(--nox-text-primary);
	}
</style>
