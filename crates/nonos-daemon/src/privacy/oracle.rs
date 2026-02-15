use nonos_types::NodeId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DomainPrivacyScore {
    pub domain: String,
    pub score: u8,
    pub voter_count: u32,
    pub trackers_detected: Vec<String>,
    pub fingerprinting: Vec<String>,
    pub cookie_behavior: CookieBehavior,
    pub updated_at: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum CookieBehavior {
    None,
    SessionOnly,
    Persistent,
    CrossSite,
    Supercookies,
}

impl Default for CookieBehavior {
    fn default() -> Self {
        Self::None
    }
}

pub struct PrivacyOracle {
    scores: Arc<RwLock<HashMap<String, DomainPrivacyScore>>>,
    tracker_patterns: Vec<String>,
    fingerprint_patterns: Vec<String>,
}

impl PrivacyOracle {
    pub fn new() -> Self {
        Self {
            scores: Arc::new(RwLock::new(HashMap::new())),
            tracker_patterns: Self::default_tracker_patterns(),
            fingerprint_patterns: Self::default_fingerprint_patterns(),
        }
    }

    fn default_tracker_patterns() -> Vec<String> {
        vec![
            "google-analytics.com".into(),
            "googletagmanager.com".into(),
            "facebook.com/tr".into(),
            "doubleclick.net".into(),
            "amazon-adsystem.com".into(),
            "hotjar.com".into(),
            "fullstory.com".into(),
            "segment.io".into(),
            "mixpanel.com".into(),
            "amplitude.com".into(),
            "heapanalytics.com".into(),
            "clarity.ms".into(),
            "newrelic.com".into(),
            "datadog-ci.com".into(),
        ]
    }

    fn default_fingerprint_patterns() -> Vec<String> {
        vec![
            "canvas.toDataURL".into(),
            "WebGLRenderingContext".into(),
            "AudioContext".into(),
            "navigator.plugins".into(),
            "navigator.mimeTypes".into(),
            "screen.colorDepth".into(),
            "getClientRects".into(),
        ]
    }

    pub async fn analyze_domain(&self, domain: &str, page_content: Option<&str>) -> DomainPrivacyScore {
        let mut trackers_detected = Vec::new();
        let mut fingerprinting = Vec::new();
        let mut score: i32 = 100;

        for pattern in &self.tracker_patterns {
            if domain.contains(pattern) || page_content.map(|c| c.contains(pattern)).unwrap_or(false) {
                trackers_detected.push(pattern.clone());
                score -= 10;
            }
        }

        if let Some(content) = page_content {
            for pattern in &self.fingerprint_patterns {
                if content.contains(pattern) {
                    fingerprinting.push(pattern.clone());
                    score -= 5;
                }
            }
        }

        let cookie_behavior = self.analyze_cookie_behavior(page_content);
        match cookie_behavior {
            CookieBehavior::CrossSite => score -= 20,
            CookieBehavior::Supercookies => score -= 30,
            CookieBehavior::Persistent => score -= 10,
            _ => {}
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        DomainPrivacyScore {
            domain: domain.to_string(),
            score: score.max(0).min(100) as u8,
            voter_count: 1,
            trackers_detected,
            fingerprinting,
            cookie_behavior,
            updated_at: now,
        }
    }

    fn analyze_cookie_behavior(&self, content: Option<&str>) -> CookieBehavior {
        if let Some(content) = content {
            if content.contains("evercookie") || content.contains("localStorage.setItem") {
                return CookieBehavior::Supercookies;
            }
            if content.contains("SameSite=None") {
                return CookieBehavior::CrossSite;
            }
            if content.contains("document.cookie") {
                return CookieBehavior::Persistent;
            }
        }
        CookieBehavior::None
    }

    pub async fn submit_vote(&self, _node_id: &NodeId, score: DomainPrivacyScore) {
        let mut scores = self.scores.write().await;

        if let Some(existing) = scores.get_mut(&score.domain) {
            let total_weight = existing.voter_count + 1;
            existing.score = ((existing.score as u32 * existing.voter_count + score.score as u32)
                / total_weight) as u8;
            existing.voter_count = total_weight;

            for tracker in score.trackers_detected {
                if !existing.trackers_detected.contains(&tracker) {
                    existing.trackers_detected.push(tracker);
                }
            }

            for fp in score.fingerprinting {
                if !existing.fingerprinting.contains(&fp) {
                    existing.fingerprinting.push(fp);
                }
            }

            existing.updated_at = score.updated_at;
        } else {
            scores.insert(score.domain.clone(), score);
        }
    }

    pub async fn get_score(&self, domain: &str) -> Option<DomainPrivacyScore> {
        self.scores.read().await.get(domain).cloned()
    }

    pub async fn get_all_scores(&self) -> Vec<DomainPrivacyScore> {
        self.scores.read().await.values().cloned().collect()
    }

    pub async fn score_count(&self) -> usize {
        self.scores.read().await.len()
    }

    pub async fn clear_scores(&self) {
        self.scores.write().await.clear();
    }

    pub fn tracker_pattern_count(&self) -> usize {
        self.tracker_patterns.len()
    }

    pub fn fingerprint_pattern_count(&self) -> usize {
        self.fingerprint_patterns.len()
    }
}

impl Default for PrivacyOracle {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_privacy_oracle_tracker_detection() {
        let oracle = PrivacyOracle::new();
        let score = oracle.analyze_domain("google-analytics.com", None).await;
        assert!(score.score < 100);
        assert!(!score.trackers_detected.is_empty());
    }

    #[tokio::test]
    async fn test_privacy_oracle_fingerprint_detection() {
        let oracle = PrivacyOracle::new();
        let content = "function test() { canvas.toDataURL(); }";
        let score = oracle.analyze_domain("example.com", Some(content)).await;
        assert!(!score.fingerprinting.is_empty());
    }

    #[tokio::test]
    async fn test_cookie_behavior_detection() {
        let oracle = PrivacyOracle::new();
        let content = "SameSite=None; Secure";
        let score = oracle.analyze_domain("example.com", Some(content)).await;
        assert_eq!(score.cookie_behavior, CookieBehavior::CrossSite);
    }
}
