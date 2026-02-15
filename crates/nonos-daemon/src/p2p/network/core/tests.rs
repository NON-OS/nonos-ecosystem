use super::super::config::NetworkConfig;
use super::network::P2pNetwork;

#[test]
fn test_network_creation() {
    let network = P2pNetwork::new(9432, 50);
    assert_eq!(network.config.port, 9432);
    assert!(!network.is_running());
}

#[test]
fn test_network_with_config() {
    let config = NetworkConfig {
        port: 9999,
        max_connections: 100,
        enable_rate_limiting: false,
        ..Default::default()
    };
    let network = P2pNetwork::with_config(config);
    assert_eq!(network.config.port, 9999);
    assert!(!network.config.enable_rate_limiting);
}
