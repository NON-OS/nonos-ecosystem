use super::*;
use nonos_types::NodeId;

#[tokio::test]
async fn test_network_creation() {
    let network = P2pNetwork::new(9432, 100);
    assert_eq!(network.port(), 9432);
    assert!(!network.is_running());
}

#[tokio::test]
async fn test_message_encoding() {
    let beacon = HealthBeaconData {
        node_id: NodeId::from_bytes([1u8; 32]),
        timestamp: chrono::Utc::now(),
        uptime_secs: 3600,
        version: "1.0.0".to_string(),
        peer_count: 10,
        cpu_usage: Some(25.0),
        memory_usage: Some(50.0),
    };

    let msg = P2pMessage::HealthBeacon(beacon);
    let encoded = msg.encode();
    let decoded = P2pMessage::decode(&encoded);

    assert!(decoded.is_some());
    if let Some(P2pMessage::HealthBeacon(data)) = decoded {
        assert_eq!(data.uptime_secs, 3600);
        assert_eq!(data.version, "1.0.0");
    }
}

#[tokio::test]
async fn test_peer_info() {
    let info = PeerInfo {
        id: "12D3KooW...".to_string(),
        addresses: vec!["/ip4/127.0.0.1/tcp/9432".to_string()],
        connected_at: chrono::Utc::now(),
        bytes_sent: 1024,
        bytes_received: 2048,
        latency_ms: Some(50),
        protocol_version: Some("/nonos/1.0.0".to_string()),
        agent_version: Some("nonos-node/1.0.0".to_string()),
        ..Default::default()
    };

    assert_eq!(info.latency_ms, Some(50));
}
