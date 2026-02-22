mod bindings;
mod config;
mod client;
mod auto_claim;
mod rpc;

pub use bindings::{NoxStaking, NoxToken};
pub use config::{
    ContractConfig,
    NOX_TOKEN_MAINNET, NOX_STAKING_VAULT, NOX_TOKEN_IMPLEMENTATION,
    NOX_STAKING_CONTRACT_MAINNET, NOX_STAKING_CONTRACT_SEPOLIA,
    NOX_TOKEN_SEPOLIA, STAKING_GENESIS_TIMESTAMP_SEPOLIA,
    USE_SEPOLIA, EPOCH_DURATION_SECS, STAKING_GENESIS_TIMESTAMP,
    COLLATERAL_MANAGER_SEPOLIA, WORK_REGISTRY_SEPOLIA,
    FEE_ROUTER_SEPOLIA, PRIVACY_LIQUIDITY_POOL_SEPOLIA, REWARD_DISTRIBUTOR_SEPOLIA,
    nox_staking_contract, nox_token_contract, staking_genesis_timestamp,
    current_epoch, is_staking_configured,
};
pub use client::ContractClient;
pub use auto_claim::AutoClaimManager;
pub use rpc::{RpcProvider, RpcEndpoint, MAINNET_RPC_ENDPOINTS, SEPOLIA_RPC_ENDPOINTS};

#[cfg(test)]
mod tests;
