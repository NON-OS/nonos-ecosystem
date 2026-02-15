use nonos_crypto::{poseidon_commitment, compute_nullifier, PoseidonMerkleTree};
use nonos_types::{NodeId, NonosResult};
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{debug, info};

pub struct ZkIdentityService {
    _node_id: NodeId,
    identity_tree: Arc<RwLock<PoseidonMerkleTree>>,
    nullifiers: Arc<RwLock<HashSet<[u8; 32]>>>,
    proofs_issued: AtomicU64,
    verifications_done: AtomicU64,
}

impl ZkIdentityService {
    pub fn new(node_id: NodeId) -> Self {
        Self {
            _node_id: node_id,
            identity_tree: Arc::new(RwLock::new(PoseidonMerkleTree::new(20))),
            nullifiers: Arc::new(RwLock::new(HashSet::new())),
            proofs_issued: AtomicU64::new(0),
            verifications_done: AtomicU64::new(0),
        }
    }

    pub async fn register_identity(&self, commitment: [u8; 32]) -> NonosResult<usize> {
        let mut tree = self.identity_tree.write().await;
        let index = tree.insert(commitment);
        info!("Registered identity commitment at index {}", index);
        Ok(index)
    }

    pub async fn verify_identity_proof(
        &self,
        commitment: &[u8; 32],
        proof: &[([u8; 32], bool)],
        nullifier: &[u8; 32],
    ) -> NonosResult<bool> {
        {
            let nullifiers = self.nullifiers.read().await;
            if nullifiers.contains(nullifier) {
                debug!("Identity proof rejected: nullifier already used");
                return Ok(false);
            }
        }

        let tree = self.identity_tree.read().await;
        let root = tree.root();
        let valid = PoseidonMerkleTree::verify_proof(commitment, proof, &root);

        if valid {
            let mut nullifiers = self.nullifiers.write().await;
            nullifiers.insert(*nullifier);
            self.verifications_done.fetch_add(1, Ordering::Relaxed);
            debug!("Identity proof verified successfully");
        }

        Ok(valid)
    }

    pub fn create_commitment(secret: &[u8], blinding: &[u8; 32]) -> [u8; 32] {
        poseidon_commitment(secret, blinding)
    }

    pub fn create_nullifier(spending_key: &[u8; 32], commitment: &[u8; 32]) -> [u8; 32] {
        compute_nullifier(spending_key, commitment)
    }

    pub async fn tree_root(&self) -> [u8; 32] {
        self.identity_tree.read().await.root()
    }

    pub async fn nullifier_count(&self) -> usize {
        self.nullifiers.read().await.len()
    }

    pub fn stats(&self) -> (u64, u64) {
        (
            self.proofs_issued.load(Ordering::Relaxed),
            self.verifications_done.load(Ordering::Relaxed),
        )
    }

    pub async fn run(self: Arc<Self>, shutdown: Arc<AtomicBool>) -> NonosResult<()> {
        info!("ZK Identity service started");
        let mut ticker = interval(Duration::from_secs(60));

        while !shutdown.load(Ordering::Relaxed) {
            ticker.tick().await;
            let (issued, verified) = self.stats();
            debug!("ZK Identity: {} issued, {} verified", issued, verified);
        }

        info!("ZK Identity service stopped");
        Ok(())
    }
}
