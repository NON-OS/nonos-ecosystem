//! Application state for the NONOS Dashboard

use std::sync::Arc;
use tokio::sync::RwLock;
use crate::theme::Theme;
use crate::globe::Globe;
use serde::Deserialize;

/// Main application state
pub struct App {
    /// Current tab index
    pub tab: u8,
    /// API URL for the daemon
    pub api_url: String,
    /// Current theme
    pub theme: Theme,
    /// Shared data state
    pub data: Arc<RwLock<AppData>>,
    /// Log scroll position
    pub log_scroll: usize,
    /// Show help overlay
    pub show_help: bool,
    /// Animation frame counter
    pub frame: u64,
}

impl App {
    pub fn new(api_url: String, theme: Theme) -> Self {
        Self {
            tab: 0,
            api_url,
            theme,
            data: Arc::new(RwLock::new(AppData::default())),
            log_scroll: 0,
            show_help: false,
            frame: 0,
        }
    }

    pub fn tick(&mut self) {
        self.frame = self.frame.wrapping_add(1);
    }

    pub fn scroll_up(&mut self) {
        if self.log_scroll > 0 {
            self.log_scroll -= 1;
        }
    }

    pub fn scroll_down(&mut self) {
        self.log_scroll += 1;
    }

    pub fn refresh(&mut self) {
        // Data will be refreshed by the background task
    }
}

// API Response types for deserializing real data
#[derive(Debug, Deserialize, Default)]
pub struct StatusApiResponse {
    pub node_id: Option<String>,
    pub tier: Option<String>,
    pub uptime_secs: Option<u64>,
    pub active_connections: Option<u64>,
    pub total_requests: Option<u64>,
    pub successful_requests: Option<u64>,
    pub quality_score: Option<f64>,
    pub staked_nox: Option<f64>,
    pub pending_rewards: Option<f64>,
    pub streak_days: Option<u32>,
}


#[derive(Debug, Deserialize, Default)]
pub struct PeersApiResponse {
    pub peers: Option<Vec<PeerGeoInfo>>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct PeerGeoInfo {
    pub id: String,
    pub lat: f64,
    pub lon: f64,
    pub city: String,
    pub country_code: String,
    pub latency_ms: Option<u32>,
    pub connected: bool,
    pub is_bootstrap: bool,
}

#[derive(Debug, Deserialize, Default)]
pub struct PrivacyStatsApiResponse {
    pub available: Option<bool>,
    pub zk_proofs_issued: Option<u64>,
    pub zk_verifications: Option<u64>,
    pub cache_hits: Option<u64>,
    pub cache_misses: Option<u64>,
    pub cache_mix_ops: Option<u64>,
    pub tracking_blocked: Option<u64>,
    pub tracking_total: Option<u64>,
    pub tracking_block_rate: Option<f64>,
}


/// Application data state
#[derive(Default)]
pub struct AppData {
    // Connection status
    pub connected: bool,

    // Node info
    pub node_id: String,
    pub uptime_secs: u64,
    pub quality_score: f64,
    pub tier: String,
    pub peers: u64,
    pub pending_rewards: f64,
    pub staked_nox: f64,
    pub streak_days: u32,

    // Request stats
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,

    // Services status
    pub services: ServicesStatus,

    // Privacy stats (REAL data)
    pub privacy_available: bool,
    pub zk_proofs_issued: u64,
    pub zk_verifications: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub cache_mix_ops: u64,
    pub tracking_blocked: u64,
    pub tracking_total: u64,
    pub tracking_block_rate: f64,

    // Resource usage (calculated from system if available)
    pub cpu_usage: u8,
    pub memory_usage: u8,
    pub disk_usage: u8,

    // History for sparklines (real request counts)
    pub request_history: Vec<u64>,
    pub bandwidth_history: Vec<u64>,
    last_request_count: u64,

    // Recent activity (derived from real events)
    pub recent_activity: Vec<ActivityEntry>,

    // Peer list (REAL data)
    pub peer_list: Vec<PeerInfo>,

    // Identities
    pub identities: Vec<IdentityInfo>,

    // Logs
    pub logs: Vec<LogEntry>,

    // Globe for 3D visualization
    pub globe: Globe,
}

impl AppData {
    /// Update from status API response
    pub fn update_status(&mut self, stats: StatusApiResponse) {
        self.connected = true;

        if let Some(id) = stats.node_id {
            self.node_id = id;
        }
        if let Some(uptime) = stats.uptime_secs {
            self.uptime_secs = uptime;
        }
        if let Some(quality) = stats.quality_score {
            self.quality_score = quality;
        }
        if let Some(tier) = stats.tier {
            self.tier = tier;
        }
        if let Some(peers) = stats.active_connections {
            self.peers = peers;
        }
        if let Some(rewards) = stats.pending_rewards {
            self.pending_rewards = rewards;
        }
        if let Some(staked) = stats.staked_nox {
            self.staked_nox = staked;
        }
        if let Some(streak) = stats.streak_days {
            self.streak_days = streak;
        }
        if let Some(total) = stats.total_requests {
            self.total_requests = total;
        }
        if let Some(successful) = stats.successful_requests {
            self.successful_requests = successful;
        }

        // Calculate failed requests
        self.failed_requests = self.total_requests.saturating_sub(self.successful_requests);

        // Update request history with real delta
        let delta = self.total_requests.saturating_sub(self.last_request_count);
        self.last_request_count = self.total_requests;

        if self.request_history.len() >= 60 {
            self.request_history.remove(0);
        }
        self.request_history.push(delta);
    }

    /// Update from peers API response with geo data
    pub fn update_peers(&mut self, peers_resp: PeersApiResponse) {
        use crate::globe::GeoNode;

        if let Some(peers) = peers_resp.peers {
            // Update peer list for display
            self.peer_list = peers.iter().map(|p| {
                PeerInfo {
                    id: p.id.clone(),
                    location: format!("{}, {}", p.city, p.country_code),
                    latency: p.latency_ms.unwrap_or(0),
                    connected: p.connected,
                }
            }).collect();

            // Update globe with peer locations
            self.globe.nodes.clear();
            for peer in &peers {
                self.globe.nodes.push(GeoNode {
                    lat: peer.lat,
                    lon: peer.lon,
                    is_bootstrap: peer.is_bootstrap,
                });
            }
        }
    }

    /// Update from privacy stats API response
    pub fn update_privacy_stats(&mut self, stats: PrivacyStatsApiResponse) {
        self.privacy_available = stats.available.unwrap_or(false);

        if let Some(v) = stats.zk_proofs_issued { self.zk_proofs_issued = v; }
        if let Some(v) = stats.zk_verifications { self.zk_verifications = v; }
        if let Some(v) = stats.cache_hits { self.cache_hits = v; }
        if let Some(v) = stats.cache_misses { self.cache_misses = v; }
        if let Some(v) = stats.cache_mix_ops { self.cache_mix_ops = v; }
        if let Some(v) = stats.tracking_blocked { self.tracking_blocked = v; }
        if let Some(v) = stats.tracking_total { self.tracking_total = v; }
        if let Some(v) = stats.tracking_block_rate { self.tracking_block_rate = v; }

        // Update services based on privacy availability
        if self.privacy_available {
            self.services.zk_identity = self.zk_proofs_issued > 0 || self.zk_verifications > 0;
            self.services.cache_mixer = self.cache_mix_ops > 0 || self.cache_hits > 0;
            self.services.tracking_blocker = self.tracking_total > 0;
        }

        // Add activity based on privacy stats changes
        if self.zk_proofs_issued > 0 {
            self.add_activity("info", &format!("ZK proofs issued: {}", self.zk_proofs_issued));
        }
        if self.tracking_blocked > 0 {
            self.add_activity("info", &format!("Tracking requests blocked: {}", self.tracking_blocked));
        }
    }

    /// Update system resource usage using real system metrics
    pub fn update_resources(&mut self, cpu_usage: u8, memory_usage: u8, disk_usage: u8) {
        self.cpu_usage = cpu_usage;
        self.memory_usage = memory_usage;
        self.disk_usage = disk_usage;
    }

    /// Add activity entry
    fn add_activity(&mut self, level: &str, message: &str) {
        // Avoid duplicates
        if self.recent_activity.iter().any(|a| a.message == message) {
            return;
        }

        if self.recent_activity.len() >= 10 {
            self.recent_activity.remove(0);
        }
        self.recent_activity.push(ActivityEntry {
            time: chrono::Local::now().format("%H:%M:%S").to_string(),
            level: level.to_string(),
            message: message.to_string(),
        });
    }

    /// Update from combined JSON (for backward compatibility)
    pub fn update(&mut self, stats: serde_json::Value) {
        // Parse as StatusApiResponse
        if let Ok(status) = serde_json::from_value::<StatusApiResponse>(stats.clone()) {
            self.update_status(status);
        }

        // Update services based on connection
        if self.connected {
            self.services.p2p_network = self.peers > 0;
            self.services.health_beacon = true;
            self.services.quality_oracle = self.quality_score > 0.0;
        }

        // Add connection activity
        if self.connected && self.recent_activity.is_empty() {
            self.add_activity("info", "Connected to NONOS network");
        }

        // Add quality update
        if self.quality_score > 0.0 {
            self.add_activity("info", &format!("Quality score: {:.1}%", self.quality_score * 100.0));
        }
    }
}

#[derive(Default)]
pub struct ServicesStatus {
    pub zk_identity: bool,
    pub cache_mixer: bool,
    pub tracking_blocker: bool,
    pub stealth_scanner: bool,
    pub p2p_network: bool,
    pub health_beacon: bool,
    pub quality_oracle: bool,
}

pub struct ActivityEntry {
    pub time: String,
    pub level: String,
    pub message: String,
}

#[derive(Clone)]
pub struct PeerInfo {
    pub id: String,
    pub location: String,
    pub latency: u32,
    pub connected: bool,
}

pub struct IdentityInfo {
    pub id: String,
    pub label: String,
    pub created: String,
    pub registered: bool,
}

pub struct LogEntry {
    pub time: String,
    pub level: String,
    pub target: String,
    pub message: String,
}
