use super::client::ContractClient;
use ethers::types::H256;
use nonos_types::{EthAddress, NonosResult, TokenAmount};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

pub struct AutoClaimManager {
    client: Arc<RwLock<ContractClient>>,
    threshold: TokenAmount,
    enabled: Arc<RwLock<bool>>,
    last_claim: Arc<RwLock<Option<chrono::DateTime<chrono::Utc>>>>,
    staker_address: EthAddress,
}

impl AutoClaimManager {
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

    pub async fn enable(&self) {
        *self.enabled.write().await = true;
        info!("Auto-claim enabled with threshold {} NOX", self.threshold.to_decimal());
    }

    pub async fn disable(&self) {
        *self.enabled.write().await = false;
        info!("Auto-claim disabled");
    }

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

    pub async fn set_threshold(&mut self, threshold: TokenAmount) {
        self.threshold = threshold;
        info!("Auto-claim threshold set to {} NOX", threshold.to_decimal());
    }

    pub async fn last_claim_time(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        *self.last_claim.read().await
    }
}
