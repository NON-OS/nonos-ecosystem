use crate::{P2pNetwork, NodeStorage};
use nonos_types::{NonosError, NonosResult};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

pub struct BootstrapService {
    network: Arc<RwLock<P2pNetwork>>,
    _storage: Arc<NodeStorage>,
    port: u16,
}

impl BootstrapService {
    pub fn new(network: Arc<RwLock<P2pNetwork>>, storage: Arc<NodeStorage>, port: u16) -> Self {
        Self { network, _storage: storage, port }
    }

    pub async fn run(&self, shutdown: Arc<AtomicBool>) -> NonosResult<()> {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", self.port))
            .await
            .map_err(|e| NonosError::Network(format!("Failed to bind: {}", e)))?;

        info!("Bootstrap service listening on port {}", self.port);

        loop {
            if shutdown.load(Ordering::SeqCst) {
                info!("Bootstrap service shutting down");
                break;
            }

            match tokio::time::timeout(Duration::from_secs(1), listener.accept()).await {
                Ok(Ok((socket, addr))) => {
                    let network = self.network.clone();
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_request(socket, addr, network).await {
                            warn!("Bootstrap request failed from {}: {}", addr, e);
                        }
                    });
                }
                Ok(Err(e)) => warn!("Accept error: {}", e),
                Err(_) => {}
            }
        }

        Ok(())
    }

    async fn handle_request(
        mut socket: TcpStream,
        addr: SocketAddr,
        network: Arc<RwLock<P2pNetwork>>,
    ) -> NonosResult<()> {
        debug!("Bootstrap request from {}", addr);

        let mut buf = [0u8; 4];
        socket.read_exact(&mut buf).await
            .map_err(|e| NonosError::Network(format!("Read error: {}", e)))?;

        match &buf {
            b"PEER" => {
                let peers = network.read().await.get_known_peers();
                let response = serde_json::to_vec(&peers)
                    .map_err(|e| NonosError::Internal(format!("Serialize error: {}", e)))?;

                let len = (response.len() as u32).to_be_bytes();
                socket.write_all(&len).await
                    .map_err(|e| NonosError::Network(format!("Write error: {}", e)))?;
                socket.write_all(&response).await
                    .map_err(|e| NonosError::Network(format!("Write error: {}", e)))?;

                debug!("Sent {} peers to {}", peers.len(), addr);
            }
            b"CONF" => {
                let config = BootstrapConfig {
                    network_id: "nonos-mainnet".to_string(),
                    protocol_version: 1,
                    min_peers: 4,
                    max_peers: 50,
                };
                let response = serde_json::to_vec(&config)
                    .map_err(|e| NonosError::Internal(format!("Serialize error: {}", e)))?;

                let len = (response.len() as u32).to_be_bytes();
                socket.write_all(&len).await
                    .map_err(|e| NonosError::Network(format!("Write error: {}", e)))?;
                socket.write_all(&response).await
                    .map_err(|e| NonosError::Network(format!("Write error: {}", e)))?;

                debug!("Sent config to {}", addr);
            }
            _ => warn!("Unknown request type from {}", addr),
        }

        Ok(())
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct BootstrapConfig {
    pub network_id: String,
    pub protocol_version: u32,
    pub min_peers: u32,
    pub max_peers: u32,
}
