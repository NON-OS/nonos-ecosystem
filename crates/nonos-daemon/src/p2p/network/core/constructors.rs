use super::super::config::NetworkConfig;
use super::network::P2pNetwork;
use crate::config::{BootstrapMode, NodeRole};
use crate::p2p::peer_store::new_shared_peer_store;
use crate::p2p::types::{BackoffStrategy, NetworkStats};
use libp2p::PeerId;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;
use tracing::info;

impl P2pNetwork {
    pub fn new(port: u16, max_connections: u32) -> Self {
        Self::with_config(NetworkConfig {
            port,
            max_connections,
            ..Default::default()
        })
    }

    pub fn from_node_config(node_config: &crate::config::NodeConfig) -> Self {
        let local_key = libp2p::identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());

        let max_peers = node_config.effective_max_peers();
        let ban_threshold = node_config.network.ban_threshold as i32;

        info!(
            "Created P2P network with peer ID: {}, role: {}, max_peers: {}",
            local_peer_id, node_config.role, max_peers
        );

        let config = NetworkConfig {
            port: node_config.port,
            max_connections: max_peers,
            messages_per_sec: node_config.rate_limits.p2p_messages_per_second,
            bytes_per_sec: node_config.bandwidth_limit,
            enable_rate_limiting: node_config.rate_limits.enabled,
            enable_circuit_breaker: true,
            max_message_size: node_config.network.max_message_size,
            idle_timeout: Duration::from_secs(node_config.network.keepalive_secs),
            dial_timeout: Duration::from_secs(node_config.network.dial_timeout_secs),
            bootstrap_on_start: node_config.network.bootstrap_mode != BootstrapMode::None,
            custom_bootstrap_nodes: node_config.network.custom_bootstrap_peers.clone(),
        };

        Self {
            local_peer_id,
            config,
            node_role: node_config.role,
            bootstrap_mode: node_config.network.bootstrap_mode.clone(),
            custom_bootstrap_peers: node_config.network.custom_bootstrap_peers.clone(),
            peer_store: new_shared_peer_store(max_peers, ban_threshold),
            peers: Arc::new(RwLock::new(HashMap::new())),
            banned_peers: Arc::new(RwLock::new(HashMap::new())),
            running: Arc::new(AtomicBool::new(false)),
            command_tx: None,
            event_rx: Arc::new(RwLock::new(None)),
            stats: Arc::new(NetworkStats::new()),
            subscribed_topics: Arc::new(RwLock::new(Vec::new())),
            local_key,
            rate_limiters: Arc::new(RwLock::new(HashMap::new())),
            circuit_breakers: Arc::new(RwLock::new(HashMap::new())),
            bootstrap_backoff: Arc::new(RwLock::new(
                BackoffStrategy::exponential(Duration::from_secs(1), Duration::from_secs(300))
                    .with_max_attempts(10),
            )),
            started_at: None,
        }
    }

    pub fn with_config(config: NetworkConfig) -> Self {
        let local_key = libp2p::identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());

        info!("Created P2P network with peer ID: {}", local_peer_id);

        Self {
            local_peer_id,
            node_role: NodeRole::Local,
            bootstrap_mode: BootstrapMode::Official,
            custom_bootstrap_peers: config.custom_bootstrap_nodes.clone(),
            peer_store: new_shared_peer_store(config.max_connections, 100),
            config,
            peers: Arc::new(RwLock::new(HashMap::new())),
            banned_peers: Arc::new(RwLock::new(HashMap::new())),
            running: Arc::new(AtomicBool::new(false)),
            command_tx: None,
            event_rx: Arc::new(RwLock::new(None)),
            stats: Arc::new(NetworkStats::new()),
            subscribed_topics: Arc::new(RwLock::new(Vec::new())),
            local_key,
            rate_limiters: Arc::new(RwLock::new(HashMap::new())),
            circuit_breakers: Arc::new(RwLock::new(HashMap::new())),
            bootstrap_backoff: Arc::new(RwLock::new(
                BackoffStrategy::exponential(Duration::from_secs(1), Duration::from_secs(300))
                    .with_max_attempts(10),
            )),
            started_at: None,
        }
    }

    pub fn with_keypair(
        keypair: libp2p::identity::Keypair,
        port: u16,
        max_connections: u32,
    ) -> Self {
        let local_peer_id = PeerId::from(keypair.public());

        info!(
            "Created P2P network with existing peer ID: {}",
            local_peer_id
        );

        Self {
            local_peer_id,
            node_role: NodeRole::Local,
            bootstrap_mode: BootstrapMode::Official,
            custom_bootstrap_peers: Vec::new(),
            peer_store: new_shared_peer_store(max_connections, 100),
            config: NetworkConfig {
                port,
                max_connections,
                ..Default::default()
            },
            peers: Arc::new(RwLock::new(HashMap::new())),
            banned_peers: Arc::new(RwLock::new(HashMap::new())),
            running: Arc::new(AtomicBool::new(false)),
            command_tx: None,
            event_rx: Arc::new(RwLock::new(None)),
            stats: Arc::new(NetworkStats::new()),
            subscribed_topics: Arc::new(RwLock::new(Vec::new())),
            local_key: keypair,
            rate_limiters: Arc::new(RwLock::new(HashMap::new())),
            circuit_breakers: Arc::new(RwLock::new(HashMap::new())),
            bootstrap_backoff: Arc::new(RwLock::new(
                BackoffStrategy::exponential(Duration::from_secs(1), Duration::from_secs(300))
                    .with_max_attempts(10),
            )),
            started_at: None,
        }
    }

    pub fn with_keypair_and_config(
        keypair: libp2p::identity::Keypair,
        config: NetworkConfig,
    ) -> Self {
        let local_peer_id = PeerId::from(keypair.public());

        info!(
            "Created P2P network with existing peer ID: {} and custom config",
            local_peer_id
        );

        Self {
            local_peer_id,
            node_role: NodeRole::Local,
            bootstrap_mode: BootstrapMode::Official,
            custom_bootstrap_peers: config.custom_bootstrap_nodes.clone(),
            peer_store: new_shared_peer_store(config.max_connections, 100),
            config,
            peers: Arc::new(RwLock::new(HashMap::new())),
            banned_peers: Arc::new(RwLock::new(HashMap::new())),
            running: Arc::new(AtomicBool::new(false)),
            command_tx: None,
            event_rx: Arc::new(RwLock::new(None)),
            stats: Arc::new(NetworkStats::new()),
            subscribed_topics: Arc::new(RwLock::new(Vec::new())),
            local_key: keypair,
            rate_limiters: Arc::new(RwLock::new(HashMap::new())),
            circuit_breakers: Arc::new(RwLock::new(HashMap::new())),
            bootstrap_backoff: Arc::new(RwLock::new(
                BackoffStrategy::exponential(Duration::from_secs(1), Duration::from_secs(300))
                    .with_max_attempts(10),
            )),
            started_at: None,
        }
    }
}
