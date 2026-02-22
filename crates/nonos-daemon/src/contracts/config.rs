use nonos_types::EthAddress;

#[derive(Clone, Debug)]
pub struct ContractConfig {
    pub rpc_url: String,
    pub staking_address: EthAddress,
    pub token_address: EthAddress,
    pub chain_id: u64,
}

impl Default for ContractConfig {
    fn default() -> Self {
        Self {
            rpc_url: "http://localhost:8545".to_string(),
            staking_address: EthAddress::zero(),
            token_address: EthAddress::zero(),
            chain_id: 31337,
        }
    }
}

pub const NOX_TOKEN_MAINNET: &str = "0x0a26c80Be4E060e688d7C23aDdB92cBb5D2C9eCA";

pub const NOX_STAKING_VAULT: &str = "0xD548558A7e2666D5580125448B446358F5990423";

pub const NOX_STAKING_CONTRACT_MAINNET: &str = "";

pub const NOX_TOKEN_SEPOLIA: &str = "0xC87799c4517Dcdfc65bfefa3Be64Beb89668c66c";

pub const NOX_STAKING_CONTRACT_SEPOLIA: &str = "0x7c34956eb5e92460307846D754dD4d1a2400B652";

pub const COLLATERAL_MANAGER_SEPOLIA: &str = "0x7366785f977C7ee9Bce920c91fC851B5F1bF1983";

pub const WORK_REGISTRY_SEPOLIA: &str = "0x7dFFC765475f99714564297C8a016500382BB95D";

pub const FEE_ROUTER_SEPOLIA: &str = "0x95f99a47C7471Ef4db176c45f4d4fE6Fada45A10";

pub const PRIVACY_LIQUIDITY_POOL_SEPOLIA: &str = "0x33221345a0dF2638852dc05D0E51d66cE63C874E";

pub const REWARD_DISTRIBUTOR_SEPOLIA: &str = "0x5b111830208EfdAA0D90e24F6a57EB3491F84724";

pub const STAKING_GENESIS_TIMESTAMP_SEPOLIA: u64 = 1740085200;

pub const USE_SEPOLIA: bool = true;

pub const STAKING_GENESIS_TIMESTAMP: u64 = 0;

pub const EPOCH_DURATION_SECS: u64 = 7 * 24 * 60 * 60;

pub const NOX_TOKEN_IMPLEMENTATION: &str = "0x6a9cae706d659f1e156f5406358ce8d9ef462c59";

pub fn nox_staking_contract() -> &'static str {
    if USE_SEPOLIA {
        NOX_STAKING_CONTRACT_SEPOLIA
    } else {
        NOX_STAKING_CONTRACT_MAINNET
    }
}

pub fn nox_token_contract() -> &'static str {
    if USE_SEPOLIA {
        NOX_TOKEN_SEPOLIA
    } else {
        NOX_TOKEN_MAINNET
    }
}

pub fn staking_genesis_timestamp() -> u64 {
    if USE_SEPOLIA {
        STAKING_GENESIS_TIMESTAMP_SEPOLIA
    } else {
        STAKING_GENESIS_TIMESTAMP
    }
}

pub fn current_epoch() -> Option<u64> {
    let genesis = staking_genesis_timestamp();
    if genesis == 0 {
        return None;
    }
    let now = chrono::Utc::now().timestamp() as u64;
    if now < genesis {
        return None;
    }
    Some((now - genesis) / EPOCH_DURATION_SECS)
}

pub fn is_staking_configured() -> bool {
    !nox_staking_contract().is_empty() && staking_genesis_timestamp() > 0
}

pub(crate) fn parse_eth_address(hex: &str) -> EthAddress {
    let hex = hex.strip_prefix("0x").unwrap_or(hex);
    let bytes = hex::decode(hex).expect("Invalid hex address");
    let mut addr = [0u8; 20];
    addr.copy_from_slice(&bytes);
    EthAddress(addr)
}

impl ContractConfig {
    pub fn mainnet(staking: EthAddress) -> Self {
        Self {
            rpc_url: "https://ethereum-rpc.publicnode.com".to_string(),
            staking_address: staking,
            token_address: parse_eth_address(NOX_TOKEN_MAINNET),
            chain_id: 1,
        }
    }

    pub fn mainnet_alchemy(staking: EthAddress, api_key: &str) -> Self {
        Self {
            rpc_url: format!("https://eth-mainnet.g.alchemy.com/v2/{}", api_key),
            staking_address: staking,
            token_address: parse_eth_address(NOX_TOKEN_MAINNET),
            chain_id: 1,
        }
    }

    pub fn mainnet_infura(staking: EthAddress, api_key: &str) -> Self {
        Self {
            rpc_url: format!("https://mainnet.infura.io/v3/{}", api_key),
            staking_address: staking,
            token_address: parse_eth_address(NOX_TOKEN_MAINNET),
            chain_id: 1,
        }
    }

    pub fn base(staking: EthAddress, token: EthAddress) -> Self {
        Self {
            rpc_url: "https://mainnet.base.org".to_string(),
            staking_address: staking,
            token_address: token,
            chain_id: 8453,
        }
    }

    pub fn arbitrum(staking: EthAddress, token: EthAddress) -> Self {
        Self {
            rpc_url: "https://arb1.arbitrum.io/rpc".to_string(),
            staking_address: staking,
            token_address: token,
            chain_id: 42161,
        }
    }

    pub fn sepolia(staking: EthAddress, token: EthAddress) -> Self {
        Self {
            rpc_url: "https://ethereum-sepolia-rpc.publicnode.com".to_string(),
            staking_address: staking,
            token_address: token,
            chain_id: 11155111,
        }
    }

    pub fn nox_token_mainnet() -> EthAddress {
        parse_eth_address(NOX_TOKEN_MAINNET)
    }

    pub fn nox_staking_vault() -> EthAddress {
        parse_eth_address(NOX_STAKING_VAULT)
    }

    pub fn mainnet_default() -> Self {
        let staking_addr = nox_staking_contract();
        Self {
            rpc_url: "https://ethereum-rpc.publicnode.com".to_string(),
            staking_address: if staking_addr.is_empty() {
                EthAddress::zero()
            } else {
                parse_eth_address(staking_addr)
            },
            token_address: parse_eth_address(nox_token_contract()),
            chain_id: if USE_SEPOLIA { 11155111 } else { 1 },
        }
    }

    pub fn sepolia_testnet() -> Self {
        let staking_addr = NOX_STAKING_CONTRACT_SEPOLIA;
        Self {
            rpc_url: "https://ethereum-sepolia-rpc.publicnode.com".to_string(),
            staking_address: if staking_addr.is_empty() {
                EthAddress::zero()
            } else {
                parse_eth_address(staking_addr)
            },
            token_address: if NOX_TOKEN_SEPOLIA.is_empty() {
                EthAddress::zero()
            } else {
                parse_eth_address(NOX_TOKEN_SEPOLIA)
            },
            chain_id: 11155111,
        }
    }
}
