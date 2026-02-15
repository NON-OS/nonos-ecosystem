use nonos_crypto::poseidon_hash;
use nonos_types::{NodeId, NonosResult};
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{debug, info};

const KNOWN_TRACKERS: &[&str] = &[
    "google-analytics.com", "googletagmanager.com", "facebook.com",
    "connect.facebook.net", "doubleclick.net", "googlesyndication.com",
    "googleadservices.com", "amazon-adsystem.com", "scorecardresearch.com",
    "quantserve.com", "adsrvr.org", "criteo.com", "taboola.com",
    "outbrain.com", "chartbeat.com", "mixpanel.com", "segment.io",
    "amplitude.com", "hotjar.com", "fullstory.com", "mouseflow.com",
    "crazyegg.com", "clarity.ms", "newrelic.com", "sentry.io",
];

const TRACKING_PATTERNS: &[&str] = &[
    "google-analytics.com", "analytics.google.com", "/ga.js", "/gtag/",
    "gtm.js", "analytics.js", "facebook.com/tr", "fbevents.js",
    "connect.facebook", "pixel.facebook", "doubleclick.net",
    "googlesyndication", "googleadservices", "adservice.google", "pagead",
    "hotjar.com", "fullstory.com", "mouseflow.com", "clarity.ms",
    "crazyegg.com", "logrocket.com", "hubspot.com", "marketo.com",
    "pardot.com", "eloqua.com", "fingerprint", "fp.js", "fpjs",
];

const TRACKING_PARAMS: &[&str] = &[
    "utm_source", "utm_medium", "utm_campaign", "utm_term", "utm_content",
    "fbclid", "fb_action_ids", "fb_action_types", "fb_source", "fb_ref",
    "gclid", "gclsrc", "dclid", "msclkid", "twclid", "mc_cid", "mc_eid",
    "_hsenc", "_hsmi", "hsCtaTracking", "oly_enc_id", "oly_anon_id",
    "vero_id", "nr_email_referer", "mkt_tok", "trk_contact", "trk_msg",
];

pub struct TrackingBlockerService {
    _node_id: NodeId,
    blocked_domains: Arc<RwLock<HashSet<[u8; 32]>>>,
    blocked_domain_strings: Arc<RwLock<HashSet<String>>>,
    blocked_patterns: Arc<RwLock<Vec<String>>>,
    blocked_params: Arc<RwLock<HashSet<String>>>,
    requests_blocked: AtomicU64,
    total_requests: AtomicU64,
    fingerprint_blocked: AtomicU64,
}

impl TrackingBlockerService {
    pub fn new(node_id: NodeId) -> Self {
        let mut blocked_hashes = HashSet::new();
        let mut blocked_strings = HashSet::new();

        for domain in KNOWN_TRACKERS {
            blocked_hashes.insert(poseidon_hash(domain.as_bytes()));
            blocked_strings.insert(domain.to_string());
        }

        let patterns: Vec<String> = TRACKING_PATTERNS.iter().map(|s| s.to_string()).collect();
        let params: HashSet<String> = TRACKING_PARAMS.iter().map(|s| s.to_string()).collect();

        Self {
            _node_id: node_id,
            blocked_domains: Arc::new(RwLock::new(blocked_hashes)),
            blocked_domain_strings: Arc::new(RwLock::new(blocked_strings)),
            blocked_patterns: Arc::new(RwLock::new(patterns)),
            blocked_params: Arc::new(RwLock::new(params)),
            requests_blocked: AtomicU64::new(0),
            total_requests: AtomicU64::new(0),
            fingerprint_blocked: AtomicU64::new(0),
        }
    }

    pub async fn should_block_domain(&self, domain: &str) -> bool {
        let domain_lower = domain.to_lowercase();
        self.total_requests.fetch_add(1, Ordering::Relaxed);

        let domain_hash = poseidon_hash(domain_lower.as_bytes());
        if self.blocked_domains.read().await.contains(&domain_hash) {
            self.requests_blocked.fetch_add(1, Ordering::Relaxed);
            return true;
        }

        for blocked in self.blocked_domain_strings.read().await.iter() {
            if domain_lower.contains(blocked) || blocked.contains(&domain_lower) {
                self.requests_blocked.fetch_add(1, Ordering::Relaxed);
                return true;
            }
        }

        for pattern in self.blocked_patterns.read().await.iter() {
            if domain_lower.contains(&pattern.to_lowercase()) {
                self.requests_blocked.fetch_add(1, Ordering::Relaxed);
                return true;
            }
        }

        false
    }

    pub async fn should_block_url(&self, url: &str) -> (bool, Option<String>) {
        let url_lower = url.to_lowercase();
        self.total_requests.fetch_add(1, Ordering::Relaxed);

        for pattern in self.blocked_patterns.read().await.iter() {
            if url_lower.contains(&pattern.to_lowercase()) {
                self.requests_blocked.fetch_add(1, Ordering::Relaxed);
                return (true, Some(format!("Matched: {}", pattern)));
            }
        }

        if url_lower.contains("fingerprint") ||
           (url_lower.contains("canvas") && url_lower.contains("font")) ||
           (url_lower.contains("webgl") && url_lower.contains("render")) {
            self.fingerprint_blocked.fetch_add(1, Ordering::Relaxed);
            return (true, Some("Fingerprinting detected".to_string()));
        }

        (false, None)
    }

    pub async fn strip_tracking_params(&self, url: &str) -> String {
        if let Some(query_start) = url.find('?') {
            let base = &url[..query_start];
            let query = &url[query_start + 1..];
            let blocked_params = self.blocked_params.read().await;

            let cleaned: Vec<&str> = query.split('&')
                .filter(|param| {
                    param.split('=').next().map_or(true, |key| !blocked_params.contains(key))
                })
                .collect();

            if cleaned.is_empty() {
                base.to_string()
            } else {
                format!("{}?{}", base, cleaned.join("&"))
            }
        } else {
            url.to_string()
        }
    }

    pub async fn block_domain(&self, domain: &str) {
        let domain_lower = domain.to_lowercase();
        self.blocked_domains.write().await.insert(poseidon_hash(domain_lower.as_bytes()));
        self.blocked_domain_strings.write().await.insert(domain_lower);
        info!("Added to blocklist: {}", domain);
    }

    pub async fn unblock_domain(&self, domain: &str) {
        let domain_lower = domain.to_lowercase();
        self.blocked_domains.write().await.remove(&poseidon_hash(domain_lower.as_bytes()));
        self.blocked_domain_strings.write().await.remove(&domain_lower);
        info!("Removed from blocklist: {}", domain);
    }

    pub fn stats(&self) -> (u64, u64, u64) {
        (
            self.requests_blocked.load(Ordering::Relaxed),
            self.total_requests.load(Ordering::Relaxed),
            self.fingerprint_blocked.load(Ordering::Relaxed),
        )
    }

    pub async fn detailed_stats(&self) -> TrackingBlockerStats {
        let (blocked, total, fingerprint) = self.stats();
        TrackingBlockerStats {
            requests_blocked: blocked,
            total_requests: total,
            fingerprint_blocked: fingerprint,
            block_rate: if total > 0 { (blocked as f64 / total as f64) * 100.0 } else { 0.0 },
            blocked_domains_count: self.blocked_domain_strings.read().await.len(),
            blocked_patterns_count: self.blocked_patterns.read().await.len(),
            blocked_params_count: self.blocked_params.read().await.len(),
        }
    }

    pub async fn run(self: Arc<Self>, shutdown: Arc<AtomicBool>) -> NonosResult<()> {
        info!("Tracking Blocker started");
        let mut ticker = interval(Duration::from_secs(60));

        while !shutdown.load(Ordering::Relaxed) {
            ticker.tick().await;
            let (blocked, total, fp) = self.stats();
            let rate = if total > 0 { (blocked as f64 / total as f64) * 100.0 } else { 0.0 };
            debug!("Tracker blocker: {:.1}% blocked, {} fingerprint", rate, fp);
        }

        info!("Tracking Blocker stopped");
        Ok(())
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TrackingBlockerStats {
    pub requests_blocked: u64,
    pub total_requests: u64,
    pub fingerprint_blocked: u64,
    pub block_rate: f64,
    pub blocked_domains_count: usize,
    pub blocked_patterns_count: usize,
    pub blocked_params_count: usize,
}
