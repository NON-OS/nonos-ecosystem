use serde::Deserialize;
use sysinfo::{System, CpuRefreshKind, MemoryRefreshKind};
use crate::globe::Globe;

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
    pub peers: Option<Vec<PeerApiData>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PeerApiData {
    pub peer_id: String,
    pub addr: Option<String>,
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

pub struct AppData {
    pub connected: bool,
    pub node_id: String,
    pub uptime_secs: u64,
    pub quality_score: f64,
    pub tier: String,
    pub peers: u64,
    pub pending_rewards: f64,
    pub staked_nox: f64,
    pub streak_days: u32,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub services: ServicesStatus,
    pub privacy_available: bool,
    pub zk_proofs_issued: u64,
    pub zk_verifications: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub cache_mix_ops: u64,
    pub tracking_blocked: u64,
    pub tracking_total: u64,
    pub tracking_block_rate: f64,
    pub cpu_usage: u8,
    pub memory_usage: u8,
    pub disk_usage: u8,
    pub request_history: Vec<u64>,
    pub bandwidth_history: Vec<u64>,
    pub last_request_count: u64,
    pub recent_activity: Vec<ActivityEntry>,
    pub peer_list: Vec<PeerDisplayInfo>,
    pub identities: Vec<IdentityInfo>,
    pub logs: Vec<LogEntry>,
    pub globe: Globe,
}

impl Default for AppData {
    fn default() -> Self {
        Self {
            connected: false,
            node_id: String::new(),
            uptime_secs: 0,
            quality_score: 0.0,
            tier: String::new(),
            peers: 0,
            pending_rewards: 0.0,
            staked_nox: 0.0,
            streak_days: 0,
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            services: ServicesStatus::default(),
            privacy_available: false,
            zk_proofs_issued: 0,
            zk_verifications: 0,
            cache_hits: 0,
            cache_misses: 0,
            cache_mix_ops: 0,
            tracking_blocked: 0,
            tracking_total: 0,
            tracking_block_rate: 0.0,
            cpu_usage: 0,
            memory_usage: 0,
            disk_usage: 0,
            request_history: Vec::new(),
            bandwidth_history: Vec::new(),
            last_request_count: 0,
            recent_activity: Vec::new(),
            peer_list: Vec::new(),
            identities: Vec::new(),
            logs: Vec::new(),
            globe: Globe::default(),
        }
    }
}

impl AppData {
    pub fn update_status(&mut self, stats: StatusApiResponse) {
        self.connected = true;
        if let Some(id) = stats.node_id { self.node_id = id; }
        if let Some(uptime) = stats.uptime_secs { self.uptime_secs = uptime; }
        if let Some(quality) = stats.quality_score { self.quality_score = quality; }
        if let Some(tier) = stats.tier { self.tier = tier; }
        if let Some(peers) = stats.active_connections { self.peers = peers; }
        if let Some(rewards) = stats.pending_rewards { self.pending_rewards = rewards; }
        if let Some(staked) = stats.staked_nox { self.staked_nox = staked; }
        if let Some(streak) = stats.streak_days { self.streak_days = streak; }
        if let Some(total) = stats.total_requests { self.total_requests = total; }
        if let Some(successful) = stats.successful_requests { self.successful_requests = successful; }

        self.failed_requests = self.total_requests.saturating_sub(self.successful_requests);

        let delta = self.total_requests.saturating_sub(self.last_request_count);
        self.last_request_count = self.total_requests;

        if self.request_history.len() >= 60 { self.request_history.remove(0); }
        self.request_history.push(delta);
    }

    pub fn update_peers(&mut self, peers_resp: PeersApiResponse) -> Vec<(String, String)> {
        let mut ips_to_lookup = Vec::new();

        if let Some(peers) = peers_resp.peers {
            for peer in peers {
                let ip = peer.addr.as_ref()
                    .and_then(|addr| parse_ip_from_multiaddr(addr))
                    .unwrap_or_default();

                if !ip.is_empty() && !is_private_ip(&ip) {
                    ips_to_lookup.push((peer.peer_id.clone(), ip.clone()));
                }

                if !self.peer_list.iter().any(|p| p.id == peer.peer_id) {
                    self.peer_list.push(PeerDisplayInfo {
                        id: peer.peer_id,
                        location: String::new(),
                        latency: 0,
                        connected: true,
                    });
                }
            }
        }

        ips_to_lookup
    }

    pub fn update_peer_location(&mut self, peer_id: &str, lat: f64, lon: f64, city: &str, country: &str) {
        if let Some(peer) = self.peer_list.iter_mut().find(|p| p.id == peer_id) {
            peer.location = format!("{}, {}", city, country);
        }
        self.globe.add_peer(lat, lon, city);
    }

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

        if self.privacy_available {
            self.services.zk_identity = self.zk_proofs_issued > 0 || self.zk_verifications > 0;
            self.services.cache_mixer = self.cache_mix_ops > 0 || self.cache_hits > 0;
            self.services.tracking_blocker = self.tracking_total > 0;
        }
    }

    pub fn update_resources(&mut self, sys: &mut System) {
        sys.refresh_cpu_specifics(CpuRefreshKind::new().with_cpu_usage());
        sys.refresh_memory_specifics(MemoryRefreshKind::new().with_ram());

        let cpu_usage: f32 = sys.cpus().iter().map(|c| c.cpu_usage()).sum::<f32>()
            / sys.cpus().len().max(1) as f32;
        self.cpu_usage = cpu_usage.min(100.0) as u8;

        let total_mem = sys.total_memory();
        let used_mem = sys.used_memory();
        if total_mem > 0 {
            self.memory_usage = ((used_mem as f64 / total_mem as f64) * 100.0) as u8;
        }

        let disks = sysinfo::Disks::new_with_refreshed_list();
        for disk in disks.list() {
            let total = disk.total_space();
            let available = disk.available_space();
            if total > 0 {
                let used = total - available;
                self.disk_usage = ((used as f64 / total as f64) * 100.0) as u8;
                break;
            }
        }
    }

    pub fn add_activity(&mut self, level: &str, message: &str) {
        if self.recent_activity.iter().any(|a| a.message == message) { return; }
        if self.recent_activity.len() >= 10 { self.recent_activity.remove(0); }
        self.recent_activity.push(ActivityEntry {
            time: chrono::Local::now().format("%H:%M:%S").to_string(),
            level: level.to_string(),
            message: message.to_string(),
        });
    }

    pub fn update(&mut self, stats: serde_json::Value) {
        if let Ok(status) = serde_json::from_value::<StatusApiResponse>(stats) {
            self.update_status(status);
        }
        if self.connected {
            self.services.p2p_network = self.peers > 0;
            self.services.health_beacon = true;
            self.services.quality_oracle = self.quality_score > 0.0;
        }
        if self.connected && self.recent_activity.is_empty() {
            self.add_activity("info", "Connected to NONOS network");
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
pub struct PeerDisplayInfo {
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

fn parse_ip_from_multiaddr(addr: &str) -> Option<String> {
    for part in addr.split('/') {
        if part.parse::<std::net::Ipv4Addr>().is_ok() {
            return Some(part.to_string());
        }
    }
    None
}

fn is_private_ip(ip: &str) -> bool {
    if let Ok(addr) = ip.parse::<std::net::Ipv4Addr>() {
        let octets = addr.octets();
        return octets[0] == 10
            || (octets[0] == 172 && (16..=31).contains(&octets[1]))
            || (octets[0] == 192 && octets[1] == 168)
            || octets[0] == 127;
    }
    false
}
