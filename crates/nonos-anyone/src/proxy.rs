use crate::CircuitManager;
use nonos_types::{NonosError, NonosResult};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

const SOCKS5_NO_AUTH: u8 = 0x00;
const _SOCKS5_USERNAME_PASSWORD: u8 = 0x02;
const SOCKS5_NO_ACCEPTABLE: u8 = 0xFF;

const SOCKS5_CMD_CONNECT: u8 = 0x01;
const SOCKS5_CMD_BIND: u8 = 0x02;
const SOCKS5_CMD_UDP_ASSOCIATE: u8 = 0x03;

const SOCKS5_ATYP_IPV4: u8 = 0x01;
const SOCKS5_ATYP_DOMAIN: u8 = 0x03;
const SOCKS5_ATYP_IPV6: u8 = 0x04;

const SOCKS5_REP_SUCCESS: u8 = 0x00;
const SOCKS5_REP_GENERAL_FAILURE: u8 = 0x01;
const _SOCKS5_REP_CONNECTION_NOT_ALLOWED: u8 = 0x02;
const _SOCKS5_REP_NETWORK_UNREACHABLE: u8 = 0x03;
const _SOCKS5_REP_HOST_UNREACHABLE: u8 = 0x04;
const _SOCKS5_REP_CONNECTION_REFUSED: u8 = 0x05;
const _SOCKS5_REP_TTL_EXPIRED: u8 = 0x06;
const SOCKS5_REP_COMMAND_NOT_SUPPORTED: u8 = 0x07;
const SOCKS5_REP_ADDRESS_TYPE_NOT_SUPPORTED: u8 = 0x08;

pub struct Socks5Proxy {
    listen_addr: SocketAddr,
    upstream_addr: SocketAddr,
    circuits: Arc<CircuitManager>,
    running: Arc<RwLock<bool>>,
    connection_count: Arc<RwLock<u64>>,
    bytes_transferred: Arc<RwLock<u64>>,
}

impl Socks5Proxy {
    pub fn new(listen_addr: SocketAddr, upstream_addr: SocketAddr, circuits: Arc<CircuitManager>) -> Self {
        Self {
            listen_addr,
            upstream_addr,
            circuits,
            running: Arc::new(RwLock::new(false)),
            connection_count: Arc::new(RwLock::new(0)),
            bytes_transferred: Arc::new(RwLock::new(0)),
        }
    }

    pub fn with_anyone_default(listen_addr: SocketAddr, circuits: Arc<CircuitManager>) -> Self {
        let upstream_addr = SocketAddr::from(([127, 0, 0, 1], 9050));
        Self::new(listen_addr, upstream_addr, circuits)
    }

    pub async fn start(&self) -> NonosResult<()> {
        if *self.running.read().await {
            return Err(NonosError::Network("Proxy already running".into()));
        }

        let listener = TcpListener::bind(self.listen_addr)
            .await
            .map_err(|e| NonosError::Network(format!("Failed to bind: {}", e)))?;

        info!("SOCKS5 proxy listening on {} -> upstream {}", self.listen_addr, self.upstream_addr);
        *self.running.write().await = true;

        let running = self.running.clone();
        let circuits = self.circuits.clone();
        let connection_count = self.connection_count.clone();
        let bytes_transferred = self.bytes_transferred.clone();
        let upstream_addr = self.upstream_addr;

        tokio::spawn(async move {
            loop {
                if !*running.read().await {
                    break;
                }

                match listener.accept().await {
                    Ok((stream, addr)) => {
                        debug!("New connection from {}", addr);
                        *connection_count.write().await += 1;

                        let circuits = circuits.clone();
                        let bytes_counter = bytes_transferred.clone();

                        tokio::spawn(async move {
                            match handle_connection(stream, upstream_addr, circuits, bytes_counter).await {
                                Ok(bytes) => {
                                    debug!("Connection completed, {} bytes transferred", bytes);
                                }
                                Err(e) => {
                                    warn!("Connection error: {}", e);
                                }
                            }
                        });
                    }
                    Err(e) => {
                        error!("Accept error: {}", e);
                    }
                }
            }
        });

        Ok(())
    }

    pub async fn stop(&self) {
        *self.running.write().await = false;
        info!("SOCKS5 proxy stopped");
    }

    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    pub async fn connection_count(&self) -> u64 {
        *self.connection_count.read().await
    }

    pub async fn bytes_transferred(&self) -> u64 {
        *self.bytes_transferred.read().await
    }
}

async fn handle_connection(
    mut client: TcpStream,
    upstream_addr: SocketAddr,
    circuits: Arc<CircuitManager>,
    bytes_counter: Arc<RwLock<u64>>,
) -> NonosResult<u64> {
    let mut buf = [0u8; 256];

    client.read_exact(&mut buf[..2]).await.map_err(|e| {
        NonosError::Network(format!("Failed to read handshake: {}", e))
    })?;

    let version = buf[0];
    if version != 0x05 {
        return Err(NonosError::Network(format!(
            "Unsupported SOCKS version: {}",
            version
        )));
    }

    let nmethods = buf[1] as usize;
    client
        .read_exact(&mut buf[..nmethods])
        .await
        .map_err(|e| NonosError::Network(format!("Failed to read auth methods: {}", e)))?;

    let supports_no_auth = buf[..nmethods].contains(&SOCKS5_NO_AUTH);
    if !supports_no_auth {
        client
            .write_all(&[0x05, SOCKS5_NO_ACCEPTABLE])
            .await
            .map_err(|e| NonosError::Network(e.to_string()))?;
        return Err(NonosError::Network("No acceptable auth method".into()));
    }

    client
        .write_all(&[0x05, SOCKS5_NO_AUTH])
        .await
        .map_err(|e| NonosError::Network(e.to_string()))?;

    client.read_exact(&mut buf[..4]).await.map_err(|e| {
        NonosError::Network(format!("Failed to read request: {}", e))
    })?;

    let _version = buf[0];
    let cmd = buf[1];
    let _reserved = buf[2];
    let atyp = buf[3];

    let (host, port, raw_addr) = match atyp {
        SOCKS5_ATYP_IPV4 => {
            client.read_exact(&mut buf[..6]).await.map_err(|e| {
                NonosError::Network(format!("Failed to read IPv4 address: {}", e))
            })?;
            let ip = format!("{}.{}.{}.{}", buf[0], buf[1], buf[2], buf[3]);
            let port = u16::from_be_bytes([buf[4], buf[5]]);
            let mut raw = vec![atyp];
            raw.extend_from_slice(&buf[..6]);
            (ip, port, raw)
        }
        SOCKS5_ATYP_DOMAIN => {
            client.read_exact(&mut buf[..1]).await.map_err(|e| {
                NonosError::Network(format!("Failed to read domain length: {}", e))
            })?;
            let len = buf[0] as usize;
            client.read_exact(&mut buf[..len + 2]).await.map_err(|e| {
                NonosError::Network(format!("Failed to read domain: {}", e))
            })?;
            let domain = String::from_utf8_lossy(&buf[..len]).to_string();
            let port = u16::from_be_bytes([buf[len], buf[len + 1]]);
            let mut raw = vec![atyp, len as u8];
            raw.extend_from_slice(&buf[..len + 2]);
            (domain, port, raw)
        }
        SOCKS5_ATYP_IPV6 => {
            client.read_exact(&mut buf[..18]).await.map_err(|e| {
                NonosError::Network(format!("Failed to read IPv6 address: {}", e))
            })?;
            let ip = format!(
                "{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}",
                u16::from_be_bytes([buf[0], buf[1]]),
                u16::from_be_bytes([buf[2], buf[3]]),
                u16::from_be_bytes([buf[4], buf[5]]),
                u16::from_be_bytes([buf[6], buf[7]]),
                u16::from_be_bytes([buf[8], buf[9]]),
                u16::from_be_bytes([buf[10], buf[11]]),
                u16::from_be_bytes([buf[12], buf[13]]),
                u16::from_be_bytes([buf[14], buf[15]]),
            );
            let port = u16::from_be_bytes([buf[16], buf[17]]);
            let mut raw = vec![atyp];
            raw.extend_from_slice(&buf[..18]);
            (ip, port, raw)
        }
        _ => {
            send_reply(&mut client, SOCKS5_REP_ADDRESS_TYPE_NOT_SUPPORTED).await?;
            return Err(NonosError::Network(format!(
                "Unsupported address type: {}",
                atyp
            )));
        }
    };

    debug!("SOCKS5 request: cmd={}, host={}, port={}", cmd, host, port);

    match cmd {
        SOCKS5_CMD_CONNECT => {
            let circuit_id = circuits.get_circuit_for_domain(&host).await?;
            debug!("Using circuit {:?} for {}", circuit_id, host);

            let mut upstream = match TcpStream::connect(upstream_addr).await {
                Ok(s) => s,
                Err(e) => {
                    error!("Failed to connect to upstream Anyone proxy: {}", e);
                    send_reply(&mut client, SOCKS5_REP_GENERAL_FAILURE).await?;
                    return Err(NonosError::Network(format!("Upstream connection failed: {}", e)));
                }
            };

            upstream.write_all(&[0x05, 0x01, 0x00]).await.map_err(|e| {
                NonosError::Network(format!("Upstream handshake failed: {}", e))
            })?;

            let mut response = [0u8; 2];
            upstream.read_exact(&mut response).await.map_err(|e| {
                NonosError::Network(format!("Upstream handshake response failed: {}", e))
            })?;

            if response[0] != 0x05 || response[1] != 0x00 {
                send_reply(&mut client, SOCKS5_REP_GENERAL_FAILURE).await?;
                return Err(NonosError::Network("Upstream auth failed".into()));
            }

            let mut request = vec![0x05, 0x01, 0x00];
            request.extend_from_slice(&raw_addr);
            upstream.write_all(&request).await.map_err(|e| {
                NonosError::Network(format!("Upstream connect request failed: {}", e))
            })?;

            let mut upstream_response = [0u8; 10];
            upstream.read_exact(&mut upstream_response).await.map_err(|e| {
                NonosError::Network(format!("Upstream connect response failed: {}", e))
            })?;

            client.write_all(&upstream_response).await.map_err(|e| {
                NonosError::Network(format!("Failed to send response to client: {}", e))
            })?;

            if upstream_response[1] != SOCKS5_REP_SUCCESS {
                return Err(NonosError::Network("Upstream connect failed".into()));
            }

            info!("Connected to {}:{} via Anyone Network (circuit {:?})", host, port, circuit_id);

            let (client_read, client_write) = client.into_split();
            let (upstream_read, upstream_write) = upstream.into_split();

            let mut client_read = client_read;
            let mut client_write = client_write;
            let mut upstream_read = upstream_read;
            let mut upstream_write = upstream_write;

            let (c2u, u2c) = tokio::join!(
                async {
                    let mut buf = [0u8; 8192];
                    let mut total = 0u64;
                    loop {
                        match client_read.read(&mut buf).await {
                            Ok(0) => break,
                            Ok(n) => {
                                if upstream_write.write_all(&buf[..n]).await.is_err() {
                                    break;
                                }
                                total += n as u64;
                            }
                            Err(_) => break,
                        }
                    }
                    total
                },
                async {
                    let mut buf = [0u8; 8192];
                    let mut total = 0u64;
                    loop {
                        match upstream_read.read(&mut buf).await {
                            Ok(0) => break,
                            Ok(n) => {
                                if client_write.write_all(&buf[..n]).await.is_err() {
                                    break;
                                }
                                total += n as u64;
                            }
                            Err(_) => break,
                        }
                    }
                    total
                }
            );

            let total_bytes = c2u + u2c;
            *bytes_counter.write().await += total_bytes;

            Ok(total_bytes)
        }
        SOCKS5_CMD_BIND => {
            send_reply(&mut client, SOCKS5_REP_COMMAND_NOT_SUPPORTED).await?;
            Err(NonosError::Network("BIND not supported".into()))
        }
        SOCKS5_CMD_UDP_ASSOCIATE => {
            send_reply(&mut client, SOCKS5_REP_COMMAND_NOT_SUPPORTED).await?;
            Err(NonosError::Network("UDP ASSOCIATE not supported".into()))
        }
        _ => {
            send_reply(&mut client, SOCKS5_REP_COMMAND_NOT_SUPPORTED).await?;
            Err(NonosError::Network(format!("Unknown command: {}", cmd)))
        }
    }
}

async fn send_reply(stream: &mut TcpStream, rep: u8) -> NonosResult<()> {
    let reply = [
        0x05,
        rep,
        0x00,
        0x01,
        0x00, 0x00, 0x00, 0x00,
        0x00, 0x00,
    ];

    stream
        .write_all(&reply)
        .await
        .map_err(|e| NonosError::Network(format!("Failed to send reply: {}", e)))
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct IsolationKey {
    pub domain: String,
    pub tab_id: Option<u64>,
}

impl IsolationKey {
    pub fn from_domain(domain: impl Into<String>) -> Self {
        Self {
            domain: domain.into(),
            tab_id: None,
        }
    }

    pub fn with_tab(domain: impl Into<String>, tab_id: u64) -> Self {
        Self {
            domain: domain.into(),
            tab_id: Some(tab_id),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_isolation_key() {
        let key1 = IsolationKey::from_domain("example.com");
        let key2 = IsolationKey::from_domain("example.com");
        let key3 = IsolationKey::with_tab("example.com", 1);

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }
}
