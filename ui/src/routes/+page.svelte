<script lang="ts">
	import { browserStore } from '$lib/stores/browser';
	import { onMount } from 'svelte';

	let frame: HTMLIFrameElement;
	let loading = false;
	let error = '';
	let browsing = false;
	let content = '';
	let history: string[] = [];
	let pos = -1;

	const P = 'http://localhost:9060/proxy?url=';

	async function inlineCss(html: string): Promise<string> {
		const linkRegex = /<link[^>]*>/gi;
		const links = [...html.matchAll(linkRegex)];
		const cssLinks: { tag: string; href: string }[] = [];

		for (const m of links) {
			const tag = m[0];
			const isStylesheet = /rel\s*=\s*["']stylesheet["']/i.test(tag);
			const hrefMatch = tag.match(/href\s*=\s*["']([^"']+)["']/i);
			if (isStylesheet && hrefMatch) {
				let href = hrefMatch[1];
				if (href.includes('localhost:9060')) {
					const u = href.match(/proxy\?url=(.+)$/);
					if (u) href = decodeURIComponent(u[1]);
				}
				if (href.startsWith('http') || href.startsWith('//')) {
					if (href.startsWith('//')) href = 'https:' + href;
					cssLinks.push({ tag, href });
				}
			}
		}

		if (!cssLinks.length) return html;

		const fetches = cssLinks.map(async ({ tag, href }) => {
			try {
				const r = await fetch(P + encodeURIComponent(href), { signal: AbortSignal.timeout(10000) });
				if (r.ok) {
					const css = await r.text();
					return { tag, css };
				}
			} catch {}
			return { tag, css: null };
		});

		const results = await Promise.all(fetches);
		let out = html;
		for (const { tag, css } of results) {
			if (css) out = out.replace(tag, `<style>${css}</style>`);
		}
		return out;
	}

	async function load(url: string) {
		if (!url) return;
		loading = true;
		error = '';
		browsing = true;
		content = '';

		if (history[pos] !== url) {
			history = [...history.slice(0, pos + 1), url];
			pos = history.length - 1;
		}

		browserStore.setCurrentUrl(url);

		try {
			const res = await fetch(P + encodeURIComponent(url));
			if (!res.ok) throw new Error('Failed');
			let html = await res.text();
			html = await inlineCss(html);
			content = html;
			loading = false;
		} catch (e) {
			loading = false;
			error = 'Failed to load - check if Anyone Network is connected';
		}
	}

	function search(e: SubmitEvent) {
		e.preventDefault();
		const fd = new FormData(e.target as HTMLFormElement);
		const q = (fd.get('q') as string || '').trim();
		if (!q) return;
		let url: string;
		if (q.startsWith('http://') || q.startsWith('https://')) url = q;
		else if (q.includes('.') && !q.includes(' ')) url = 'https://' + q;
		else url = 'https://html.duckduckgo.com/html/?q=' + encodeURIComponent(q);
		browserStore.setCurrentUrl(url);
	}

	function go(site: string) { browserStore.setCurrentUrl(site); }
	function back() { if (pos > 0) { pos--; browserStore.setCurrentUrl(history[pos]); load(history[pos]); } }
	function forward() { if (pos < history.length - 1) { pos++; browserStore.setCurrentUrl(history[pos]); load(history[pos]); } }
	function refresh() { if (history[pos]) load(history[pos]); }
	function home() {
		browsing = false;
		content = '';
		history = [];
		pos = -1;
		browserStore.clearUrl();
	}

	onMount(() => {
		let lastLoaded = '';
		const unsub = browserStore.subscribe(s => {
			if (s.currentUrl && s.currentUrl !== lastLoaded && s.currentUrl !== history[pos]) {
				lastLoaded = s.currentUrl;
				load(s.currentUrl);
			}
		});
		(window as any).__nonosBrowser = { goBack: back, goForward: forward, reload: refresh, goHome: home };

		function onMessage(e: MessageEvent) {
			if (e.data && e.data.type === 'navigate' && e.data.url) {
				lastLoaded = e.data.url;
				load(e.data.url);
			}
		}
		window.addEventListener('message', onMessage);

		return () => { unsub(); window.removeEventListener('message', onMessage); };
	});
</script>

<div class="page">
	{#if browsing}
		<div class="view">
			<iframe bind:this={frame} title="Browser" class="frame" srcdoc={content}></iframe>
			{#if loading}<div class="bar"><div class="progress"></div></div>{/if}
			{#if error}<div class="err"><span>{error}</span><button on:click={refresh}>Retry</button></div>{/if}
			<div class="badge"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/><path d="M9 12l2 2 4-4"/></svg><span>Protected via Anyone Network</span></div>
		</div>
	{:else}
		<div class="home">
			<div class="content">
				<svg class="logo" viewBox="0 0 1428 500" fill="none" xmlns="http://www.w3.org/2000/svg">
					<path d="M1361.48 155.388C1379.33 155.388 1393.96 159.853 1405.31 168.822L1405.84 169.244C1417.04 178.166 1423.37 189.937 1424.81 204.527L1424.86 205.076H1396.67L1396.62 204.638C1395.73 197.536 1392 191.299 1385.37 185.924L1385.37 185.919C1378.77 180.392 1370.01 177.604 1359.03 177.604C1348.79 177.604 1340.48 180.298 1334.06 185.648L1334.05 185.654C1327.66 190.803 1324.44 198.084 1324.44 207.568C1324.44 214.357 1326.32 219.851 1330.03 224.094L1330.77 224.888C1334.56 228.8 1339.05 231.847 1344.26 234.031C1350.04 236.199 1358.09 238.732 1368.42 241.63H1368.42C1380.93 245.078 1391.02 248.53 1398.67 251.989L1399.39 252.303C1406.78 255.616 1413.13 260.733 1418.45 267.642L1418.96 268.307C1424.15 275.248 1426.72 284.576 1426.72 296.24C1426.72 305.581 1424.24 314.374 1419.3 322.61C1414.35 330.858 1407.03 337.536 1397.35 342.65C1387.65 347.772 1376.23 350.323 1363.11 350.323C1350.54 350.323 1339.23 348.137 1329.18 343.754L1329.17 343.75V343.749C1319.31 339.184 1311.54 332.967 1305.86 325.092L1305.86 325.088C1300.17 317.023 1297.24 307.767 1297.06 297.337L1297.05 296.828H1324.4L1324.44 297.278C1325.34 306.046 1328.91 313.46 1335.17 319.542C1341.57 325.409 1350.86 328.38 1363.11 328.38C1374.82 328.38 1383.94 325.499 1390.53 319.798C1397.31 313.911 1400.69 306.434 1400.69 297.328C1400.69 290.171 1398.73 284.402 1394.83 279.978C1390.9 275.508 1385.98 272.108 1380.07 269.778C1374.11 267.43 1366.06 264.896 1355.91 262.179C1343.39 258.912 1333.3 255.641 1325.65 252.364L1325.65 252.362C1318.13 249.062 1311.63 243.931 1306.14 236.982L1306.14 236.976L1306.14 236.97C1300.79 229.787 1298.15 220.242 1298.15 208.384C1298.15 197.959 1300.8 188.704 1306.12 180.637C1311.43 172.574 1318.85 166.35 1328.36 161.962L1328.36 161.96C1338.05 157.574 1349.09 155.388 1361.48 155.388Z" fill="white" stroke="white"/>
					<path d="M1185.11 148C1204.69 148 1221.29 152.307 1234.89 160.92C1248.49 169.533 1258.82 181.502 1265.89 196.824C1272.97 212.147 1276.5 229.872 1276.5 250C1276.5 270.128 1272.97 287.853 1265.89 303.176C1258.82 318.498 1248.49 330.467 1234.89 339.08C1221.29 347.693 1204.69 352 1185.11 352C1165.62 352 1149.07 347.693 1135.47 339.08C1121.87 330.467 1111.49 318.498 1104.33 303.176C1097.26 287.853 1093.72 270.128 1093.72 250C1093.72 229.872 1097.26 212.147 1104.33 196.824C1111.49 181.502 1121.87 169.533 1135.47 160.92C1149.07 152.307 1165.62 148 1185.11 148ZM1185.11 171.12C1170.51 171.03 1158.36 174.294 1148.66 180.912C1139.05 187.531 1131.8 196.779 1126.9 208.656C1122.01 220.534 1119.51 234.315 1119.42 250C1119.33 265.595 1121.73 279.286 1126.63 291.072C1131.53 302.859 1138.83 312.107 1148.53 318.816C1158.32 325.435 1170.51 328.789 1185.11 328.88C1199.71 328.971 1211.86 325.707 1221.56 319.088C1231.35 312.379 1238.65 303.085 1243.46 291.208C1248.35 279.331 1250.8 265.595 1250.8 250C1250.8 234.315 1248.35 220.579 1243.46 208.792C1238.65 197.005 1231.35 187.802 1221.56 181.184C1211.86 174.565 1199.71 171.211 1185.11 171.12Z" fill="white"/>
					<path d="M557.964 157.564L558.112 157.789L656.747 307.087V157.564H682.499V348.42H656.979L656.83 348.195L558.195 198.625V348.42H532.443V157.564H557.964Z" fill="white" stroke="white"/>
					<path d="M724.874 351.456L707.602 335.952L731.946 309.16L735.754 305.488L850.538 179.688L852.714 176.832L878.01 148.952L895.282 164.456L867.81 194.784L864.954 197.096L750.17 323.168L747.45 326.704L724.874 351.456ZM800.082 352C780.588 352 764.042 347.693 750.442 339.08C736.842 330.467 726.46 318.499 719.298 303.176C712.226 287.853 708.69 270.128 708.69 250C708.69 229.872 712.226 212.147 719.298 196.824C726.46 181.501 736.842 169.533 750.442 160.92C764.042 152.307 780.588 148 800.082 148C819.666 148 836.258 152.307 849.858 160.92C863.458 169.533 873.794 181.501 880.866 196.824C887.938 212.147 891.474 229.872 891.474 250C891.474 270.128 887.938 287.853 880.866 303.176C873.794 318.499 863.458 330.467 849.858 339.08C836.258 347.693 819.666 352 800.082 352ZM800.082 328.88C814.679 328.971 826.828 325.707 836.53 319.088C846.322 312.379 853.62 303.085 858.426 291.208C863.322 279.331 865.77 265.595 865.77 250C865.77 234.315 863.322 220.579 858.426 208.792C853.62 197.005 846.322 187.803 836.53 181.184C826.828 174.565 814.679 171.211 800.082 171.12C785.484 171.029 773.335 174.293 763.634 180.912C754.023 187.531 746.77 196.779 741.874 208.656C736.978 220.533 734.484 234.315 734.394 250C734.303 265.595 736.706 279.285 741.602 291.072C746.498 302.859 753.796 312.107 763.498 318.816C773.29 325.435 785.484 328.789 800.082 328.88Z" fill="white"/>
					<path d="M943.021 157.564L943.169 157.789L1041.8 307.087V157.564H1067.56V348.42H1042.04L1041.89 348.195L943.252 198.625V348.42H917.5V157.564H943.021Z" fill="white" stroke="white"/>
					<path d="M280.574 205.148C281.041 206.156 281.49 207.184 281.918 208.232C286.849 220.104 289.315 233.94 289.315 249.739C289.315 265.447 286.849 279.283 281.918 291.246C277.078 303.21 269.726 312.571 259.863 319.329C250.092 325.996 237.854 329.283 223.15 329.191C208.447 329.1 196.165 325.721 186.302 319.055C184.37 317.719 182.534 316.281 180.791 314.745L280.574 205.148Z" fill="#66FFFF"/>
					<path d="M223.15 170.288C237.854 170.379 250.092 173.758 259.863 180.425C262.495 182.203 264.945 184.169 267.219 186.318L166.722 296.461C165.838 294.739 165.013 292.955 164.246 291.109C159.315 279.237 156.895 265.447 156.986 249.739C157.078 233.94 159.589 220.059 164.521 208.096C169.452 196.132 176.758 186.817 186.438 180.15C196.21 173.484 208.447 170.197 223.15 170.288Z" fill="#66FFFF"/>
					<path fill-rule="evenodd" clip-rule="evenodd" d="M370 0C414.183 0 450 35.8172 450 80V420C450 464.183 414.183 500 370 500H80C35.8172 500 0 464.183 0 420V80C2.3583e-05 35.8173 35.8173 2.57867e-05 80 0H370ZM223.15 147C203.516 147 186.849 151.338 173.15 160.014C159.452 168.689 148.996 180.744 141.781 196.178C134.658 211.611 131.096 229.465 131.096 249.739C131.096 270.013 134.658 287.868 141.781 303.302C143.839 307.704 146.162 311.83 148.747 315.683L130 336.315L147.397 351.932L164.499 333.18C167.223 335.441 170.106 337.538 173.15 339.466C186.849 348.142 203.516 352.479 223.15 352.479C242.876 352.479 259.589 348.142 273.288 339.466C286.986 330.79 297.397 318.735 304.521 303.302C311.644 287.868 315.205 270.013 315.205 249.739C315.205 229.465 311.644 211.611 304.521 196.178C302.836 192.528 300.965 189.068 298.912 185.796L319.041 163.575L301.644 147.959L283.659 167.779C280.441 164.945 276.985 162.355 273.288 160.014C259.589 151.338 242.876 147 223.15 147Z" fill="#66FFFF"/>
				</svg>
				<form class="search" on:submit={search}>
					<div class="box">
						<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="11" cy="11" r="8"/><path d="M21 21l-4.35-4.35"/></svg>
						<input type="text" name="q" placeholder="Search privately..." autocomplete="off" spellcheck="false"/>
						<button type="submit"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M5 12h14M12 5l7 7-7 7"/></svg></button>
					</div>
				</form>
				<div class="grid">
					<button on:click={() => go('https://check.en.anyone.tech')}><div class="icon"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/><path d="M9 12l2 2 4-4"/></svg></div><span>Privacy Check</span></button>
					<button on:click={() => go('https://html.duckduckgo.com/html/')}><div class="icon"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><circle cx="11" cy="11" r="8"/><path d="M21 21l-4.35-4.35"/></svg></div><span>DuckDuckGo</span></button>
					<button on:click={() => go('https://github.com/NON-OS')}><div class="icon"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M9 19c-5 1.5-5-2.5-7-3m14 6v-3.87a3.37 3.37 0 0 0-.94-2.61c3.14-.35 6.44-1.54 6.44-7A5.44 5.44 0 0 0 20 4.77 5.07 5.07 0 0 0 19.91 1S18.73.65 16 2.48a13.38 13.38 0 0 0-7 0C6.27.65 5.09 1 5.09 1A5.07 5.07 0 0 0 5 4.77a5.44 5.44 0 0 0-1.5 3.78c0 5.42 3.3 6.61 6.44 7A3.37 3.37 0 0 0 9 18.13V22"/></svg></div><span>GitHub</span></button>
					<button on:click={() => go('https://etherscan.io/token/0x0a26c80Be4E060e688d7C23aDdB92cBb5D2C9eCA')}><div class="icon nox"><svg viewBox="0 0 450 500" fill="none"><path d="M280.574 205.148C281.041 206.156 281.49 207.184 281.918 208.232C286.849 220.104 289.315 233.94 289.315 249.739C289.315 265.447 286.849 279.283 281.918 291.246C277.078 303.21 269.726 312.571 259.863 319.329C250.092 325.996 237.854 329.283 223.15 329.191C208.447 329.1 196.165 325.721 186.302 319.055C184.37 317.719 182.534 316.281 180.791 314.745L280.574 205.148Z" fill="currentColor"/><path d="M223.15 170.288C237.854 170.379 250.092 173.758 259.863 180.425C262.495 182.203 264.945 184.169 267.219 186.318L166.722 296.461C165.838 294.739 165.013 292.955 164.246 291.109C159.315 279.237 156.895 265.447 156.986 249.739C157.078 233.94 159.589 220.059 164.521 208.096C169.452 196.132 176.758 186.817 186.438 180.15C196.21 173.484 208.447 170.197 223.15 170.288Z" fill="currentColor"/><path fill-rule="evenodd" clip-rule="evenodd" d="M370 0C414.183 0 450 35.8172 450 80V420C450 464.183 414.183 500 370 500H80C35.8172 500 0 464.183 0 420V80C2.3583e-05 35.8173 35.8173 2.57867e-05 80 0H370ZM223.15 147C203.516 147 186.849 151.338 173.15 160.014C159.452 168.689 148.996 180.744 141.781 196.178C134.658 211.611 131.096 229.465 131.096 249.739C131.096 270.013 134.658 287.868 141.781 303.302C143.839 307.704 146.162 311.83 148.747 315.683L130 336.315L147.397 351.932L164.499 333.18C167.223 335.441 170.106 337.538 173.15 339.466C186.849 348.142 203.516 352.479 223.15 352.479C242.876 352.479 259.589 348.142 273.288 339.466C286.986 330.79 297.397 318.735 304.521 303.302C311.644 287.868 315.205 270.013 315.205 249.739C315.205 229.465 311.644 211.611 304.521 196.178C302.836 192.528 300.965 189.068 298.912 185.796L319.041 163.575L301.644 147.959L283.659 167.779C280.441 164.945 276.985 162.355 273.288 160.014C259.589 151.338 242.876 147 223.15 147Z" fill="currentColor"/></svg></div><span>NOX Token</span></button>
					<button on:click={() => go('https://anyone.io')}><div class="icon"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><circle cx="12" cy="5" r="3"/><circle cx="5" cy="19" r="3"/><circle cx="19" cy="19" r="3"/><path d="M12 8v4M8.5 14.5L5.5 17M15.5 14.5l3 2.5"/><circle cx="12" cy="14" r="2"/></svg></div><span>Anyone</span></button>
					<button on:click={() => go('https://wikipedia.org')}><div class="icon"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20"/><path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z"/></svg></div><span>Wikipedia</span></button>
				</div>
				<p class="info"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/></svg>All traffic encrypted via Anyone Network</p>
			</div>
		</div>
	{/if}
</div>

<style>
.page{height:100%;display:flex;flex-direction:column;background:var(--nox-bg-primary)}
.view{flex:1;display:flex;flex-direction:column;position:relative;background:#fff}
.frame{flex:1;width:100%;height:100%;border:none;background:#fff}
.bar{position:absolute;top:0;left:0;right:0;height:3px;background:rgba(102,255,255,0.2);overflow:hidden}
.progress{height:100%;width:30%;background:var(--nox-accent-primary);animation:load 1.5s ease-in-out infinite}
@keyframes load{0%{transform:translateX(-100%)}100%{transform:translateX(400%)}}
.err{position:absolute;top:8px;left:50%;transform:translateX(-50%);background:rgba(255,100,100,0.9);color:#fff;padding:8px 16px;border-radius:8px;display:flex;align-items:center;gap:12px;font-size:14px}
.err button{background:rgba(255,255,255,0.2);border:none;color:#fff;padding:4px 12px;border-radius:4px;cursor:pointer}
.badge{position:absolute;bottom:12px;right:12px;display:flex;align-items:center;gap:6px;background:rgba(10,16,21,0.85);padding:6px 12px;border-radius:20px;color:var(--nox-success);font-size:12px;border:1px solid rgba(102,255,255,0.2)}
.badge svg{width:14px;height:14px}
.home{flex:1;display:flex;align-items:center;justify-content:center;padding:var(--nox-space-xl);overflow:auto;background:url('/bg-grid.png') center/cover no-repeat;background-color:#0a1015}
.content{max-width:600px;width:100%;text-align:center;display:flex;flex-direction:column;align-items:center}
.logo{width:280px;height:auto;filter:drop-shadow(0 0 60px rgba(102,255,255,0.5));margin-bottom:var(--nox-space-xl)}
.search{width:100%;margin-bottom:var(--nox-space-xl)}
.box{display:flex;align-items:center;background:rgba(15,25,35,0.8);border:1px solid rgba(102,255,255,0.2);border-radius:var(--nox-radius-xl);padding:var(--nox-space-xs);backdrop-filter:blur(12px)}
.box:focus-within{border-color:var(--nox-accent-primary);box-shadow:0 0 0 3px rgba(102,255,255,0.15)}
.box svg{width:24px;height:24px;margin-left:var(--nox-space-md);color:var(--nox-text-muted)}
.box input{flex:1;padding:var(--nox-space-md);background:transparent;border:none;font-size:var(--nox-text-lg);color:var(--nox-text-primary)}
.box input:focus{outline:none}
.box input::placeholder{color:var(--nox-text-muted)}
.box button{width:48px;height:48px;display:flex;align-items:center;justify-content:center;background:var(--nox-accent-gradient);border-radius:var(--nox-radius-lg);color:var(--nox-bg-primary)}
.box button svg{width:20px;height:20px;margin:0}
.box button:hover{box-shadow:var(--nox-shadow-glow);transform:translateX(2px)}
.grid{display:grid;grid-template-columns:repeat(3,1fr);gap:var(--nox-space-md);width:100%;margin-bottom:var(--nox-space-xl)}
.grid button{display:flex;flex-direction:column;align-items:center;gap:var(--nox-space-sm);padding:var(--nox-space-lg);background:rgba(15,25,35,0.7);border:1px solid rgba(102,255,255,0.1);border-radius:var(--nox-radius-lg);backdrop-filter:blur(8px);color:var(--nox-text-secondary);font-size:var(--nox-text-sm)}
.grid button:hover{border-color:var(--nox-accent-primary);background:rgba(15,25,35,0.9);transform:translateY(-2px)}
.icon{width:40px;height:40px;display:flex;align-items:center;justify-content:center;border-radius:var(--nox-radius-md);background:rgba(102,255,255,0.1);color:var(--nox-accent-primary)}
.icon svg{width:22px;height:22px}
.icon.nox svg{width:28px;height:28px}
.info{display:flex;align-items:center;justify-content:center;gap:var(--nox-space-sm);font-size:var(--nox-text-sm);color:var(--nox-text-muted);opacity:0.6}
.info svg{width:16px;height:16px}
@media(max-width:600px){.grid{grid-template-columns:repeat(2,1fr)}}
</style>
