use super::commands::{OutputFormat, PeersAction};
use nonos_types::NonosResult;
use std::path::PathBuf;

pub async fn handle_peers(
    action: Option<PeersAction>,
    _data_dir: &PathBuf,
    format: &OutputFormat,
) -> NonosResult<()> {
    let api_port = std::env::var("NONOS_API_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8420u16);

    match action {
        Some(PeersAction::List) | None => {
            let url = format!("http://127.0.0.1:{}/api/peers", api_port);
            match reqwest::get(&url).await {
                Ok(response) if response.status().is_success() => {
                    let peers: serde_json::Value = response.json().await.unwrap_or_default();
                    match format {
                        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&peers).unwrap()),
                        OutputFormat::Text => {
                            println!("\x1b[38;5;46mConnected Peers\x1b[0m");
                            println!("\x1b[38;5;245m{}\x1b[0m", "═".repeat(50));
                            if let Some(count) = peers.get("count") {
                                println!("Total: \x1b[38;5;51m{}\x1b[0m", count);
                            }
                            if let Some(peer_list) = peers.get("peers").and_then(|p| p.as_array()) {
                                for peer in peer_list {
                                    println!("  \x1b[38;5;46m●\x1b[0m {}", peer.as_str().unwrap_or("unknown"));
                                }
                            }
                        }
                    }
                }
                _ => println!("\x1b[38;5;245mDaemon not running\x1b[0m"),
            }
        }
        Some(PeersAction::Show { peer_id }) => {
            println!("Peer details for \x1b[38;5;51m{}\x1b[0m (not yet implemented)", peer_id);
        }
        Some(PeersAction::Ban { peer_id }) => {
            println!("Banning peer \x1b[38;5;196m{}\x1b[0m (not yet implemented)", peer_id);
        }
        Some(PeersAction::Unban { peer_id }) => {
            println!("Unbanning peer \x1b[38;5;46m{}\x1b[0m (not yet implemented)", peer_id);
        }
    }
    Ok(())
}

pub async fn show_stats(_data_dir: &PathBuf, format: &OutputFormat) -> NonosResult<()> {
    let api_port = std::env::var("NONOS_API_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8420u16);
    let url = format!("http://127.0.0.1:{}/api/metrics", api_port);

    match reqwest::get(&url).await {
        Ok(response) if response.status().is_success() => {
            let stats: serde_json::Value = response.json().await.unwrap_or_default();
            match format {
                OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&stats).unwrap()),
                OutputFormat::Text => {
                    println!("\x1b[38;5;46mNetwork Statistics\x1b[0m");
                    println!("\x1b[38;5;245m{}\x1b[0m", "═".repeat(50));
                    println!("{}", serde_json::to_string_pretty(&stats).unwrap_or_default());
                }
            }
        }
        _ => println!("\x1b[38;5;245mDaemon not running. Start with: nonos run\x1b[0m"),
    }
    Ok(())
}
