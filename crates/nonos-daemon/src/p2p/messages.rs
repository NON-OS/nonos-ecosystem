// NONOS GNU AFFERO GENERAL PUBLIC LICENSE
// Version 3, 19 November 2007
// Copyright (C) 2025 NON-OS <team@nonos.systems>

use nonos_types::NodeId;
use serde::{Deserialize, Serialize};

/// Message types for P2P protocol
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum P2pMessage {
    /// Health beacon
    HealthBeacon(HealthBeaconData),
    /// Quality report
    QualityReport(QualityReportData),
    /// Bootstrap request
    BootstrapRequest,
    /// Bootstrap response with peer addresses
    BootstrapResponse(Vec<String>),
    /// Node announcement
    NodeAnnouncement(NodeAnnouncementData),
}

impl P2pMessage {
    /// Encode message to bytes
    pub fn encode(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_default()
    }

    /// Decode message from bytes
    pub fn decode(data: &[u8]) -> Option<Self> {
        serde_json::from_slice(data).ok()
    }
}

/// Health beacon data
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HealthBeaconData {
    /// Node ID
    pub node_id: NodeId,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Uptime seconds
    pub uptime_secs: u64,
    /// Version
    pub version: String,
    /// Number of peers
    pub peer_count: usize,
    /// CPU usage percent
    pub cpu_usage: Option<f32>,
    /// Memory usage percent
    pub memory_usage: Option<f32>,
}

/// Quality report data
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QualityReportData {
    /// Node ID
    pub node_id: NodeId,
    /// Epoch number
    pub epoch: u64,
    /// Success rate (0-1)
    pub success_rate: f64,
    /// Average latency ms
    pub avg_latency_ms: u32,
    /// Request count
    pub request_count: u64,
    /// Uptime during epoch
    pub uptime_percent: f64,
}

/// Node announcement data
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeAnnouncementData {
    /// Node ID
    pub node_id: NodeId,
    /// Staking tier
    pub tier: String,
    /// Staked amount (as string for precision)
    pub staked_amount: String,
    /// Services offered
    pub services: Vec<String>,
    /// Listen addresses
    pub addresses: Vec<String>,
}
