use crate::config::{BootstrapMode, NodeRole};
use crate::p2p::peer_store::SharedPeerStore;
use crate::p2p::types::{
    BackoffStrategy, BanEntry, CircuitBreaker, NetworkCommand, NetworkEvent, NetworkStats,
    PeerInfo, RateLimiter,
};
use libp2p::PeerId;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;

use super::super::config::NetworkConfig;

pub struct P2pNetwork {
    pub(crate) local_peer_id: PeerId,
    pub(crate) config: NetworkConfig,
    pub(crate) node_role: NodeRole,
    pub(crate) bootstrap_mode: BootstrapMode,
    pub(crate) custom_bootstrap_peers: Vec<String>,
    pub(crate) peer_store: SharedPeerStore,
    pub(crate) peers: Arc<RwLock<HashMap<PeerId, PeerInfo>>>,
    pub(crate) banned_peers: Arc<RwLock<HashMap<PeerId, BanEntry>>>,
    pub(crate) running: Arc<AtomicBool>,
    pub(crate) command_tx: Option<mpsc::Sender<NetworkCommand>>,
    pub(crate) event_rx: Arc<RwLock<Option<mpsc::Receiver<NetworkEvent>>>>,
    pub(crate) stats: Arc<NetworkStats>,
    pub(crate) subscribed_topics: Arc<RwLock<Vec<String>>>,
    pub(crate) local_key: libp2p::identity::Keypair,
    pub(crate) rate_limiters: Arc<RwLock<HashMap<PeerId, RateLimiter>>>,
    pub(crate) circuit_breakers: Arc<RwLock<HashMap<PeerId, CircuitBreaker>>>,
    pub(crate) bootstrap_backoff: Arc<RwLock<BackoffStrategy>>,
    pub(crate) started_at: Option<Instant>,
}
