// NONOS GNU AFFERO GENERAL PUBLIC LICENSE
// Version 3, 19 November 2007
// Copyright (C) 2025 NON-OS <team@nonos.systems>

//! Distributed Cookie Vault
//!
//! Shamir's Secret Sharing for sensitive session data using GF(256) arithmetic.
//! - Threshold-based secret splitting with information-theoretic security
//! - Each byte is shared independently using a degree-(k-1) polynomial
//! - Reconstruction via Lagrange interpolation in GF(256)
//! - No single point of compromise with threshold scheme

use nonos_crypto::random_bytes;
use nonos_types::{NonosError, NonosResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

/// GF(256) finite field arithmetic
/// Using irreducible polynomial x^8 + x^4 + x^3 + x + 1 (0x11B)
/// with generator 3 (standard for AES)
mod gf256 {
    /// Multiply by x in GF(256) - used for table generation
    const fn xtime(x: u8) -> u8 {
        if x & 0x80 != 0 {
            ((x << 1) ^ 0x1B) as u8
        } else {
            (x << 1) as u8
        }
    }

    /// Multiply two elements using Russian peasant algorithm
    const fn gf_mul(mut a: u8, mut b: u8) -> u8 {
        let mut result = 0u8;
        while b != 0 {
            if b & 1 != 0 {
                result ^= a;
            }
            a = xtime(a);
            b >>= 1;
        }
        result
    }

    /// Pre-computed logarithm table (base 3) for GF(256)
    const LOG: [u8; 256] = {
        let mut log = [0u8; 256];
        let mut x: u8 = 1;
        let mut i: u8 = 0;
        loop {
            log[x as usize] = i;
            x = gf_mul(x, 3); // Multiply by generator 3
            if i == 254 {
                break;
            }
            i += 1;
        }
        log
    };

    /// Pre-computed exponential table (powers of 3) for GF(256)
    const EXP: [u8; 512] = {
        let mut exp = [0u8; 512];
        let mut x: u8 = 1;
        let mut i: usize = 0;
        while i < 255 {
            exp[i] = x;
            exp[i + 255] = x; // Duplicate for easy modular access
            x = gf_mul(x, 3); // Multiply by generator 3
            i += 1;
        }
        exp
    };

    /// Addition in GF(256) is XOR
    #[inline]
    pub fn add(a: u8, b: u8) -> u8 {
        a ^ b
    }

    /// Subtraction in GF(256) is also XOR (same as addition)
    #[inline]
    pub fn sub(a: u8, b: u8) -> u8 {
        a ^ b
    }

    /// Multiplication in GF(256) using log/exp tables
    #[inline]
    pub fn mul(a: u8, b: u8) -> u8 {
        if a == 0 || b == 0 {
            return 0;
        }
        let log_a = LOG[a as usize] as usize;
        let log_b = LOG[b as usize] as usize;
        EXP[log_a + log_b]
    }

    /// Division in GF(256): a / b using log tables
    #[inline]
    pub fn div(a: u8, b: u8) -> u8 {
        if a == 0 {
            return 0;
        }
        if b == 0 {
            panic!("Division by zero in GF(256)");
        }
        let log_a = LOG[a as usize] as usize;
        let log_b = LOG[b as usize] as usize;
        // a / b = exp(log(a) - log(b) mod 255)
        let diff = if log_a >= log_b {
            log_a - log_b
        } else {
            255 + log_a - log_b
        };
        EXP[diff]
    }

    /// Multiplicative inverse in GF(256)
    /// Used by div() tests and available for external use
    #[inline]
    #[allow(dead_code)]
    pub fn inv(a: u8) -> u8 {
        if a == 0 {
            panic!("Cannot invert zero in GF(256)");
        }
        let log_a = LOG[a as usize] as usize;
        EXP[255 - log_a]
    }

    /// Evaluate polynomial at point x in GF(256)
    /// coefficients[0] is the constant term (the secret)
    pub fn eval_poly(coefficients: &[u8], x: u8) -> u8 {
        if coefficients.is_empty() {
            return 0;
        }
        // Horner's method for polynomial evaluation
        let mut result = coefficients[coefficients.len() - 1];
        for i in (0..coefficients.len() - 1).rev() {
            result = add(mul(result, x), coefficients[i]);
        }
        result
    }

    /// Lagrange interpolation at x=0 to recover the secret
    /// Takes (x_i, y_i) pairs and returns f(0)
    pub fn interpolate_at_zero(points: &[(u8, u8)]) -> u8 {
        let mut result: u8 = 0;

        for i in 0..points.len() {
            let (xi, yi) = points[i];

            // Calculate Lagrange basis polynomial L_i(0)
            // L_i(0) = product of (0 - x_j) / (x_i - x_j) for j != i
            // Since we evaluate at 0: L_i(0) = product of x_j / (x_j - x_i) for j != i
            let mut basis: u8 = 1;

            for j in 0..points.len() {
                if i != j {
                    let (xj, _) = points[j];
                    // basis *= x_j / (x_j - x_i)
                    // In GF(256), subtraction is XOR
                    let numerator = xj;
                    let denominator = sub(xj, xi);
                    basis = mul(basis, div(numerator, denominator));
                }
            }

            // result += y_i * L_i(0)
            result = add(result, mul(yi, basis));
        }

        result
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_gf256_basic() {
            // Addition is XOR
            assert_eq!(add(0x53, 0xCA), 0x53 ^ 0xCA);

            // Multiplication properties
            assert_eq!(mul(0, 0xCA), 0); // Zero absorbs
            assert_eq!(mul(1, 0xCA), 0xCA); // Identity
            assert_eq!(mul(0xCA, 1), 0xCA); // Commutative identity

            // Division is inverse of multiplication
            for a in [5u8, 17, 83, 202, 255] {
                for b in [3u8, 7, 13, 100, 200] {
                    let product = mul(a, b);
                    assert_eq!(div(product, b), a, "div(mul({}, {}), {}) != {}", a, b, b, a);
                }
            }

            // Inverse property: a * inv(a) = 1
            for a in [2u8, 3, 53, 83, 127, 200, 255] {
                assert_eq!(mul(a, inv(a)), 1, "mul({}, inv({})) != 1", a, a);
            }
        }

        #[test]
        fn test_polynomial_eval() {
            // f(x) = 3 + 2x + x^2 at x=2
            let coeffs = [3u8, 2, 1];
            let result = eval_poly(&coeffs, 2);
            // Manual calculation: 3 + 2*2 + 1*4 in GF(256)
            let expected = add(add(3, mul(2, 2)), mul(1, mul(2, 2)));
            assert_eq!(result, expected);
        }

        #[test]
        fn test_interpolation() {
            // Create a polynomial f(x) = secret + a1*x
            // and verify interpolation recovers the secret
            let secret = 42u8;
            let a1 = 17u8;

            // Generate points: f(1), f(2)
            let y1 = add(secret, mul(a1, 1));
            let y2 = add(secret, mul(a1, 2));

            let points = [(1u8, y1), (2u8, y2)];
            let recovered = interpolate_at_zero(&points);

            assert_eq!(recovered, secret);
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecretShare {
    /// Share index (x coordinate), must be non-zero and unique
    pub index: u8,
    /// Share values (y coordinates for each byte of the secret)
    pub value: Vec<u8>,
    /// Node ID that holds this share
    pub node_id: [u8; 32],
}

/// Distributed Cookie Vault using Shamir's Secret Sharing
pub struct DistributedCookieVault {
    /// Minimum shares needed to reconstruct (k in k-of-n)
    threshold: u8,
    /// Total shares to generate (n in k-of-n)
    total_shares: u8,
    /// Stored shares indexed by cookie ID
    shares: Arc<RwLock<HashMap<String, Vec<SecretShare>>>>,
}

impl DistributedCookieVault {
    /// Create a new vault with k-of-n threshold scheme
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

    /// Split a cookie value into shares using Shamir's Secret Sharing
    /// Each byte of the cookie is shared independently
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

        // Initialize shares with empty value vectors
        let mut shares: Vec<SecretShare> = (0..self.total_shares)
            .map(|i| SecretShare {
                index: (i + 1) as u8, // Indices 1, 2, 3, ... (never 0)
                value: Vec::with_capacity(cookie_value.len()),
                node_id: node_ids[i as usize],
            })
            .collect();

        // For each byte of the secret, create a polynomial and evaluate at each x
        for &secret_byte in cookie_value {
            // Generate random coefficients for degree-(k-1) polynomial
            // coefficients[0] = secret_byte (constant term)
            let mut coefficients = vec![secret_byte];
            let random = random_bytes::<32>();
            for i in 1..self.threshold as usize {
                coefficients.push(random[i % 32]);
            }

            // Evaluate polynomial at each share's x coordinate
            for share in &mut shares {
                let y = gf256::eval_poly(&coefficients, share.index);
                share.value.push(y);
            }
        }

        // Store shares
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

    /// Reconstruct a cookie value from shares using Lagrange interpolation
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

        // Verify all shares have the same length
        let value_len = shares[0].value.len();
        if !shares.iter().all(|s| s.value.len() == value_len) {
            return Err(NonosError::Crypto(
                "Share value lengths do not match".to_string(),
            ));
        }

        // Verify all indices are unique and non-zero
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

        // Reconstruct each byte using Lagrange interpolation
        let mut secret = Vec::with_capacity(value_len);

        for byte_idx in 0..value_len {
            // Collect (x, y) points for this byte position
            let points: Vec<(u8, u8)> = shares
                .iter()
                .take(self.threshold as usize)
                .map(|s| (s.index, s.value[byte_idx]))
                .collect();

            // Interpolate to find f(0) = secret byte
            let secret_byte = gf256::interpolate_at_zero(&points);
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

    /// Get stored shares for a cookie
    pub async fn get_shares(&self, cookie_id: &str) -> Option<Vec<SecretShare>> {
        self.shares.read().await.get(cookie_id).cloned()
    }

    /// Remove and return shares for a cookie
    pub async fn remove_shares(&self, cookie_id: &str) -> Option<Vec<SecretShare>> {
        self.shares.write().await.remove(cookie_id)
    }

    /// Count of stored cookies
    pub async fn stored_cookie_count(&self) -> usize {
        self.shares.read().await.len()
    }

    /// Clear all stored shares
    pub async fn clear(&self) {
        self.shares.write().await.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_split_and_reconstruct_exact_threshold() {
        let vault = DistributedCookieVault::new(3, 5);
        let cookie = b"secret_session_cookie_value_12345";
        let nodes: Vec<[u8; 32]> = (0..5)
            .map(|i| {
                let mut arr = [0u8; 32];
                arr[0] = i;
                arr
            })
            .collect();

        let shares = vault
            .split_cookie("test_cookie", cookie, &nodes)
            .await
            .unwrap();
        assert_eq!(shares.len(), 5);

        // Reconstruct with exactly threshold shares
        let result = vault
            .reconstruct_cookie("test_cookie", &shares[0..3])
            .await
            .unwrap();
        assert_eq!(result, cookie.to_vec());
    }

    #[tokio::test]
    async fn test_split_and_reconstruct_all_shares() {
        let vault = DistributedCookieVault::new(3, 5);
        let cookie = b"another secret value";
        let nodes: Vec<[u8; 32]> = (0..5)
            .map(|i| {
                let mut arr = [0u8; 32];
                arr[0] = i;
                arr
            })
            .collect();

        let shares = vault
            .split_cookie("test_cookie2", cookie, &nodes)
            .await
            .unwrap();

        // Reconstruct with all shares
        let result = vault
            .reconstruct_cookie("test_cookie2", &shares)
            .await
            .unwrap();
        assert_eq!(result, cookie.to_vec());
    }

    #[tokio::test]
    async fn test_reconstruct_with_different_share_subsets() {
        let vault = DistributedCookieVault::new(3, 5);
        let cookie = b"test secret data";
        let nodes: Vec<[u8; 32]> = (0..5)
            .map(|i| {
                let mut arr = [0u8; 32];
                arr[0] = i;
                arr
            })
            .collect();

        let shares = vault
            .split_cookie("test_cookie3", cookie, &nodes)
            .await
            .unwrap();

        // Any 3 shares should work
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
        let vault = DistributedCookieVault::new(3, 5);
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
        let vault = DistributedCookieVault::new(2, 3);
        let shares = vec![
            SecretShare {
                index: 1,
                value: vec![1, 2, 3],
                node_id: [0u8; 32],
            },
            SecretShare {
                index: 1, // Duplicate!
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
        let vault = DistributedCookieVault::new(2, 3);
        let shares = vec![
            SecretShare {
                index: 0, // Invalid!
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
        let vault = DistributedCookieVault::new(2, 3);
        let cookie = b"";
        let nodes: Vec<[u8; 32]> = (0..3)
            .map(|i| {
                let mut arr = [0u8; 32];
                arr[0] = i;
                arr
            })
            .collect();

        let shares = vault
            .split_cookie("empty", cookie, &nodes)
            .await
            .unwrap();
        let result = vault.reconstruct_cookie("empty", &shares).await.unwrap();
        assert_eq!(result, cookie.to_vec());
    }

    #[tokio::test]
    async fn test_single_byte_cookie() {
        let vault = DistributedCookieVault::new(2, 3);
        let cookie = b"X";
        let nodes: Vec<[u8; 32]> = (0..3)
            .map(|i| {
                let mut arr = [0u8; 32];
                arr[0] = i;
                arr
            })
            .collect();

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
    #[should_panic]
    fn test_invalid_threshold_zero() {
        let _ = DistributedCookieVault::new(0, 5);
    }

    #[test]
    #[should_panic]
    fn test_invalid_threshold_exceeds_total() {
        let _ = DistributedCookieVault::new(6, 5);
    }

    #[test]
    fn test_gf256_inverse() {
        // Test that inv(a) * a = 1 for all non-zero values
        for a in 1..=255u8 {
            let a_inv = gf256::inv(a);
            let product = gf256::mul(a, a_inv);
            assert_eq!(product, 1, "inv({}) * {} should equal 1, got {}", a, a, product);
        }
    }

    #[test]
    fn test_gf256_div_uses_inv() {
        // Test that a / b = a * inv(b)
        for a in 1..=50u8 {
            for b in 1..=50u8 {
                let quotient = gf256::div(a, b);
                let product = gf256::mul(a, gf256::inv(b));
                assert_eq!(quotient, product, "{} / {} via div vs mul*inv", a, b);
            }
        }
    }
}
