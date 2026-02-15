use super::bindings::{NoxStaking, NoxToken};
use super::config::ContractConfig;
use ethers::{
    middleware::SignerMiddleware,
    providers::{Http, Middleware, Provider},
    signers::{LocalWallet, Signer},
    types::{Address, H256, U256},
};
use nonos_types::{
    EthAddress, NodeId, NodeTier, NonosError, NonosResult,
    TokenAmount, NOX_DECIMALS,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

pub struct ContractClient {
    provider_url: String,
    provider: Option<Arc<Provider<Http>>>,
    signer: Option<Arc<SignerMiddleware<Provider<Http>, LocalWallet>>>,
    staking_address: Address,
    token_address: Address,
    connected: Arc<RwLock<bool>>,
    chain_id: u64,
}

impl ContractClient {
    pub fn new(config: ContractConfig) -> Self {
        let staking_address = Address::from_slice(&config.staking_address.0);
        let token_address = Address::from_slice(&config.token_address.0);

        Self {
            provider_url: config.rpc_url,
            provider: None,
            signer: None,
            staking_address,
            token_address,
            connected: Arc::new(RwLock::new(false)),
            chain_id: config.chain_id,
        }
    }

    pub async fn connect(&mut self) -> NonosResult<()> {
        info!("Connecting to RPC: {}", self.provider_url);

        let provider = Provider::<Http>::try_from(&self.provider_url)
            .map_err(|e| NonosError::Network(format!("Failed to create provider: {}", e)))?;

        let chain_id = provider
            .get_chainid()
            .await
            .map_err(|e| NonosError::Network(format!("Failed to get chain ID: {}", e)))?;

        if chain_id.as_u64() != self.chain_id {
            return Err(NonosError::Network(format!(
                "Chain ID mismatch: expected {}, got {}",
                self.chain_id,
                chain_id.as_u64()
            )));
        }

        self.provider = Some(Arc::new(provider));
        *self.connected.write().await = true;

        info!("Connected to chain {}", self.chain_id);
        Ok(())
    }

    pub async fn set_wallet(&mut self, private_key: &str) -> NonosResult<Address> {
        let provider = self.provider.as_ref()
            .ok_or_else(|| NonosError::Network("Not connected".into()))?
            .clone();

        let wallet: LocalWallet = private_key
            .parse()
            .map_err(|e| NonosError::Wallet(format!("Invalid private key: {}", e)))?;

        let wallet = wallet.with_chain_id(self.chain_id);
        let address = wallet.address();

        let client = SignerMiddleware::new((*provider).clone(), wallet);
        self.signer = Some(Arc::new(client));

        info!("Wallet set: {}", address);
        Ok(address)
    }

    pub async fn is_connected(&self) -> bool {
        *self.connected.read().await
    }

    pub async fn get_balance(&self, address: &EthAddress) -> NonosResult<TokenAmount> {
        let provider = self.provider.as_ref()
            .ok_or_else(|| NonosError::Network("Not connected".into()))?;

        let addr = Address::from_slice(&address.0);
        let token = NoxToken::new(self.token_address, provider.clone());

        let balance = token
            .balance_of(addr)
            .call()
            .await
            .map_err(|e| NonosError::Contract(format!("Failed to get balance: {}", e)))?;

        Ok(TokenAmount::from_raw(balance.as_u128(), NOX_DECIMALS))
    }

    pub async fn get_stake(&self, address: &EthAddress) -> NonosResult<TokenAmount> {
        let provider = self.provider.as_ref()
            .ok_or_else(|| NonosError::Network("Not connected".into()))?;

        let addr = Address::from_slice(&address.0);
        let staking = NoxStaking::new(self.staking_address, provider.clone());

        let stake = staking
            .get_stake(addr)
            .call()
            .await
            .map_err(|e| NonosError::Contract(format!("Failed to get stake: {}", e)))?;

        Ok(TokenAmount::from_raw(stake.as_u128(), NOX_DECIMALS))
    }

    pub async fn get_pending_rewards(&self, address: &EthAddress) -> NonosResult<TokenAmount> {
        let provider = self.provider.as_ref()
            .ok_or_else(|| NonosError::Network("Not connected".into()))?;

        let addr = Address::from_slice(&address.0);
        let staking = NoxStaking::new(self.staking_address, provider.clone());

        let rewards = staking
            .get_pending_rewards(addr)
            .call()
            .await
            .map_err(|e| NonosError::Contract(format!("Failed to get rewards: {}", e)))?;

        Ok(TokenAmount::from_raw(rewards.as_u128(), NOX_DECIMALS))
    }

    pub async fn get_tier(&self, address: &EthAddress) -> NonosResult<NodeTier> {
        let provider = self.provider.as_ref()
            .ok_or_else(|| NonosError::Network("Not connected".into()))?;

        let addr = Address::from_slice(&address.0);
        let staking = NoxStaking::new(self.staking_address, provider.clone());

        let tier = staking
            .get_tier(addr)
            .call()
            .await
            .map_err(|e| NonosError::Contract(format!("Failed to get tier: {}", e)))?;

        Ok(NodeTier::from_index(tier as u8))
    }

    pub async fn approve(&self, amount: &TokenAmount) -> NonosResult<H256> {
        let signer = self.signer.as_ref()
            .ok_or_else(|| NonosError::Wallet("No wallet configured".into()))?;

        info!("Approving {} NOX for staking", amount.to_decimal());

        let token = NoxToken::new(self.token_address, signer.clone());
        let call = token.approve(self.staking_address, U256::from(amount.raw));
        let pending = call.send().await
            .map_err(|e| NonosError::Contract(format!("Failed to approve: {}", e)))?;

        let receipt = pending.await
            .map_err(|e| NonosError::Contract(format!("Approval transaction failed: {}", e)))?
            .ok_or_else(|| NonosError::Contract("No receipt for approval".into()))?;

        info!("Approval confirmed: {:?}", receipt.transaction_hash);
        Ok(receipt.transaction_hash)
    }

    pub async fn stake(&self, amount: &TokenAmount) -> NonosResult<H256> {
        let signer = self.signer.as_ref()
            .ok_or_else(|| NonosError::Wallet("No wallet configured".into()))?;

        info!("Staking {} NOX", amount.to_decimal());

        let staking = NoxStaking::new(self.staking_address, signer.clone());
        let call = staking.stake(U256::from(amount.raw));
        let pending = call.send().await
            .map_err(|e| NonosError::Contract(format!("Failed to stake: {}", e)))?;

        let receipt = pending.await
            .map_err(|e| NonosError::Contract(format!("Stake transaction failed: {}", e)))?
            .ok_or_else(|| NonosError::Contract("No receipt for stake".into()))?;

        info!("Stake confirmed: {:?}", receipt.transaction_hash);
        Ok(receipt.transaction_hash)
    }

    pub async fn unstake(&self, amount: &TokenAmount) -> NonosResult<H256> {
        let signer = self.signer.as_ref()
            .ok_or_else(|| NonosError::Wallet("No wallet configured".into()))?;

        info!("Unstaking {} NOX", amount.to_decimal());

        let staking = NoxStaking::new(self.staking_address, signer.clone());
        let call = staking.unstake(U256::from(amount.raw));
        let pending = call.send().await
            .map_err(|e| NonosError::Contract(format!("Failed to unstake: {}", e)))?;

        let receipt = pending.await
            .map_err(|e| NonosError::Contract(format!("Unstake transaction failed: {}", e)))?
            .ok_or_else(|| NonosError::Contract("No receipt for unstake".into()))?;

        info!("Unstake confirmed: {:?}", receipt.transaction_hash);
        Ok(receipt.transaction_hash)
    }

    pub async fn claim_rewards(&self) -> NonosResult<(H256, TokenAmount)> {
        let signer = self.signer.as_ref()
            .ok_or_else(|| NonosError::Wallet("No wallet configured".into()))?;

        info!("Claiming rewards");

        let staking = NoxStaking::new(self.staking_address, signer.clone());
        let call = staking.claim_rewards();
        let pending = call.send().await
            .map_err(|e| NonosError::Contract(format!("Failed to claim: {}", e)))?;

        let receipt = pending.await
            .map_err(|e| NonosError::Contract(format!("Claim transaction failed: {}", e)))?
            .ok_or_else(|| NonosError::Contract("No receipt for claim".into()))?;

        let claimed = TokenAmount::zero(NOX_DECIMALS);

        info!("Rewards claimed: {:?}", receipt.transaction_hash);
        Ok((receipt.transaction_hash, claimed))
    }

    pub async fn set_tier(&self, tier: NodeTier) -> NonosResult<H256> {
        let signer = self.signer.as_ref()
            .ok_or_else(|| NonosError::Wallet("No wallet configured".into()))?;

        info!("Setting tier to {:?}", tier);

        let tier_index = tier.to_index();
        let staking = NoxStaking::new(self.staking_address, signer.clone());
        let call = staking.set_tier(tier_index);
        let pending = call.send().await
            .map_err(|e| NonosError::Contract(format!("Failed to set tier: {}", e)))?;

        let receipt = pending.await
            .map_err(|e| NonosError::Contract(format!("Set tier transaction failed: {}", e)))?
            .ok_or_else(|| NonosError::Contract("No receipt for set tier".into()))?;

        info!("Tier set confirmed: {:?}", receipt.transaction_hash);
        Ok(receipt.transaction_hash)
    }

    pub async fn register_node(&self, node_id: &NodeId) -> NonosResult<H256> {
        let signer = self.signer.as_ref()
            .ok_or_else(|| NonosError::Wallet("No wallet configured".into()))?;

        info!("Registering node: {}", node_id);

        let node_bytes: [u8; 32] = node_id.0;
        let staking = NoxStaking::new(self.staking_address, signer.clone());
        let call = staking.register_node(node_bytes);
        let pending = call.send().await
            .map_err(|e| NonosError::Contract(format!("Failed to register node: {}", e)))?;

        let receipt = pending.await
            .map_err(|e| NonosError::Contract(format!("Register node transaction failed: {}", e)))?
            .ok_or_else(|| NonosError::Contract("No receipt for register node".into()))?;

        info!("Node registered: {:?}", receipt.transaction_hash);
        Ok(receipt.transaction_hash)
    }

    pub async fn is_node_registered(&self, node_id: &NodeId) -> NonosResult<bool> {
        let provider = self.provider.as_ref()
            .ok_or_else(|| NonosError::Network("Not connected".into()))?;

        let staking = NoxStaking::new(self.staking_address, provider.clone());

        let node_bytes: [u8; 32] = node_id.0;
        let registered = staking
            .is_node_registered(node_bytes)
            .call()
            .await
            .map_err(|e| NonosError::Contract(format!("Failed to check node registration: {}", e)))?;

        Ok(registered)
    }

    pub async fn get_node_stake(&self, node_id: &NodeId) -> NonosResult<TokenAmount> {
        let provider = self.provider.as_ref()
            .ok_or_else(|| NonosError::Network("Not connected".into()))?;

        let staking = NoxStaking::new(self.staking_address, provider.clone());

        let node_bytes: [u8; 32] = node_id.0;
        let stake = staking
            .get_node_stake(node_bytes)
            .call()
            .await
            .map_err(|e| NonosError::Contract(format!("Failed to get node stake: {}", e)))?;

        Ok(TokenAmount::from_raw(stake.as_u128(), NOX_DECIMALS))
    }
}
