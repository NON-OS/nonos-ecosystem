use super::*;
use crate::NodeStorage;
use nonos_types::Blake3Hash;
use std::sync::Arc;

#[tokio::test]
async fn test_service_manager() {
    let manager = ServiceManager::new();
    assert_eq!(manager.get_state(ServiceType::HealthBeacon).await, ServiceState::Stopped);
    assert_eq!(manager.get_state(ServiceType::QualityOracle).await, ServiceState::Stopped);
}

#[tokio::test]
async fn test_cache_service() {
    let storage = Arc::new(NodeStorage::open_memory().unwrap());
    let cache = CacheService::new(storage, 10);

    let hash = Blake3Hash::zero();
    let data = vec![1, 2, 3, 4, 5];

    cache.put(hash, data.clone(), 3600).await;

    let retrieved = cache.get(&hash).await;
    assert_eq!(retrieved, Some(data));

    let stats = cache.stats().await;
    assert_eq!(stats.entry_count, 1);
    assert_eq!(stats.total_size, 5);
}

#[tokio::test]
async fn test_cache_eviction() {
    let storage = Arc::new(NodeStorage::open_memory().unwrap());
    let cache = CacheService::new(storage, 1);

    for i in 0..100u8 {
        let mut hash = Blake3Hash::zero();
        hash.0[0] = i;
        let data = vec![0u8; 20000];
        cache.put(hash, data, 3600).await;
    }

    let stats = cache.stats().await;
    assert!(stats.total_size <= 1048576);
}
