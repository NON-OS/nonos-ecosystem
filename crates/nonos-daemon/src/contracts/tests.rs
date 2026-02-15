use super::*;
use nonos_types::EthAddress;

#[test]
fn test_contract_config_default() {
    let config = ContractConfig::default();
    assert_eq!(config.chain_id, 31337);
    assert!(config.rpc_url.contains("localhost"));
}

#[test]
fn test_contract_config_chains() {
    let mainnet = ContractConfig::mainnet(EthAddress::zero());
    assert_eq!(mainnet.chain_id, 1);
    assert_eq!(mainnet.token_address, ContractConfig::nox_token_mainnet());

    let base = ContractConfig::base(EthAddress::zero(), EthAddress::zero());
    assert_eq!(base.chain_id, 8453);

    let arbitrum = ContractConfig::arbitrum(EthAddress::zero(), EthAddress::zero());
    assert_eq!(arbitrum.chain_id, 42161);

    let sepolia = ContractConfig::sepolia(EthAddress::zero(), EthAddress::zero());
    assert_eq!(sepolia.chain_id, 11155111);
}

#[test]
fn test_nox_token_mainnet_address() {
    let addr = ContractConfig::nox_token_mainnet();
    assert_eq!(
        hex::encode(&addr.0),
        "0a26c80be4e060e688d7c23addb92cbb5d2c9eca"
    );
}

#[test]
fn test_contract_client_creation() {
    let config = ContractConfig::default();
    let _client = ContractClient::new(config);
}
