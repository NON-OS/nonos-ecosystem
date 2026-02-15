use nonos_crypto::random_bytes;
use nonos_types::{NonosError, NonosResult};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use super::gf256;
use super::types::SecretShare;

#[derive(Debug)]
pub struct DistributedCookieVault {
    threshold: u8,
    total_shares: u8,
    shares: Arc<RwLock<HashMap<String, Vec<SecretShare>>>>,
}

impl DistributedCookieVault {
    pub fn new(threshold: u8, total_shares: u8) -> NonosResult<Self> {
        if threshold == 0 {
            return Err(NonosError::Config("Threshold must be > 0".into()));
        }
        if threshold > total_shares {
            return Err(NonosError::Config(
                "Threshold must be <= total_shares".into(),
            ));
        }

        Ok(Self {
            threshold,
            total_shares,
            shares: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub fn threshold(&self) -> u8 {
        self.threshold
    }

    pub fn total_shares(&self) -> u8 {
        self.total_shares
    }

    pub async fn split_cookie(
        &self,
        cookie_id: &str,
        cookie_value: &[u8],
        node_ids: &[[u8; 32]],
    ) -> NonosResult<Vec<SecretShare>> {
        if node_ids.len() < self.total_shares as usize {
            return Err(NonosError::Internal(format!(
                "Not enough nodes for sharing: need {}, got {}",
                self.total_shares,
                node_ids.len()
            )));
        }

        let mut shares: Vec<SecretShare> = (0..self.total_shares)
            .map(|i| SecretShare {
                index: i + 1,
                value: Vec::with_capacity(cookie_value.len()),
                node_id: node_ids[i as usize],
            })
            .collect();

        for &secret_byte in cookie_value {
            let mut coefficients = vec![secret_byte];
            let random = random_bytes::<32>();
            for i in 1..self.threshold as usize {
                coefficients.push(random[i % 32]);
            }

            for share in &mut shares {
                let y = gf256::eval_poly(&coefficients, share.index);
                share.value.push(y);
            }
        }

        {
            let mut storage = self.shares.write().await;
            storage.insert(cookie_id.to_string(), shares.clone());
        }

        info!(
            "Split cookie '{}' ({} bytes) into {} shares (threshold: {})",
            cookie_id,
            cookie_value.len(),
            self.total_shares,
            self.threshold
        );

        Ok(shares)
    }

    pub async fn reconstruct_cookie(
        &self,
        cookie_id: &str,
        shares: &[SecretShare],
    ) -> NonosResult<Vec<u8>> {
        if shares.len() < self.threshold as usize {
            return Err(NonosError::Crypto(format!(
                "Insufficient shares: need {}, got {}",
                self.threshold,
                shares.len()
            )));
        }

        let value_len = shares[0].value.len();
        if !shares.iter().all(|s| s.value.len() == value_len) {
            return Err(NonosError::Crypto(
                "Share value lengths do not match".to_string(),
            ));
        }

        let mut seen_indices = std::collections::HashSet::new();
        for share in shares {
            if share.index == 0 {
                return Err(NonosError::Crypto(
                    "Share index cannot be zero".to_string(),
                ));
            }
            if !seen_indices.insert(share.index) {
                return Err(NonosError::Crypto(format!(
                    "Duplicate share index: {}",
                    share.index
                )));
            }
        }

        let mut secret = Vec::with_capacity(value_len);

        for byte_idx in 0..value_len {
            let points: Vec<(u8, u8)> = shares
                .iter()
                .take(self.threshold as usize)
                .map(|s| (s.index, s.value[byte_idx]))
                .collect();

            let secret_byte = gf256::interpolate_at_zero(&points).ok_or_else(|| {
                NonosError::Crypto("Interpolation failed - possible duplicate share indices".into())
            })?;
            secret.push(secret_byte);
        }

        info!(
            "Reconstructed cookie '{}' ({} bytes) from {} shares",
            cookie_id,
            secret.len(),
            shares.len()
        );

        Ok(secret)
    }

    pub async fn get_shares(&self, cookie_id: &str) -> Option<Vec<SecretShare>> {
        self.shares.read().await.get(cookie_id).cloned()
    }

    pub async fn remove_shares(&self, cookie_id: &str) -> Option<Vec<SecretShare>> {
        self.shares.write().await.remove(cookie_id)
    }

    pub async fn stored_cookie_count(&self) -> usize {
        self.shares.read().await.len()
    }

    pub async fn clear(&self) {
        self.shares.write().await.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_nodes(count: u8) -> Vec<[u8; 32]> {
        (0..count)
            .map(|i| {
                let mut arr = [0u8; 32];
                arr[0] = i;
                arr
            })
            .collect()
    }

    #[tokio::test]
    async fn test_split_and_reconstruct_exact_threshold() {
        let vault = DistributedCookieVault::new(3, 5).unwrap();
        let cookie = b"secret_session_cookie_value_12345";
        let nodes = make_nodes(5);

        let shares = vault
            .split_cookie("test_cookie", cookie, &nodes)
            .await
            .unwrap();
        assert_eq!(shares.len(), 5);

        let result = vault
            .reconstruct_cookie("test_cookie", &shares[0..3])
            .await
            .unwrap();
        assert_eq!(result, cookie.to_vec());
    }

    #[tokio::test]
    async fn test_split_and_reconstruct_all_shares() {
        let vault = DistributedCookieVault::new(3, 5).unwrap();
        let cookie = b"another secret value";
        let nodes = make_nodes(5);

        let shares = vault
            .split_cookie("test_cookie2", cookie, &nodes)
            .await
            .unwrap();

        let result = vault
            .reconstruct_cookie("test_cookie2", &shares)
            .await
            .unwrap();
        assert_eq!(result, cookie.to_vec());
    }

    #[tokio::test]
    async fn test_reconstruct_with_different_share_subsets() {
        let vault = DistributedCookieVault::new(3, 5).unwrap();
        let cookie = b"test secret data";
        let nodes = make_nodes(5);

        let shares = vault
            .split_cookie("test_cookie3", cookie, &nodes)
            .await
            .unwrap();

        let subsets = [
            vec![0, 1, 2],
            vec![0, 1, 3],
            vec![0, 1, 4],
            vec![0, 2, 3],
            vec![0, 2, 4],
            vec![0, 3, 4],
            vec![1, 2, 3],
            vec![1, 2, 4],
            vec![1, 3, 4],
            vec![2, 3, 4],
        ];

        for indices in &subsets {
            let subset: Vec<SecretShare> = indices.iter().map(|&i| shares[i].clone()).collect();
            let result = vault
                .reconstruct_cookie("test_cookie3", &subset)
                .await
                .unwrap();
            assert_eq!(
                result,
                cookie.to_vec(),
                "Failed with indices {:?}",
                indices
            );
        }
    }

    #[tokio::test]
    async fn test_insufficient_shares() {
        let vault = DistributedCookieVault::new(3, 5).unwrap();
        let shares = vec![
            SecretShare {
                index: 1,
                value: vec![1, 2, 3],
                node_id: [0u8; 32],
            },
            SecretShare {
                index: 2,
                value: vec![4, 5, 6],
                node_id: [1u8; 32],
            },
        ];

        let result = vault.reconstruct_cookie("test", &shares).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Insufficient"));
    }

    #[tokio::test]
    async fn test_duplicate_indices_rejected() {
        let vault = DistributedCookieVault::new(2, 3).unwrap();
        let shares = vec![
            SecretShare {
                index: 1,
                value: vec![1, 2, 3],
                node_id: [0u8; 32],
            },
            SecretShare {
                index: 1,
                value: vec![4, 5, 6],
                node_id: [1u8; 32],
            },
        ];

        let result = vault.reconstruct_cookie("test", &shares).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Duplicate"));
    }

    #[tokio::test]
    async fn test_zero_index_rejected() {
        let vault = DistributedCookieVault::new(2, 3).unwrap();
        let shares = vec![
            SecretShare {
                index: 0,
                value: vec![1, 2, 3],
                node_id: [0u8; 32],
            },
            SecretShare {
                index: 1,
                value: vec![4, 5, 6],
                node_id: [1u8; 32],
            },
        ];

        let result = vault.reconstruct_cookie("test", &shares).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("zero"));
    }

    #[tokio::test]
    async fn test_empty_cookie() {
        let vault = DistributedCookieVault::new(2, 3).unwrap();
        let cookie = b"";
        let nodes = make_nodes(3);

        let shares = vault
            .split_cookie("empty", cookie, &nodes)
            .await
            .unwrap();
        let result = vault.reconstruct_cookie("empty", &shares).await.unwrap();
        assert_eq!(result, cookie.to_vec());
    }

    #[tokio::test]
    async fn test_single_byte_cookie() {
        let vault = DistributedCookieVault::new(2, 3).unwrap();
        let cookie = b"X";
        let nodes = make_nodes(3);

        let shares = vault
            .split_cookie("single", cookie, &nodes)
            .await
            .unwrap();
        let result = vault
            .reconstruct_cookie("single", &shares[0..2])
            .await
            .unwrap();
        assert_eq!(result, cookie.to_vec());
    }

    #[test]
    fn test_invalid_threshold_zero() {
        let result = DistributedCookieVault::new(0, 5);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Threshold"));
    }

    #[test]
    fn test_invalid_threshold_exceeds_total() {
        let result = DistributedCookieVault::new(6, 5);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Threshold"));
    }
}
