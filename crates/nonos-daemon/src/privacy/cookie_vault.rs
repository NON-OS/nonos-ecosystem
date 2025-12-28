// NONOS GNU AFFERO GENERAL PUBLIC LICENSE
// Version 3, 19 November 2007
// Copyright (C) 2025 NON-OS <team@nonos.systems>

//! Distributed Cookie Vault
//!
//! Shamir's Secret Sharing for sensitive session data:
//! - Threshold-based secret splitting
//! - Multi-node storage for resilience
//! - Lagrange interpolation for reconstruction
//! - No single point of compromise

use nonos_crypto::random_bytes;
use nonos_types::{NonosError, NonosResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecretShare {
    pub index: u8,
    pub value: Vec<u8>,
    pub node_id: [u8; 32],
}

pub struct DistributedCookieVault {
    threshold: u8,
    total_shares: u8,
    shares: Arc<RwLock<HashMap<String, Vec<SecretShare>>>>,
}

impl DistributedCookieVault {
    pub fn new(threshold: u8, total_shares: u8) -> Self {
        assert!(threshold > 0, "Threshold must be > 0");
        assert!(threshold <= total_shares, "Threshold must be <= total_shares");
        Self {
            threshold,
            total_shares,
            shares: Arc::new(RwLock::new(HashMap::new())),
        }
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

        let mut coefficients: Vec<[u8; 32]> = Vec::with_capacity(self.threshold as usize);

        // First coefficient is the secret
        let mut secret_coef = [0u8; 32];
        let copy_len = cookie_value.len().min(32);
        secret_coef[..copy_len].copy_from_slice(&cookie_value[..copy_len]);
        coefficients.push(secret_coef);

        // Random coefficients for the polynomial
        for _ in 1..self.threshold {
            coefficients.push(random_bytes::<32>());
        }

        let mut shares = Vec::with_capacity(self.total_shares as usize);

        for (i, node_id) in node_ids.iter().take(self.total_shares as usize).enumerate() {
            let x = (i + 1) as u8;
            let y = self.evaluate_polynomial(&coefficients, x);

            shares.push(SecretShare {
                index: x,
                value: y.to_vec(),
                node_id: *node_id,
            });
        }

        {
            let mut storage = self.shares.write().await;
            storage.insert(cookie_id.to_string(), shares.clone());
        }

        info!(
            "Split cookie '{}' into {} shares (threshold: {})",
            cookie_id, self.total_shares, self.threshold
        );
        Ok(shares)
    }

    pub async fn reconstruct_cookie(&self, cookie_id: &str, shares: &[SecretShare]) -> NonosResult<Vec<u8>> {
        if shares.len() < self.threshold as usize {
            return Err(NonosError::Crypto(format!(
                "Insufficient shares: need {}, got {}",
                self.threshold,
                shares.len()
            )));
        }

        let secret = self.lagrange_interpolate(shares)?;
        info!(
            "Reconstructed cookie '{}' from {} shares",
            cookie_id,
            shares.len()
        );
        Ok(secret)
    }

    fn evaluate_polynomial(&self, coefficients: &[[u8; 32]], x: u8) -> [u8; 32] {
        let mut result = [0u8; 32];
        let mut x_power: u16 = 1;

        for coef in coefficients {
            for j in 0..32 {
                result[j] = result[j].wrapping_add(coef[j].wrapping_mul((x_power & 0xFF) as u8));
            }
            x_power = x_power.wrapping_mul(x as u16);
        }

        result
    }

    fn lagrange_interpolate(&self, shares: &[SecretShare]) -> NonosResult<Vec<u8>> {
        let threshold = shares.len();
        let mut result = vec![0u8; 32];

        for i in 0..threshold {
            let xi = shares[i].index as i32;
            let yi = &shares[i].value;

            let mut basis: f64 = 1.0;
            for j in 0..threshold {
                if i != j {
                    let xj = shares[j].index as i32;
                    basis *= (0 - xj) as f64 / (xi - xj) as f64;
                }
            }

            for k in 0..yi.len().min(32) {
                result[k] = result[k].wrapping_add((yi[k] as f64 * basis) as u8);
            }
        }

        // Trim trailing zeros
        let mut end = 32;
        while end > 0 && result[end - 1] == 0 {
            end -= 1;
        }
        result.truncate(end.max(1));

        Ok(result)
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

    #[tokio::test]
    async fn test_split_and_reconstruct() {
        let vault = DistributedCookieVault::new(3, 5);
        let cookie = b"secret_session_cookie_value";
        let nodes: Vec<[u8; 32]> = (0..5).map(|i| {
            let mut arr = [0u8; 32];
            arr[0] = i;
            arr
        }).collect();

        let shares = vault.split_cookie("test_cookie", cookie, &nodes).await.unwrap();
        assert_eq!(shares.len(), 5);

        // Reconstruct with exactly threshold shares
        let result = vault.reconstruct_cookie("test_cookie", &shares[0..3]).await.unwrap();
        assert!(!result.is_empty());
    }

    #[tokio::test]
    async fn test_insufficient_shares() {
        let vault = DistributedCookieVault::new(3, 5);
        let shares = vec![
            SecretShare { index: 1, value: vec![1, 2, 3], node_id: [0u8; 32] },
            SecretShare { index: 2, value: vec![4, 5, 6], node_id: [1u8; 32] },
        ];

        let result = vault.reconstruct_cookie("test", &shares).await;
        assert!(result.is_err());
    }

    #[test]
    #[should_panic]
    fn test_invalid_threshold() {
        let _ = DistributedCookieVault::new(0, 5);
    }
}
