use nonos_types::EthAddress;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct RewardsConfig {
    pub contract: EthAddress,
    pub reward_address: EthAddress,
    pub auto_claim: bool,
    pub auto_claim_threshold: u64,
    pub rpc_url: Option<String>,
}

impl Default for RewardsConfig {
    fn default() -> Self {
        Self {
            contract: EthAddress::zero(),
            reward_address: EthAddress::zero(),
            auto_claim: false,
            auto_claim_threshold: 100,
            rpc_url: None,
        }
    }
}
