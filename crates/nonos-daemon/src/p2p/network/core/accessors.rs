use super::super::config::NetworkConfig;
use super::network::P2pNetwork;
use crate::config::{BootstrapMode, NodeRole};
use crate::p2p::peer_store::SharedPeerStore;
use libp2p::PeerId;
use std::sync::atomic::Ordering;

impl P2pNetwork {
    pub fn local_peer_id(&self) -> &PeerId {
        &self.local_peer_id
    }

    pub fn config(&self) -> &NetworkConfig {
        &self.config
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    pub fn port(&self) -> u16 {
        self.config.port
    }

    pub fn node_role(&self) -> NodeRole {
        self.node_role
    }

    pub fn bootstrap_mode(&self) -> &BootstrapMode {
        &self.bootstrap_mode
    }

    pub fn peer_store(&self) -> &SharedPeerStore {
        &self.peer_store
    }
}
