// NONOS GNU AFFERO GENERAL PUBLIC LICENSE
// Version 3, 19 November 2007
// Copyright (C) 2025 NON-OS <team@nonos.systems>

use super::client::ContractClient;
use ethers::types::H256;
use nonos_types::{EthAddress, NonosResult, TokenAmount};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Auto-claim manager for automatic reward claiming
pub struct AutoClaimManager {
    /// Contract client
    client: Arc<RwLock<ContractClient>>,
    /// Minimum threshold for auto-claim (in NOX)
    threshold: TokenAmount,
    /// Auto-claim enabled
    enabled: Arc<RwLock<bool>>,
    /// Last claim timestamp
    last_claim: Arc<RwLock<Option<chrono::DateTime<chrono::Utc>>>>,
    /// Staker address
    staker_address: EthAddress,
}

impl AutoClaimManager {
    /// Create new auto-claim manager
    pub fn new(
        client: Arc<RwLock<ContractClient>>,
        staker_address: EthAddress,
        threshold: TokenAmount,
    ) -> Self {
        Self {
            client,
            threshold,
            enabled: Arc::new(RwLock::new(false)),
            last_claim: Arc::new(RwLock::new(None)),
            staker_address,
        }
    }

    /// Enable auto-claim
    pub async fn enable(&self) {
        *self.enabled.write().await = true;
        info!("Auto-claim enabled with threshold {} NOX", self.threshold.to_decimal());
    }

    /// Disable auto-claim
    pub async fn disable(&self) {
        *self.enabled.write().await = false;
        info!("Auto-claim disabled");
    }

    /// Check and claim if threshold reached
    pub async fn check_and_claim(&self) -> NonosResult<Option<H256>> {
        if !*self.enabled.read().await {
            return Ok(None);
        }

        let client = self.client.read().await;
        let pending = client.get_pending_rewards(&self.staker_address).await?;

        if pending.raw >= self.threshold.raw {
            info!("Auto-claiming {} NOX (threshold: {} NOX)",
                pending.to_decimal(),
                self.threshold.to_decimal()
            );

            drop(client);
            let client = self.client.read().await;
            let (tx_hash, _) = client.claim_rewards().await?;

            *self.last_claim.write().await = Some(chrono::Utc::now());

            Ok(Some(tx_hash))
        } else {
            debug!("Pending {} NOX below threshold {} NOX",
                pending.to_decimal(),
                self.threshold.to_decimal()
            );
            Ok(None)
        }
    }

    /// Set threshold
    pub async fn set_threshold(&mut self, threshold: TokenAmount) {
        self.threshold = threshold;
        info!("Auto-claim threshold set to {} NOX", threshold.to_decimal());
    }

    /// Get last claim time
    pub async fn last_claim_time(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        *self.last_claim.read().await
    }
}
