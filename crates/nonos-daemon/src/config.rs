// NONOS GNU AFFERO GENERAL PUBLIC LICENSE
// Version 3, 19 November 2007
// Copyright (C) 2025 NON-OS <team@nonos.systems>

//! Configuration Module
//!
//! Provides production-grade configuration management with:
//! - Safe defaults (localhost-only API binding)
//! - Comprehensive validation
//! - Environment variable overrides
//! - TOML serialization
//! - Sensitive value redaction in logs

use nonos_types::{EthAddress, NonosError, NonosResult};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use tracing::{info, warn};

/// Default ports
pub const DEFAULT_P2P_PORT: u16 = 9432;
pub const DEFAULT_API_PORT: u16 = 8420;
pub const DEFAULT_BOOTSTRAP_PORT: u16 = 9735;

/// Default limits
pub const DEFAULT_MAX_CONNECTIONS: u32 = 1000;
pub const DEFAULT_MAX_MESSAGE_SIZE: usize = 65536;
pub const DEFAULT_RATE_LIMIT_RPS: u32 = 100;

/// Node role in the network
///
/// Defines how this node participates in the NONØS network.
/// Roles affect peer limits, service availability, and resource usage.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum NodeRole {
    /// Personal node for a single user
    /// - Lower peer limits
    /// - Optimized for privacy
    /// - Minimal resource usage
    #[default]
    Local,
    /// Community relay node
    /// - Higher peer limits
    /// - Serves other nodes
    /// - Medium resource usage
    Relay,
    /// High-availability backbone node
    /// - Maximum peer limits
    /// - Critical network infrastructure
    /// - High resource usage
    Backbone,
}

impl NodeRole {
    /// Get the recommended maximum peer count for this role
    pub fn max_peers(&self) -> u32 {
        match self {
            NodeRole::Local => 25,
            NodeRole::Relay => 100,
            NodeRole::Backbone => 500,
        }
    }

    /// Get the recommended minimum peer count for this role
    pub fn min_peers(&self) -> u32 {
        match self {
            NodeRole::Local => 3,
            NodeRole::Relay => 10,
            NodeRole::Backbone => 25,
        }
    }

    /// Check if this role should serve bootstrap requests
    pub fn serves_bootstrap(&self) -> bool {
        matches!(self, NodeRole::Relay | NodeRole::Backbone)
    }

    /// Check if this role should maintain high availability
    pub fn high_availability(&self) -> bool {
        matches!(self, NodeRole::Backbone)
    }
}

impl std::fmt::Display for NodeRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeRole::Local => write!(f, "local"),
            NodeRole::Relay => write!(f, "relay"),
            NodeRole::Backbone => write!(f, "backbone"),
        }
    }
}

/// Bootstrap mode for network joining
///
/// Controls how the node discovers and connects to the network.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum BootstrapMode {
    /// Use official NONØS bootstrap nodes
    #[default]
    Official,
    /// Use custom bootstrap peers only
    Custom,
    /// No bootstrap - isolated/lab networks
    None,
}

impl std::fmt::Display for BootstrapMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BootstrapMode::Official => write!(f, "official"),
            BootstrapMode::Custom => write!(f, "custom"),
            BootstrapMode::None => write!(f, "none"),
        }
    }
}

/// Node configuration with safe production defaults
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct NodeConfig {
    /// Node role in the network (local, relay, backbone)
    pub role: NodeRole,

    /// Data directory for node storage
    pub data_dir: PathBuf,

    /// P2P listen port
    pub port: u16,

    /// Maximum concurrent P2P connections (auto-adjusted based on role if 0)
    pub max_connections: u32,

    /// Bandwidth limit in bytes/sec (0 = unlimited)
    pub bandwidth_limit: u64,

    /// Services configuration
    pub services: ServicesConfig,

    /// Network configuration
    pub network: NetworkConfig,

    /// Rewards configuration
    pub rewards: RewardsConfig,

    /// Security configuration
    pub security: SecurityConfig,

    /// Logging configuration
    pub logging: LoggingConfig,

    /// API server configuration
    pub api: ApiConfig,

    /// Rate limiting configuration
    pub rate_limits: RateLimitConfig,
}

/// Services enabled on this node
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ServicesConfig {
    /// Enable health beacon (publishes node health)
    pub health_beacon: bool,
    /// Health beacon interval in seconds
    pub health_beacon_interval_secs: u64,
    /// Enable quality oracle (records metrics)
    pub quality_oracle: bool,
    /// Quality oracle interval in seconds
    pub quality_oracle_interval_secs: u64,
    /// Enable bootstrap service (helps new nodes join)
    pub bootstrap: bool,
    /// Bootstrap service port
    pub bootstrap_port: u16,
    /// Enable public resource caching
    pub cache: bool,
    /// Cache size in MB
    pub cache_size_mb: u32,
    /// Cache max age in seconds
    pub cache_max_age_secs: u64,
}

/// Network configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct NetworkConfig {
    /// Bootstrap mode (official, custom, none)
    pub bootstrap_mode: BootstrapMode,
    /// Custom bootstrap peers (used when bootstrap_mode is Custom)
    pub custom_bootstrap_peers: Vec<String>,
    /// Public announce address (auto-detected if None)
    pub announce_address: Option<String>,
    /// Enable UPnP for NAT traversal
    pub upnp: bool,
    /// Maximum inbound message size in bytes
    pub max_message_size: usize,
    /// Connection timeout in seconds
    pub connection_timeout_secs: u64,
    /// Dial timeout in seconds
    pub dial_timeout_secs: u64,
    /// Keep-alive interval in seconds
    pub keepalive_secs: u64,
    /// Maximum pending dial attempts
    pub max_pending_dials: u32,
    /// Peer penalty threshold for banning (0-100)
    pub ban_threshold: u8,
    /// Ban duration in seconds for protocol violations
    pub ban_duration_secs: u64,
}

/// Rewards configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct RewardsConfig {
    /// Staking contract address
    pub contract: EthAddress,
    /// Reward wallet address
    pub reward_address: EthAddress,
    /// Enable auto-claim of rewards
    pub auto_claim: bool,
    /// Auto-claim threshold in NOX
    pub auto_claim_threshold: u64,
    /// RPC endpoint for blockchain interaction
    pub rpc_url: Option<String>,
}

/// Security configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct SecurityConfig {
    /// Enable automatic updates
    pub auto_update: bool,
    /// Update channel
    pub update_channel: UpdateChannel,
    /// Enable encrypted storage for sensitive data
    pub encrypted_storage: bool,
    /// Require API authentication
    pub api_auth_required: bool,
    /// API authentication token (should be set via env var)
    #[serde(skip_serializing)]
    pub api_auth_token: Option<String>,
}

/// Update channel for automatic updates
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum UpdateChannel {
    /// Stable releases only
    #[default]
    Stable,
    /// Beta releases
    Beta,
    /// No automatic updates
    Manual,
}

/// Logging configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct LoggingConfig {
    /// Log level
    pub level: LogLevel,
    /// Log file path (None = stdout only)
    pub file: Option<PathBuf>,
    /// Enable JSON structured logging
    pub json: bool,
    /// Include timestamps
    pub timestamps: bool,
    /// Include source location
    pub source_location: bool,
}

/// Log level
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LogLevel {
    Error,
    Warn,
    #[default]
    Info,
    Debug,
    Trace,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Error => write!(f, "error"),
            LogLevel::Warn => write!(f, "warn"),
            LogLevel::Info => write!(f, "info"),
            LogLevel::Debug => write!(f, "debug"),
            LogLevel::Trace => write!(f, "trace"),
        }
    }
}

/// API server configuration with SAFE defaults
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ApiConfig {
    /// Enable API server
    pub enabled: bool,
    /// Bind address (LOCALHOST BY DEFAULT for security)
    pub bind_address: IpAddr,
    /// API port
    pub port: u16,
    /// Request timeout in seconds
    pub request_timeout_secs: u64,
    /// Maximum request body size in bytes
    pub max_body_size: usize,
    /// Enable CORS
    pub cors_enabled: bool,
    /// CORS allowed origins (empty = all)
    pub cors_origins: Vec<String>,
}

/// Rate limiting configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct RateLimitConfig {
    /// Enable rate limiting
    pub enabled: bool,
    /// Maximum requests per second per IP
    pub requests_per_second: u32,
    /// Burst size (requests allowed in burst)
    pub burst_size: u32,
    /// P2P message rate limit per peer per second
    pub p2p_messages_per_second: u32,
    /// P2P burst size
    pub p2p_burst_size: u32,
}

// ===== Default Implementations =====

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

impl Default for ServicesConfig {
    fn default() -> Self {
        Self {
            health_beacon: true,
            health_beacon_interval_secs: 60,
            quality_oracle: true,
            quality_oracle_interval_secs: 300,
            bootstrap: false,
            bootstrap_port: DEFAULT_BOOTSTRAP_PORT,
            cache: false,
            cache_size_mb: 1024,
            cache_max_age_secs: 86400,
        }
    }
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            bootstrap_mode: BootstrapMode::Official,
            custom_bootstrap_peers: Vec::new(),
            announce_address: None,
            upnp: true,
            max_message_size: DEFAULT_MAX_MESSAGE_SIZE,
            connection_timeout_secs: 30,
            dial_timeout_secs: 10,
            keepalive_secs: 60,
            max_pending_dials: 16,
            ban_threshold: 80,
            ban_duration_secs: 3600,
        }
    }
}

impl NetworkConfig {
    /// Official NONØS bootstrap nodes
    pub const OFFICIAL_BOOTSTRAP_NODES: &'static [&'static str] = &[
        "/ip4/5.255.99.170/tcp/9432/p2p/12D3KooWAHtxSqGkTbpYjmJW55BpScGkezkWF3sQBK7Pv3CHjTPE",
        "/dns4/boot1.nonos.systems/tcp/9432/p2p/12D3KooWAHtxSqGkTbpYjmJW55BpScGkezkWF3sQBK7Pv3CHjTPE",
    ];

    /// Get the effective bootstrap peers based on mode
    pub fn bootstrap_peers(&self) -> Vec<String> {
        match self.bootstrap_mode {
            BootstrapMode::Official => {
                Self::OFFICIAL_BOOTSTRAP_NODES
                    .iter()
                    .map(|s| s.to_string())
                    .collect()
            }
            BootstrapMode::Custom => self.custom_bootstrap_peers.clone(),
            BootstrapMode::None => Vec::new(),
        }
    }
}

impl Default for RewardsConfig {
    fn default() -> Self {
        Self {
            contract: EthAddress::zero(),
            reward_address: EthAddress::zero(),
            auto_claim: false,
            auto_claim_threshold: 100,
            rpc_url: None,
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            auto_update: false,
            update_channel: UpdateChannel::Stable,
            encrypted_storage: false,
            api_auth_required: false,
            api_auth_token: None,
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            file: None,
            json: false,
            timestamps: true,
            source_location: false,
        }
    }
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            bind_address: IpAddr::V4(Ipv4Addr::LOCALHOST),
            port: DEFAULT_API_PORT,
            request_timeout_secs: 30,
            max_body_size: 1024 * 1024,
            cors_enabled: true,
            cors_origins: vec![],
        }
    }
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            requests_per_second: DEFAULT_RATE_LIMIT_RPS,
            burst_size: 200,
            p2p_messages_per_second: 50,
            p2p_burst_size: 100,
        }
    }
}

// ===== NodeConfig Implementation =====

impl NodeConfig {
    /// Load configuration from file with environment variable overrides
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

    /// Save configuration to file
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

    /// Apply environment variable overrides
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
            // Adjust max_connections based on role if not explicitly set
            if self.max_connections == NodeRole::Local.max_peers() {
                self.max_connections = self.role.max_peers();
            }
        }
    }

    /// Validate configuration
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

        // Validate bootstrap configuration
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
            BootstrapMode::Official => {
                // Official bootstrap peers are always valid
            }
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

    /// Get API socket address
    pub fn api_socket_addr(&self) -> SocketAddr {
        SocketAddr::new(self.api.bind_address, self.api.port)
    }

    /// Get P2P listen address as multiaddr string
    pub fn p2p_listen_addr(&self) -> String {
        format!("/ip4/0.0.0.0/tcp/{}", self.port)
    }

    /// Check if API is bound to localhost only
    pub fn api_is_localhost_only(&self) -> bool {
        match self.api.bind_address {
            IpAddr::V4(addr) => addr.is_loopback(),
            IpAddr::V6(addr) => addr.is_loopback(),
        }
    }

    /// Get a redacted version of config for logging (hides sensitive values)
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

    /// Get effective maximum peers based on role
    pub fn effective_max_peers(&self) -> u32 {
        if self.max_connections == 0 {
            self.role.max_peers()
        } else {
            self.max_connections
        }
    }
}

/// Redacted config for safe logging
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_validation() {
        let config = NodeConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_api_defaults_to_localhost() {
        let config = NodeConfig::default();
        assert!(config.api_is_localhost_only());
        assert_eq!(config.api.bind_address, IpAddr::V4(Ipv4Addr::LOCALHOST));
    }

    #[test]
    fn test_invalid_port() {
        let mut config = NodeConfig::default();
        config.port = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_duplicate_ports() {
        let mut config = NodeConfig::default();
        config.port = 8420;
        config.api.port = 8420;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_custom_bootstrap_without_peers() {
        let mut config = NodeConfig::default();
        config.network.bootstrap_mode = BootstrapMode::Custom;
        config.network.custom_bootstrap_peers.clear();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_invalid_multiaddr() {
        let mut config = NodeConfig::default();
        config.network.bootstrap_mode = BootstrapMode::Custom;
        config.network.custom_bootstrap_peers = vec!["invalid-address".to_string()];
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_node_roles() {
        assert_eq!(NodeRole::Local.max_peers(), 25);
        assert_eq!(NodeRole::Relay.max_peers(), 100);
        assert_eq!(NodeRole::Backbone.max_peers(), 500);
        assert!(!NodeRole::Local.serves_bootstrap());
        assert!(NodeRole::Relay.serves_bootstrap());
        assert!(NodeRole::Backbone.high_availability());
    }

    #[test]
    fn test_bootstrap_modes() {
        let mut config = NetworkConfig::default();

        // Official mode should return official nodes
        config.bootstrap_mode = BootstrapMode::Official;
        assert!(!config.bootstrap_peers().is_empty());

        // Custom mode with peers
        config.bootstrap_mode = BootstrapMode::Custom;
        config.custom_bootstrap_peers = vec!["/ip4/1.2.3.4/tcp/9432/p2p/12D3KooWTest".to_string()];
        assert_eq!(config.bootstrap_peers().len(), 1);

        // None mode should return empty
        config.bootstrap_mode = BootstrapMode::None;
        assert!(config.bootstrap_peers().is_empty());
    }

    #[test]
    fn test_redacted_config() {
        let config = NodeConfig::default();
        let redacted = config.redacted();
        assert!(format!("{}", redacted).contains("API:"));
    }

    #[test]
    fn test_config_serialization() {
        let config = NodeConfig::default();
        let toml_str = toml::to_string_pretty(&config).expect("Failed to serialize");
        let parsed: NodeConfig = toml::from_str(&toml_str).expect("Failed to parse");
        assert_eq!(parsed.port, config.port);
    }
}
