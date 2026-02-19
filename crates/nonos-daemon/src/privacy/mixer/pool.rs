use super::note::Note;
use super::types::{AssetId, SpendRequest, SpendResult};
use nonos_crypto::poseidon_canonical::{bytes_to_fr, fr_to_bytes, poseidon_hash_fields, PoseidonMerkleTree};
use nonos_types::{NonosError, NonosResult};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

const MAX_NULLIFIERS: usize = 1_000_000;
const MAX_NOTES: usize = 1_048_576;
const MAX_ACCEPTED_ROOTS: usize = 256;

pub struct NoteMixer {
    tree: Arc<RwLock<PoseidonMerkleTree>>,
    nullifiers: Arc<RwLock<HashSet<[u8; 32]>>>,
    nullifier_order: Arc<RwLock<VecDeque<[u8; 32]>>>,
    commitment_index: Arc<RwLock<HashMap<[u8; 32], usize>>>,
    accepted_roots: Arc<RwLock<VecDeque<[u8; 32]>>>,
    production_mode: AtomicBool,
    deposits: AtomicU64,
    spends: AtomicU64,
    failed_spends: AtomicU64,
    tvl: Arc<RwLock<HashMap<AssetId, u128>>>,
}

impl NoteMixer {
    pub fn new() -> Self {
        let tree = PoseidonMerkleTree::new(20);
        let initial_root = tree.root();
        let mut accepted = VecDeque::new();
        accepted.push_back(initial_root);

        Self {
            tree: Arc::new(RwLock::new(tree)),
            nullifiers: Arc::new(RwLock::new(HashSet::with_capacity(1024))),
            nullifier_order: Arc::new(RwLock::new(VecDeque::with_capacity(1024))),
            commitment_index: Arc::new(RwLock::new(HashMap::new())),
            accepted_roots: Arc::new(RwLock::new(accepted)),
            production_mode: AtomicBool::new(false),
            deposits: AtomicU64::new(0),
            spends: AtomicU64::new(0),
            failed_spends: AtomicU64::new(0),
            tvl: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn set_production_mode(&self, enabled: bool) {
        self.production_mode.store(enabled, Ordering::SeqCst);
        if enabled {
            info!("Note Mixer: production mode enabled");
        }
    }

    pub async fn deposit(&self, note: &mut Note) -> NonosResult<usize> {
        {
            let index = self.commitment_index.read().await;
            if index.len() >= MAX_NOTES {
                return Err(NonosError::Internal("Mixer pool full".into()));
            }
            if index.contains_key(&note.commitment()) {
                return Err(NonosError::Internal("Note already deposited".into()));
            }
        }

        let tree_index = {
            let mut tree = self.tree.write().await;
            tree.insert(note.commitment())
        };

        let new_root = {
            let tree = self.tree.read().await;
            tree.root()
        };

        {
            let mut roots = self.accepted_roots.write().await;
            roots.push_back(new_root);
            if roots.len() > MAX_ACCEPTED_ROOTS {
                roots.pop_front();
            }
        }

        {
            let mut index = self.commitment_index.write().await;
            index.insert(note.commitment(), tree_index);
        }

        {
            let mut tvl = self.tvl.write().await;
            *tvl.entry(note.public.asset).or_insert(0) = tvl
                .get(&note.public.asset)
                .unwrap_or(&0)
                .saturating_add(note.public.amount);
        }

        note.set_tree_index(tree_index);
        self.deposits.fetch_add(1, Ordering::Relaxed);
        Ok(tree_index)
    }

    pub async fn get_proof(&self, commitment: &[u8; 32]) -> NonosResult<Vec<([u8; 32], bool)>> {
        let index = {
            let idx = self.commitment_index.read().await;
            *idx.get(commitment)
                .ok_or_else(|| NonosError::Internal("Note not found".into()))?
        };

        let tree = self.tree.read().await;
        Ok(tree.proof(index))
    }

    pub async fn root(&self) -> [u8; 32] {
        self.tree.read().await.root()
    }

    pub async fn is_root_accepted(&self, root: &[u8; 32]) -> bool {
        let roots = self.accepted_roots.read().await;
        roots.iter().any(|r| r == root)
    }

    pub async fn is_spent(&self, nullifier: &[u8; 32]) -> bool {
        self.nullifiers.read().await.contains(nullifier)
    }

    pub async fn spend(&self, request: &SpendRequest) -> NonosResult<SpendResult> {
        if self.is_spent(&request.nullifier).await {
            self.failed_spends.fetch_add(1, Ordering::Relaxed);
            warn!("Spend rejected: nullifier already spent");
            return Ok(SpendResult {
                success: false,
                reason: Some("Double-spend attempt".into()),
                tx_hash: None,
            });
        }

        if !self.is_root_accepted(&request.merkle_root).await {
            self.failed_spends.fetch_add(1, Ordering::Relaxed);
            warn!("Spend rejected: invalid Merkle root");
            return Ok(SpendResult {
                success: false,
                reason: Some("Invalid Merkle root".into()),
                tx_hash: None,
            });
        }

        if self.production_mode.load(Ordering::SeqCst) && request.proof.is_empty() {
            self.failed_spends.fetch_add(1, Ordering::Relaxed);
            error!("Production mode: proof required");
            return Ok(SpendResult {
                success: false,
                reason: Some("Proof required in production".into()),
                tx_hash: None,
            });
        }

        {
            let mut nullifiers = self.nullifiers.write().await;
            let mut order = self.nullifier_order.write().await;

            while nullifiers.len() >= MAX_NULLIFIERS {
                if let Some(old) = order.pop_front() {
                    nullifiers.remove(&old);
                }
            }

            nullifiers.insert(request.nullifier);
            order.push_back(request.nullifier);
        }

        self.spends.fetch_add(1, Ordering::Relaxed);

        let tx_hash = {
            let result = poseidon_hash_fields(&[
                bytes_to_fr(&request.nullifier),
                bytes_to_fr(&request.recipient),
            ]);
            fr_to_bytes(&result)
        };

        Ok(SpendResult {
            success: true,
            reason: None,
            tx_hash: Some(tx_hash),
        })
    }

    pub fn stats(&self) -> (u64, u64, u64) {
        (
            self.deposits.load(Ordering::Relaxed),
            self.spends.load(Ordering::Relaxed),
            self.failed_spends.load(Ordering::Relaxed),
        )
    }

    pub async fn tvl(&self, asset: &AssetId) -> u128 {
        *self.tvl.read().await.get(asset).unwrap_or(&0)
    }

    pub async fn note_count(&self) -> usize {
        self.commitment_index.read().await.len()
    }

    pub async fn spent_count(&self) -> usize {
        self.nullifiers.read().await.len()
    }
}

impl Default for NoteMixer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::note::Note;
    use super::super::types::ASSET_NOX;
    use rand::RngCore;

    fn random_bytes<const N: usize>() -> [u8; N] {
        let mut bytes = [0u8; N];
        rand::thread_rng().fill_bytes(&mut bytes);
        bytes
    }

    #[tokio::test]
    async fn test_deposit() {
        let mixer = NoteMixer::new();
        let mut note = Note::new(random_bytes(), 1000, ASSET_NOX, random_bytes());

        let index = mixer.deposit(&mut note).await.unwrap();
        assert_eq!(index, 0);
        assert_eq!(mixer.tvl(&ASSET_NOX).await, 1000);
    }

    #[tokio::test]
    async fn test_duplicate_deposit_fails() {
        let mixer = NoteMixer::new();
        let mut note = Note::new(random_bytes(), 1000, ASSET_NOX, random_bytes());

        mixer.deposit(&mut note).await.unwrap();
        assert!(mixer.deposit(&mut note).await.is_err());
    }

    #[tokio::test]
    async fn test_spend() {
        let mixer = NoteMixer::new();
        let mut note = Note::new(random_bytes(), 1000, ASSET_NOX, random_bytes());
        mixer.deposit(&mut note).await.unwrap();

        let root = mixer.root().await;
        let proof = mixer.get_proof(&note.commitment()).await.unwrap();

        let request = SpendRequest {
            merkle_root: root,
            nullifier: note.nullifier(),
            recipient: random_bytes(),
            fee: 10,
            merkle_path: proof,
            proof: vec![],
        };

        let result = mixer.spend(&request).await.unwrap();
        assert!(result.success);
        assert!(mixer.is_spent(&note.nullifier()).await);
    }

    #[tokio::test]
    async fn test_double_spend_fails() {
        let mixer = NoteMixer::new();
        let mut note = Note::new(random_bytes(), 1000, ASSET_NOX, random_bytes());
        mixer.deposit(&mut note).await.unwrap();

        let root = mixer.root().await;
        let proof = mixer.get_proof(&note.commitment()).await.unwrap();

        let request = SpendRequest {
            merkle_root: root,
            nullifier: note.nullifier(),
            recipient: random_bytes(),
            fee: 10,
            merkle_path: proof,
            proof: vec![],
        };

        assert!(mixer.spend(&request).await.unwrap().success);
        assert!(!mixer.spend(&request).await.unwrap().success);
    }

    #[tokio::test]
    async fn test_wrong_root_fails() {
        let mixer = NoteMixer::new();
        let mut note = Note::new(random_bytes(), 1000, ASSET_NOX, random_bytes());
        mixer.deposit(&mut note).await.unwrap();

        let request = SpendRequest {
            merkle_root: [0xff; 32],
            nullifier: note.nullifier(),
            recipient: random_bytes(),
            fee: 10,
            merkle_path: vec![],
            proof: vec![],
        };

        assert!(!mixer.spend(&request).await.unwrap().success);
    }
}
