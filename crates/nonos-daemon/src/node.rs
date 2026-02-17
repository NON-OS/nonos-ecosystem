use crate::{
    NodeConfig, NodeStorage, P2pNetwork, RewardTracker, NodeMetricsCollector,
    ServiceManager, ServiceConfig, PrivacyServiceManager, StorageConfig, ProxiedHttpClient,
};
use nonos_anyone::{AnyoneClient, AnyoneClientBuilder, SecurityPreset};
use nonos_crypto::NodeIdentity;
use nonos_types::{
    NodeId, NodeMetrics, NodeStatus, NodeTier, NonosError, NonosResult,
};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::info;

#[cfg(unix)]
use nix::libc;

pub struct Node {
    identity: NodeIdentity,
    config: NodeConfig,
    status: Arc<RwLock<NodeStatus>>,
    network: Option<Arc<RwLock<P2pNetwork>>>,
    metrics: Arc<NodeMetricsCollector>,
    rewards: Arc<RewardTracker>,
    storage: Option<Arc<NodeStorage>>,
    services: Option<Arc<RwLock<ServiceManager>>>,
    privacy: Option<Arc<PrivacyServiceManager>>,
    anyone: Option<Arc<AnyoneClient>>,
    http_client: Arc<ProxiedHttpClient>,
    start_time: Option<chrono::DateTime<chrono::Utc>>,
}

impl Node {
    pub fn new(config: NodeConfig) -> NonosResult<Self> {
        let identity = NodeIdentity::generate();

        Ok(Self {
            identity,
            config,
            status: Arc::new(RwLock::new(NodeStatus::Stopped)),
            network: None,
            metrics: Arc::new(NodeMetricsCollector::new()),
            rewards: Arc::new(RewardTracker::new()),
            storage: None,
            services: None,
            privacy: None,
            anyone: None,
            http_client: Arc::new(ProxiedHttpClient::new()),
            start_time: None,
        })
    }

    pub fn with_identity(config: NodeConfig, identity: NodeIdentity) -> Self {
        Self {
            identity,
            config,
            status: Arc::new(RwLock::new(NodeStatus::Stopped)),
            network: None,
            metrics: Arc::new(NodeMetricsCollector::new()),
            rewards: Arc::new(RewardTracker::new()),
            storage: None,
            services: None,
            privacy: None,
            anyone: None,
            http_client: Arc::new(ProxiedHttpClient::new()),
            start_time: None,
        }
    }

    pub fn id(&self) -> NodeId {
        self.identity.node_id()
    }

    pub async fn status(&self) -> NodeStatus {
        *self.status.read().await
    }

    pub async fn start(&mut self) -> NonosResult<()> {
        let current_status = *self.status.read().await;
        if current_status != NodeStatus::Stopped {
            return Err(NonosError::Internal(format!(
                "Cannot start from state {:?}",
                current_status
            )));
        }

        info!("Starting NONOS node: {}", self.id());
        *self.status.write().await = NodeStatus::Starting;

        self.config.validate()?;

        let data_dir = &self.config.data_dir;
        if !data_dir.exists() {
            std::fs::create_dir_all(data_dir)
                .map_err(|e| NonosError::Storage(format!("Failed to create data dir: {}", e)))?;
        }

        let storage_config = StorageConfig {
            path: data_dir.join("db"),
            ..Default::default()
        };
        let storage = NodeStorage::open(storage_config)?;
        self.storage = Some(Arc::new(storage));

        let identity_path = data_dir.join("p2p_identity.key");
        let keypair = if identity_path.exists() {
            let key_bytes = std::fs::read(&identity_path)
                .map_err(|e| NonosError::Storage(format!("Failed to read P2P identity: {}", e)))?;
            libp2p::identity::Keypair::from_protobuf_encoding(&key_bytes)
                .map_err(|e| NonosError::Storage(format!("Invalid P2P identity: {}", e)))?
        } else {
            let keypair = libp2p::identity::Keypair::generate_ed25519();
            let key_bytes = keypair.to_protobuf_encoding()
                .map_err(|e| NonosError::Storage(format!("Failed to encode P2P identity: {}", e)))?;
            std::fs::write(&identity_path, &key_bytes)
                .map_err(|e| NonosError::Storage(format!("Failed to save P2P identity: {}", e)))?;
            info!("Generated new P2P identity: {}", keypair.public().to_peer_id());
            keypair
        };

        let mut network = P2pNetwork::with_keypair(keypair, self.config.port, self.config.max_connections);
        network.start().await?;
        let network_arc = Arc::new(RwLock::new(network));
        self.network = Some(network_arc.clone());

        let privacy = PrivacyServiceManager::new(self.id());
        privacy.start_all().await?;
        self.privacy = Some(Arc::new(privacy));

        if self.config.anyone.enabled {
            self.start_anyone_network().await?;
        }

        self.start_services(network_arc).await?;

        self.start_time = Some(chrono::Utc::now());
        *self.status.write().await = NodeStatus::Running;

        info!("NONOS node started successfully");
        Ok(())
    }

    async fn start_services(&mut self, network: Arc<RwLock<P2pNetwork>>) -> NonosResult<()> {
        let storage = self.storage.as_ref()
            .ok_or_else(|| NonosError::Internal("Storage not initialized".into()))?
            .clone();

        let service_config = ServiceConfig {
            health_beacon: self.config.services.health_beacon,
            quality_oracle: self.config.services.quality_oracle,
            bootstrap: self.config.services.bootstrap,
            cache: self.config.services.cache,
            bootstrap_port: 9735,
            cache_size_mb: self.config.services.cache_size_mb,
            beacon_interval_secs: 60,
            quality_interval_secs: 300,
        };

        let mut manager = ServiceManager::new();
        manager.start_all(
            self.id(),
            network,
            self.metrics.clone(),
            storage,
            service_config,
        ).await?;

        self.services = Some(Arc::new(RwLock::new(manager)));

        info!("All node services started");
        Ok(())
    }

    async fn start_anyone_network(&mut self) -> NonosResult<()> {
        info!("Starting Anyone Network client for anonymous traffic routing");

        let security = match self.config.anyone.security_level {
            crate::config::SecurityLevel::Standard => SecurityPreset::Standard,
            crate::config::SecurityLevel::Enhanced => SecurityPreset::Enhanced,
            crate::config::SecurityLevel::Maximum => SecurityPreset::Maximum,
        };

        let anyone_config = self.config.anyone.to_anyone_config(&self.config.data_dir);

        let client = AnyoneClientBuilder::new()
            .data_dir(anyone_config.data_dir)
            .socks_port(self.config.anyone.socks_port)
            .security(security)
            .build()
            .map_err(|e| NonosError::Network(format!("Failed to build Anyone client: {}", e)))?;

        if self.config.anyone.auto_start {
            client.start().await.map_err(|e| {
                NonosError::Network(format!("Failed to start Anyone Network: {}", e))
            })?;

            self.http_client
                .configure_proxy(self.config.anyone.socks_port)
                .await?;

            info!(
                "Anyone Network connected - all outbound traffic routed through SOCKS5 port {}",
                self.config.anyone.socks_port
            );
        }

        self.anyone = Some(Arc::new(client));
        Ok(())
    }

    pub async fn stop(&mut self) -> NonosResult<()> {
        let current_status = *self.status.read().await;
        if current_status == NodeStatus::Stopped {
            return Ok(());
        }

        info!("Stopping NONOS node");
        *self.status.write().await = NodeStatus::Stopped;

        if let Some(ref services) = self.services {
            services.write().await.stop_all().await;
        }
        self.services = None;

        if let Some(ref privacy) = self.privacy {
            privacy.stop_all();
        }
        self.privacy = None;

        if let Some(ref anyone) = self.anyone {
            let _ = anyone.stop().await;
        }
        self.anyone = None;
        self.http_client.disable_proxy().await;

        if let Some(ref network) = self.network {
            network.write().await.shutdown().await;
        }
        self.network = None;

        if let Some(ref storage) = self.storage {
            let _ = storage.flush();
        }

        self.metrics.flush();

        info!("NONOS node stopped");
        Ok(())
    }

    pub fn uptime_secs(&self) -> u64 {
        self.start_time
            .map(|t| chrono::Utc::now().signed_duration_since(t).num_seconds() as u64)
            .unwrap_or(0)
    }

    pub async fn metrics(&self) -> NodeMetrics {
        let quality = self.metrics.quality_score();
        let staked = self.rewards.staked_amount().await;
        let pending = self.rewards.pending_rewards().await;
        let tier = NodeTier::from_stake(staked.raw as u64 / 10u64.pow(18));

        NodeMetrics {
            node_id: self.id(),
            status: *self.status.read().await,
            tier,
            quality,
            staked,
            pending_rewards: pending,
            streak: self.rewards.current_streak().await,
            uptime_secs: self.uptime_secs(),
            active_connections: if let Some(ref network) = self.network {
                network.read().await.connection_count()
            } else {
                0
            },
            total_requests: self.metrics.total_requests(),
            successful_requests: self.metrics.successful_requests(),
        }
    }

    pub fn config(&self) -> &NodeConfig {
        &self.config
    }

    pub fn network(&self) -> Option<Arc<RwLock<P2pNetwork>>> {
        self.network.clone()
    }

    pub fn metrics_collector(&self) -> Arc<NodeMetricsCollector> {
        self.metrics.clone()
    }

    pub fn storage(&self) -> Option<Arc<NodeStorage>> {
        self.storage.clone()
    }

    pub fn privacy(&self) -> Option<Arc<PrivacyServiceManager>> {
        self.privacy.clone()
    }

    pub fn services(&self) -> Option<Arc<RwLock<ServiceManager>>> {
        self.services.clone()
    }

    pub fn anyone(&self) -> Option<Arc<AnyoneClient>> {
        self.anyone.clone()
    }

    pub fn http_client(&self) -> Arc<ProxiedHttpClient> {
        self.http_client.clone()
    }

    pub async fn is_anyone_connected(&self) -> bool {
        if let Some(ref anyone) = self.anyone {
            anyone.is_connected().await
        } else {
            false
        }
    }

    pub async fn diagnose(&self) -> DiagnosticReport {
        let mut report = DiagnosticReport::new();

        report.add_check("Port reachability", self.check_port_reachable().await);
        report.add_check("Peer connectivity", self.check_peer_connectivity().await);
        report.add_check("Disk space", self.check_disk_space());
        report.add_check("Memory usage", self.check_memory_usage());
        report.add_check("Clock sync", self.check_time_sync().await);
        report.add_check("Network latency", self.check_network_latency().await);

        report
    }

    async fn check_port_reachable(&self) -> CheckResult {
        let port = self.config.port;
        let addr = format!("127.0.0.1:{}", port);

        match TcpStream::connect_timeout(
            &addr.parse::<SocketAddr>().unwrap(),
            Duration::from_secs(2),
        ) {
            Ok(_) => CheckResult::Pass(format!("Port {} is listening", port)),
            Err(_) => {
                let external_check = tokio::task::spawn_blocking(move || {
                    if let Ok(addrs) = format!("boot1.nonos.systems:9432").to_socket_addrs() {
                        for addr in addrs {
                            if TcpStream::connect_timeout(&addr, Duration::from_secs(5)).is_ok() {
                                return true;
                            }
                        }
                    }
                    false
                }).await.unwrap_or(false);

                if external_check {
                    CheckResult::Warn(format!("Port {} may be firewalled", port))
                } else {
                    CheckResult::Fail(format!("Port {} unreachable", port))
                }
            }
        }
    }

    async fn check_peer_connectivity(&self) -> CheckResult {
        if let Some(ref network) = self.network {
            let peers = network.read().await.peer_count();
            if peers >= 5 {
                CheckResult::Pass(format!("Connected to {} peers (healthy)", peers))
            } else if peers > 0 {
                CheckResult::Warn(format!("Connected to {} peers (low)", peers))
            } else {
                CheckResult::Fail("No peers connected".into())
            }
        } else {
            CheckResult::Fail("Network not initialized".into())
        }
    }

    fn check_disk_space(&self) -> CheckResult {
        let data_dir = &self.config.data_dir;

        #[cfg(unix)]
        {
            use std::ffi::CString;
            use std::mem::MaybeUninit;

            let path_cstr = match CString::new(data_dir.to_string_lossy().as_bytes()) {
                Ok(s) => s,
                Err(_) => return CheckResult::Warn("Could not check disk space".into()),
            };

            unsafe {
                let mut stat: MaybeUninit<libc::statvfs> = MaybeUninit::uninit();
                if libc::statvfs(path_cstr.as_ptr(), stat.as_mut_ptr()) == 0 {
                    let stat = stat.assume_init();
                    let available_gb = (stat.f_bavail as u64 * stat.f_frsize as u64) / (1024 * 1024 * 1024);
                    let total_gb = (stat.f_blocks as u64 * stat.f_frsize as u64) / (1024 * 1024 * 1024);
                    let used_percent = 100 - (available_gb * 100 / total_gb.max(1));

                    if available_gb >= 10 {
                        CheckResult::Pass(format!("{}GB available ({}% used)", available_gb, used_percent))
                    } else if available_gb >= 2 {
                        CheckResult::Warn(format!("Low disk: {}GB available", available_gb))
                    } else {
                        CheckResult::Fail(format!("Critical: {}GB available", available_gb))
                    }
                } else {
                    CheckResult::Warn("Could not check disk space".into())
                }
            }
        }

        #[cfg(not(unix))]
        {
            if data_dir.exists() {
                CheckResult::Pass("Disk accessible".into())
            } else {
                CheckResult::Fail("Data directory not accessible".into())
            }
        }
    }

    fn check_memory_usage(&self) -> CheckResult {
        #[cfg(unix)]
        {
            if let Ok(contents) = std::fs::read_to_string("/proc/meminfo") {
                let mut total_kb = 0u64;
                let mut available_kb = 0u64;

                for line in contents.lines() {
                    if line.starts_with("MemTotal:") {
                        total_kb = line.split_whitespace().nth(1)
                            .and_then(|s| s.parse().ok())
                            .unwrap_or(0);
                    } else if line.starts_with("MemAvailable:") {
                        available_kb = line.split_whitespace().nth(1)
                            .and_then(|s| s.parse().ok())
                            .unwrap_or(0);
                    }
                }

                if total_kb > 0 {
                    let used_percent = 100 - (available_kb * 100 / total_kb);
                    let available_mb = available_kb / 1024;

                    if used_percent < 80 {
                        CheckResult::Pass(format!("{}MB available ({}% used)", available_mb, used_percent))
                    } else if used_percent < 95 {
                        CheckResult::Warn(format!("High memory: {}% used", used_percent))
                    } else {
                        CheckResult::Fail(format!("Critical: {}% memory used", used_percent))
                    }
                } else {
                    CheckResult::Warn("Could not parse memory info".into())
                }
            } else {
                CheckResult::Warn("Could not read memory info".into())
            }
        }

        #[cfg(not(unix))]
        {
            CheckResult::Pass("Memory check not available on this platform".into())
        }
    }

    async fn check_time_sync(&self) -> CheckResult {
        let ntp_servers = [
            "time.google.com:123",
            "pool.ntp.org:123",
            "time.cloudflare.com:123",
        ];

        let local_time = chrono::Utc::now().timestamp();

        for server in ntp_servers {
            if let Ok(addrs) = server.to_socket_addrs() {
                for addr in addrs {
                    if TcpStream::connect_timeout(&addr, Duration::from_secs(2)).is_ok() {
                        return CheckResult::Pass(format!("NTP reachable, local time: {}", local_time));
                    }
                }
            }
        }

        let system_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let drift = (local_time - system_time).abs();
        if drift < 60 {
            CheckResult::Warn(format!("NTP unreachable, drift: {}s", drift))
        } else {
            CheckResult::Fail(format!("Clock drift too high: {}s", drift))
        }
    }

    async fn check_network_latency(&self) -> CheckResult {
        if let Some(ref network) = self.network {
            let peers = network.read().await.peers();
            if peers.is_empty() {
                return CheckResult::Warn("No peers to measure latency".into());
            }

            let latencies: Vec<u32> = peers.iter()
                .filter_map(|p| p.latency_ms)
                .collect();

            if latencies.is_empty() {
                return CheckResult::Warn("No latency data available".into());
            }

            let avg = latencies.iter().sum::<u32>() / latencies.len() as u32;
            let max = *latencies.iter().max().unwrap_or(&0);

            if avg < 100 {
                CheckResult::Pass(format!("Avg latency: {}ms, max: {}ms", avg, max))
            } else if avg < 500 {
                CheckResult::Warn(format!("High latency: avg {}ms", avg))
            } else {
                CheckResult::Fail(format!("Very high latency: {}ms", avg))
            }
        } else {
            CheckResult::Fail("Network not initialized".into())
        }
    }
}

#[derive(Clone, Debug)]
pub enum CheckResult {
    Pass(String),
    Warn(String),
    Fail(String),
}

impl CheckResult {
    pub fn is_pass(&self) -> bool {
        matches!(self, CheckResult::Pass(_))
    }

    pub fn is_fail(&self) -> bool {
        matches!(self, CheckResult::Fail(_))
    }

    pub fn message(&self) -> &str {
        match self {
            CheckResult::Pass(m) | CheckResult::Warn(m) | CheckResult::Fail(m) => m,
        }
    }
}

#[derive(Clone, Debug)]
pub struct DiagnosticReport {
    checks: Vec<(String, CheckResult)>,
    timestamp: chrono::DateTime<chrono::Utc>,
}

impl DiagnosticReport {
    pub fn new() -> Self {
        Self {
            checks: Vec::new(),
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn add_check(&mut self, name: &str, result: CheckResult) {
        self.checks.push((name.to_string(), result));
    }

    pub fn checks(&self) -> &[(String, CheckResult)] {
        &self.checks
    }

    pub fn timestamp(&self) -> chrono::DateTime<chrono::Utc> {
        self.timestamp
    }

    pub fn all_passed(&self) -> bool {
        self.checks.iter().all(|(_, r)| matches!(r, CheckResult::Pass(_)))
    }

    pub fn has_failures(&self) -> bool {
        self.checks.iter().any(|(_, r)| matches!(r, CheckResult::Fail(_)))
    }

    pub fn failures(&self) -> Vec<&(String, CheckResult)> {
        self.checks
            .iter()
            .filter(|(_, r)| matches!(r, CheckResult::Fail(_)))
            .collect()
    }

    pub fn warnings(&self) -> Vec<&(String, CheckResult)> {
        self.checks
            .iter()
            .filter(|(_, r)| matches!(r, CheckResult::Warn(_)))
            .collect()
    }

    pub fn summary(&self) -> String {
        let passed = self.checks.iter().filter(|(_, r)| r.is_pass()).count();
        let failed = self.failures().len();
        let warnings = self.warnings().len();
        format!("{} passed, {} warnings, {} failed", passed, warnings, failed)
    }
}

impl Default for DiagnosticReport {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for DiagnosticReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "NONOS Node Diagnostics")?;
        writeln!(f, "======================")?;
        writeln!(f, "Time: {}", self.timestamp.format("%Y-%m-%d %H:%M:%S UTC"))?;
        writeln!(f)?;

        for (name, result) in &self.checks {
            let symbol = match result {
                CheckResult::Pass(_) => "[OK]",
                CheckResult::Warn(_) => "[!!]",
                CheckResult::Fail(_) => "[XX]",
            };
            writeln!(f, "{} {}: {}", symbol, name, result.message())?;
        }

        writeln!(f)?;
        writeln!(f, "Summary: {}", self.summary())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_node_creation() {
        let config = NodeConfig::default();
        let node = Node::new(config).unwrap();
        assert_eq!(node.status().await, NodeStatus::Stopped);
    }

    #[test]
    fn test_diagnostic_report() {
        let mut report = DiagnosticReport::new();
        report.add_check("Test 1", CheckResult::Pass("OK".into()));
        report.add_check("Test 2", CheckResult::Warn("Warning".into()));
        report.add_check("Test 3", CheckResult::Fail("Failed".into()));

        assert!(!report.all_passed());
        assert!(report.has_failures());
        assert_eq!(report.failures().len(), 1);
        assert_eq!(report.warnings().len(), 1);
    }

    #[test]
    fn test_check_result() {
        let pass = CheckResult::Pass("OK".into());
        let fail = CheckResult::Fail("Error".into());

        assert!(pass.is_pass());
        assert!(!pass.is_fail());
        assert!(fail.is_fail());
        assert_eq!(pass.message(), "OK");
    }
}
