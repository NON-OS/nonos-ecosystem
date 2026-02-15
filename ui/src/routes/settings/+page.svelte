<script lang="ts">
	let securityLevel = 'safer';
	let autoConnect = true;
	let circuitRotation = '10';
	let blockTrackers = true;
	let blockAds = true;
	let darkMode = true;

	const securityLevels = [
		{
			value: 'standard',
			label: 'Standard',
			desc: 'Most websites work. Some fingerprinting possible.'
		},
		{
			value: 'safer',
			label: 'Safer',
			desc: 'JavaScript restricted on HTTP. Some sites may break.'
		},
		{
			value: 'safest',
			label: 'Safest',
			desc: 'JavaScript disabled. Maximum privacy, many sites break.'
		}
	];
</script>

<div class="settings-page">
	<h1>Settings</h1>

	<section class="settings-section">
		<h2>Security Level</h2>
		<p class="section-desc">
			Higher security levels provide better protection but may break some websites.
		</p>
		<div class="security-options">
			{#each securityLevels as level}
				<label class="security-option" class:selected={securityLevel === level.value}>
					<input
						type="radio"
						bind:group={securityLevel}
						value={level.value}
					/>
					<div class="option-content">
						<span class="option-label">{level.label}</span>
						<span class="option-desc">{level.desc}</span>
					</div>
				</label>
			{/each}
		</div>
	</section>

	<section class="settings-section">
		<h2>Network</h2>
		<div class="setting-row">
			<div class="setting-info">
				<span class="setting-label">Auto-connect on startup</span>
				<span class="setting-desc">Automatically connect to Anyone Network when app starts</span>
			</div>
			<label class="toggle">
				<input type="checkbox" bind:checked={autoConnect} />
				<span class="slider"></span>
			</label>
		</div>

		<div class="setting-row">
			<div class="setting-info">
				<span class="setting-label">Circuit rotation interval</span>
				<span class="setting-desc">How often to request a new circuit (in minutes)</span>
			</div>
			<select class="select" bind:value={circuitRotation}>
				<option value="5">5 minutes</option>
				<option value="10">10 minutes</option>
				<option value="15">15 minutes</option>
				<option value="30">30 minutes</option>
			</select>
		</div>
	</section>

	<section class="settings-section">
		<h2>Privacy</h2>
		<div class="setting-row">
			<div class="setting-info">
				<span class="setting-label">Block trackers</span>
				<span class="setting-desc">Prevent websites from tracking your activity</span>
			</div>
			<label class="toggle">
				<input type="checkbox" bind:checked={blockTrackers} />
				<span class="slider"></span>
			</label>
		</div>

		<div class="setting-row">
			<div class="setting-info">
				<span class="setting-label">Block advertisements</span>
				<span class="setting-desc">Block ads and pop-ups for faster browsing</span>
			</div>
			<label class="toggle">
				<input type="checkbox" bind:checked={blockAds} />
				<span class="slider"></span>
			</label>
		</div>
	</section>

	<section class="settings-section">
		<h2>Appearance</h2>
		<div class="setting-row">
			<div class="setting-info">
				<span class="setting-label">Dark mode</span>
				<span class="setting-desc">Use dark color scheme</span>
			</div>
			<label class="toggle">
				<input type="checkbox" bind:checked={darkMode} />
				<span class="slider"></span>
			</label>
		</div>
	</section>

	<section class="settings-section about">
		<h2>About NONOS Ecosystem</h2>
		<div class="about-info">
			<div class="about-row">
				<span class="label">Version</span>
				<span class="value">1.0.0</span>
			</div>
			<div class="about-row">
				<span class="label">Website</span>
				<a href="https://nonos.systems" target="_blank" class="value link">nonos.systems</a>
			</div>
			<div class="about-row">
				<span class="label">GitHub</span>
				<a href="https://github.com/NON-OS" target="_blank" class="value link">github.com/NON-OS</a>
			</div>
			<div class="about-row">
				<span class="label">License</span>
				<span class="value">MIT</span>
			</div>
		</div>
		<p class="copyright">
			Copyright &copy; 2024 NON-OS &lt;team@nonos.systems&gt;
		</p>
	</section>
</div>

<style>
	.settings-page {
		max-width: 700px;
		margin: 0 auto;
	}

	h1 {
		font-size: 28px;
		font-weight: 700;
		margin-bottom: var(--nox-space-lg);
	}

	h2 {
		font-size: 16px;
		font-weight: 600;
		margin-bottom: var(--nox-space-md);
	}

	.section-desc {
		color: var(--nox-text-secondary);
		font-size: 13px;
		margin-bottom: var(--nox-space-md);
	}

	.settings-section {
		background: var(--nox-bg-secondary);
		border: 1px solid var(--nox-border);
		border-radius: var(--nox-radius-lg);
		padding: var(--nox-space-lg);
		margin-bottom: var(--nox-space-lg);
	}

	.security-options {
		display: flex;
		flex-direction: column;
		gap: var(--nox-space-sm);
	}

	.security-option {
		display: flex;
		align-items: flex-start;
		gap: var(--nox-space-md);
		padding: var(--nox-space-md);
		background: var(--nox-bg-tertiary);
		border: 2px solid var(--nox-border);
		border-radius: var(--nox-radius-md);
		cursor: pointer;
		transition: all var(--nox-transition-fast);
	}

	.security-option:hover {
		border-color: var(--nox-accent-primary);
	}

	.security-option.selected {
		border-color: var(--nox-accent-primary);
		background: rgba(99, 102, 241, 0.1);
	}

	.security-option input {
		margin-top: 4px;
	}

	.option-content {
		display: flex;
		flex-direction: column;
		gap: var(--nox-space-xs);
	}

	.option-label {
		font-weight: 500;
	}

	.option-desc {
		font-size: 12px;
		color: var(--nox-text-secondary);
	}

	.setting-row {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: var(--nox-space-md) 0;
		border-bottom: 1px solid var(--nox-border);
	}

	.setting-row:last-child {
		border-bottom: none;
	}

	.setting-info {
		display: flex;
		flex-direction: column;
		gap: var(--nox-space-xs);
	}

	.setting-label {
		font-weight: 500;
	}

	.setting-desc {
		font-size: 12px;
		color: var(--nox-text-secondary);
	}

	.toggle {
		position: relative;
		width: 44px;
		height: 24px;
		cursor: pointer;
	}

	.toggle input {
		opacity: 0;
		width: 0;
		height: 0;
	}

	.slider {
		position: absolute;
		inset: 0;
		background: var(--nox-bg-tertiary);
		border: 1px solid var(--nox-border);
		border-radius: var(--nox-radius-full);
		transition: all var(--nox-transition-fast);
	}

	.slider::before {
		content: '';
		position: absolute;
		width: 18px;
		height: 18px;
		left: 2px;
		top: 2px;
		background: white;
		border-radius: 50%;
		transition: transform var(--nox-transition-fast);
	}

	.toggle input:checked + .slider {
		background: var(--nox-accent-primary);
		border-color: var(--nox-accent-primary);
	}

	.toggle input:checked + .slider::before {
		transform: translateX(20px);
	}

	.select {
		padding: var(--nox-space-sm) var(--nox-space-md);
		background: var(--nox-bg-tertiary);
		border: 1px solid var(--nox-border);
		border-radius: var(--nox-radius-md);
		color: var(--nox-text-primary);
		font-size: 13px;
	}

	.about {
		text-align: center;
	}

	.about-info {
		display: flex;
		flex-direction: column;
		gap: var(--nox-space-sm);
		text-align: left;
		margin-bottom: var(--nox-space-lg);
	}

	.about-row {
		display: flex;
		justify-content: space-between;
		padding: var(--nox-space-sm) 0;
	}

	.about-row .label {
		color: var(--nox-text-muted);
	}

	.about-row .value {
		font-family: var(--nox-font-mono);
	}

	.about-row .link {
		color: var(--nox-accent-primary);
	}

	.copyright {
		font-size: 12px;
		color: var(--nox-text-muted);
	}
</style>
