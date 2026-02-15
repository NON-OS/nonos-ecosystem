mod api;
mod constants;
mod logging;
mod network;
mod node;
mod rate_limit;
mod rewards;
mod security;
mod services;
mod types;

pub use api::ApiConfig;
pub use constants::*;
pub use logging::LoggingConfig;
pub use network::NetworkConfig;
pub use node::{NodeConfig, RedactedConfig};
pub use rate_limit::RateLimitConfig;
pub use rewards::RewardsConfig;
pub use security::SecurityConfig;
pub use services::ServicesConfig;
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

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

        config.bootstrap_mode = BootstrapMode::Official;
        assert!(!config.bootstrap_peers().is_empty());

        config.bootstrap_mode = BootstrapMode::Custom;
        config.custom_bootstrap_peers = vec!["/ip4/1.2.3.4/tcp/9432/p2p/12D3KooWTest".to_string()];
        assert_eq!(config.bootstrap_peers().len(), 1);

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
