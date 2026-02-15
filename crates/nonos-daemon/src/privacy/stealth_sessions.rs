use nonos_crypto::{blake3_hash, random_bytes};
use nonos_types::NonosResult;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

#[derive(Clone, Debug)]
pub struct StealthSession {
    pub stealth_address: [u8; 20],
    pub ephemeral_pubkey: [u8; 33],
    session_secret: [u8; 32],
    pub session_id: [u8; 16],
    pub created_at: u64,
    pub visited_domains: Vec<[u8; 32]>,
}

impl StealthSession {
    pub fn is_expired(&self, max_age_secs: u64) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now > self.created_at + max_age_secs
    }

    pub fn visit_count(&self) -> usize {
        self.visited_domains.len()
    }
}

impl Drop for StealthSession {
    fn drop(&mut self) {
        self.session_secret = [0u8; 32];
    }
}

pub struct StealthSessionManager {
    sessions: Arc<RwLock<HashMap<[u8; 16], StealthSession>>>,
    stealth_spend_pubkey: Option<[u8; 33]>,
    stealth_view_pubkey: Option<[u8; 33]>,
    max_session_age_secs: u64,
}

impl StealthSessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            stealth_spend_pubkey: None,
            stealth_view_pubkey: None,
            max_session_age_secs: 3600,
        }
    }

    pub fn with_max_age(max_age_secs: u64) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            stealth_spend_pubkey: None,
            stealth_view_pubkey: None,
            max_session_age_secs: max_age_secs,
        }
    }

    pub fn init_stealth_keys(&mut self, spend_pubkey: [u8; 33], view_pubkey: [u8; 33]) {
        self.stealth_spend_pubkey = Some(spend_pubkey);
        self.stealth_view_pubkey = Some(view_pubkey);
    }

    pub async fn create_session(&self) -> NonosResult<StealthSession> {
        let ephemeral_secret = random_bytes::<32>();
        let ephemeral_pubkey = blake3_hash(&ephemeral_secret);
        let mut pubkey = [0u8; 33];
        pubkey[0] = 0x02;
        pubkey[1..].copy_from_slice(&ephemeral_pubkey.0);

        let stealth_secret = random_bytes::<32>();
        let stealth_hash = blake3_hash(&stealth_secret);
        let mut stealth_address = [0u8; 20];
        stealth_address.copy_from_slice(&stealth_hash.0[..20]);

        let session_id = random_bytes::<16>();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let session = StealthSession {
            stealth_address,
            ephemeral_pubkey: pubkey,
            session_secret: stealth_secret,
            session_id,
            created_at: now,
            visited_domains: Vec::new(),
        };

        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id, session.clone());

        info!("Created stealth session: {:?}", hex::encode(&session_id));
        Ok(session)
    }

    pub async fn get_session(&self, session_id: &[u8; 16]) -> Option<StealthSession> {
        self.sessions.read().await.get(session_id).cloned()
    }

    pub async fn record_visit(&self, session_id: &[u8; 16], domain: &str) -> NonosResult<()> {
        let domain_hash = blake3_hash(domain.as_bytes());
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.visited_domains.push(domain_hash.0);
        }
        Ok(())
    }

    pub async fn end_session(&self, session_id: &[u8; 16]) -> NonosResult<()> {
        let mut sessions = self.sessions.write().await;
        sessions.remove(session_id);
        info!("Ended stealth session: {:?}", hex::encode(session_id));
        Ok(())
    }

    pub async fn active_session_count(&self) -> usize {
        self.sessions.read().await.len()
    }

    pub async fn cleanup_expired(&self) {
        let mut sessions = self.sessions.write().await;
        sessions.retain(|_, session| !session.is_expired(self.max_session_age_secs));
    }

    pub async fn session_stats(&self) -> (usize, usize) {
        let sessions = self.sessions.read().await;
        let total_visits: usize = sessions.values().map(|s| s.visited_domains.len()).sum();
        (sessions.len(), total_visits)
    }
}

impl Default for StealthSessionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_stealth_session_creation() {
        let manager = StealthSessionManager::new();
        let session = manager.create_session().await.unwrap();
        assert_ne!(session.stealth_address, [0u8; 20]);
        assert_eq!(manager.active_session_count().await, 1);
    }

    #[tokio::test]
    async fn test_stealth_session_visit() {
        let manager = StealthSessionManager::new();
        let session = manager.create_session().await.unwrap();
        manager.record_visit(&session.session_id, "example.com").await.unwrap();

        let updated = manager.get_session(&session.session_id).await.unwrap();
        assert_eq!(updated.visited_domains.len(), 1);
    }

    #[tokio::test]
    async fn test_stealth_session_end() {
        let manager = StealthSessionManager::new();
        let session = manager.create_session().await.unwrap();
        manager.end_session(&session.session_id).await.unwrap();
        assert_eq!(manager.active_session_count().await, 0);
    }
}
