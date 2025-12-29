// NONOS GNU AFFERO GENERAL PUBLIC LICENSE
// Version 3, 19 November 2007
// Copyright (C) 2025 NON-OS <team@nonos.systems>

use nonos_types::EthAddress;

/// Contract configuration
#[derive(Clone, Debug)]
pub struct ContractConfig {
    /// RPC endpoint URL
    pub rpc_url: String,
    /// Staking contract address
    pub staking_address: EthAddress,
    /// Token contract address
    pub token_address: EthAddress,
    /// Chain ID
    pub chain_id: u64,
}

impl Default for ContractConfig {
    fn default() -> Self {
        Self {
            rpc_url: "http://localhost:8545".to_string(),
            staking_address: EthAddress::zero(),
            token_address: EthAddress::zero(),
            chain_id: 31337, // Hardhat default
        }
    }
}

/// Official NOX Token mainnet address (UUPS Proxy)
/// Contract: https://etherscan.io/token/0x0a26c80Be4E060e688d7C23aDdB92cBb5D2C9eCA
pub const NOX_TOKEN_MAINNET: &str = "0x0a26c80Be4E060e688d7C23aDdB92cBb5D2C9eCA";

/// NOX Staking Vault address (holds 32M NOX = 4% rewards pool)
/// Address: https://etherscan.io/address/0xD548558A7e2666D5580125448B446358F5990423
pub const NOX_STAKING_VAULT: &str = "0xD548558A7e2666D5580125448B446358F5990423";

/// NOX Staking Contract mainnet address
/// Deployed and funded from NOX_STAKING_VAULT
/// Contract: https://etherscan.io/address/0x... (pending deployment)
///
/// IMPORTANT: This address must be set before mainnet launch.
/// Deploy using: `forge script script/Deploy.s.sol --rpc-url mainnet --broadcast`
pub const NOX_STAKING_CONTRACT_MAINNET: &str = ""; // Set after mainnet deployment

/// NOX Token address on Sepolia testnet
/// Deploy first, then update this address
pub const NOX_TOKEN_SEPOLIA: &str = ""; // Set after Sepolia deployment

/// NOX Staking Contract on Sepolia testnet
pub const NOX_STAKING_CONTRACT_SEPOLIA: &str = ""; // Set after Sepolia deployment

/// Sepolia staking genesis timestamp (Unix seconds)
pub const STAKING_GENESIS_TIMESTAMP_SEPOLIA: u64 = 0; // Set after Sepolia deployment

/// Set this to true to use Sepolia testnet, false for mainnet
pub const USE_SEPOLIA: bool = true;

/// Staking contract genesis timestamp (Unix seconds)
/// Used for epoch calculation - set to deployment block timestamp
pub const STAKING_GENESIS_TIMESTAMP: u64 = 0; // Set after deployment

/// Epoch duration in seconds (7 days)
pub const EPOCH_DURATION_SECS: u64 = 7 * 24 * 60 * 60;

/// NOX Token implementation address (upgradeable via UUPS)
pub const NOX_TOKEN_IMPLEMENTATION: &str = "0x6a9cae706d659f1e156f5406358ce8d9ef462c59";

/// Get the active staking contract address
pub fn nox_staking_contract() -> &'static str {
    if USE_SEPOLIA {
        NOX_STAKING_CONTRACT_SEPOLIA
    } else {
        NOX_STAKING_CONTRACT_MAINNET
    }
}

/// Get the active token contract address
pub fn nox_token_contract() -> &'static str {
    if USE_SEPOLIA {
        NOX_TOKEN_SEPOLIA
    } else {
        NOX_TOKEN_MAINNET
    }
}

/// Get the active genesis timestamp
pub fn staking_genesis_timestamp() -> u64 {
    if USE_SEPOLIA {
        STAKING_GENESIS_TIMESTAMP_SEPOLIA
    } else {
        STAKING_GENESIS_TIMESTAMP
    }
}

/// Calculate current epoch number from genesis timestamp
/// Returns None if genesis timestamp is not set or in the future
pub fn current_epoch() -> Option<u64> {
    let genesis = staking_genesis_timestamp();
    if genesis == 0 {
        return None; // Not deployed yet
    }
    let now = chrono::Utc::now().timestamp() as u64;
    if now < genesis {
        return None; // Genesis in future
    }
    Some((now - genesis) / EPOCH_DURATION_SECS)
}

/// Check if staking contract is configured
pub fn is_staking_configured() -> bool {
    !nox_staking_contract().is_empty() && staking_genesis_timestamp() > 0
}

/// Parse hex address string to EthAddress
pub(crate) fn parse_eth_address(hex: &str) -> EthAddress {
    let hex = hex.strip_prefix("0x").unwrap_or(hex);
    let bytes = hex::decode(hex).expect("Invalid hex address");
    let mut addr = [0u8; 20];
    addr.copy_from_slice(&bytes);
    EthAddress(addr)
}

impl ContractConfig {
    /// Create config for Ethereum mainnet with official NOX Token
    /// Uses the deployed UUPS proxy at 0x0a26c80Be4E060e688d7C23aDdB92cBb5D2C9eCA
    pub fn mainnet(staking: EthAddress) -> Self {
        Self {
            rpc_url: "https://ethereum-rpc.publicnode.com".to_string(),
            staking_address: staking,
            token_address: parse_eth_address(NOX_TOKEN_MAINNET),
            chain_id: 1,
        }
    }

    /// Create config for Ethereum mainnet with official NOX Token (alternative RPC)
    pub fn mainnet_alchemy(staking: EthAddress, api_key: &str) -> Self {
        Self {
            rpc_url: format!("https://eth-mainnet.g.alchemy.com/v2/{}", api_key),
            staking_address: staking,
            token_address: parse_eth_address(NOX_TOKEN_MAINNET),
            chain_id: 1,
        }
    }

    /// Create config for Ethereum mainnet with official NOX Token (Infura)
    pub fn mainnet_infura(staking: EthAddress, api_key: &str) -> Self {
        Self {
            rpc_url: format!("https://mainnet.infura.io/v3/{}", api_key),
            staking_address: staking,
            token_address: parse_eth_address(NOX_TOKEN_MAINNET),
            chain_id: 1,
        }
    }

    /// Create config for Base with NOX Token
    pub fn base(staking: EthAddress, token: EthAddress) -> Self {
        Self {
            rpc_url: "https://mainnet.base.org".to_string(),
            staking_address: staking,
            token_address: token,
            chain_id: 8453,
        }
    }

    /// Create config for Arbitrum with NOX Token
    pub fn arbitrum(staking: EthAddress, token: EthAddress) -> Self {
        Self {
            rpc_url: "https://arb1.arbitrum.io/rpc".to_string(),
            staking_address: staking,
            token_address: token,
            chain_id: 42161,
        }
    }

    /// Create config for Sepolia testnet
    pub fn sepolia(staking: EthAddress, token: EthAddress) -> Self {
        Self {
            rpc_url: "https://ethereum-sepolia-rpc.publicnode.com".to_string(),
            staking_address: staking,
            token_address: token,
            chain_id: 11155111,
        }
    }

    /// Get the official mainnet NOX token address
    pub fn nox_token_mainnet() -> EthAddress {
        parse_eth_address(NOX_TOKEN_MAINNET)
    }

    /// Get the staking vault address (holds 32M NOX)
    pub fn nox_staking_vault() -> EthAddress {
        parse_eth_address(NOX_STAKING_VAULT)
    }

    /// Create mainnet config with official NOX token and staking contract addresses
    pub fn mainnet_default() -> Self {
        let staking_addr = nox_staking_contract();
        Self {
            rpc_url: "https://ethereum-rpc.publicnode.com".to_string(),
            staking_address: if staking_addr.is_empty() {
                EthAddress::zero() // Not deployed yet
            } else {
                parse_eth_address(staking_addr)
            },
            token_address: parse_eth_address(nox_token_contract()),
            chain_id: if USE_SEPOLIA { 11155111 } else { 1 },
        }
    }

    /// Create Sepolia testnet config
    pub fn sepolia_testnet() -> Self {
        let staking_addr = NOX_STAKING_CONTRACT_SEPOLIA;
        Self {
            rpc_url: "https://ethereum-sepolia-rpc.publicnode.com".to_string(),
            staking_address: if staking_addr.is_empty() {
                EthAddress::zero() // Not deployed yet
            } else {
                parse_eth_address(staking_addr)
            },
            token_address: if NOX_TOKEN_SEPOLIA.is_empty() {
                EthAddress::zero()
            } else {
                parse_eth_address(NOX_TOKEN_SEPOLIA)
            },
            chain_id: 11155111, // Sepolia
        }
    }
}
