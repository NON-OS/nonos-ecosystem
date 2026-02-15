use super::super::config::{
    CIRCUIT_BREAKER_FAILURE_THRESHOLD, CIRCUIT_BREAKER_RESET_TIMEOUT,
    CIRCUIT_BREAKER_SUCCESS_THRESHOLD, MIN_PEERS,
};
use super::super::helpers::extract_peer_id;
use super::network::P2pNetwork;
use crate::p2p::types::{CircuitBreaker, NetworkCommand, PeerInfo};
use libp2p::{Multiaddr, PeerId};
use nonos_types::{NonosError, NonosResult};
use std::sync::atomic::Ordering;
use tracing::debug;

impl P2pNetwork {
    pub async fn connect(&self, address: &str) -> NonosResult<String> {
        let multiaddr: Multiaddr = address
            .parse()
            .map_err(|e| NonosError::Network(format!("Invalid multiaddress: {}", e)))?;

        if let Some(peer_id) = extract_peer_id(&multiaddr) {
            if self.is_banned(&peer_id) {
                return Err(NonosError::Network(format!("Peer {} is banned", peer_id)));
            }

            if self.config.enable_circuit_breaker {
                let mut circuit_breakers = self.circuit_breakers.write();
                let cb = circuit_breakers.entry(peer_id).or_insert_with(|| {
                    CircuitBreaker::new(
                        CIRCUIT_BREAKER_FAILURE_THRESHOLD,
                        CIRCUIT_BREAKER_SUCCESS_THRESHOLD,
                        CIRCUIT_BREAKER_RESET_TIMEOUT,
                    )
                });

                if !cb.should_allow() {
                    self.stats
                        .circuit_breaker_trips
                        .fetch_add(1, Ordering::Relaxed);
                    return Err(NonosError::Network(format!(
                        "Circuit breaker open for peer {}",
                        peer_id
                    )));
                }
            }
        }

        self.stats
            .connection_attempts
            .fetch_add(1, Ordering::Relaxed);

        if let Some(tx) = &self.command_tx {
            tx.send(NetworkCommand::Connect(multiaddr))
                .await
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

    pub fn peer_count(&self) -> usize {
        self.peers.read().len()
    }

    pub fn peers(&self) -> Vec<PeerInfo> {
        self.peers.read().values().cloned().collect()
    }

    pub fn get_peer(&self, peer_id: &PeerId) -> Option<PeerInfo> {
        self.peers.read().get(peer_id).cloned()
    }

    pub fn get_known_peers(&self) -> Vec<String> {
        let peers = self.peers.read();
        peers
            .values()
            .filter_map(|info| {
                if info.addresses.is_empty() {
                    None
                } else {
                    Some(format!("{}/p2p/{}", info.addresses[0], info.id))
                }
            })
            .collect()
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

    pub fn needs_more_peers(&self) -> bool {
        self.peers.read().len() < MIN_PEERS
    }

    pub fn has_capacity(&self) -> bool {
        self.peers.read().len() < self.config.max_connections as usize
    }
}
