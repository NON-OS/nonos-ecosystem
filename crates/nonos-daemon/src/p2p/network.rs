// NONOS GNU AFFERO GENERAL PUBLIC LICENSE
// Version 3, 19 November 2007
// Copyright (C) 2025 NON-OS <team@nonos.systems>

use super::behaviour::NonosBehaviour;
use super::peer_store::{SharedPeerStore, PenaltyReason, new_shared_peer_store};
use super::swarm::run_swarm;
use super::types::{
    BackoffStrategy, BanEntry, CircuitBreaker, NetworkCommand, NetworkEvent,
    NetworkStats, NetworkStatsSnapshot, PeerInfo, RateLimiter,
};
use super::topics;
use crate::config::{BootstrapMode, NodeRole};
use libp2p::{
    gossipsub, identify, kad, noise, ping, tcp, yamux,
    Multiaddr, PeerId, SwarmBuilder,
};
use nonos_types::{NonosError, NonosResult};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

pub(crate) const PROTOCOL_VERSION: &str = "/nonos/1.0.0";

const DEFAULT_MESSAGES_PER_SEC: u32 = 100;
const DEFAULT_BYTES_PER_SEC: u64 = 1024 * 1024;
const CIRCUIT_BREAKER_FAILURE_THRESHOLD: u32 = 5;
const CIRCUIT_BREAKER_SUCCESS_THRESHOLD: u32 = 3;
const CIRCUIT_BREAKER_RESET_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_MESSAGE_SIZE: usize = 64 * 1024;
const MAX_PEERS: usize = 50;
const MIN_PEERS: usize = 3;

/// Get bootstrap nodes based on mode
pub(crate) fn get_bootstrap_nodes_for_mode(
    mode: &BootstrapMode,
    custom_peers: &[String],
) -> Vec<String> {
    match mode {
        BootstrapMode::Official => {
            crate::config::NetworkConfig::OFFICIAL_BOOTSTRAP_NODES
                .iter()
                .map(|s| s.to_string())
                .collect()
        }
        BootstrapMode::Custom => custom_peers.to_vec(),
        BootstrapMode::None => Vec::new(),
    }
}

/// Legacy function for backward compatibility
pub(crate) fn get_bootstrap_nodes() -> Vec<String> {
    if let Ok(nodes) = std::env::var("NONOS_BOOTSTRAP_NODES") {
        return nodes.split(',').map(|s| s.trim().to_string()).collect();
    }

    crate::config::NetworkConfig::OFFICIAL_BOOTSTRAP_NODES
        .iter()
        .map(|s| s.to_string())
        .collect()
}

#[derive(Debug, Clone)]
pub struct NetworkConfig {
    pub port: u16,
    pub max_connections: u32,
    pub messages_per_sec: u32,
    pub bytes_per_sec: u64,
    pub enable_rate_limiting: bool,
    pub enable_circuit_breaker: bool,
    pub max_message_size: usize,
    pub idle_timeout: Duration,
    pub dial_timeout: Duration,
    pub bootstrap_on_start: bool,
    pub custom_bootstrap_nodes: Vec<String>,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            port: 9432,
            max_connections: MAX_PEERS as u32,
            messages_per_sec: DEFAULT_MESSAGES_PER_SEC,
            bytes_per_sec: DEFAULT_BYTES_PER_SEC,
            enable_rate_limiting: true,
            enable_circuit_breaker: true,
            max_message_size: MAX_MESSAGE_SIZE,
            idle_timeout: Duration::from_secs(60),
            dial_timeout: Duration::from_secs(10),
            bootstrap_on_start: true,
            custom_bootstrap_nodes: Vec::new(),
        }
    }
}

pub struct P2pNetwork {
    local_peer_id: PeerId,
    config: NetworkConfig,
    node_role: NodeRole,
    bootstrap_mode: BootstrapMode,
    custom_bootstrap_peers: Vec<String>,
    peer_store: SharedPeerStore,
    peers: Arc<RwLock<HashMap<PeerId, PeerInfo>>>,
    banned_peers: Arc<RwLock<HashMap<PeerId, BanEntry>>>,
    running: Arc<AtomicBool>,
    command_tx: Option<mpsc::Sender<NetworkCommand>>,
    event_rx: Arc<RwLock<Option<mpsc::Receiver<NetworkEvent>>>>,
    stats: Arc<NetworkStats>,
    subscribed_topics: Arc<RwLock<Vec<String>>>,
    local_key: libp2p::identity::Keypair,
    rate_limiters: Arc<RwLock<HashMap<PeerId, RateLimiter>>>,
    circuit_breakers: Arc<RwLock<HashMap<PeerId, CircuitBreaker>>>,
    bootstrap_backoff: Arc<RwLock<BackoffStrategy>>,
    started_at: Option<Instant>,
}

impl P2pNetwork {
    pub fn new(port: u16, max_connections: u32) -> Self {
        Self::with_config(NetworkConfig {
            port,
            max_connections,
            ..Default::default()
        })
    }

    /// Create a P2pNetwork from the main NodeConfig
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
            bootstrap_backoff: Arc::new(RwLock::new(BackoffStrategy::exponential(
                Duration::from_secs(1),
                Duration::from_secs(300),
            ).with_max_attempts(10))),
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
            bootstrap_backoff: Arc::new(RwLock::new(BackoffStrategy::exponential(
                Duration::from_secs(1),
                Duration::from_secs(300),
            ).with_max_attempts(10))),
            started_at: None,
        }
    }

    pub fn with_keypair(keypair: libp2p::identity::Keypair, port: u16, max_connections: u32) -> Self {
        let local_peer_id = PeerId::from(keypair.public());

        info!("Created P2P network with existing peer ID: {}", local_peer_id);

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
            bootstrap_backoff: Arc::new(RwLock::new(BackoffStrategy::exponential(
                Duration::from_secs(1),
                Duration::from_secs(300),
            ).with_max_attempts(10))),
            started_at: None,
        }
    }

    pub fn with_keypair_and_config(keypair: libp2p::identity::Keypair, config: NetworkConfig) -> Self {
        let local_peer_id = PeerId::from(keypair.public());

        info!("Created P2P network with existing peer ID: {} and custom config", local_peer_id);

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
            bootstrap_backoff: Arc::new(RwLock::new(BackoffStrategy::exponential(
                Duration::from_secs(1),
                Duration::from_secs(300),
            ).with_max_attempts(10))),
            started_at: None,
        }
    }

    pub fn local_peer_id(&self) -> &PeerId {
        &self.local_peer_id
    }

    pub fn config(&self) -> &NetworkConfig {
        &self.config
    }

    pub async fn start(&mut self) -> NonosResult<()> {
        if self.running.load(Ordering::Relaxed) {
            return Ok(());
        }

        info!("Starting P2P network on port {}", self.config.port);

        let swarm = SwarmBuilder::with_existing_identity(self.local_key.clone())
            .with_tokio()
            .with_tcp(
                tcp::Config::default(),
                noise::Config::new,
                yamux::Config::default,
            )
            .map_err(|e| NonosError::Network(format!("Failed to create transport: {}", e)))?
            .with_behaviour(|key| {
                let store = kad::store::MemoryStore::new(key.public().to_peer_id());
                #[allow(deprecated)]
                let kademlia_config = kad::Config::default();
                let kademlia = kad::Behaviour::with_config(
                    key.public().to_peer_id(),
                    store,
                    kademlia_config,
                );

                let gossipsub_config = gossipsub::ConfigBuilder::default()
                    .heartbeat_interval(Duration::from_secs(10))
                    .validation_mode(gossipsub::ValidationMode::Strict)
                    .max_transmit_size(self.config.max_message_size)
                    .message_id_fn(|msg| {
                        let hash = blake3::hash(&msg.data);
                        gossipsub::MessageId::from(hash.as_bytes().to_vec())
                    })
                    .build()
                    .expect("Valid gossipsub config");

                let gossipsub = gossipsub::Behaviour::new(
                    gossipsub::MessageAuthenticity::Signed(key.clone()),
                    gossipsub_config,
                )
                .expect("Valid gossipsub behaviour");

                let identify = identify::Behaviour::new(identify::Config::new(
                    PROTOCOL_VERSION.to_string(),
                    key.public(),
                ));

                let ping = ping::Behaviour::new(ping::Config::new());

                Ok(NonosBehaviour {
                    kademlia,
                    gossipsub,
                    identify,
                    ping,
                })
            })
            .map_err(|e| NonosError::Network(format!("Failed to create behaviour: {}", e)))?
            .with_swarm_config(|c| c.with_idle_connection_timeout(self.config.idle_timeout))
            .build();

        let (command_tx, command_rx) = mpsc::channel::<NetworkCommand>(256);
        let (event_tx, event_rx) = mpsc::channel::<NetworkEvent>(256);

        self.command_tx = Some(command_tx.clone());
        *self.event_rx.write() = Some(event_rx);

        let peers = self.peers.clone();
        let banned_peers = self.banned_peers.clone();
        let stats = self.stats.clone();
        let running = self.running.clone();
        let port = self.config.port;
        let rate_limiters = self.rate_limiters.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            run_swarm(
                swarm,
                command_rx,
                event_tx,
                peers,
                banned_peers,
                stats,
                running,
                port,
                rate_limiters,
                config,
            ).await;
        });

        self.running.store(true, Ordering::Relaxed);
        self.started_at = Some(Instant::now());

        self.subscribe(topics::HEALTH_BEACON).await?;
        self.subscribe(topics::QUALITY_REPORTS).await?;
        self.subscribe(topics::PEER_DISCOVERY).await?;
        self.subscribe(topics::NODE_ANNOUNCEMENTS).await?;

        if self.config.bootstrap_on_start {
            self.bootstrap().await?;
        }

        info!("P2P network started successfully");
        Ok(())
    }

    async fn bootstrap(&self) -> NonosResult<()> {
        if let Some(tx) = &self.command_tx {
            tx.send(NetworkCommand::Bootstrap).await
                .map_err(|e| NonosError::Network(format!("Failed to send bootstrap command: {}", e)))?;
        }
        Ok(())
    }

    pub async fn shutdown(&mut self) {
        info!("Shutting down P2P network");

        if let Some(tx) = &self.command_tx {
            let _ = tx.send(NetworkCommand::Shutdown).await;
        }

        self.running.store(false, Ordering::Relaxed);
        self.peers.write().clear();
        self.command_tx = None;
        self.started_at = None;
    }

    pub async fn connect(&self, address: &str) -> NonosResult<String> {
        let multiaddr: Multiaddr = address.parse()
            .map_err(|e| NonosError::Network(format!("Invalid multiaddress: {}", e)))?;

        if let Some(peer_id) = extract_peer_id(&multiaddr) {
            if self.is_banned(&peer_id) {
                return Err(NonosError::Network(format!("Peer {} is banned", peer_id)));
            }

            if self.config.enable_circuit_breaker {
                let mut circuit_breakers = self.circuit_breakers.write();
                let cb = circuit_breakers
                    .entry(peer_id)
                    .or_insert_with(|| CircuitBreaker::new(
                        CIRCUIT_BREAKER_FAILURE_THRESHOLD,
                        CIRCUIT_BREAKER_SUCCESS_THRESHOLD,
                        CIRCUIT_BREAKER_RESET_TIMEOUT,
                    ));

                if !cb.should_allow() {
                    self.stats.circuit_breaker_trips.fetch_add(1, Ordering::Relaxed);
                    return Err(NonosError::Network(format!(
                        "Circuit breaker open for peer {}",
                        peer_id
                    )));
                }
            }
        }

        self.stats.connection_attempts.fetch_add(1, Ordering::Relaxed);

        if let Some(tx) = &self.command_tx {
            tx.send(NetworkCommand::Connect(multiaddr)).await
                .map_err(|e| NonosError::Network(format!("Failed to send connect command: {}", e)))?;
        }

        Ok(format!("connecting_to_{}", address))
    }

    pub async fn disconnect(&self, peer_id: &str) {
        if let Ok(peer) = peer_id.parse::<PeerId>() {
            if let Some(tx) = &self.command_tx {
                let _ = tx.send(NetworkCommand::Disconnect(peer)).await;
            }
            self.peers.write().remove(&peer);
            self.rate_limiters.write().remove(&peer);
            debug!("Disconnected from peer: {}", peer_id);
        }
    }

    pub fn is_banned(&self, peer_id: &PeerId) -> bool {
        if let Some(ban) = self.banned_peers.read().get(peer_id) {
            if ban.is_expired() {
                return false;
            }
            return true;
        }
        false
    }

    pub async fn ban_peer(&self, peer_id: PeerId, duration: Duration, reason: &str) -> NonosResult<()> {
        let ban = BanEntry::new(peer_id, duration, reason);

        self.banned_peers.write().insert(peer_id, ban.clone());
        self.stats.banned_peers.fetch_add(1, Ordering::Relaxed);

        if let Some(tx) = &self.command_tx {
            tx.send(NetworkCommand::BanPeer(peer_id, duration)).await
                .map_err(|e| NonosError::Network(format!("Failed to send ban command: {}", e)))?;
        }

        self.disconnect(&peer_id.to_string()).await;

        warn!("Banned peer {} for {}: {}", peer_id, humanize_duration(duration), reason);
        Ok(())
    }

    pub async fn unban_peer(&self, peer_id: PeerId) -> NonosResult<()> {
        self.banned_peers.write().remove(&peer_id);

        if let Some(tx) = &self.command_tx {
            tx.send(NetworkCommand::UnbanPeer(peer_id)).await
                .map_err(|e| NonosError::Network(format!("Failed to send unban command: {}", e)))?;
        }

        info!("Unbanned peer {}", peer_id);
        Ok(())
    }

    pub fn cleanup_expired_bans(&self) {
        let mut banned = self.banned_peers.write();
        banned.retain(|_, ban| !ban.is_expired());
    }

    pub fn banned_peers(&self) -> Vec<BanEntry> {
        self.banned_peers.read().values().cloned().collect()
    }

    pub fn peer_count(&self) -> usize {
        self.peers.read().len()
    }

    pub fn peers(&self) -> Vec<PeerInfo> {
        self.peers.read().values().cloned().collect()
    }

    pub fn get_peer(&self, peer_id: &PeerId) -> Option<PeerInfo> {
        self.peers.read().get(peer_id).cloned()
    }

    pub async fn broadcast(&self, topic: &str, message: &[u8]) -> NonosResult<()> {
        if message.len() > self.config.max_message_size {
            return Err(NonosError::Network(format!(
                "Message too large: {} bytes (max: {})",
                message.len(),
                self.config.max_message_size
            )));
        }

        if let Some(tx) = &self.command_tx {
            tx.send(NetworkCommand::Publish {
                topic: topic.to_string(),
                data: message.to_vec(),
            }).await
            .map_err(|e| NonosError::Network(format!("Failed to broadcast: {}", e)))?;

            self.stats.messages_published.fetch_add(1, Ordering::Relaxed);
            self.stats.bytes_sent.fetch_add(message.len() as u64, Ordering::Relaxed);
        }
        Ok(())
    }

    pub async fn publish(&self, topic: &str, message: &[u8]) -> NonosResult<()> {
        self.broadcast(topic, message).await
    }

    pub fn get_known_peers(&self) -> Vec<String> {
        let peers = self.peers.read();
        peers.values()
            .filter_map(|info| {
                if info.addresses.is_empty() {
                    None
                } else {
                    Some(format!("{}/p2p/{}", info.addresses[0], info.id))
                }
            })
            .collect()
    }

    pub async fn subscribe(&self, topic: &str) -> NonosResult<()> {
        if let Some(tx) = &self.command_tx {
            tx.send(NetworkCommand::Subscribe(topic.to_string())).await
                .map_err(|e| NonosError::Network(format!("Failed to subscribe: {}", e)))?;

            self.subscribed_topics.write().push(topic.to_string());
            self.stats.active_topics.fetch_add(1, Ordering::Relaxed);
            debug!("Subscribed to topic: {}", topic);
        }
        Ok(())
    }

    pub async fn unsubscribe(&self, topic: &str) -> NonosResult<()> {
        if let Some(tx) = &self.command_tx {
            tx.send(NetworkCommand::Unsubscribe(topic.to_string())).await
                .map_err(|e| NonosError::Network(format!("Failed to unsubscribe: {}", e)))?;

            self.subscribed_topics.write().retain(|t| t != topic);
            self.stats.active_topics.fetch_sub(1, Ordering::Relaxed);
        }
        Ok(())
    }

    pub fn subscribed_topics(&self) -> Vec<String> {
        self.subscribed_topics.read().clone()
    }

    pub fn connection_count(&self) -> u32 {
        self.peers.read().len() as u32
    }

    pub fn ping(&self, peer_id: &str) -> NonosResult<u32> {
        if let Ok(peer) = peer_id.parse::<PeerId>() {
            if let Some(info) = self.peers.read().get(&peer) {
                if let Some(latency) = info.latency_ms {
                    return Ok(latency);
                }
            }
        }
        Ok(0)
    }

    pub fn stats(&self) -> NetworkStatsSnapshot {
        let mut snapshot = self.stats.snapshot();
        snapshot.peer_count = self.peers.read().len() as u64;
        snapshot.active_topics = self.subscribed_topics.read().len() as u64;
        snapshot
    }

    pub fn stats_ref(&self) -> Arc<NetworkStats> {
        Arc::clone(&self.stats)
    }

    pub async fn recv_event(&self) -> Option<NetworkEvent> {
        if let Some(rx) = &mut *self.event_rx.write() {
            rx.recv().await
        } else {
            None
        }
    }

    pub fn try_recv_event(&self) -> Option<NetworkEvent> {
        if let Some(rx) = &mut *self.event_rx.write() {
            rx.try_recv().ok()
        } else {
            None
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    pub fn uptime(&self) -> Option<Duration> {
        self.started_at.map(|t| t.elapsed())
    }

    pub async fn set_rate_limit(&self, messages_per_sec: u32, bytes_per_sec: u64) -> NonosResult<()> {
        if let Some(tx) = &self.command_tx {
            tx.send(NetworkCommand::SetRateLimit {
                messages_per_sec,
                bytes_per_sec,
            }).await
            .map_err(|e| NonosError::Network(format!("Failed to set rate limit: {}", e)))?;
        }
        Ok(())
    }

    pub fn record_peer_success(&self, peer_id: &PeerId) {
        if let Some(info) = self.peers.write().get_mut(peer_id) {
            info.record_success();
        }

        if self.config.enable_circuit_breaker {
            if let Some(cb) = self.circuit_breakers.write().get_mut(peer_id) {
                cb.record_success();
            }
        }
    }

    pub fn record_peer_failure(&self, peer_id: &PeerId) {
        if let Some(info) = self.peers.write().get_mut(peer_id) {
            info.record_failure();
        }

        if self.config.enable_circuit_breaker {
            if let Some(cb) = self.circuit_breakers.write().get_mut(peer_id) {
                cb.record_failure();
            }
        }

        self.stats.connection_failures.fetch_add(1, Ordering::Relaxed);
    }

    pub fn needs_more_peers(&self) -> bool {
        self.peers.read().len() < MIN_PEERS
    }

    pub fn has_capacity(&self) -> bool {
        self.peers.read().len() < self.config.max_connections as usize
    }

    pub fn get_bootstrap_nodes(&self) -> Vec<String> {
        get_bootstrap_nodes_for_mode(&self.bootstrap_mode, &self.custom_bootstrap_peers)
    }

    pub fn port(&self) -> u16 {
        self.config.port
    }

    /// Get the node role
    pub fn node_role(&self) -> NodeRole {
        self.node_role
    }

    /// Get the bootstrap mode
    pub fn bootstrap_mode(&self) -> &BootstrapMode {
        &self.bootstrap_mode
    }

    /// Get access to the peer store
    pub fn peer_store(&self) -> &SharedPeerStore {
        &self.peer_store
    }

    /// Apply a penalty to a peer via the peer store
    pub fn apply_peer_penalty(&self, peer_id: &PeerId, reason: PenaltyReason) {
        if let Some(score) = self.peer_store.apply_penalty(peer_id, reason.clone()) {
            if score >= super::peer_store::MAX_PENALTY_SCORE {
                self.stats.banned_peers.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    /// Record a successful peer interaction via the peer store
    pub fn record_peer_interaction_success(&self, peer_id: &PeerId) {
        self.peer_store.record_success(peer_id);
        self.record_peer_success(peer_id);
    }

    /// Record a failed peer interaction via the peer store
    pub fn record_peer_interaction_failure(&self, peer_id: &PeerId) {
        self.peer_store.record_failure(peer_id);
        self.record_peer_failure(peer_id);
    }

    /// Get trustworthy peers from the peer store
    pub fn trustworthy_peers(&self) -> Vec<super::peer_store::PeerEntry> {
        self.peer_store.trustworthy_peers()
    }

    /// Get peer store statistics
    pub fn peer_store_stats(&self) -> super::peer_store::PeerStoreStats {
        self.peer_store.stats()
    }

    /// Cleanup expired bans and sidelines in the peer store
    pub fn cleanup_peer_store(&self) {
        self.peer_store.cleanup_expired();
        self.cleanup_expired_bans();
    }

    /// Bootstrap with retry using exponential backoff
    pub async fn bootstrap_with_retry(&self) -> NonosResult<()> {
        loop {
            match self.bootstrap().await {
                Ok(_) => {
                    self.bootstrap_backoff.write().reset();
                    return Ok(());
                }
                Err(e) => {
                    let delay = {
                        let mut backoff = self.bootstrap_backoff.write();
                        backoff.next_delay()
                    };
                    if let Some(delay) = delay {
                        warn!("Bootstrap failed: {}. Retrying in {:?}", e, delay);
                        tokio::time::sleep(delay).await;
                    } else {
                        let attempts = self.bootstrap_backoff.read().attempts();
                        return Err(NonosError::Network(format!(
                            "Bootstrap failed after {} attempts: {}",
                            attempts,
                            e
                        )));
                    }
                }
            }
        }
    }

    /// Get current bootstrap backoff state
    pub fn bootstrap_backoff_state(&self) -> (u32, bool) {
        let backoff = self.bootstrap_backoff.read();
        (backoff.attempts(), backoff.is_exhausted())
    }
}

pub(crate) fn extract_peer_id(addr: &Multiaddr) -> Option<PeerId> {
    addr.iter().find_map(|p| {
        if let libp2p::multiaddr::Protocol::P2p(peer_id) = p {
            Some(peer_id)
        } else {
            None
        }
    })
}

fn humanize_duration(d: Duration) -> String {
    let secs = d.as_secs();
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m", secs / 60)
    } else if secs < 86400 {
        format!("{}h", secs / 3600)
    } else {
        format!("{}d", secs / 86400)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_config_default() {
        let config = NetworkConfig::default();
        assert_eq!(config.port, 9432);
        assert!(config.enable_rate_limiting);
        assert!(config.enable_circuit_breaker);
    }

    #[test]
    fn test_extract_peer_id() {
        let addr: Multiaddr = "/ip4/127.0.0.1/tcp/9432/p2p/12D3KooWAHtxSqGkTbpYjmJW55BpScGkezkWF3sQBK7Pv3CHjTPE"
            .parse()
            .unwrap();
        let peer_id = extract_peer_id(&addr);
        assert!(peer_id.is_some());
    }

    #[test]
    fn test_humanize_duration() {
        assert_eq!(humanize_duration(Duration::from_secs(30)), "30s");
        assert_eq!(humanize_duration(Duration::from_secs(120)), "2m");
        assert_eq!(humanize_duration(Duration::from_secs(7200)), "2h");
        assert_eq!(humanize_duration(Duration::from_secs(172800)), "2d");
    }

    #[test]
    fn test_bootstrap_nodes() {
        let nodes = get_bootstrap_nodes();
        assert!(!nodes.is_empty());
    }

    #[test]
    fn test_network_creation() {
        let network = P2pNetwork::new(9432, 50);
        assert_eq!(network.config.port, 9432);
        assert!(!network.is_running());
    }

    #[test]
    fn test_network_with_config() {
        let config = NetworkConfig {
            port: 9999,
            max_connections: 100,
            enable_rate_limiting: false,
            ..Default::default()
        };
        let network = P2pNetwork::with_config(config);
        assert_eq!(network.config.port, 9999);
        assert!(!network.config.enable_rate_limiting);
    }
}
