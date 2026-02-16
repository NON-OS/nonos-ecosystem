use super::commands::OutputFormat;
use super::utils::print_banner;
use libp2p::{identity::Keypair, PeerId};
use nonos_daemon::NodeConfig;
use nonos_types::{EthAddress, NonosResult};
use std::path::PathBuf;

const BUILD_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(all(target_arch = "x86_64", target_os = "macos"))]
const BUILD_TARGET: &str = "x86_64-apple-darwin";
#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
const BUILD_TARGET: &str = "aarch64-apple-darwin";
#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
const BUILD_TARGET: &str = "x86_64-unknown-linux-gnu";
#[cfg(all(target_arch = "aarch64", target_os = "linux"))]
const BUILD_TARGET: &str = "aarch64-unknown-linux-gnu";
#[cfg(all(target_arch = "x86_64", target_os = "windows"))]
const BUILD_TARGET: &str = "x86_64-pc-windows-msvc";
#[cfg(not(any(
    all(target_arch = "x86_64", target_os = "macos"),
    all(target_arch = "aarch64", target_os = "macos"),
    all(target_arch = "x86_64", target_os = "linux"),
    all(target_arch = "aarch64", target_os = "linux"),
    all(target_arch = "x86_64", target_os = "windows"),
)))]
const BUILD_TARGET: &str = "unknown";

pub async fn show_info(
    config_path: &PathBuf,
    data_dir: &PathBuf,
    format: &OutputFormat,
) -> NonosResult<()> {
    let config = if config_path.exists() {
        NodeConfig::load(config_path)?
    } else {
        NodeConfig::default()
    };

    let p2p_identity_path = data_dir.join("p2p_identity.key");
    let (peer_id, peer_id_short) = if p2p_identity_path.exists() {
        let key_bytes = std::fs::read(&p2p_identity_path)
            .map_err(|e| nonos_types::NonosError::Config(format!("Failed to read P2P identity: {}", e)))?;
        let keypair = Keypair::from_protobuf_encoding(&key_bytes)
            .map_err(|e| nonos_types::NonosError::Config(format!("Invalid P2P identity: {}", e)))?;
        let peer_id = PeerId::from(keypair.public());
        let full = peer_id.to_string();
        let short = format!("{}...{}", &full[..8], &full[full.len()-6..]);
        (full, short)
    } else {
        ("Not initialized".to_string(), "Not initialized".to_string())
    };

    let nickname_path = data_dir.join("nickname");
    let nickname = std::fs::read_to_string(&nickname_path).ok();

    let reward_addr = if config.rewards.reward_address != EthAddress::zero() {
        Some(format!("0x{}", hex::encode(config.rewards.reward_address.0)))
    } else {
        None
    };

    let identities_dir = data_dir.join("identities");
    let identity_count = if identities_dir.exists() {
        std::fs::read_dir(&identities_dir)
            .map(|entries| entries.filter(|e| e.is_ok()).count())
            .unwrap_or(0)
    } else {
        0
    };

    let p2p_port = config.port;
    let bootstrap_addr = if peer_id != "Not initialized" {
        format!("/ip4/<YOUR_IP>/tcp/{}/p2p/{}", p2p_port, peer_id)
    } else {
        "Not available".to_string()
    };

    match format {
        OutputFormat::Json => {
            let info = serde_json::json!({
                "version": BUILD_VERSION,
                "target": BUILD_TARGET,
                "peer_id": peer_id,
                "bootstrap_multiaddr": bootstrap_addr,
                "p2p_port": p2p_port,
                "config_path": config_path.to_string_lossy(),
                "data_dir": data_dir.to_string_lossy(),
                "nickname": nickname,
                "reward_address": reward_addr,
                "identity_count": identity_count,
            });
            println!("{}", serde_json::to_string_pretty(&info).unwrap());
        }
        OutputFormat::Text => {
            println!("\x1b[38;5;46mNONOS Daemon Information\x1b[0m");
            println!("\x1b[38;5;245m{}\x1b[0m", "═".repeat(70));
            println!();
            println!("Version:        \x1b[38;5;51m{}\x1b[0m", BUILD_VERSION);
            println!("Build Target:   \x1b[38;5;245m{}\x1b[0m", BUILD_TARGET);
            println!("Peer ID:        \x1b[38;5;226m{}\x1b[0m", peer_id_short);
            println!("Full Peer ID:   \x1b[38;5;245m{}\x1b[0m", peer_id);
            println!("P2P Port:       \x1b[38;5;51m{}\x1b[0m", p2p_port);
            println!();
            println!("\x1b[38;5;46mBootstrap Multiaddr (replace <YOUR_IP>):\x1b[0m");
            println!("  \x1b[38;5;51m{}\x1b[0m", bootstrap_addr);
            println!();
            println!("Config:         \x1b[38;5;245m{:?}\x1b[0m", config_path);
            println!("Data Dir:       \x1b[38;5;245m{:?}\x1b[0m", data_dir);
            if let Some(ref name) = nickname {
                println!("Nickname:       \x1b[38;5;51m{}\x1b[0m", name);
            }
            if let Some(ref addr) = reward_addr {
                println!("Reward Address: \x1b[38;5;51m{}\x1b[0m", addr);
            }
            println!("ZK Identities:  \x1b[38;5;46m{}\x1b[0m", identity_count);
            println!();
            println!("\x1b[38;5;245m{}\x1b[0m", "═".repeat(70));
        }
    }

    Ok(())
}

pub async fn show_status(data_dir: &PathBuf, format: &OutputFormat) -> NonosResult<()> {
    let api_port = std::env::var("NONOS_API_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8420u16);

    let url = format!("http://127.0.0.1:{}/api/status", api_port);

    match reqwest::get(&url).await {
        Ok(response) => {
            if response.status().is_success() {
                let status: serde_json::Value = response.json().await.unwrap_or_default();
                match format {
                    OutputFormat::Json => {
                        println!("{}", serde_json::to_string_pretty(&status).unwrap());
                    }
                    OutputFormat::Text => {
                        println!("\x1b[38;5;46m* NONOS Daemon: RUNNING\x1b[0m");
                        println!("\x1b[38;5;245m{}\x1b[0m", "═".repeat(50));
                        if let Some(node_id) = status.get("node_id") {
                            println!("Node ID:         \x1b[38;5;226m{}\x1b[0m", node_id.as_str().unwrap_or("unknown"));
                        }
                        if let Some(uptime) = status.get("uptime_secs") {
                            let secs = uptime.as_u64().unwrap_or(0);
                            println!("Uptime:          \x1b[38;5;51m{}h {}m {}s\x1b[0m", secs / 3600, (secs % 3600) / 60, secs % 60);
                        }
                        if let Some(quality) = status.get("quality_score") {
                            println!("Quality Score:   \x1b[38;5;46m{:.2}%\x1b[0m", quality.as_f64().unwrap_or(0.0) * 100.0);
                        }
                        if let Some(tier) = status.get("tier") {
                            println!("Tier:            \x1b[38;5;226m{}\x1b[0m", tier.as_str().unwrap_or("unknown"));
                        }
                        if let Some(conns) = status.get("active_connections") {
                            println!("Connections:     \x1b[38;5;51m{}\x1b[0m", conns);
                        }
                        if let Some(reqs) = status.get("total_requests") {
                            println!("Total Requests:  \x1b[38;5;51m{}\x1b[0m", reqs);
                        }
                        if let Some(pending) = status.get("pending_rewards") {
                            println!("Pending Rewards: \x1b[38;5;46m{:.4} NOX\x1b[0m", pending.as_f64().unwrap_or(0.0));
                        }
                        println!("\x1b[38;5;245m{}\x1b[0m", "═".repeat(50));
                    }
                }
            } else {
                println!("\x1b[38;5;196m* NONOS Daemon: ERROR\x1b[0m (API returned {})", response.status());
            }
        }
        Err(_) => {
            let pid_file = data_dir.join("nonos.pid");
            if pid_file.exists() {
                println!("\x1b[38;5;196m* NONOS Daemon: STOPPED\x1b[0m (stale PID file exists)");
            } else {
                println!("\x1b[38;5;245m* NONOS Daemon: NOT RUNNING\x1b[0m");
            }
            println!();
            println!("Start with: \x1b[38;5;51mnonos run\x1b[0m");
        }
    }

    Ok(())
}

pub fn show_version() {
    print_banner();
    println!("\x1b[38;5;46mBuild Information\x1b[0m");
    println!("\x1b[38;5;245m{}\x1b[0m", "═".repeat(50));
    println!("  Version:   \x1b[38;5;51m{}\x1b[0m", BUILD_VERSION);
    println!("  Target:    \x1b[38;5;245m{}\x1b[0m", BUILD_TARGET);
    println!("  Profile:   \x1b[38;5;245m{}\x1b[0m", if cfg!(debug_assertions) { "debug" } else { "release" });
    println!();
    println!("\x1b[38;5;46mComponents\x1b[0m");
    println!("\x1b[38;5;245m{}\x1b[0m", "═".repeat(50));
    println!("  P2P:       \x1b[38;5;51mlibp2p\x1b[0m (Kademlia DHT + GossipSub)");
    println!("  Privacy:   \x1b[38;5;51mAnyone Network\x1b[0m (SOCKS5)");
    println!("  ZK:        \x1b[38;5;51mGroth16 + Poseidon\x1b[0m (BN254 curve)");
    println!("  Storage:   \x1b[38;5;51msled\x1b[0m (Embedded database)");
    println!("  Crypto:    \x1b[38;5;51med25519, secp256k1, blake3\x1b[0m");
    println!();
    println!("\x1b[38;5;245mLicense:     AGPL-3.0\x1b[0m");
    println!("\x1b[38;5;245mRepository:  https://github.com/NON-OS/nonos\x1b[0m");
    println!("\x1b[38;5;245mWebsite:     https://nonos.systems\x1b[0m");
}
