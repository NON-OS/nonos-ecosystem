#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod blockchain;
mod browser;
mod helpers;
mod network;
mod node;
mod privacy;
mod proxy;
mod staking;
mod state;
mod types;
mod wallet;

use state::AppState;
use tauri::Manager;

#[tauri::command]
fn get_app_info() -> types::AppInfo {
    types::AppInfo {
        name: "NONOS Ecosystem",
        version: env!("CARGO_PKG_VERSION"),
        platform: std::env::consts::OS,
        arch: std::env::consts::ARCH,
        build: if cfg!(debug_assertions) { "debug" } else { "release" },
    }
}

fn main() {
    let state = AppState::default();
    let network_state_for_setup = state.network.clone();

    tauri::Builder::default()
        .manage(state)
        .setup(move |app| {
            let window = app.get_window("main").unwrap();

            tauri::async_runtime::spawn(async move {
                proxy::start_local_proxy_server().await;
            });

            let network_for_spawn = network_state_for_setup.clone();
            tauri::async_runtime::spawn(async move {
                let _ = network::auto_start_anon(network_for_spawn).await;
            });

            window
                .eval(
                    r#"
                window.nonos = {
                    version: '1.0.0',
                    network: {
                        connect: () => window.__TAURI__.invoke('network_connect'),
                        disconnect: () => window.__TAURI__.invoke('network_disconnect'),
                        getStatus: () => window.__TAURI__.invoke('network_get_status'),
                        newIdentity: () => window.__TAURI__.invoke('network_new_identity'),
                    },
                    wallet: {
                        getStatus: () => window.__TAURI__.invoke('wallet_get_status'),
                        create: (password) => window.__TAURI__.invoke('wallet_create', { password }),
                        import: (mnemonic, password) => window.__TAURI__.invoke('wallet_import', { mnemonic, password }),
                        unlock: (password) => window.__TAURI__.invoke('wallet_unlock', { password }),
                        lock: () => window.__TAURI__.invoke('wallet_lock'),
                        getAddress: () => window.__TAURI__.invoke('wallet_get_address'),
                        sendEth: (to, amount) => window.__TAURI__.invoke('wallet_send_eth', { to, amount: String(amount) }),
                        sendNox: (to, amount) => window.__TAURI__.invoke('wallet_send_nox', { to, amount: String(amount) }),
                        getTransactions: () => window.__TAURI__.invoke('wallet_get_transactions'),
                    },
                    staking: {
                        getStatus: () => window.__TAURI__.invoke('staking_get_status'),
                        stake: (amount) => window.__TAURI__.invoke('staking_stake', { amount }),
                        unstake: (amount) => window.__TAURI__.invoke('staking_unstake', { amount }),
                        claimRewards: () => window.__TAURI__.invoke('staking_claim_rewards'),
                        withdraw: () => window.__TAURI__.invoke('staking_withdraw'),
                    },
                    node: {
                        getStatus: () => window.__TAURI__.invoke('node_get_status'),
                        startEmbedded: () => window.__TAURI__.invoke('node_start_embedded'),
                        stopEmbedded: () => window.__TAURI__.invoke('node_stop_embedded'),
                        getConnected: () => window.__TAURI__.invoke('node_get_connected'),
                    },
                    browser: {
                        navigate: (url) => window.__TAURI__.invoke('browser_navigate', { url }),
                        getSocksProxy: () => window.__TAURI__.invoke('browser_get_socks_proxy'),
                        proxyFetch: (url, options = {}) => window.__TAURI__.invoke('proxy_fetch', {
                            url,
                            method: options.method || 'GET',
                            headers: options.headers || null,
                            body: options.body || null,
                        }),
                    },
                    privacy: {
                        getStats: () => window.__TAURI__.invoke('privacy_get_stats'),
                        checkTracking: (domain) => window.__TAURI__.invoke('privacy_check_tracking', { domain }),
                        blockDomain: (domain) => window.__TAURI__.invoke('privacy_block_domain', { domain }),
                        generateIdentity: (name) => window.__TAURI__.invoke('privacy_generate_identity', { name }),
                        getIdentityRoot: () => window.__TAURI__.invoke('privacy_get_identity_root'),
                        cacheStore: (content) => window.__TAURI__.invoke('privacy_cache_store', { content }),
                    },
                    getAppInfo: () => window.__TAURI__.invoke('get_app_info'),
                    onNetworkStatus: (callback) => {
                        return window.__TAURI__.event.listen('nonos://network-status', (event) => callback(event.payload));
                    },
                    onIdentityChanged: (callback) => {
                        return window.__TAURI__.event.listen('nonos://identity-changed', callback);
                    },
                    onNodeStarted: (callback) => {
                        return window.__TAURI__.event.listen('nonos://node-started', callback);
                    },
                    onNodeStopped: (callback) => {
                        return window.__TAURI__.event.listen('nonos://node-stopped', callback);
                    },
                };
            "#,
                )
                .ok();

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_app_info,
            network::network_connect,
            network::network_disconnect,
            network::network_get_status,
            network::network_new_identity,
            wallet::wallet_get_status,
            wallet::wallet_create,
            wallet::wallet_import,
            wallet::wallet_unlock,
            wallet::wallet_lock,
            wallet::wallet_get_address,
            wallet::wallet_send_eth,
            wallet::wallet_send_nox,
            wallet::wallet_get_transactions,
            staking::staking_get_status,
            staking::staking_stake,
            staking::staking_unstake,
            staking::staking_claim_rewards,
            staking::staking_withdraw,
            node::node_get_status,
            node::node_start_embedded,
            node::node_stop_embedded,
            node::node_get_connected,
            browser::browser_navigate,
            browser::browser_close_tab,
            browser::browser_get_tabs,
            browser::browser_get_socks_proxy,
            browser::proxy_fetch,
            browser::get_proxy_url,
            privacy::privacy_get_stats,
            privacy::privacy_check_tracking,
            privacy::privacy_block_domain,
            privacy::privacy_generate_identity,
            privacy::privacy_get_identity_root,
            privacy::privacy_cache_store,
        ])
        .run(tauri::generate_context!())
        .expect("error while running NONOS Ecosystem browser");
}
