use nonos_crypto::{keccak256, sign_message};
use nonos_types::{
    Blake3Hash, EcdsaSignature, EthAddress, NonosResult, Secp256k1PrivateKey,
    TokenAmount,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransactionType {
    Legacy,
    AccessList,
    Eip1559,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransactionRequest {
    pub chain_id: u64,
    pub to: EthAddress,
    pub value: u128,
    pub data: Vec<u8>,
    pub gas_limit: u64,
    pub max_fee_per_gas: u128,
    pub max_priority_fee_per_gas: u128,
    pub nonce: Option<u64>,
}

impl TransactionRequest {
    pub fn transfer(to: EthAddress, amount: TokenAmount, chain_id: u64) -> Self {
        Self {
            chain_id,
            to,
            value: amount.raw,
            data: Vec::new(),
            gas_limit: 21_000,
            max_fee_per_gas: 0,
            max_priority_fee_per_gas: 0,
            nonce: None,
        }
    }

    pub fn contract_call(to: EthAddress, data: Vec<u8>, chain_id: u64) -> Self {
        Self {
            chain_id,
            to,
            value: 0,
            data,
            gas_limit: 100_000,
            max_fee_per_gas: 0,
            max_priority_fee_per_gas: 0,
            nonce: None,
        }
    }

    pub fn with_gas(
        mut self,
        gas_limit: u64,
        max_fee_per_gas: u128,
        max_priority_fee_per_gas: u128,
    ) -> Self {
        self.gas_limit = gas_limit;
        self.max_fee_per_gas = max_fee_per_gas;
        self.max_priority_fee_per_gas = max_priority_fee_per_gas;
        self
    }

    pub fn with_nonce(mut self, nonce: u64) -> Self {
        self.nonce = Some(nonce);
        self
    }

    pub fn signing_hash(&self, nonce: u64) -> [u8; 32] {
        let mut rlp_data = Vec::new();

        rlp_data.push(0x02);

        let mut fields = Vec::new();

        fields.extend(&self.chain_id.to_be_bytes());
        fields.extend(&nonce.to_be_bytes());
        fields.extend(&self.max_priority_fee_per_gas.to_be_bytes());
        fields.extend(&self.max_fee_per_gas.to_be_bytes());
        fields.extend(&self.gas_limit.to_be_bytes());
        fields.extend(&self.to.0);
        fields.extend(&self.value.to_be_bytes());
        fields.extend(&self.data);

        rlp_data.extend(&fields);

        keccak256(&rlp_data)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SignedTransaction {
    pub request: TransactionRequest,
    pub nonce: u64,
    pub signature: EcdsaSignature,
    pub hash: Blake3Hash,
}

impl SignedTransaction {
    pub fn raw_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.push(0x02);

        bytes.extend(&self.request.chain_id.to_be_bytes());
        bytes.extend(&self.nonce.to_be_bytes());
        bytes.extend(&self.request.max_priority_fee_per_gas.to_be_bytes());
        bytes.extend(&self.request.max_fee_per_gas.to_be_bytes());
        bytes.extend(&self.request.gas_limit.to_be_bytes());
        bytes.extend(&self.request.to.0);
        bytes.extend(&self.request.value.to_be_bytes());
        bytes.extend(&self.request.data);

        bytes.extend(&self.signature.r);
        bytes.extend(&self.signature.s);
        bytes.push(self.signature.v);

        bytes
    }

    pub fn raw_hex(&self) -> String {
        format!("0x{}", hex::encode(self.raw_bytes()))
    }
}

pub struct TransactionSigner;

impl TransactionSigner {
    pub fn sign(
        request: TransactionRequest,
        nonce: u64,
        private_key: &Secp256k1PrivateKey,
    ) -> NonosResult<SignedTransaction> {
        let signing_hash = request.signing_hash(nonce);
        let signature = sign_message(private_key, &signing_hash)?;

        let mut tx_data = Vec::new();
        tx_data.push(0x02);
        tx_data.extend(&request.chain_id.to_be_bytes());
        tx_data.extend(&nonce.to_be_bytes());
        tx_data.extend(&request.to.0);
        tx_data.extend(&request.value.to_be_bytes());
        tx_data.extend(&signature.r);
        tx_data.extend(&signature.s);

        let tx_hash = nonos_crypto::blake3_hash(&tx_data);

        Ok(SignedTransaction {
            request,
            nonce,
            signature,
            hash: tx_hash,
        })
    }
}

pub struct Erc20Encoder;

impl Erc20Encoder {
    pub fn transfer(to: &EthAddress, amount: &TokenAmount) -> Vec<u8> {
        let mut data = Vec::with_capacity(68);

        data.extend(&[0xa9, 0x05, 0x9c, 0xbb]);

        data.extend(&[0u8; 12]);
        data.extend(&to.0);

        let amount_bytes = amount.raw.to_be_bytes();
        data.extend(&[0u8; 16]);
        data.extend(&amount_bytes);

        data
    }

    pub fn approve(spender: &EthAddress, amount: &TokenAmount) -> Vec<u8> {
        let mut data = Vec::with_capacity(68);

        data.extend(&[0x09, 0x5e, 0xa7, 0xb3]);

        data.extend(&[0u8; 12]);
        data.extend(&spender.0);

        let amount_bytes = amount.raw.to_be_bytes();
        data.extend(&[0u8; 16]);
        data.extend(&amount_bytes);

        data
    }

    pub fn balance_of(owner: &EthAddress) -> Vec<u8> {
        let mut data = Vec::with_capacity(36);

        data.extend(&[0x70, 0xa0, 0x82, 0x31]);

        data.extend(&[0u8; 12]);
        data.extend(&owner.0);

        data
    }
}

pub struct StakingEncoder;

impl StakingEncoder {
    pub fn stake(amount: &TokenAmount) -> Vec<u8> {
        let mut data = Vec::with_capacity(36);

        data.extend(&[0xa6, 0x94, 0xfc, 0x3a]);

        let amount_bytes = amount.raw.to_be_bytes();
        data.extend(&[0u8; 16]);
        data.extend(&amount_bytes);

        data
    }

    pub fn unstake(amount: &TokenAmount) -> Vec<u8> {
        let mut data = Vec::with_capacity(36);

        data.extend(&[0x2e, 0x17, 0xde, 0x78]);

        let amount_bytes = amount.raw.to_be_bytes();
        data.extend(&[0u8; 16]);
        data.extend(&amount_bytes);

        data
    }

    pub fn claim_rewards() -> Vec<u8> {
        vec![0x37, 0x2b, 0x9f, 0x58]
    }

    pub fn get_stake(staker: &EthAddress) -> Vec<u8> {
        let mut data = Vec::with_capacity(36);

        data.extend(&[0x7a, 0x76, 0x64, 0x60]);

        data.extend(&[0u8; 12]);
        data.extend(&staker.0);

        data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transfer_request() {
        let to = EthAddress::from_bytes([0xab; 20]);
        let amount = TokenAmount::from_raw(1_000_000_000_000_000_000, 18);

        let request = TransactionRequest::transfer(to, amount, 1)
            .with_gas(21_000, 30_000_000_000, 1_000_000_000)
            .with_nonce(5);

        assert_eq!(request.gas_limit, 21_000);
        assert_eq!(request.nonce, Some(5));
    }

    #[test]
    fn test_erc20_transfer_encoding() {
        let to = EthAddress::from_bytes([0xab; 20]);
        let amount = TokenAmount::from_raw(100_000_000_000_000_000_000, 18);

        let data = Erc20Encoder::transfer(&to, &amount);

        assert_eq!(&data[..4], &[0xa9, 0x05, 0x9c, 0xbb]);
        assert_eq!(data.len(), 68);
    }

    #[test]
    fn test_staking_encoding() {
        let amount = TokenAmount::from_raw(10_000_000_000_000_000_000_000, 18);

        let stake_data = StakingEncoder::stake(&amount);
        assert_eq!(&stake_data[..4], &[0xa6, 0x94, 0xfc, 0x3a]);

        let claim_data = StakingEncoder::claim_rewards();
        assert_eq!(claim_data.len(), 4);
    }
}
