use super::{HealthBeacon, QualityOracle, BootstrapService, CacheService};
use crate::{NodeMetricsCollector, P2pNetwork, NodeStorage};
use nonos_types::{NodeId, NonosResult};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{error, info};

pub struct ServiceManager {
    states: Arc<RwLock<HashMap<ServiceType, ServiceState>>>,
    shutdown: Arc<AtomicBool>,
    handles: Arc<RwLock<Vec<tokio::task::JoinHandle<()>>>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ServiceType {
    HealthBeacon,
    QualityOracle,
    Bootstrap,
    Cache,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServiceState {
    Stopped,
    Starting,
    Running,
    Failed,
}

impl ServiceManager {
    pub fn new() -> Self {
        let mut states = HashMap::new();
        states.insert(ServiceType::HealthBeacon, ServiceState::Stopped);
        states.insert(ServiceType::QualityOracle, ServiceState::Stopped);
        states.insert(ServiceType::Bootstrap, ServiceState::Stopped);
        states.insert(ServiceType::Cache, ServiceState::Stopped);

        Self {
            states: Arc::new(RwLock::new(states)),
            shutdown: Arc::new(AtomicBool::new(false)),
            handles: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn start_all(
        &mut self,
        node_id: NodeId,
        network: Arc<RwLock<P2pNetwork>>,
        metrics: Arc<NodeMetricsCollector>,
        storage: Arc<NodeStorage>,
        config: ServiceConfig,
    ) -> NonosResult<()> {
        self.shutdown.store(false, Ordering::SeqCst);

        if config.health_beacon {
            self.start_service(ServiceType::HealthBeacon, {
                let beacon = HealthBeacon::new(node_id, network.clone(), metrics.clone());
                let shutdown = self.shutdown.clone();
                async move { beacon.run(shutdown).await }
            }).await;
        }

        if config.quality_oracle {
            self.start_service(ServiceType::QualityOracle, {
                let oracle = QualityOracle::new(node_id, metrics.clone(), storage.clone());
                let shutdown = self.shutdown.clone();
                async move { oracle.run(shutdown).await }
            }).await;
        }

        if config.bootstrap {
            self.start_service(ServiceType::Bootstrap, {
                let bootstrap = BootstrapService::new(network.clone(), storage.clone(), config.bootstrap_port);
                let shutdown = self.shutdown.clone();
                async move { bootstrap.run(shutdown).await }
            }).await;
        }

        if config.cache {
            self.start_service(ServiceType::Cache, {
                let cache = CacheService::new(storage.clone(), config.cache_size_mb);
                let shutdown = self.shutdown.clone();
                async move { cache.run(shutdown).await }
            }).await;
        }

        Ok(())
    }

    async fn start_service<F>(&mut self, service_type: ServiceType, task: F)
    where
        F: std::future::Future<Output = NonosResult<()>> + Send + 'static,
    {
        *self.states.write().await.get_mut(&service_type).unwrap() = ServiceState::Starting;

        let states = self.states.clone();
        let handle = tokio::spawn(async move {
            if let Err(e) = task.await {
                error!("{:?} failed: {}", service_type, e);
                *states.write().await.get_mut(&service_type).unwrap() = ServiceState::Failed;
            }
        });

        self.handles.write().await.push(handle);
        *self.states.write().await.get_mut(&service_type).unwrap() = ServiceState::Running;
        info!("{:?} started", service_type);
    }

    pub async fn stop_all(&mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
        tokio::time::sleep(Duration::from_millis(100)).await;

        let handles = std::mem::take(&mut *self.handles.write().await);
        for handle in handles {
            handle.abort();
        }

        for state in self.states.write().await.values_mut() {
            *state = ServiceState::Stopped;
        }

        info!("All services stopped");
    }

    pub async fn get_state(&self, service: ServiceType) -> ServiceState {
        *self.states.read().await.get(&service).unwrap_or(&ServiceState::Stopped)
    }

    pub async fn all_running(&self) -> bool {
        self.states.read().await.values().all(|s| *s == ServiceState::Running || *s == ServiceState::Stopped)
    }
}

impl Default for ServiceManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug)]
pub struct ServiceConfig {
    pub health_beacon: bool,
    pub quality_oracle: bool,
    pub bootstrap: bool,
    pub cache: bool,
    pub bootstrap_port: u16,
    pub cache_size_mb: u32,
    pub beacon_interval_secs: u64,
    pub quality_interval_secs: u64,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            health_beacon: true,
            quality_oracle: true,
            bootstrap: false,
            cache: false,
            bootstrap_port: 9735,
            cache_size_mb: 1024,
            beacon_interval_secs: 60,
            quality_interval_secs: 300,
        }
    }
}
