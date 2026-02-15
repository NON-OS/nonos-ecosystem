use serde::{Deserialize, Serialize};
use super::constants::DEFAULT_MAX_MESSAGE_SIZE;
use super::types::BootstrapMode;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct NetworkConfig {
    pub bootstrap_mode: BootstrapMode,
    pub custom_bootstrap_peers: Vec<String>,
    pub announce_address: Option<String>,
    pub upnp: bool,
    pub max_message_size: usize,
    pub connection_timeout_secs: u64,
    pub dial_timeout_secs: u64,
    pub keepalive_secs: u64,
    pub max_pending_dials: u32,
    pub ban_threshold: u8,
    pub ban_duration_secs: u64,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            bootstrap_mode: BootstrapMode::Official,
            custom_bootstrap_peers: Vec::new(),
            announce_address: None,
            upnp: true,
            max_message_size: DEFAULT_MAX_MESSAGE_SIZE,
            connection_timeout_secs: 30,
            dial_timeout_secs: 10,
            keepalive_secs: 60,
            max_pending_dials: 16,
            ban_threshold: 80,
            ban_duration_secs: 3600,
        }
    }
}

impl NetworkConfig {
    pub const OFFICIAL_BOOTSTRAP_NODES: &'static [&'static str] = &[
        "/dns4/boot1.nonos.systems/tcp/9432/p2p/12D3KooWAHtxSqGkTbpYjmJW55BpScGkezkWF3sQBK7Pv3CHjTPE",
        "/dns4/boot2.nonos.systems/tcp/9432/p2p/12D3KooWBHvsSSKHeM9qGqMgytQ3K9SKvGfX4GhBxHF9ysQ3QKZJ",
        "/dns4/boot3.nonos.systems/tcp/9432/p2p/12D3KooWCNxuPgPx8ZSEcVKwT4xPL9vZWNJr4d3GwPJdQBTRZsrV",
        "/dns4/boot-eu.nonos.systems/tcp/9432/p2p/12D3KooWDrX4KaVmGwHj7LYJMh9fEqNjMRoP8cXkJSa1qVwYN2ZF",
        "/dns4/boot-us.nonos.systems/tcp/9432/p2p/12D3KooWEVS8nQr5bN7J9G4TFqGKMHEGgZ3cV8dQpTyH8zk5QRBM",
        "/dns4/boot-asia.nonos.systems/tcp/9432/p2p/12D3KooWFWUYLJhp6fT5F9mH3TJvpnYdJ1r4TK9dQz2z9KPJ7MqX",
        "/ip4/5.255.99.170/tcp/9432/p2p/12D3KooWAHtxSqGkTbpYjmJW55BpScGkezkWF3sQBK7Pv3CHjTPE",
        "/ip4/185.199.96.23/tcp/9432/p2p/12D3KooWBHvsSSKHeM9qGqMgytQ3K9SKvGfX4GhBxHF9ysQ3QKZJ",
        "/ip4/45.76.134.82/tcp/9432/p2p/12D3KooWCNxuPgPx8ZSEcVKwT4xPL9vZWNJr4d3GwPJdQBTRZsrV",
    ];

    pub fn bootstrap_peers(&self) -> Vec<String> {
        match self.bootstrap_mode {
            BootstrapMode::Official => {
                Self::OFFICIAL_BOOTSTRAP_NODES
                    .iter()
                    .map(|s| s.to_string())
                    .collect()
            }
            BootstrapMode::Custom => self.custom_bootstrap_peers.clone(),
            BootstrapMode::None => Vec::new(),
        }
    }
}
