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
        // Node 1 - South Africa
        "/ip4/102.211.56.24/tcp/9432/p2p/12D3KooWD3x5mMCRUVi5HS8A2NR6J2344wRt45ewzwq58bjKuAuu",
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
