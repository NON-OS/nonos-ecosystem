use super::*;
use nonos_types::NodeId;

#[tokio::test]
async fn test_zk_identity_service() {
    let service = ZkIdentityService::new(NodeId::from_bytes(rand::random()));
    let secret = b"test_secret";
    let blinding: [u8; 32] = rand::random();
    let commitment = ZkIdentityService::create_commitment(secret, &blinding);
    let index = service.register_identity(commitment).await.unwrap();
    assert_eq!(index, 0);
    assert_ne!(service.tree_root().await, [0u8; 32]);
}

#[tokio::test]
async fn test_cache_mixing() {
    let service = CacheMixingService::new(NodeId::from_bytes(rand::random()), 100);
    let hash: [u8; 32] = rand::random();
    let data = b"test data".to_vec();

    let commitment = service.store_mixed(hash, data.clone()).await.unwrap();
    assert_ne!(commitment, [0u8; 32]);

    let retrieved = service.retrieve_mixed(&hash).await.unwrap();
    assert_eq!(retrieved, data);

    let (hits, misses, ops) = service.stats();
    assert_eq!(hits, 1);
    assert_eq!(misses, 0);
    assert_eq!(ops, 1);
}

#[tokio::test]
async fn test_tracking_blocker() {
    let service = TrackingBlockerService::new(NodeId::from_bytes(rand::random()));

    assert!(service.should_block_domain("google-analytics.com").await);
    assert!(service.should_block_domain("connect.facebook.net").await);
    assert!(!service.should_block_domain("example.com").await);
    assert!(!service.should_block_domain("rust-lang.org").await);
    assert!(service.should_block_domain("www.doubleclick.net").await);
}

#[tokio::test]
async fn test_param_stripping() {
    let service = TrackingBlockerService::new(NodeId::from_bytes(rand::random()));

    let url = "https://example.com/page?foo=bar&utm_source=test&id=123";
    let cleaned = service.strip_tracking_params(url).await;
    assert!(cleaned.contains("foo=bar"));
    assert!(cleaned.contains("id=123"));
    assert!(!cleaned.contains("utm_source"));
}
