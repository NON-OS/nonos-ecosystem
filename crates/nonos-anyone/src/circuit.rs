use nonos_types::{
    CircuitId, CircuitInfo, CircuitStatus, ConnectionStatus, NetworkStatus,
    NonosResult,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

pub struct CircuitManager {
    circuits: Arc<RwLock<HashMap<CircuitId, CircuitInfo>>>,
    default_circuit: Arc<RwLock<Option<CircuitId>>>,
    assignments: Arc<RwLock<HashMap<String, CircuitId>>>,
    status: Arc<RwLock<ConnectionStatus>>,
    bootstrap_progress: Arc<RwLock<u8>>,
}

impl CircuitManager {
    pub fn new() -> Self {
        Self {
            circuits: Arc::new(RwLock::new(HashMap::new())),
            default_circuit: Arc::new(RwLock::new(None)),
            assignments: Arc::new(RwLock::new(HashMap::new())),
            status: Arc::new(RwLock::new(ConnectionStatus::Disconnected)),
            bootstrap_progress: Arc::new(RwLock::new(0)),
        }
    }

    pub async fn status(&self) -> ConnectionStatus {
        *self.status.read().await
    }

    pub async fn bootstrap_progress(&self) -> u8 {
        *self.bootstrap_progress.read().await
    }

    pub async fn set_status(&self, status: ConnectionStatus) {
        *self.status.write().await = status;
    }

    pub async fn set_bootstrap_progress(&self, progress: u8) {
        *self.bootstrap_progress.write().await = progress.min(100);
    }

    pub async fn create_circuit(&self) -> NonosResult<CircuitId> {
        let id = CircuitId::new();

        let circuit = CircuitInfo {
            id,
            path: Vec::new(),
            status: CircuitStatus::Building,
            created_at: chrono::Utc::now(),
            bytes_sent: 0,
            bytes_received: 0,
        };

        self.circuits.write().await.insert(id, circuit);
        debug!("Created circuit: {:?}", id);

        Ok(id)
    }

    pub async fn get_circuit(&self, id: &CircuitId) -> Option<CircuitInfo> {
        self.circuits.read().await.get(id).cloned()
    }

    pub async fn active_circuits(&self) -> Vec<CircuitInfo> {
        self.circuits
            .read()
            .await
            .values()
            .filter(|c| c.status == CircuitStatus::Ready || c.status == CircuitStatus::Active)
            .cloned()
            .collect()
    }

    pub async fn circuit_count(&self) -> usize {
        self.circuits.read().await.len()
    }

    pub async fn update_circuit_status(&self, id: &CircuitId, status: CircuitStatus) {
        if let Some(circuit) = self.circuits.write().await.get_mut(id) {
            circuit.status = status;
        }
    }

    pub async fn close_circuit(&self, id: &CircuitId) -> NonosResult<()> {
        self.update_circuit_status(id, CircuitStatus::Closing).await;

        self.assignments
            .write()
            .await
            .retain(|_, circuit_id| circuit_id != id);

        self.update_circuit_status(id, CircuitStatus::Closed).await;

        debug!("Closed circuit: {:?}", id);
        Ok(())
    }

    pub async fn get_circuit_for_domain(&self, domain: &str) -> NonosResult<CircuitId> {
        if let Some(id) = self.assignments.read().await.get(domain) {
            if let Some(circuit) = self.get_circuit(id).await {
                if circuit.status == CircuitStatus::Ready || circuit.status == CircuitStatus::Active
                {
                    return Ok(*id);
                }
            }
        }

        let id = self.create_circuit().await?;

        self.update_circuit_status(&id, CircuitStatus::Ready).await;

        self.assignments
            .write()
            .await
            .insert(domain.to_string(), id);

        Ok(id)
    }

    pub async fn default_circuit(&self) -> NonosResult<CircuitId> {
        if let Some(id) = *self.default_circuit.read().await {
            if let Some(circuit) = self.get_circuit(&id).await {
                if circuit.status == CircuitStatus::Ready || circuit.status == CircuitStatus::Active
                {
                    return Ok(id);
                }
            }
        }

        let id = self.create_circuit().await?;
        self.update_circuit_status(&id, CircuitStatus::Ready).await;
        *self.default_circuit.write().await = Some(id);

        Ok(id)
    }

    pub async fn new_identity(&self) -> NonosResult<()> {
        info!("Rotating all circuits for new identity");

        let circuit_ids: Vec<CircuitId> = self.circuits.read().await.keys().copied().collect();

        for id in circuit_ids {
            self.close_circuit(&id).await?;
        }

        self.assignments.write().await.clear();

        *self.default_circuit.write().await = None;

        self.circuits
            .write()
            .await
            .retain(|_, c| c.status != CircuitStatus::Closed);

        Ok(())
    }

    pub async fn network_status(&self) -> NetworkStatus {
        let status = *self.status.read().await;
        let bootstrap = *self.bootstrap_progress.read().await;
        let circuits = self.circuits.read().await;

        let active_count = circuits
            .values()
            .filter(|c| c.status == CircuitStatus::Ready || c.status == CircuitStatus::Active)
            .count() as u32;

        let (bytes_sent, bytes_received) = circuits.values().fold((0u64, 0u64), |acc, c| {
            (acc.0 + c.bytes_sent, acc.1 + c.bytes_received)
        });

        NetworkStatus {
            status,
            bootstrap_progress: bootstrap,
            active_circuits: active_count,
            bytes_sent,
            bytes_received,
            socks_port: Some(9150),
        }
    }

    pub async fn update_traffic(&self, id: &CircuitId, sent: u64, received: u64) {
        if let Some(circuit) = self.circuits.write().await.get_mut(id) {
            circuit.bytes_sent += sent;
            circuit.bytes_received += received;
        }
    }

    pub async fn cleanup_stale_circuits(&self, max_age_secs: i64) {
        let now = chrono::Utc::now();
        let mut to_close = Vec::new();

        for (id, circuit) in self.circuits.read().await.iter() {
            let age = now
                .signed_duration_since(circuit.created_at)
                .num_seconds();
            if age > max_age_secs {
                to_close.push(*id);
            }
        }

        for id in to_close {
            if let Err(e) = self.close_circuit(&id).await {
                warn!("Failed to close stale circuit {:?}: {}", id, e);
            }
        }
    }
}

impl Default for CircuitManager {
    fn default() -> Self {
        Self::new()
    }
}

pub struct CircuitBuilder {
    entry: Option<String>,
    middle: Vec<String>,
    exit: Option<String>,
    exclude_countries: Vec<String>,
    require_countries: Vec<String>,
}

impl CircuitBuilder {
    pub fn new() -> Self {
        Self {
            entry: None,
            middle: Vec::new(),
            exit: None,
            exclude_countries: Vec::new(),
            require_countries: Vec::new(),
        }
    }

    pub fn with_entry(mut self, fingerprint: impl Into<String>) -> Self {
        self.entry = Some(fingerprint.into());
        self
    }

    pub fn with_middle(mut self, fingerprint: impl Into<String>) -> Self {
        self.middle.push(fingerprint.into());
        self
    }

    pub fn with_exit(mut self, fingerprint: impl Into<String>) -> Self {
        self.exit = Some(fingerprint.into());
        self
    }

    pub fn exclude_countries(mut self, countries: Vec<String>) -> Self {
        self.exclude_countries = countries;
        self
    }

    pub fn require_exit_in(mut self, countries: Vec<String>) -> Self {
        self.require_countries = countries;
        self
    }
}

impl Default for CircuitBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_creation() {
        let manager = CircuitManager::new();

        let id = manager.create_circuit().await.unwrap();
        let circuit = manager.get_circuit(&id).await.unwrap();

        assert_eq!(circuit.status, CircuitStatus::Building);
    }

    #[tokio::test]
    async fn test_domain_isolation() {
        let manager = CircuitManager::new();

        let id1 = manager.get_circuit_for_domain("example.com").await.unwrap();
        let id2 = manager.get_circuit_for_domain("example.com").await.unwrap();
        let id3 = manager.get_circuit_for_domain("other.com").await.unwrap();

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[tokio::test]
    async fn test_new_identity() {
        let manager = CircuitManager::new();

        let _id1 = manager.create_circuit().await.unwrap();
        let _id2 = manager.create_circuit().await.unwrap();

        assert_eq!(manager.circuit_count().await, 2);

        manager.new_identity().await.unwrap();

        let active = manager.active_circuits().await;
        assert!(active.is_empty());
    }

    #[tokio::test]
    async fn test_network_status() {
        let manager = CircuitManager::new();
        manager.set_status(ConnectionStatus::Connected).await;
        manager.set_bootstrap_progress(100).await;

        let status = manager.network_status().await;
        assert_eq!(status.status, ConnectionStatus::Connected);
        assert_eq!(status.bootstrap_progress, 100);
    }
}
