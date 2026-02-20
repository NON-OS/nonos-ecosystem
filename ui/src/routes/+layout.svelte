<script lang="ts">
	import '../app.css';
	import { onMount, onDestroy } from 'svelte';
	import Sidebar from '$lib/components/Sidebar.svelte';
	import TopBar from '$lib/components/TopBar.svelte';

	let networkStatus = { connected: false, bootstrap_progress: 0, circuits: 0 };
	let walletAddress = '';
	let statusInterval: ReturnType<typeof setInterval>;

	onMount(() => {
		const init = async () => {
			let retries = 0;
			while (!window.nonos && retries < 20) {
				await new Promise(r => setTimeout(r, 250));
				retries++;
			}

			if (window.nonos) {
				console.log('NONOS Ecosystem bridge initialized - version:', window.nonos.version);
			} else {
				console.error('NONOS bridge not available');
			}

			const updateStatus = async () => {
				try {
					if (window.nonos) {
						networkStatus = await window.nonos.network.getStatus();
						try {
							walletAddress = await window.nonos.wallet.getAddress() || '';
						} catch {}
					}
				} catch (e) {
					console.error('Failed to get status:', e);
				}
			};

			updateStatus();
			statusInterval = setInterval(updateStatus, 5000);

			if (window.nonos?.onNetworkStatus) {
				window.nonos.onNetworkStatus((status: typeof networkStatus) => {
					networkStatus = status;
				});
			}
		};

		init();
	});

	onDestroy(() => {
		if (statusInterval) clearInterval(statusInterval);
	});
</script>

<div class="app-layout">
	<Sidebar {networkStatus} />
	<div class="main-content">
		<TopBar {networkStatus} {walletAddress} />
		<main>
			<slot />
		</main>
	</div>
</div>

<style>
	.app-layout {
		display: flex;
		height: 100vh;
		overflow: hidden;
	}

	.main-content {
		flex: 1;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	main {
		flex: 1;
		overflow: auto;
		padding: var(--nox-space-lg);
	}
</style>
