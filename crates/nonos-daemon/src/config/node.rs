use nonos_types::{NonosError, NonosResult};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use tracing::{info, warn};

use super::api::ApiConfig;
use super::constants::DEFAULT_P2P_PORT;
use super::logging::LoggingConfig;
use super::network::NetworkConfig;
use super::rate_limit::RateLimitConfig;
use super::rewards::RewardsConfig;
use super::security::SecurityConfig;
use super::services::ServicesConfig;
use super::types::{BootstrapMode, LogLevel, NodeRole, SecurityWarning, WarningSeverity};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct NodeConfig {
    pub role: NodeRole,
    pub data_dir: PathBuf,
    pub port: u16,
    pub max_connections: u32,
    pub bandwidth_limit: u64,
    pub services: ServicesConfig,
    pub network: NetworkConfig,
    pub rewards: RewardsConfig,
    pub security: SecurityConfig,
    pub logging: LoggingConfig,
    pub api: ApiConfig,
    pub rate_limits: RateLimitConfig,
}

impl Default for NodeConfig {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/var/lib/nonos"));
        let data_dir = home.join(".nonos");
        let role = NodeRole::default();

        Self {
            role,
            data_dir,
            port: DEFAULT_P2P_PORT,
            max_connections: role.max_peers(),
            bandwidth_limit: 0,
            services: ServicesConfig::default(),
            network: NetworkConfig::default(),
            rewards: RewardsConfig::default(),
            security: SecurityConfig::default(),
            logging: LoggingConfig::default(),
            api: ApiConfig::default(),
            rate_limits: RateLimitConfig::default(),
        }
    }
}

impl NodeConfig {
    pub fn load(path: impl AsRef<std::path::Path>) -> NonosResult<Self> {
        let path = path.as_ref();

        let mut config = if path.exists() {
            let contents = std::fs::read_to_string(path)
                .map_err(|e| NonosError::Config(format!("Failed to read config: {}", e)))?;

            toml::from_str(&contents)
                .map_err(|e| NonosError::Config(format!("Failed to parse config: {}", e)))?
        } else {
            info!("Config file not found, using defaults");
            Self::default()
        };

        config.apply_env_overrides();
        config.validate()?;

        Ok(config)
    }

    pub fn save(&self, path: impl AsRef<std::path::Path>) -> NonosResult<()> {
        let contents = toml::to_string_pretty(self)
            .map_err(|e| NonosError::Config(format!("Failed to serialize config: {}", e)))?;

        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| NonosError::Config(format!("Failed to create config dir: {}", e)))?;
        }

        std::fs::write(path.as_ref(), contents)
            .map_err(|e| NonosError::Config(format!("Failed to write config: {}", e)))?;

        info!("Configuration saved to {:?}", path.as_ref());
        Ok(())
    }

    fn apply_env_overrides(&mut self) {
        if let Ok(dir) = std::env::var("NONOS_DATA_DIR") {
            self.data_dir = PathBuf::from(dir);
        }

        if let Ok(port) = std::env::var("NONOS_P2P_PORT") {
            if let Ok(p) = port.parse() {
                self.port = p;
            }
        }

        if let Ok(port) = std::env::var("NONOS_API_PORT") {
            if let Ok(p) = port.parse() {
                self.api.port = p;
            }
        }

        if let Ok(bind) = std::env::var("NONOS_API_BIND") {
            if let Ok(addr) = bind.parse() {
                self.api.bind_address = addr;
                if bind != "127.0.0.1" && bind != "::1" {
                    warn!(
                        "API server binding to non-localhost address: {}. Ensure proper firewall rules.",
                        bind
                    );
                }
            }
        }

        if let Ok(rpc) = std::env::var("NONOS_RPC_URL") {
            self.rewards.rpc_url = Some(rpc);
        }

        if let Ok(level) = std::env::var("NONOS_LOG_LEVEL") {
            self.logging.level = match level.to_lowercase().as_str() {
                "error" => LogLevel::Error,
                "warn" => LogLevel::Warn,
                "info" => LogLevel::Info,
                "debug" => LogLevel::Debug,
                "trace" => LogLevel::Trace,
                _ => LogLevel::Info,
            };
        }

        if std::env::var("NONOS_LOG_JSON").is_ok() {
            self.logging.json = true;
        }

        if let Ok(token) = std::env::var("NONOS_API_TOKEN") {
            self.security.api_auth_token = Some(token);
            self.security.api_auth_required = true;
        }

        if let Ok(nodes) = std::env::var("NONOS_BOOTSTRAP_NODES") {
            self.network.custom_bootstrap_peers = nodes
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if !self.network.custom_bootstrap_peers.is_empty() {
                self.network.bootstrap_mode = BootstrapMode::Custom;
            }
        }

        if let Ok(mode) = std::env::var("NONOS_BOOTSTRAP_MODE") {
            self.network.bootstrap_mode = match mode.to_lowercase().as_str() {
                "official" => BootstrapMode::Official,
                "custom" => BootstrapMode::Custom,
                "none" => BootstrapMode::None,
                _ => BootstrapMode::Official,
            };
        }

        if let Ok(role) = std::env::var("NONOS_ROLE") {
            self.role = match role.to_lowercase().as_str() {
                "local" => NodeRole::Local,
                "relay" => NodeRole::Relay,
                "backbone" => NodeRole::Backbone,
                _ => NodeRole::Local,
            };
            if self.max_connections == NodeRole::Local.max_peers() {
                self.max_connections = self.role.max_peers();
            }
        }
    }

    pub fn validate(&self) -> NonosResult<()> {
        if self.port == 0 {
            return Err(NonosError::Config("P2P port cannot be 0".into()));
        }

        if self.api.enabled && self.api.port == 0 {
            return Err(NonosError::Config("API port cannot be 0".into()));
        }

        if self.port == self.api.port {
            return Err(NonosError::Config(
                "P2P and API ports cannot be the same".into(),
            ));
        }

        match self.network.bootstrap_mode {
            BootstrapMode::Custom => {
                if self.network.custom_bootstrap_peers.is_empty() {
                    return Err(NonosError::Config(
                        "Bootstrap mode is 'custom' but no custom_bootstrap_peers configured".into(),
                    ));
                }
                for peer in &self.network.custom_bootstrap_peers {
                    if !peer.starts_with("/ip4/") && !peer.starts_with("/ip6/") && !peer.starts_with("/dns") {
                        return Err(NonosError::Config(format!(
                            "Invalid multiaddress format: {}",
                            peer
                        )));
                    }
                }
            }
            BootstrapMode::None => {
                warn!("Bootstrap mode is 'none' - node will not connect to the network automatically");
            }
            BootstrapMode::Official => {}
        }

        if self.services.health_beacon_interval_secs < 10 {
            return Err(NonosError::Config(
                "Health beacon interval must be at least 10 seconds".into(),
            ));
        }

        if self.services.quality_oracle_interval_secs < 60 {
            return Err(NonosError::Config(
                "Quality oracle interval must be at least 60 seconds".into(),
            ));
        }

        if self.network.max_message_size < 1024 {
            return Err(NonosError::Config(
                "Max message size must be at least 1024 bytes".into(),
            ));
        }

        if self.api.max_body_size < 1024 {
            return Err(NonosError::Config(
                "Max body size must be at least 1024 bytes".into(),
            ));
        }

        if self.security.api_auth_required && self.security.api_auth_token.is_none() {
            warn!("API auth required but no token set. Set NONOS_API_TOKEN environment variable.");
        }

        Ok(())
    }

    pub fn check_security_warnings(&self) -> Vec<SecurityWarning> {
        let mut warnings = Vec::new();

        if !self.security.api_auth_required {
            warnings.push(SecurityWarning {
                severity: WarningSeverity::High,
                message: "API authentication is disabled. Anyone can access the API.".into(),
                recommendation: "Set api_auth_required = true and configure NONOS_API_TOKEN.".into(),
            });
        } else if self.security.api_auth_token.is_none() {
            warnings.push(SecurityWarning {
                severity: WarningSeverity::High,
                message: "API auth required but no token configured.".into(),
                recommendation: "Set NONOS_API_TOKEN environment variable or api_auth_token in config.".into(),
            });
        }

        if !self.api_is_localhost_only() {
            warnings.push(SecurityWarning {
                severity: WarningSeverity::Medium,
                message: format!("API server bound to non-localhost address: {}", self.api.bind_address),
                recommendation: "Ensure firewall rules restrict access. Use localhost binding if possible.".into(),
            });
        }

        if !self.rate_limits.enabled {
            warnings.push(SecurityWarning {
                severity: WarningSeverity::Medium,
                message: "Rate limiting is disabled.".into(),
                recommendation: "Enable rate limiting to prevent abuse: rate_limits.enabled = true".into(),
            });
        }

        if self.security.encrypted_storage {
            warnings.push(SecurityWarning {
                severity: WarningSeverity::Medium,
                message: "encrypted_storage is set but storage encryption is not fully implemented.".into(),
                recommendation: "Storage encryption requires additional setup. Secrets are stored unencrypted.".into(),
            });
        }

        if matches!(self.network.bootstrap_mode, BootstrapMode::None) {
            warnings.push(SecurityWarning {
                severity: WarningSeverity::Low,
                message: "Bootstrap mode is 'none'. Node will not auto-connect to network.".into(),
                recommendation: "Use 'official' or 'custom' bootstrap mode for normal operation.".into(),
            });
        }

        warnings
    }

    pub fn log_security_warnings(&self) {
        let warnings = self.check_security_warnings();
        if warnings.is_empty() {
            info!("Security check passed - no warnings");
            return;
        }

        for warning in &warnings {
            match warning.severity {
                WarningSeverity::High => {
                    warn!("SECURITY: {}", warning.message);
                    warn!("  -> {}", warning.recommendation);
                }
                WarningSeverity::Medium => {
                    warn!("{}", warning.message);
                    info!("  -> {}", warning.recommendation);
                }
                WarningSeverity::Low => {
                    info!("Note: {}", warning.message);
                }
            }
        }

        let high_count = warnings.iter().filter(|w| matches!(w.severity, WarningSeverity::High)).count();
        if high_count > 0 {
            warn!("{} high-severity security warning(s). Review configuration.", high_count);
        }
    }

    pub fn api_socket_addr(&self) -> SocketAddr {
        SocketAddr::new(self.api.bind_address, self.api.port)
    }

    pub fn p2p_listen_addr(&self) -> String {
        format!("/ip4/0.0.0.0/tcp/{}", self.port)
    }

    pub fn api_is_localhost_only(&self) -> bool {
        match self.api.bind_address {
            IpAddr::V4(addr) => addr.is_loopback(),
            IpAddr::V6(addr) => addr.is_loopback(),
        }
    }

    pub fn redacted(&self) -> RedactedConfig {
        RedactedConfig {
            role: self.role,
            data_dir: self.data_dir.clone(),
            port: self.port,
            max_connections: self.max_connections,
            api_port: self.api.port,
            api_bind: self.api.bind_address,
            bootstrap_mode: self.network.bootstrap_mode.clone(),
            bootstrap_peer_count: self.network.bootstrap_peers().len(),
            services_enabled: vec![
                ("health_beacon", self.services.health_beacon),
                ("quality_oracle", self.services.quality_oracle),
                ("bootstrap", self.services.bootstrap),
                ("cache", self.services.cache),
            ],
            api_auth_required: self.security.api_auth_required,
            rate_limiting: self.rate_limits.enabled,
        }
    }

    pub fn effective_max_peers(&self) -> u32 {
        if self.max_connections == 0 {
            self.role.max_peers()
        } else {
            self.max_connections
        }
    }
}

#[derive(Debug)]
pub struct RedactedConfig {
    pub role: NodeRole,
    pub data_dir: PathBuf,
    pub port: u16,
    pub max_connections: u32,
    pub api_port: u16,
    pub api_bind: IpAddr,
    pub bootstrap_mode: BootstrapMode,
    pub bootstrap_peer_count: usize,
    pub services_enabled: Vec<(&'static str, bool)>,
    pub api_auth_required: bool,
    pub rate_limiting: bool,
}

impl std::fmt::Display for RedactedConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "NONOS Node Configuration")?;
        writeln!(f, "========================")?;
        writeln!(f, "Role: {}", self.role)?;
        writeln!(f, "Data dir: {:?}", self.data_dir)?;
        writeln!(f, "P2P port: {}", self.port)?;
        writeln!(f, "Max connections: {}", self.max_connections)?;
        writeln!(f, "API: {}:{}", self.api_bind, self.api_port)?;
        writeln!(f, "Bootstrap: {} ({} peers)", self.bootstrap_mode, self.bootstrap_peer_count)?;
        writeln!(f, "Services:")?;
        for (name, enabled) in &self.services_enabled {
            writeln!(f, "  {}: {}", name, if *enabled { "ON" } else { "OFF" })?;
        }
        writeln!(f, "API auth: {}", self.api_auth_required)?;
        writeln!(f, "Rate limiting: {}", self.rate_limiting)?;
        Ok(())
    }
}
