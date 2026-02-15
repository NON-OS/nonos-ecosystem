use super::cookie_vault::DistributedCookieVault;
use super::credentials::CredentialManager;
use super::fingerprint::FingerprintNormalizer;
use super::mixnet::{MixnetProcessor, MixnetKeypair};
use super::oracle::PrivacyOracle;
use super::pir::PrivateContentRetrieval;
use super::stealth_sessions::StealthSessionManager;
use super::zk_sessions::ZkSessionManager;
use nonos_types::NonosResult;
use std::collections::HashMap;
use tracing::warn;

pub struct AdvancedPrivacyManager {
    pub zk_sessions: ZkSessionManager,
    pub mixnet: MixnetProcessor,
    pub pir: PrivateContentRetrieval,
    pub privacy_oracle: PrivacyOracle,
    pub stealth_sessions: StealthSessionManager,
    pub credentials: CredentialManager,
    pub fingerprint: FingerprintNormalizer,
    pub cookie_vault: DistributedCookieVault,
}

impl AdvancedPrivacyManager {
    pub fn new() -> Self {
        Self {
            zk_sessions: ZkSessionManager::new(),
            mixnet: MixnetProcessor::new(),
            pir: PrivateContentRetrieval::new(10000),
            privacy_oracle: PrivacyOracle::new(),
            stealth_sessions: StealthSessionManager::new(),
            credentials: CredentialManager::new(),
            fingerprint: FingerprintNormalizer::new(),
            cookie_vault: DistributedCookieVault::new(3, 5).expect("Failed to create cookie vault"),
        }
    }

    pub fn with_config(
        pir_cache_size: usize,
        mixnet_pool_size: usize,
        mixnet_max_delay_ms: u64,
        cookie_threshold: u8,
        cookie_shares: u8,
    ) -> Self {
        let keypair = MixnetKeypair::generate();

        Self {
            zk_sessions: ZkSessionManager::new(),
            mixnet: MixnetProcessor::with_config(keypair, mixnet_pool_size, mixnet_max_delay_ms),
            pir: PrivateContentRetrieval::new(pir_cache_size),
            privacy_oracle: PrivacyOracle::new(),
            stealth_sessions: StealthSessionManager::new(),
            credentials: CredentialManager::new(),
            fingerprint: FingerprintNormalizer::new(),
            cookie_vault: DistributedCookieVault::new(cookie_threshold, cookie_shares).expect("Failed to create cookie vault"),
        }
    }

    pub async fn process_request(
        &self,
        url: &str,
        headers: &mut HashMap<String, String>,
        session_id: &[u8; 16],
    ) -> NonosResult<()> {
        self.fingerprint.normalize_headers(headers);

        let domain = self.extract_domain(url);
        if let Some(domain) = domain {
            self.stealth_sessions.record_visit(session_id, &domain).await?;

            if let Some(score) = self.privacy_oracle.get_score(&domain).await {
                if score.score < 30 {
                    warn!("Low privacy score for {}: {}", domain, score.score);
                }
            }
        }

        Ok(())
    }

    pub async fn analyze_and_process(
        &self,
        url: &str,
        content: Option<&str>,
        headers: &mut HashMap<String, String>,
        session_id: &[u8; 16],
    ) -> NonosResult<Option<super::oracle::DomainPrivacyScore>> {
        self.fingerprint.normalize_headers(headers);

        let domain = match self.extract_domain(url) {
            Some(d) => d,
            None => return Ok(None),
        };

        self.stealth_sessions.record_visit(session_id, &domain).await?;

        let score = self.privacy_oracle.analyze_domain(&domain, content).await;

        if score.score < 30 {
            warn!("Low privacy score for {}: {}", domain, score.score);
        }

        Ok(Some(score))
    }

    fn extract_domain(&self, url: &str) -> Option<String> {
        let url = url.trim();
        let without_scheme = url
            .strip_prefix("https://")
            .or_else(|| url.strip_prefix("http://"))
            .unwrap_or(url);

        let domain = without_scheme
            .split('/')
            .next()?
            .split(':')
            .next()?;

        if domain.is_empty() {
            None
        } else {
            Some(domain.to_string())
        }
    }

    pub async fn stats(&self) -> AdvancedPrivacyStats {
        let (session_count, total_visits) = self.stealth_sessions.session_stats().await;
        AdvancedPrivacyStats {
            active_sessions: session_count,
            total_domain_visits: total_visits,
            zk_nullifiers: self.zk_sessions.nullifier_count().await,
            pir_cache_size: self.pir.cache_size().await,
            privacy_scores_cached: self.privacy_oracle.score_count().await,
            credentials_issued: self.credentials.issued_proof_count().await,
            cookies_stored: self.cookie_vault.stored_cookie_count().await,
            mixnet_pool_size: self.mixnet.pool_size().await,
        }
    }

    pub async fn cleanup(&self) {
        self.pir.cleanup_expired().await;
        self.stealth_sessions.cleanup_expired().await;
        self.credentials.cleanup_expired().await;
    }
}

impl Default for AdvancedPrivacyManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct AdvancedPrivacyStats {
    pub active_sessions: usize,
    pub total_domain_visits: usize,
    pub zk_nullifiers: usize,
    pub pir_cache_size: usize,
    pub privacy_scores_cached: usize,
    pub credentials_issued: usize,
    pub cookies_stored: usize,
    pub mixnet_pool_size: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_advanced_privacy_manager_creation() {
        let manager = AdvancedPrivacyManager::new();
        let stats = manager.stats().await;
        assert_eq!(stats.active_sessions, 0);
    }

    #[tokio::test]
    async fn test_request_processing() {
        let manager = AdvancedPrivacyManager::new();
        let session = manager.stealth_sessions.create_session().await.unwrap();

        let mut headers = HashMap::new();
        headers.insert("User-Agent".into(), "Custom Agent".into());

        manager.process_request(
            "https://example.com/page",
            &mut headers,
            &session.session_id,
        ).await.unwrap();

        assert!(headers.get("User-Agent").unwrap().contains("Chrome"));
    }

    #[test]
    fn test_domain_extraction() {
        let manager = AdvancedPrivacyManager::new();

        assert_eq!(
            manager.extract_domain("https://example.com/path"),
            Some("example.com".to_string())
        );
        assert_eq!(
            manager.extract_domain("http://sub.example.com:8080/"),
            Some("sub.example.com".to_string())
        );
        assert_eq!(manager.extract_domain(""), None);
    }
}
