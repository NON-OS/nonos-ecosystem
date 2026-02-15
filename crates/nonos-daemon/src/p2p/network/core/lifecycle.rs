use super::super::config::PROTOCOL_VERSION;
use super::network::P2pNetwork;
use crate::p2p::behaviour::NonosBehaviour;
use crate::p2p::swarm::run_swarm;
use crate::p2p::topics;
use crate::p2p::types::NetworkCommand;
use libp2p::{gossipsub, identify, kad, noise, ping, tcp, yamux, SwarmBuilder};
use nonos_types::{NonosError, NonosResult};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::info;

impl P2pNetwork {
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
        let (event_tx, event_rx) = mpsc::channel(256);

        self.command_tx = Some(command_tx.clone());
        *self.event_rx.write() = Some(event_rx);

        let peers = self.peers.clone();
        let banned_peers = self.banned_peers.clone();
        let stats = self.stats.clone();
        let running = self.running.clone();
        let rate_limiters = self.rate_limiters.clone();
        let config = self.config.clone();
        let port = self.config.port;

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
}
