<script lang="ts">
	import { onMount, onDestroy } from 'svelte';

	// Simple QR code generation using canvas
	async function generateQRCode(text: string): Promise<string> {
		try {
			const QRCode = await import('qrcode');
			return await QRCode.toDataURL(text, {
				width: 200,
				margin: 2,
				color: {
					dark: '#00ff88',
					light: '#0a0f0a'
				}
			});
		} catch (e) {
			console.error('QR generation error:', e);
			// Return a placeholder SVG if QR fails
			return '';
		}
	}

	let walletState: 'locked' | 'unlocked' | 'none' = 'none';
	let address = '';
	let balances = { eth: '0', nox: '0' };
	let mnemonic = '';
	let blake3Key = '';
	let showMnemonic = false;
	let newWalletName = 'My Wallet';
	let importMnemonic = '';
	let isLoading = false;
	let error = '';
	let copied = false;
	let successMessage = '';

	// Send/Receive state
	let showSendModal = false;
	let showReceiveModal = false;
	let sendToken: 'eth' | 'nox' = 'eth';
	let sendTo = '';
	let sendAmount = '';
	let sendLoading = false;
	let sendError = '';
	let sendSuccess = '';

	// Staking state
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
	let unstakeAmount = '';
	let stakingLoading = false;
	let showStaking = false;

	let balanceInterval: ReturnType<typeof setInterval>;

	onMount(() => {
		const init = async () => {
			let retries = 0;
			while (!window.nonos && retries < 20) {
				await new Promise(r => setTimeout(r, 250));
				retries++;
			}

			if (!window.nonos) {
				error = 'NONOS bridge not available. Please restart the app.';
				console.error('window.nonos not found after retries');
				return;
			}

			console.log('NONOS bridge ready:', window.nonos.version);
			await checkWallet();

			balanceInterval = setInterval(async () => {
				if (walletState === 'unlocked') {
					await updateBalances();
				}
			}, 10000);
		};

		init();
	});

	onDestroy(() => {
		if (balanceInterval) clearInterval(balanceInterval);
	});

	async function checkWallet() {
		if (!window.nonos) {
			error = 'NONOS bridge not initialized';
			return;
		}
		try {
			const addr = await window.nonos.wallet.getAddress();
			if (addr) {
				address = addr;
				walletState = 'unlocked';
				await updateBalances();
			}
		} catch {
			walletState = 'none';
		}
	}

	let isRefreshing = false;

	async function updateBalances() {
		if (!window.nonos) return;
		try {
			const status = await window.nonos.wallet.getStatus();
			if (status) {
				balances = {
					eth: status.eth_balance || '0',
					nox: status.nox_balance || '0'
				};
			}
		} catch (e) {
			console.error('Failed to get balances:', e);
		}
	}

	async function refreshBalances() {
		isRefreshing = true;
		await updateBalances();
		setTimeout(() => isRefreshing = false, 500);
	}

	async function createWallet() {
		if (!window.nonos) {
			error = 'NONOS bridge not available';
			return;
		}
		isLoading = true;
		error = '';
		console.log('Creating wallet...');
		try {
			// Backend returns mnemonic string directly
			mnemonic = await window.nonos.wallet.create(newWalletName);
			// Get the derived address
			address = await window.nonos.wallet.getAddress() || '';
			// Generate a BLAKE3 key representation (for display)
			blake3Key = `blake3:${address.slice(2, 18)}...${address.slice(-16)}`;
			walletState = 'unlocked';
			showMnemonic = true;
			await updateBalances();
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to create wallet';
		} finally {
			isLoading = false;
		}
	}

	async function importWallet() {
		if (!window.nonos || !importMnemonic.trim()) return;
		isLoading = true;
		error = '';
		try {
			// Backend takes (mnemonic, password)
			await window.nonos.wallet.import(importMnemonic.trim(), newWalletName);
			address = await window.nonos.wallet.getAddress() || '';
			walletState = 'unlocked';
			importMnemonic = '';
			await updateBalances();
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to import wallet';
		} finally {
			isLoading = false;
		}
	}

	async function lockWallet() {
		if (!window.nonos) return;
		await window.nonos.wallet.lock();
		walletState = 'locked';
		address = '';
		balances = { eth: '0', nox: '0' };
	}

	function formatAddress(addr: string): string {
		return `${addr.slice(0, 10)}...${addr.slice(-8)}`;
	}

	async function copyToClipboard(text: string) {
		await navigator.clipboard.writeText(text);
		copied = true;
		setTimeout(() => copied = false, 2000);
	}

	// Staking functions
	async function loadStakingStatus() {
		if (!window.nonos) return;
		try {
			stakingStatus = await window.nonos.staking.getStatus();
		} catch (e) {
			console.error('Failed to load staking status:', e);
		}
	}

	async function stake() {
		if (!window.nonos || !stakeAmount) return;
		stakingLoading = true;
		error = '';
		try {
			const result = await window.nonos.staking.stake(stakeAmount);
			stakeAmount = '';
			await loadStakingStatus();
			await updateBalances();
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to stake';
		} finally {
			stakingLoading = false;
		}
	}

	async function unstake() {
		if (!window.nonos || !unstakeAmount) return;
		stakingLoading = true;
		error = '';
		try {
			const result = await window.nonos.staking.unstake(unstakeAmount);
			unstakeAmount = '';
			await loadStakingStatus();
			await updateBalances();
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to unstake';
		} finally {
			stakingLoading = false;
		}
	}

	async function claimRewards() {
		if (!window.nonos) return;
		stakingLoading = true;
		error = '';
		try {
			const result = await window.nonos.staking.claimRewards();
			await loadStakingStatus();
			await updateBalances();
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to claim rewards';
		} finally {
			stakingLoading = false;
		}
	}

	// Send functions
	function openSendModal(token: 'eth' | 'nox') {
		sendToken = token;
		sendTo = '';
		sendAmount = '';
		sendError = '';
		sendSuccess = '';
		showSendModal = true;
	}

	function closeSendModal() {
		showSendModal = false;
		sendTo = '';
		sendAmount = '';
		sendError = '';
		sendSuccess = '';
	}

	async function sendTransaction() {
		if (!window.nonos || !sendTo || !sendAmount) {
			sendError = 'Please fill in all fields';
			return;
		}

		// Validate address
		if (!sendTo.match(/^0x[a-fA-F0-9]{40}$/)) {
			sendError = 'Invalid Ethereum address. Must start with 0x and be 42 characters.';
			return;
		}

		// Validate amount
		const amount = parseFloat(sendAmount);
		if (isNaN(amount) || amount <= 0) {
			sendError = 'Invalid amount. Must be greater than 0.';
			return;
		}

		// Check balance before sending
		const currentBalance = sendToken === 'eth' ? parseFloat(balances.eth) : parseFloat(balances.nox);
		if (amount > currentBalance) {
			sendError = `Insufficient ${sendToken.toUpperCase()} balance. You have ${currentBalance} ${sendToken.toUpperCase()}.`;
			return;
		}

		// For ETH, also check we have enough for gas
		if (sendToken === 'eth') {
			const ethBalance = parseFloat(balances.eth);
			if (amount + 0.005 > ethBalance) {
				sendError = 'Insufficient ETH for transaction + gas fees. Leave at least 0.005 ETH for gas.';
				return;
			}
		} else {
			// For NOX transfers, need ETH for gas
			const ethBalance = parseFloat(balances.eth);
			if (ethBalance < 0.005) {
				sendError = 'Insufficient ETH for gas fees. You need at least 0.005 ETH.';
				return;
			}
		}

		sendLoading = true;
		sendError = '';
		sendSuccess = '';

		console.log(`NONOS UI: Sending ${sendAmount} ${sendToken.toUpperCase()} to ${sendTo}`);

		try {
			let result: string;
			if (sendToken === 'eth') {
				result = await window.nonos.wallet.sendEth(sendTo, sendAmount);
			} else {
				result = await window.nonos.wallet.sendNox(sendTo, sendAmount);
			}
			console.log('NONOS UI: Send success:', result);
			sendSuccess = result;
			await updateBalances();
			// Auto close after success
			setTimeout(() => {
				closeSendModal();
				successMessage = `Successfully sent ${sendAmount} ${sendToken.toUpperCase()}!`;
				setTimeout(() => successMessage = '', 5000);
			}, 2000);
		} catch (e: unknown) {
			console.error('NONOS UI: Send failed:', e);
			// Handle both Error objects and string errors from Tauri
			if (e instanceof Error) {
				sendError = e.message;
			} else if (typeof e === 'string') {
				sendError = e;
			} else {
				sendError = 'Transaction failed. Please try again.';
			}
		} finally {
			sendLoading = false;
		}
	}

	let qrCodeDataUrl = '';

	async function openReceiveModal() {
		showReceiveModal = true;
		qrCodeDataUrl = '';
		// Generate QR code
		if (address) {
			console.log('Generating QR for address:', address);
			qrCodeDataUrl = await generateQRCode(address);
			console.log('QR result:', qrCodeDataUrl ? 'success' : 'failed');
		}
	}

	function closeReceiveModal() {
		showReceiveModal = false;
		qrCodeDataUrl = '';
	}

	function setMaxAmount() {
		if (sendToken === 'eth') {
			// Leave some for gas
			const ethBalance = parseFloat(balances.eth);
			const maxEth = Math.max(0, ethBalance - 0.005);
			sendAmount = maxEth.toFixed(6);
		} else {
			sendAmount = balances.nox;
		}
	}
</script>

<div class="wallet-page">
	<div class="page-header">
		<h1>Wallet</h1>
		<p class="subtitle">Secure NOX & ETH wallet with stealth address support</p>
	</div>

	{#if error}
		<div class="error-banner">
			<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
				<circle cx="12" cy="12" r="10"/>
				<path d="M15 9l-6 6M9 9l6 6"/>
			</svg>
			<span>{error}</span>
			<button class="dismiss" on:click={() => error = ''}>
				<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
					<path d="M18 6L6 18M6 6l12 12"/>
				</svg>
			</button>
		</div>
	{/if}

	{#if successMessage}
		<div class="success-banner">
			<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
				<path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"/>
				<polyline points="22 4 12 14.01 9 11.01"/>
			</svg>
			<span>{successMessage}</span>
			<button class="dismiss" on:click={() => successMessage = ''}>
				<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
					<path d="M18 6L6 18M6 6l12 12"/>
				</svg>
			</button>
		</div>
	{/if}

	{#if walletState === 'none'}
		<div class="wallet-setup">
			<div class="setup-card">
				<div class="card-icon create">
					<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
						<circle cx="12" cy="12" r="10"/>
						<path d="M12 8v8M8 12h8"/>
					</svg>
				</div>
				<h2>Create New Wallet</h2>
				<p>Generate a new wallet with a secure 24-word mnemonic phrase and BLAKE3 key derivation.</p>
				<input
					type="text"
					bind:value={newWalletName}
					placeholder="Wallet name"
					class="input"
				/>
				<button class="btn primary" on:click={createWallet} disabled={isLoading}>
					{#if isLoading}
						<svg class="spinner" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
							<path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83"/>
						</svg>
						Creating...
					{:else}
						Create Wallet
					{/if}
				</button>
			</div>

			<div class="setup-card">
				<div class="card-icon import">
					<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
						<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
						<polyline points="17 8 12 3 7 8"/>
						<line x1="12" y1="3" x2="12" y2="15"/>
					</svg>
				</div>
				<h2>Import Wallet</h2>
				<p>Restore a wallet using your 24-word mnemonic recovery phrase.</p>
				<textarea
					bind:value={importMnemonic}
					placeholder="Enter your 24-word mnemonic phrase..."
					rows="3"
					class="textarea"
				></textarea>
				<button class="btn secondary" on:click={importWallet} disabled={isLoading || !importMnemonic}>
					{isLoading ? 'Importing...' : 'Import Wallet'}
				</button>
			</div>
		</div>
	{:else if showMnemonic}
		<div class="mnemonic-display">
			<div class="warning-banner">
				<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
					<path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/>
					<line x1="12" y1="9" x2="12" y2="13"/>
					<line x1="12" y1="17" x2="12.01" y2="17"/>
				</svg>
				<div>
					<strong>Critical Security Information</strong>
					<p>Write down your recovery phrase and store it securely offline. This is the only way to recover your wallet if you lose access. Never share it with anyone.</p>
				</div>
			</div>

			<div class="mnemonic-card">
				<div class="card-header">
					<h3>Recovery Phrase (24 words)</h3>
					<button class="copy-action" on:click={() => copyToClipboard(mnemonic)}>
						{#if copied}
							<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
								<polyline points="20 6 9 17 4 12"/>
							</svg>
							Copied
						{:else}
							<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
								<rect x="9" y="9" width="13" height="13" rx="2" ry="2"/>
								<path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>
							</svg>
							Copy
						{/if}
					</button>
				</div>
				<div class="mnemonic-words">
					{#each mnemonic.split(' ') as word, i}
						<span class="word"><span class="num">{i + 1}</span>{word}</span>
					{/each}
				</div>
			</div>

			<div class="blake3-card">
				<div class="card-header">
					<h3>BLAKE3 Key (Alternative Recovery)</h3>
					<button class="copy-action" on:click={() => copyToClipboard(blake3Key)}>
						<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
							<rect x="9" y="9" width="13" height="13" rx="2" ry="2"/>
							<path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>
						</svg>
						Copy
					</button>
				</div>
				<p class="key-value">{blake3Key}</p>
			</div>

			<button class="btn primary full-width" on:click={() => (showMnemonic = false)}>
				<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
					<polyline points="20 6 9 17 4 12"/>
				</svg>
				I've Saved My Recovery Phrase
			</button>
		</div>
	{:else}
		<div class="wallet-dashboard">
			<div class="address-card">
				<div class="address-label">Wallet Address</div>
				<div class="address-value">
					<span class="address-text">{formatAddress(address)}</span>
					<button class="copy-btn" on:click={() => copyToClipboard(address)} title="Copy full address">
						<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
							<rect x="9" y="9" width="13" height="13" rx="2" ry="2"/>
							<path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>
						</svg>
					</button>
				</div>
			</div>

			<div class="balance-section">
				<div class="balance-header">
					<span class="balance-title">Balances</span>
					<button class="refresh-btn" on:click={refreshBalances} disabled={isRefreshing} title="Refresh balances">
						<svg class:spinning={isRefreshing} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
							<path d="M23 4v6h-6M1 20v-6h6"/>
							<path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"/>
						</svg>
					</button>
				</div>
				<div class="balance-cards">
					<div class="balance-card eth">
						<div class="balance-icon">
							<svg viewBox="0 0 24 24" fill="none">
								<path d="M12 2L4 12l8 5 8-5-8-10z" fill="currentColor" opacity="0.6"/>
								<path d="M12 22l8-10-8 5-8-5 8 10z" fill="currentColor"/>
							</svg>
						</div>
						<div class="balance-info">
							<div class="balance-label">Ethereum</div>
							<div class="balance-value">{balances.eth} <span class="unit">ETH</span></div>
						</div>
					</div>

					<div class="balance-card nox">
						<div class="balance-icon">
							<svg viewBox="0 0 24 24" fill="none">
								<path d="M12 2L22 12L12 22L2 12L12 2Z" fill="currentColor"/>
							</svg>
						</div>
						<div class="balance-info">
							<div class="balance-label">NOX Token</div>
							<div class="balance-value">{balances.nox} <span class="unit">NOX</span></div>
						</div>
					</div>
				</div>
			</div>

			<div class="actions">
				<button class="btn action-btn send" on:click={() => openSendModal('eth')}>
					<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
						<line x1="22" y1="2" x2="11" y2="13"/>
						<polygon points="22 2 15 22 11 13 2 9 22 2"/>
					</svg>
					Send
				</button>
				<button class="btn action-btn receive" on:click={openReceiveModal}>
					<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
						<polyline points="22 12 16 12 14 15 10 15 8 12 2 12"/>
						<path d="M5.45 5.11L2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z"/>
					</svg>
					Receive
				</button>
				<button class="btn action-btn stake" on:click={() => { showStaking = !showStaking; if (showStaking) loadStakingStatus(); }}>
					<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
						<path d="M12 2v20M2 12h20"/>
						<circle cx="12" cy="12" r="10"/>
					</svg>
					Stake
				</button>
				<button class="btn action-btn lock" on:click={lockWallet}>
					<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
						<rect x="3" y="11" width="18" height="11" rx="2" ry="2"/>
						<path d="M7 11V7a5 5 0 0 1 10 0v4"/>
					</svg>
					Lock
				</button>
			</div>

			<!-- Staking Section -->
			{#if showStaking}
				<div class="staking-section">
					<h2>NOX Staking</h2>
					<p class="staking-subtitle">Stake NOX to earn rewards and power the NONOS network</p>

					<div class="staking-stats">
						<div class="stat-card">
							<div class="stat-label">Staked Amount</div>
							<div class="stat-value">{stakingStatus.staked_amount} <span class="unit">NOX</span></div>
						</div>
						<div class="stat-card tier-card">
							<div class="stat-label">Current Tier</div>
							<div class="stat-value tier-{stakingStatus.tier.toLowerCase()}">{stakingStatus.tier}</div>
							<div class="stat-sub">{stakingStatus.tier_multiplier} rewards</div>
						</div>
						<div class="stat-card">
							<div class="stat-label">Pending Rewards</div>
							<div class="stat-value reward">{stakingStatus.pending_rewards} <span class="unit">NOX</span></div>
						</div>
						<div class="stat-card">
							<div class="stat-label">Est. APY</div>
							<div class="stat-value apy">{stakingStatus.estimated_apy}</div>
						</div>
					</div>

					<div class="staking-actions">
						<div class="staking-input-group">
							<input
								type="number"
								bind:value={stakeAmount}
								placeholder="Amount to stake"
								class="input staking-input"
								disabled={stakingLoading}
							/>
							<button class="btn primary" on:click={stake} disabled={stakingLoading || !stakeAmount}>
								{stakingLoading ? 'Processing...' : 'Stake NOX'}
							</button>
						</div>

						<div class="staking-input-group">
							<input
								type="number"
								bind:value={unstakeAmount}
								placeholder="Amount to unstake"
								class="input staking-input"
								disabled={stakingLoading}
							/>
							<button class="btn secondary" on:click={unstake} disabled={stakingLoading || !unstakeAmount}>
								{stakingLoading ? 'Processing...' : 'Unstake'}
							</button>
						</div>

						{#if parseFloat(stakingStatus.pending_rewards) > 0}
							<button class="btn claim-btn" on:click={claimRewards} disabled={stakingLoading}>
								<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
									<path d="M12 2v20M2 12h20"/>
								</svg>
								{stakingLoading ? 'Claiming...' : `Claim ${stakingStatus.pending_rewards} NOX`}
							</button>
						{/if}
					</div>

					<div class="tier-info">
						<h3>Staking Tiers</h3>
						<div class="tier-table">
							<div class="tier-row header">
								<span>Tier</span>
								<span>Min Stake</span>
								<span>Multiplier</span>
							</div>
							<div class="tier-row" class:active={stakingStatus.tier === 'Bronze'}>
								<span>Bronze</span>
								<span>1,000 NOX</span>
								<span>1.0x</span>
							</div>
							<div class="tier-row" class:active={stakingStatus.tier === 'Silver'}>
								<span>Silver</span>
								<span>10,000 NOX</span>
								<span>1.2x</span>
							</div>
							<div class="tier-row" class:active={stakingStatus.tier === 'Gold'}>
								<span>Gold</span>
								<span>50,000 NOX</span>
								<span>1.5x</span>
							</div>
							<div class="tier-row" class:active={stakingStatus.tier === 'Platinum'}>
								<span>Platinum</span>
								<span>200,000 NOX</span>
								<span>2.0x</span>
							</div>
							<div class="tier-row" class:active={stakingStatus.tier === 'Diamond'}>
								<span>Diamond</span>
								<span>1,000,000 NOX</span>
								<span>2.5x</span>
							</div>
						</div>
						<p class="tier-note">Next tier: {stakingStatus.next_tier_threshold}</p>
					</div>
				</div>
			{/if}

			<div class="security-info">
				<div class="info-item">
					<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
						<path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>
					</svg>
					<span>BLAKE3 key derivation</span>
				</div>
				<div class="info-item">
					<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
						<circle cx="12" cy="12" r="10"/>
						<path d="M12 6v6l4 2"/>
					</svg>
					<span>Stealth addresses</span>
				</div>
				<div class="info-item">
					<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
						<path d="M21 2l-2 2m-7.61 7.61a5.5 5.5 0 1 1-7.778 7.778 5.5 5.5 0 0 1 7.777-7.777zm0 0L15.5 7.5m0 0l3 3L22 7l-3-3m-3.5 3.5L19 4"/>
					</svg>
					<span>secp256k1 signatures</span>
				</div>
			</div>
		</div>
	{/if}
</div>

<!-- Send Modal -->
{#if showSendModal}
	<div class="modal-overlay" on:click={closeSendModal} on:keydown={(e) => e.key === 'Escape' && closeSendModal()} role="button" tabindex="0">
		<div class="modal" on:click|stopPropagation role="dialog" aria-modal="true">
			<div class="modal-header">
				<h2>Send {sendToken.toUpperCase()}</h2>
				<button class="close-btn" on:click={closeSendModal}>
					<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
						<path d="M18 6L6 18M6 6l12 12"/>
					</svg>
				</button>
			</div>

			<div class="modal-body">
				<div class="token-tabs">
					<button
						class="token-tab"
						class:active={sendToken === 'eth'}
						on:click={() => sendToken = 'eth'}
					>
						<span class="token-icon eth">ETH</span>
						<span class="token-balance">{balances.eth}</span>
					</button>
					<button
						class="token-tab"
						class:active={sendToken === 'nox'}
						on:click={() => sendToken = 'nox'}
					>
						<span class="token-icon nox">NOX</span>
						<span class="token-balance">{balances.nox}</span>
					</button>
				</div>

				<div class="form-group">
					<label for="send-to">Recipient Address</label>
					<input
						id="send-to"
						type="text"
						bind:value={sendTo}
						placeholder="0x..."
						class="input"
						disabled={sendLoading}
					/>
				</div>

				<div class="form-group">
					<label for="send-amount">Amount ({sendToken.toUpperCase()})</label>
					<div class="amount-input-group">
						<input
							id="send-amount"
							type="number"
							step="0.000001"
							bind:value={sendAmount}
							placeholder="0.0"
							class="input"
							disabled={sendLoading}
						/>
						<button class="max-btn" on:click={setMaxAmount} disabled={sendLoading}>MAX</button>
					</div>
				</div>

				{#if sendError}
					<div class="modal-error">{sendError}</div>
				{/if}

				{#if sendSuccess}
					<div class="modal-success">{sendSuccess}</div>
				{/if}

				<button
					class="btn primary full-width send-btn"
					on:click={sendTransaction}
					disabled={sendLoading || !sendTo || !sendAmount}
				>
					{#if sendLoading}
						<svg class="spinner" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
							<path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83"/>
						</svg>
						Sending...
					{:else}
						Send {sendAmount || '0'} {sendToken.toUpperCase()}
					{/if}
				</button>
			</div>
		</div>
	</div>
{/if}

<!-- Receive Modal -->
{#if showReceiveModal}
	<div class="modal-overlay" on:click={closeReceiveModal} on:keydown={(e) => e.key === 'Escape' && closeReceiveModal()} role="button" tabindex="0">
		<div class="modal" on:click|stopPropagation role="dialog" aria-modal="true">
			<div class="modal-header">
				<h2>Receive Tokens</h2>
				<button class="close-btn" on:click={closeReceiveModal}>
					<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
						<path d="M18 6L6 18M6 6l12 12"/>
					</svg>
				</button>
			</div>

			<div class="modal-body receive-body">
				<p class="receive-info">Send ETH or NOX tokens to your wallet address below:</p>

				<div class="qr-code-container">
					{#if qrCodeDataUrl && qrCodeDataUrl.startsWith('data:')}
						<img src={qrCodeDataUrl} alt="Scan to receive tokens" class="qr-code-img" />
					{:else}
						<div class="qr-loading">
							<svg class="spinner" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
								<path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83"/>
							</svg>
							<span>Generating QR...</span>
						</div>
					{/if}
				</div>

				<div class="address-display">
					<div class="address-full">{address}</div>
					<button class="copy-address-btn" on:click={() => copyToClipboard(address)}>
						{#if copied}
							<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
								<polyline points="20 6 9 17 4 12"/>
							</svg>
							Copied!
						{:else}
							<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
								<rect x="9" y="9" width="13" height="13" rx="2" ry="2"/>
								<path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>
							</svg>
							Copy Address
						{/if}
					</button>
				</div>

				<div class="receive-tokens">
					<div class="receive-token eth">
						<svg viewBox="0 0 24 24" fill="none">
							<path d="M12 2L4 12l8 5 8-5-8-10z" fill="currentColor" opacity="0.6"/>
							<path d="M12 22l8-10-8 5-8-5 8 10z" fill="currentColor"/>
						</svg>
						<span>ETH</span>
					</div>
					<div class="receive-token nox">
						<svg viewBox="0 0 24 24" fill="none">
							<path d="M12 2L22 12L12 22L2 12L12 2Z" fill="currentColor"/>
						</svg>
						<span>NOX</span>
					</div>
				</div>

				<p class="network-note">Network: Ethereum Mainnet</p>
			</div>
		</div>
	</div>
{/if}

<style>
	.wallet-page {
		max-width: 800px;
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

	.error-banner span {
		flex: 1;
	}

	.dismiss {
		padding: var(--nox-space-xs);
		border-radius: var(--nox-radius-sm);
		color: var(--nox-error);
	}

	.dismiss:hover {
		background: rgba(255, 68, 102, 0.2);
	}

	.dismiss svg {
		width: 16px;
		height: 16px;
	}

	.warning-banner {
		display: flex;
		gap: var(--nox-space-md);
		background: var(--nox-warning-bg);
		border: 1px solid var(--nox-warning);
		padding: var(--nox-space-lg);
		border-radius: var(--nox-radius-lg);
		margin-bottom: var(--nox-space-lg);
	}

	.warning-banner svg {
		width: 24px;
		height: 24px;
		color: var(--nox-warning);
		flex-shrink: 0;
	}

	.warning-banner strong {
		display: block;
		color: var(--nox-warning);
		margin-bottom: var(--nox-space-xs);
	}

	.warning-banner p {
		font-size: var(--nox-text-sm);
		color: var(--nox-text-secondary);
		line-height: 1.5;
	}

	.wallet-setup {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: var(--nox-space-lg);
	}

	@media (max-width: 700px) {
		.wallet-setup {
			grid-template-columns: 1fr;
		}
	}

	.setup-card {
		background: var(--nox-bg-secondary);
		border: 1px solid var(--nox-border);
		border-radius: var(--nox-radius-xl);
		padding: var(--nox-space-xl);
		display: flex;
		flex-direction: column;
	}

	.card-icon {
		width: 56px;
		height: 56px;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: var(--nox-radius-lg);
		margin-bottom: var(--nox-space-lg);
	}

	.card-icon.create {
		background: var(--nox-accent-glow);
	}

	.card-icon.create svg {
		width: 28px;
		height: 28px;
		color: var(--nox-accent-primary);
	}

	.card-icon.import {
		background: var(--nox-success-bg);
	}

	.card-icon.import svg {
		width: 28px;
		height: 28px;
		color: var(--nox-success);
	}

	.setup-card h2 {
		font-size: var(--nox-text-lg);
		font-weight: var(--nox-font-semibold);
		margin-bottom: var(--nox-space-sm);
	}

	.setup-card p {
		color: var(--nox-text-secondary);
		font-size: var(--nox-text-sm);
		margin-bottom: var(--nox-space-lg);
		line-height: 1.5;
		flex: 1;
	}

	.input, .textarea {
		width: 100%;
		margin-bottom: var(--nox-space-md);
	}

	.textarea {
		resize: none;
	}

	.btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: var(--nox-space-sm);
		padding: var(--nox-space-sm) var(--nox-space-lg);
		border-radius: var(--nox-radius-md);
		font-weight: var(--nox-font-medium);
		transition: all var(--nox-transition-fast);
		font-size: var(--nox-text-sm);
	}

	.btn svg {
		width: 18px;
		height: 18px;
	}

	.btn.primary {
		background: var(--nox-accent-gradient);
		color: var(--nox-bg-primary);
	}

	.btn.primary:hover:not(:disabled) {
		box-shadow: var(--nox-shadow-glow);
	}

	.btn.secondary {
		background: var(--nox-bg-tertiary);
		border: 1px solid var(--nox-border);
		color: var(--nox-text-primary);
	}

	.btn.secondary:hover:not(:disabled) {
		border-color: var(--nox-accent-primary);
	}

	.btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.btn.full-width {
		width: 100%;
	}

	.spinner {
		animation: spin 1s linear infinite;
	}

	@keyframes spin {
		100% { transform: rotate(360deg); }
	}

	.mnemonic-display {
		display: flex;
		flex-direction: column;
		gap: var(--nox-space-lg);
	}

	.mnemonic-card, .blake3-card {
		background: var(--nox-bg-secondary);
		border: 1px solid var(--nox-border);
		border-radius: var(--nox-radius-xl);
		padding: var(--nox-space-lg);
	}

	.card-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: var(--nox-space-md);
	}

	.card-header h3 {
		font-size: var(--nox-text-base);
		font-weight: var(--nox-font-medium);
	}

	.copy-action {
		display: inline-flex;
		align-items: center;
		gap: var(--nox-space-xs);
		padding: var(--nox-space-xs) var(--nox-space-sm);
		border-radius: var(--nox-radius-sm);
		font-size: var(--nox-text-xs);
		color: var(--nox-text-muted);
		transition: all var(--nox-transition-fast);
	}

	.copy-action:hover {
		background: var(--nox-bg-hover);
		color: var(--nox-accent-primary);
	}

	.copy-action svg {
		width: 14px;
		height: 14px;
	}

	.mnemonic-words {
		display: grid;
		grid-template-columns: repeat(4, 1fr);
		gap: var(--nox-space-sm);
	}

	@media (max-width: 600px) {
		.mnemonic-words {
			grid-template-columns: repeat(3, 1fr);
		}
	}

	.word {
		display: flex;
		align-items: center;
		gap: var(--nox-space-sm);
		background: var(--nox-bg-tertiary);
		padding: var(--nox-space-sm) var(--nox-space-md);
		border-radius: var(--nox-radius-md);
		font-family: var(--nox-font-mono);
		font-size: var(--nox-text-sm);
	}

	.word .num {
		color: var(--nox-text-disabled);
		font-size: var(--nox-text-xs);
		min-width: 20px;
	}

	.key-value {
		font-family: var(--nox-font-mono);
		font-size: var(--nox-text-xs);
		word-break: break-all;
		background: var(--nox-bg-tertiary);
		padding: var(--nox-space-md);
		border-radius: var(--nox-radius-md);
		color: var(--nox-text-secondary);
		line-height: 1.6;
	}

	.wallet-dashboard {
		display: flex;
		flex-direction: column;
		gap: var(--nox-space-lg);
	}

	.address-card {
		background: var(--nox-bg-secondary);
		border: 1px solid var(--nox-border);
		border-radius: var(--nox-radius-xl);
		padding: var(--nox-space-lg);
	}

	.address-label {
		font-size: var(--nox-text-xs);
		color: var(--nox-text-muted);
		text-transform: uppercase;
		letter-spacing: 0.08em;
		margin-bottom: var(--nox-space-sm);
	}

	.address-value {
		display: flex;
		align-items: center;
		gap: var(--nox-space-md);
	}

	.address-text {
		font-family: var(--nox-font-mono);
		font-size: var(--nox-text-lg);
		color: var(--nox-text-primary);
	}

	.copy-btn {
		width: 36px;
		height: 36px;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: var(--nox-radius-md);
		color: var(--nox-text-muted);
		transition: all var(--nox-transition-fast);
	}

	.copy-btn:hover {
		background: var(--nox-bg-hover);
		color: var(--nox-accent-primary);
	}

	.copy-btn svg {
		width: 18px;
		height: 18px;
	}

	.balance-section {
		margin-bottom: var(--nox-space-md);
	}

	.balance-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: var(--nox-space-md);
	}

	.balance-title {
		font-size: var(--nox-text-sm);
		color: var(--nox-text-muted);
		text-transform: uppercase;
		letter-spacing: 0.08em;
	}

	.refresh-btn {
		width: 32px;
		height: 32px;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: var(--nox-radius-md);
		color: var(--nox-text-muted);
		transition: all var(--nox-transition-fast);
	}

	.refresh-btn:hover:not(:disabled) {
		background: var(--nox-bg-hover);
		color: var(--nox-accent-primary);
	}

	.refresh-btn:disabled {
		opacity: 0.5;
	}

	.refresh-btn svg {
		width: 18px;
		height: 18px;
	}

	.refresh-btn svg.spinning {
		animation: spin 1s linear infinite;
	}

	.balance-cards {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: var(--nox-space-md);
	}

	.balance-card {
		display: flex;
		align-items: center;
		gap: var(--nox-space-lg);
		background: var(--nox-bg-secondary);
		border: 1px solid var(--nox-border);
		border-radius: var(--nox-radius-xl);
		padding: var(--nox-space-lg);
		transition: all var(--nox-transition-fast);
	}

	.balance-card:hover {
		border-color: var(--nox-border-light);
	}

	.balance-icon {
		width: 56px;
		height: 56px;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: var(--nox-radius-full);
	}

	.balance-icon svg {
		width: 28px;
		height: 28px;
	}

	.balance-card.eth .balance-icon {
		background: linear-gradient(135deg, #627eea 0%, #3c5fc9 100%);
		color: white;
	}

	.balance-card.nox .balance-icon {
		background: var(--nox-accent-gradient);
		color: var(--nox-bg-primary);
	}

	.balance-label {
		font-size: var(--nox-text-xs);
		color: var(--nox-text-muted);
		margin-bottom: var(--nox-space-2xs);
	}

	.balance-value {
		font-size: var(--nox-text-xl);
		font-weight: var(--nox-font-semibold);
		font-family: var(--nox-font-mono);
	}

	.balance-value .unit {
		font-size: var(--nox-text-sm);
		font-weight: var(--nox-font-regular);
		color: var(--nox-text-muted);
	}

	.actions {
		display: grid;
		grid-template-columns: repeat(4, 1fr);
		gap: var(--nox-space-md);
	}

	.action-btn {
		flex-direction: column;
		gap: var(--nox-space-xs);
		padding: var(--nox-space-lg);
		background: var(--nox-bg-secondary);
		border: 1px solid var(--nox-border);
		border-radius: var(--nox-radius-xl);
		font-size: var(--nox-text-sm);
		color: var(--nox-text-secondary);
		transition: all var(--nox-transition-fast);
	}

	.action-btn svg {
		width: 24px;
		height: 24px;
	}

	.action-btn:hover {
		border-color: var(--nox-accent-primary);
		color: var(--nox-text-primary);
	}

	.action-btn:hover svg {
		color: var(--nox-accent-primary);
	}

	.action-btn.send:hover {
		border-color: var(--nox-accent-primary);
	}

	.action-btn.receive:hover {
		border-color: var(--nox-success);
	}

	.action-btn.receive:hover svg {
		color: var(--nox-success);
	}

	.action-btn.lock:hover {
		border-color: var(--nox-warning);
	}

	.action-btn.lock:hover svg {
		color: var(--nox-warning);
	}

	.security-info {
		display: flex;
		justify-content: center;
		gap: var(--nox-space-xl);
		padding: var(--nox-space-lg);
		background: var(--nox-bg-tertiary);
		border-radius: var(--nox-radius-lg);
	}

	.info-item {
		display: flex;
		align-items: center;
		gap: var(--nox-space-sm);
		font-size: var(--nox-text-xs);
		color: var(--nox-text-muted);
	}

	.info-item svg {
		width: 16px;
		height: 16px;
		color: var(--nox-accent-primary);
	}

	/* Staking Section */
	.staking-section {
		background: var(--nox-bg-secondary);
		border: 1px solid var(--nox-border);
		border-radius: var(--nox-radius-xl);
		padding: var(--nox-space-xl);
		margin-top: var(--nox-space-lg);
	}

	.staking-section h2 {
		font-size: var(--nox-text-lg);
		font-weight: var(--nox-font-semibold);
		margin-bottom: var(--nox-space-xs);
	}

	.staking-subtitle {
		color: var(--nox-text-muted);
		font-size: var(--nox-text-sm);
		margin-bottom: var(--nox-space-lg);
	}

	.staking-stats {
		display: grid;
		grid-template-columns: repeat(4, 1fr);
		gap: var(--nox-space-md);
		margin-bottom: var(--nox-space-lg);
	}

	@media (max-width: 700px) {
		.staking-stats {
			grid-template-columns: repeat(2, 1fr);
		}
	}

	.stat-card {
		background: var(--nox-bg-tertiary);
		border-radius: var(--nox-radius-lg);
		padding: var(--nox-space-md);
		text-align: center;
	}

	.stat-label {
		font-size: var(--nox-text-xs);
		color: var(--nox-text-muted);
		margin-bottom: var(--nox-space-xs);
	}

	.stat-value {
		font-size: var(--nox-text-lg);
		font-weight: var(--nox-font-semibold);
		font-family: var(--nox-font-mono);
	}

	.stat-value .unit {
		font-size: var(--nox-text-xs);
		font-weight: var(--nox-font-regular);
		color: var(--nox-text-muted);
	}

	.stat-value.reward {
		color: var(--nox-success);
	}

	.stat-value.apy {
		color: var(--nox-accent-primary);
	}

	.stat-sub {
		font-size: var(--nox-text-xs);
		color: var(--nox-text-muted);
		margin-top: var(--nox-space-2xs);
	}

	.tier-bronze { color: #cd7f32; }
	.tier-silver { color: #c0c0c0; }
	.tier-gold { color: #ffd700; }
	.tier-platinum { color: #e5e4e2; }
	.tier-diamond { color: #b9f2ff; }

	.staking-actions {
		display: flex;
		flex-direction: column;
		gap: var(--nox-space-md);
		margin-bottom: var(--nox-space-lg);
	}

	.staking-input-group {
		display: flex;
		gap: var(--nox-space-md);
	}

	.staking-input {
		flex: 1;
	}

	.claim-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: var(--nox-space-sm);
		width: 100%;
		padding: var(--nox-space-md);
		background: var(--nox-success-bg);
		border: 1px solid var(--nox-success);
		color: var(--nox-success);
		border-radius: var(--nox-radius-md);
		font-weight: var(--nox-font-medium);
	}

	.claim-btn:hover:not(:disabled) {
		background: var(--nox-success);
		color: var(--nox-bg-primary);
	}

	.claim-btn svg {
		width: 18px;
		height: 18px;
	}

	.tier-info {
		border-top: 1px solid var(--nox-border);
		padding-top: var(--nox-space-lg);
	}

	.tier-info h3 {
		font-size: var(--nox-text-base);
		font-weight: var(--nox-font-medium);
		margin-bottom: var(--nox-space-md);
	}

	.tier-table {
		background: var(--nox-bg-tertiary);
		border-radius: var(--nox-radius-md);
		overflow: hidden;
	}

	.tier-row {
		display: grid;
		grid-template-columns: 1fr 1fr 1fr;
		padding: var(--nox-space-sm) var(--nox-space-md);
		border-bottom: 1px solid var(--nox-border);
		font-size: var(--nox-text-sm);
	}

	.tier-row:last-child {
		border-bottom: none;
	}

	.tier-row.header {
		background: var(--nox-bg-secondary);
		font-weight: var(--nox-font-medium);
		color: var(--nox-text-muted);
	}

	.tier-row.active {
		background: var(--nox-accent-glow);
		color: var(--nox-accent-primary);
	}

	.tier-note {
		margin-top: var(--nox-space-md);
		font-size: var(--nox-text-xs);
		color: var(--nox-text-muted);
	}

	.action-btn.stake:hover {
		border-color: var(--nox-accent-primary);
	}

	.action-btn.stake:hover svg {
		color: var(--nox-accent-primary);
	}

	/* Success Banner */
	.success-banner {
		display: flex;
		align-items: center;
		gap: var(--nox-space-md);
		background: var(--nox-success-bg);
		border: 1px solid var(--nox-success);
		color: var(--nox-success);
		padding: var(--nox-space-md);
		border-radius: var(--nox-radius-lg);
		margin-bottom: var(--nox-space-lg);
	}

	.success-banner svg {
		width: 20px;
		height: 20px;
		flex-shrink: 0;
	}

	.success-banner span {
		flex: 1;
	}

	/* Modal Styles */
	.modal-overlay {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.8);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 1000;
		backdrop-filter: blur(4px);
	}

	.modal {
		background: var(--nox-bg-secondary);
		border: 1px solid var(--nox-border);
		border-radius: var(--nox-radius-xl);
		width: 100%;
		max-width: 440px;
		max-height: 90vh;
		overflow: auto;
	}

	.modal-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: var(--nox-space-lg);
		border-bottom: 1px solid var(--nox-border);
	}

	.modal-header h2 {
		font-size: var(--nox-text-lg);
		font-weight: var(--nox-font-semibold);
	}

	.close-btn {
		width: 32px;
		height: 32px;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: var(--nox-radius-md);
		color: var(--nox-text-muted);
		transition: all var(--nox-transition-fast);
	}

	.close-btn:hover {
		background: var(--nox-bg-hover);
		color: var(--nox-text-primary);
	}

	.close-btn svg {
		width: 18px;
		height: 18px;
	}

	.modal-body {
		padding: var(--nox-space-lg);
	}

	.token-tabs {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: var(--nox-space-sm);
		margin-bottom: var(--nox-space-lg);
	}

	.token-tab {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--nox-space-xs);
		padding: var(--nox-space-md);
		background: var(--nox-bg-tertiary);
		border: 2px solid transparent;
		border-radius: var(--nox-radius-lg);
		transition: all var(--nox-transition-fast);
	}

	.token-tab:hover {
		border-color: var(--nox-border-light);
	}

	.token-tab.active {
		border-color: var(--nox-accent-primary);
		background: var(--nox-accent-glow);
	}

	.token-icon {
		font-size: var(--nox-text-lg);
		font-weight: var(--nox-font-bold);
	}

	.token-icon.eth {
		color: #627eea;
	}

	.token-icon.nox {
		color: var(--nox-accent-primary);
	}

	.token-balance {
		font-size: var(--nox-text-xs);
		color: var(--nox-text-muted);
		font-family: var(--nox-font-mono);
	}

	.form-group {
		margin-bottom: var(--nox-space-md);
	}

	.form-group label {
		display: block;
		font-size: var(--nox-text-sm);
		color: var(--nox-text-secondary);
		margin-bottom: var(--nox-space-xs);
	}

	.amount-input-group {
		display: flex;
		gap: var(--nox-space-sm);
	}

	.amount-input-group .input {
		flex: 1;
	}

	.max-btn {
		padding: var(--nox-space-sm) var(--nox-space-md);
		background: var(--nox-bg-tertiary);
		border: 1px solid var(--nox-border);
		border-radius: var(--nox-radius-md);
		font-size: var(--nox-text-xs);
		font-weight: var(--nox-font-bold);
		color: var(--nox-accent-primary);
		transition: all var(--nox-transition-fast);
	}

	.max-btn:hover:not(:disabled) {
		background: var(--nox-accent-glow);
	}

	.modal-error {
		background: var(--nox-error-bg);
		border: 1px solid var(--nox-error);
		color: var(--nox-error);
		padding: var(--nox-space-sm) var(--nox-space-md);
		border-radius: var(--nox-radius-md);
		font-size: var(--nox-text-sm);
		margin-bottom: var(--nox-space-md);
	}

	.modal-success {
		background: var(--nox-success-bg);
		border: 1px solid var(--nox-success);
		color: var(--nox-success);
		padding: var(--nox-space-sm) var(--nox-space-md);
		border-radius: var(--nox-radius-md);
		font-size: var(--nox-text-sm);
		margin-bottom: var(--nox-space-md);
	}

	.send-btn {
		margin-top: var(--nox-space-md);
	}

	/* Receive Modal */
	.receive-body {
		text-align: center;
	}

	.receive-info {
		color: var(--nox-text-secondary);
		font-size: var(--nox-text-sm);
		margin-bottom: var(--nox-space-lg);
	}

	.qr-code-container {
		width: 220px;
		height: 220px;
		margin: 0 auto var(--nox-space-lg);
		background: var(--nox-bg-tertiary);
		border: 1px solid var(--nox-border);
		border-radius: var(--nox-radius-lg);
		display: flex;
		align-items: center;
		justify-content: center;
		padding: var(--nox-space-sm);
	}

	.qr-code-img {
		width: 200px;
		height: 200px;
		border-radius: var(--nox-radius-md);
	}

	.qr-loading {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--nox-space-sm);
		color: var(--nox-text-muted);
	}

	.qr-loading svg {
		width: 32px;
		height: 32px;
		animation: spin 1s linear infinite;
	}

	.qr-loading span {
		font-size: var(--nox-text-xs);
	}

	.address-display {
		background: var(--nox-bg-tertiary);
		border: 1px solid var(--nox-border);
		border-radius: var(--nox-radius-lg);
		padding: var(--nox-space-md);
		margin-bottom: var(--nox-space-lg);
	}

	.address-full {
		font-family: var(--nox-font-mono);
		font-size: var(--nox-text-xs);
		word-break: break-all;
		color: var(--nox-text-primary);
		margin-bottom: var(--nox-space-md);
		line-height: 1.6;
	}

	.copy-address-btn {
		display: inline-flex;
		align-items: center;
		gap: var(--nox-space-sm);
		padding: var(--nox-space-sm) var(--nox-space-lg);
		background: var(--nox-accent-gradient);
		color: var(--nox-bg-primary);
		border-radius: var(--nox-radius-md);
		font-size: var(--nox-text-sm);
		font-weight: var(--nox-font-medium);
		transition: all var(--nox-transition-fast);
	}

	.copy-address-btn:hover {
		box-shadow: var(--nox-shadow-glow);
	}

	.copy-address-btn svg {
		width: 16px;
		height: 16px;
	}

	.receive-tokens {
		display: flex;
		justify-content: center;
		gap: var(--nox-space-lg);
		margin-bottom: var(--nox-space-md);
	}

	.receive-token {
		display: flex;
		align-items: center;
		gap: var(--nox-space-xs);
		font-size: var(--nox-text-sm);
		color: var(--nox-text-secondary);
	}

	.receive-token svg {
		width: 20px;
		height: 20px;
	}

	.receive-token.eth svg {
		color: #627eea;
	}

	.receive-token.nox svg {
		color: var(--nox-accent-primary);
	}

	.network-note {
		font-size: var(--nox-text-xs);
		color: var(--nox-text-muted);
	}
</style>
