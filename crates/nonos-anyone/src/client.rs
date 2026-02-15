use crate::{AnyoneConfig, CircuitManager, SecurityPreset};
use nonos_types::{CircuitId, ConnectionStatus, NetworkStatus, NonosError, NonosResult};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::process::{Child, Command};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClientState {
    Stopped,
    Starting,
    Bootstrapping,
    Ready,
    Stopping,
    Error,
}

pub struct AnyoneClient {
    config: AnyoneConfig,
    state: Arc<RwLock<ClientState>>,
    circuits: Arc<CircuitManager>,
    security: SecurityPreset,
    error_message: Arc<RwLock<Option<String>>>,
    anon_process: Arc<RwLock<Option<Child>>>,
    socks_addr: Arc<RwLock<SocketAddr>>,
    control_port: Arc<RwLock<u16>>,
}

impl AnyoneClient {
    pub fn new() -> Self {
        Self::with_config(AnyoneConfig::default())
    }

    pub fn with_config(config: AnyoneConfig) -> Self {
        let socks_port = config.socks_port;
        Self {
            config,
            state: Arc::new(RwLock::new(ClientState::Stopped)),
            circuits: Arc::new(CircuitManager::new()),
            security: SecurityPreset::default(),
            error_message: Arc::new(RwLock::new(None)),
            anon_process: Arc::new(RwLock::new(None)),
            socks_addr: Arc::new(RwLock::new(SocketAddr::from(([127, 0, 0, 1], socks_port)))),
            control_port: Arc::new(RwLock::new(9051)),
        }
    }

    pub async fn state(&self) -> ClientState {
        *self.state.read().await
    }

    pub fn circuits(&self) -> &Arc<CircuitManager> {
        &self.circuits
    }

    pub async fn start(&self) -> NonosResult<()> {
        let current_state = self.state().await;
        if current_state != ClientState::Stopped {
            return Err(NonosError::Network(format!(
                "Cannot start from state {:?}",
                current_state
            )));
        }

        info!("Starting Anyone network client");
        *self.state.write().await = ClientState::Starting;
        self.circuits.set_status(ConnectionStatus::Connecting).await;

        let anon_path = self.find_anon_binary().await?;
        info!("Found anon binary at: {:?}", anon_path);

        let data_dir = &self.config.data_dir;
        if !data_dir.exists() {
            tokio::fs::create_dir_all(data_dir).await.map_err(|e| {
                NonosError::Config(format!("Failed to create data dir: {}", e))
            })?;
        }

        let anonrc_path = data_dir.join("anonrc");
        self.write_anonrc(&anonrc_path).await?;

        *self.state.write().await = ClientState::Bootstrapping;
        self.circuits.set_status(ConnectionStatus::Bootstrapping).await;

        info!("Launching anon binary - connecting to Anyone Network...");
        let mut child = Command::new(&anon_path)
            .arg("-f")
            .arg(&anonrc_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| NonosError::Network(format!("Failed to launch anon: {}", e)))?;

        if let Some(stderr) = child.stderr.take() {
            let state = self.state.clone();
            let circuits = self.circuits.clone();
            let error_msg = self.error_message.clone();

            tokio::spawn(async move {
                let reader = BufReader::new(stderr);
                let mut lines = reader.lines();

                while let Ok(Some(line)) = lines.next_line().await {
                    debug!("anon: {}", line);

                    if line.contains("Bootstrapped") {
                        if let Some(pct) = parse_bootstrap_progress(&line) {
                            circuits.set_bootstrap_progress(pct).await;

                            if pct >= 100 {
                                info!("Anyone Network bootstrap complete!");
                                *state.write().await = ClientState::Ready;
                                circuits.set_status(ConnectionStatus::Connected).await;
                            }
                        }
                    }

                    if line.contains("[err]") || line.contains("[warn]") {
                        warn!("anon warning: {}", line);
                    }

                    if line.contains("Failed") || line.contains("fatal") {
                        error!("anon error: {}", line);
                        *error_msg.write().await = Some(line.clone());
                        *state.write().await = ClientState::Error;
                        circuits.set_status(ConnectionStatus::Error).await;
                    }
                }
            });
        }

        *self.anon_process.write().await = Some(child);

        let timeout = tokio::time::Duration::from_secs(120);
        let start = tokio::time::Instant::now();

        loop {
            let state = self.state().await;
            match state {
                ClientState::Ready => {
                    info!("Anyone network client ready - connected to Anyone Network");
                    break;
                }
                ClientState::Error => {
                    let msg = self.error_message.read().await.clone().unwrap_or_default();
                    return Err(NonosError::Network(format!("Bootstrap failed: {}", msg)));
                }
                _ => {}
            }

            if start.elapsed() > timeout {
                self.stop().await?;
                return Err(NonosError::Network("Bootstrap timeout".into()));
            }

            if start.elapsed() > tokio::time::Duration::from_secs(5) {
                let socks_addr = *self.socks_addr.read().await;
                if TcpStream::connect(socks_addr).await.is_ok() {
                    info!("SOCKS5 proxy is ready at {}", socks_addr);
                    *self.state.write().await = ClientState::Ready;
                    self.circuits.set_status(ConnectionStatus::Connected).await;
                    self.circuits.set_bootstrap_progress(100).await;
                    break;
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }

        let _default = self.circuits.default_circuit().await?;

        Ok(())
    }

    async fn find_anon_binary(&self) -> NonosResult<PathBuf> {
        let candidates = vec![
            self.config.data_dir.join("anon"),
            PathBuf::from("/usr/bin/anon"),
            PathBuf::from("/usr/local/bin/anon"),
            PathBuf::from("/opt/anon/bin/anon"),
            PathBuf::from("/opt/homebrew/bin/anon"),
            dirs::home_dir().map(|h| h.join(".local/bin/anon")).unwrap_or_default(),
            PathBuf::from("./anon"),
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|p| p.join("anon")))
                .unwrap_or_default(),
        ];

        for path in candidates {
            if path.exists() {
                if let Ok(metadata) = tokio::fs::metadata(&path).await {
                    if metadata.is_file() {
                        return Ok(path);
                    }
                }
            }
        }

        if let Ok(output) = Command::new("which").arg("anon").output().await {
            if output.status.success() {
                let path_str = String::from_utf8_lossy(&output.stdout);
                let path = PathBuf::from(path_str.trim());
                if path.exists() {
                    return Ok(path);
                }
            }
        }

        Err(NonosError::Config(
            "anon binary not found. Install Anyone Protocol client from https://github.com/anyone-protocol/anon-install".into()
        ))
    }

    async fn write_anonrc(&self, path: &PathBuf) -> NonosResult<()> {
        let socks_port = self.config.socks_port;
        let control_port = *self.control_port.read().await;
        let data_dir = &self.config.data_dir;

        let mut config_lines = vec![
            format!("SocksPort {}", socks_port),
            format!("ControlPort {}", control_port),
            format!("DataDirectory {}", data_dir.display()),
            "Log notice stderr".to_string(),
            "SafeLogging 1".to_string(),
            "AvoidDiskWrites 1".to_string(),
        ];

        match self.security {
            SecurityPreset::Standard => {
                config_lines.push("CircuitBuildTimeout 60".to_string());
            }
            SecurityPreset::Enhanced => {
                config_lines.push("CircuitBuildTimeout 90".to_string());
                config_lines.push("NumEntryGuards 3".to_string());
            }
            SecurityPreset::Maximum => {
                config_lines.push("CircuitBuildTimeout 120".to_string());
                config_lines.push("NumEntryGuards 1".to_string());
                config_lines.push("StrictNodes 1".to_string());
            }
        }

        if self.config.use_bridges && !self.config.bridges.is_empty() {
            config_lines.push("UseBridges 1".to_string());
            for bridge in &self.config.bridges {
                config_lines.push(format!("Bridge {}", bridge));
            }
        }

        let config_content = config_lines.join("\n") + "\n";

        tokio::fs::write(path, config_content).await.map_err(|e| {
            NonosError::Config(format!("Failed to write anonrc: {}", e))
        })?;

        info!("Wrote anonrc to {:?}", path);
        Ok(())
    }

    pub async fn connect(&self, target: &str, port: u16) -> NonosResult<TcpStream> {
        if self.state().await != ClientState::Ready {
            return Err(NonosError::Network("Client not ready".into()));
        }

        let socks_addr = *self.socks_addr.read().await;

        info!("Connecting to {}:{} through Anyone Network...", target, port);

        let mut stream = TcpStream::connect(socks_addr).await.map_err(|e| {
            NonosError::Network(format!("Failed to connect to SOCKS5 proxy: {}", e))
        })?;

        stream.write_all(&[0x05, 0x01, 0x00]).await.map_err(|e| {
            NonosError::Network(format!("SOCKS5 handshake failed: {}", e))
        })?;

        let mut response = [0u8; 2];
        stream.read_exact(&mut response).await.map_err(|e| {
            NonosError::Network(format!("SOCKS5 response failed: {}", e))
        })?;

        if response[0] != 0x05 || response[1] != 0x00 {
            return Err(NonosError::Network("SOCKS5 authentication failed".into()));
        }

        let mut request = vec![
            0x05,
            0x01,
            0x00,
            0x03,
            target.len() as u8,
        ];
        request.extend_from_slice(target.as_bytes());
        request.extend_from_slice(&port.to_be_bytes());

        stream.write_all(&request).await.map_err(|e| {
            NonosError::Network(format!("SOCKS5 connect request failed: {}", e))
        })?;

        let mut response = [0u8; 10];
        stream.read_exact(&mut response).await.map_err(|e| {
            NonosError::Network(format!("SOCKS5 connect response failed: {}", e))
        })?;

        if response[1] != 0x00 {
            let error = match response[1] {
                0x01 => "General SOCKS server failure",
                0x02 => "Connection not allowed by ruleset",
                0x03 => "Network unreachable",
                0x04 => "Host unreachable",
                0x05 => "Connection refused",
                0x06 => "TTL expired",
                0x07 => "Command not supported",
                0x08 => "Address type not supported",
                _ => "Unknown error",
            };
            return Err(NonosError::Network(format!("SOCKS5 connect failed: {}", error)));
        }

        info!("Connected to {}:{} through Anyone Network", target, port);
        Ok(stream)
    }

    pub async fn connect_isolated(
        &self,
        target: &str,
        port: u16,
        isolation_id: &str,
    ) -> NonosResult<TcpStream> {
        if self.state().await != ClientState::Ready {
            return Err(NonosError::Network("Client not ready".into()));
        }

        let socks_addr = *self.socks_addr.read().await;

        info!("Connecting to {}:{} through Anyone Network (isolated: {})...", target, port, isolation_id);

        let mut stream = TcpStream::connect(socks_addr).await.map_err(|e| {
            NonosError::Network(format!("Failed to connect to SOCKS5 proxy: {}", e))
        })?;

        stream.write_all(&[0x05, 0x02, 0x00, 0x02]).await.map_err(|e| {
            NonosError::Network(format!("SOCKS5 handshake failed: {}", e))
        })?;

        let mut response = [0u8; 2];
        stream.read_exact(&mut response).await.map_err(|e| {
            NonosError::Network(format!("SOCKS5 response failed: {}", e))
        })?;

        if response[0] != 0x05 {
            return Err(NonosError::Network("Invalid SOCKS5 version".into()));
        }

        if response[1] == 0x02 {
            let user = isolation_id.as_bytes();
            let pass = b"x";

            let mut auth = vec![0x01];
            auth.push(user.len() as u8);
            auth.extend_from_slice(user);
            auth.push(pass.len() as u8);
            auth.extend_from_slice(pass);

            stream.write_all(&auth).await.map_err(|e| {
                NonosError::Network(format!("SOCKS5 auth failed: {}", e))
            })?;

            let mut auth_response = [0u8; 2];
            stream.read_exact(&mut auth_response).await.map_err(|e| {
                NonosError::Network(format!("SOCKS5 auth response failed: {}", e))
            })?;

            if auth_response[1] != 0x00 {
                return Err(NonosError::Network("SOCKS5 authentication failed".into()));
            }
        }

        let mut request = vec![
            0x05,
            0x01,
            0x00,
            0x03,
            target.len() as u8,
        ];
        request.extend_from_slice(target.as_bytes());
        request.extend_from_slice(&port.to_be_bytes());

        stream.write_all(&request).await.map_err(|e| {
            NonosError::Network(format!("SOCKS5 connect request failed: {}", e))
        })?;

        let mut response = [0u8; 10];
        stream.read_exact(&mut response).await.map_err(|e| {
            NonosError::Network(format!("SOCKS5 connect response failed: {}", e))
        })?;

        if response[1] != 0x00 {
            return Err(NonosError::Network("SOCKS5 connect failed".into()));
        }

        info!("Connected to {}:{} through Anyone Network (isolated)", target, port);
        Ok(stream)
    }

    pub async fn stop(&self) -> NonosResult<()> {
        let current_state = self.state().await;
        if current_state == ClientState::Stopped {
            return Ok(());
        }

        info!("Stopping Anyone network client");
        *self.state.write().await = ClientState::Stopping;

        self.circuits.new_identity().await?;

        if let Some(mut child) = self.anon_process.write().await.take() {
            info!("Terminating anon process");
            let _ = child.kill().await;
        }

        self.circuits.set_status(ConnectionStatus::Disconnected).await;
        *self.state.write().await = ClientState::Stopped;

        info!("Anyone network client stopped");
        Ok(())
    }

    pub async fn restart(&self) -> NonosResult<()> {
        self.stop().await?;
        self.start().await
    }

    pub async fn network_status(&self) -> NetworkStatus {
        self.circuits.network_status().await
    }

    pub async fn new_identity(&self) -> NonosResult<()> {
        if self.state().await != ClientState::Ready {
            return Err(NonosError::Network("Client not ready".into()));
        }

        info!("Requesting new identity");

        let control_port = *self.control_port.read().await;
        let control_addr = SocketAddr::from(([127, 0, 0, 1], control_port));

        if let Ok(mut stream) = TcpStream::connect(control_addr).await {
            let _ = stream.write_all(b"AUTHENTICATE\r\n").await;
            let _ = stream.write_all(b"SIGNAL NEWNYM\r\n").await;
            let _ = stream.write_all(b"QUIT\r\n").await;
            info!("Sent NEWNYM signal to control port");
        } else {
            warn!("Could not connect to control port, rotating circuits locally");
        }

        self.circuits.new_identity().await?;

        let _default = self.circuits.default_circuit().await?;

        Ok(())
    }

    pub async fn get_circuit(&self, domain: &str) -> NonosResult<CircuitId> {
        if self.state().await != ClientState::Ready {
            return Err(NonosError::Network("Client not ready".into()));
        }

        self.circuits.get_circuit_for_domain(domain).await
    }

    pub async fn is_connected(&self) -> bool {
        self.state().await == ClientState::Ready
    }

    pub fn socks_port(&self) -> u16 {
        self.config.socks_port
    }

    pub fn socks_address(&self) -> String {
        format!("127.0.0.1:{}", self.config.socks_port)
    }

    pub fn set_security(&mut self, preset: SecurityPreset) {
        self.security = preset;
    }

    pub fn config(&self) -> &AnyoneConfig {
        &self.config
    }

    pub async fn maintenance(&self) -> NonosResult<()> {
        if self.state().await != ClientState::Ready {
            return Ok(());
        }

        self.circuits
            .cleanup_stale_circuits(self.config.circuit_rotation_secs as i64)
            .await;

        let mut process_guard = self.anon_process.write().await;
        if let Some(ref mut child) = *process_guard {
            match child.try_wait() {
                Ok(Some(status)) => {
                    warn!("anon process exited with status: {:?}", status);
                    *self.state.write().await = ClientState::Error;
                    *self.error_message.write().await = Some("anon process terminated unexpectedly".into());
                    self.circuits.set_status(ConnectionStatus::Error).await;
                }
                Ok(None) => {
                }
                Err(e) => {
                    error!("Failed to check anon process status: {}", e);
                }
            }
        }

        Ok(())
    }
}

impl Default for AnyoneClient {
    fn default() -> Self {
        Self::new()
    }
}

fn parse_bootstrap_progress(line: &str) -> Option<u8> {
    if let Some(start) = line.find("Bootstrapped ") {
        let rest = &line[start + 13..];
        if let Some(end) = rest.find('%') {
            if let Ok(pct) = rest[..end].trim().parse::<u8>() {
                return Some(pct);
            }
        }
    }
    None
}

pub struct AnyoneClientBuilder {
    config: AnyoneConfig,
    security: SecurityPreset,
}

impl AnyoneClientBuilder {
    pub fn new() -> Self {
        Self {
            config: AnyoneConfig::default(),
            security: SecurityPreset::default(),
        }
    }

    pub fn data_dir(mut self, dir: impl Into<std::path::PathBuf>) -> Self {
        self.config.data_dir = dir.into();
        self
    }

    pub fn socks_port(mut self, port: u16) -> Self {
        self.config.socks_port = port;
        self
    }

    pub fn security(mut self, preset: SecurityPreset) -> Self {
        self.security = preset;
        self.config.circuit_length = preset.circuit_length();
        self.config.circuit_rotation_secs = preset.rotation_interval();
        self
    }

    pub fn with_bridges(mut self, bridges: Vec<String>) -> Self {
        self.config.use_bridges = true;
        self.config.bridges = bridges;
        self
    }

    pub fn build(self) -> NonosResult<AnyoneClient> {
        self.config.validate().map_err(NonosError::Config)?;

        let mut client = AnyoneClient::with_config(self.config);
        client.security = self.security;
        Ok(client)
    }
}

impl Default for AnyoneClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bootstrap_progress() {
        assert_eq!(parse_bootstrap_progress("Bootstrapped 50%: Loading relay descriptors"), Some(50));
        assert_eq!(parse_bootstrap_progress("Bootstrapped 100%: Done"), Some(100));
        assert_eq!(parse_bootstrap_progress("Bootstrapped 0%: Starting"), Some(0));
        assert_eq!(parse_bootstrap_progress("Some other log line"), None);
    }

    #[tokio::test]
    async fn test_client_creation() {
        let client = AnyoneClient::new();
        assert_eq!(client.state().await, ClientState::Stopped);
        assert_eq!(client.socks_port(), 9150);
    }

    #[tokio::test]
    async fn test_builder() {
        let client = AnyoneClientBuilder::new()
            .socks_port(9151)
            .security(SecurityPreset::Maximum)
            .build()
            .unwrap();

        assert_eq!(client.socks_port(), 9151);
    }
}
