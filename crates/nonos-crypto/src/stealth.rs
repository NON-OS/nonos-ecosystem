use crate::{
    blake3_hash, blake3_hash_domain, compute_ecdh_shared_secret, derive_eth_address,
    derive_public_key, generate_private_key,
};
use nonos_types::{
    Blake3Hash, EthAddress, NonosError, NonosResult, Secp256k1PrivateKey, Secp256k1PublicKey,
    StealthAddress,
};
use secp256k1::{PublicKey, Secp256k1, SecretKey};

const STEALTH_KEY_DOMAIN: &str = "NONOS-v1-stealth-key";

const VIEW_TAG_DOMAIN: &str = "NONOS-v1-view-tag";

#[derive(Clone)]
pub struct StealthMetaAddress {
    pub spend_pubkey: Secp256k1PublicKey,
    pub view_pubkey: Secp256k1PublicKey,
}

impl StealthMetaAddress {
    pub fn new(spend_pubkey: Secp256k1PublicKey, view_pubkey: Secp256k1PublicKey) -> Self {
        Self {
            spend_pubkey,
            view_pubkey,
        }
    }

    pub fn encode(&self) -> String {
        format!(
            "st:eth:0x{}{}",
            self.spend_pubkey.to_hex(),
            self.view_pubkey.to_hex()
        )
    }

    pub fn decode(s: &str) -> NonosResult<Self> {
        let s = s
            .strip_prefix("st:eth:0x")
            .ok_or_else(|| NonosError::InvalidAddress("Invalid stealth meta-address prefix".into()))?;

        if s.len() != 132 {
            return Err(NonosError::InvalidAddress(
                "Invalid stealth meta-address length".into(),
            ));
        }

        let spend_pubkey = Secp256k1PublicKey::from_hex(&s[..66])?;
        let view_pubkey = Secp256k1PublicKey::from_hex(&s[66..])?;

        Ok(Self {
            spend_pubkey,
            view_pubkey,
        })
    }

    pub fn hash(&self) -> Blake3Hash {
        let mut data = Vec::with_capacity(66);
        data.extend_from_slice(&self.spend_pubkey.0);
        data.extend_from_slice(&self.view_pubkey.0);
        blake3_hash(&data)
    }
}

pub struct StealthKeyPair {
    pub spend_private: Secp256k1PrivateKey,
    pub spend_public: Secp256k1PublicKey,
    pub view_private: Secp256k1PrivateKey,
    pub view_public: Secp256k1PublicKey,
}

impl StealthKeyPair {
    pub fn generate() -> NonosResult<Self> {
        let spend_private = generate_private_key();
        let spend_public = derive_public_key(&spend_private)?;

        let view_private = generate_private_key();
        let view_public = derive_public_key(&view_private)?;

        Ok(Self {
            spend_private,
            spend_public,
            view_private,
            view_public,
        })
    }

    pub fn derive_from_master(master_key: &nonos_types::Blake3Key) -> NonosResult<Self> {
        let spend_seed = crate::derive_child_key(master_key, &[0x73746561, 0x6b657930]);
        let spend_private = Secp256k1PrivateKey::from_bytes(spend_seed.0);
        let spend_public = derive_public_key(&spend_private)?;

        let view_seed = crate::derive_child_key(master_key, &[0x73746561, 0x6b657931]);
        let view_private = Secp256k1PrivateKey::from_bytes(view_seed.0);
        let view_public = derive_public_key(&view_private)?;

        Ok(Self {
            spend_private,
            spend_public,
            view_private,
            view_public,
        })
    }

    pub fn meta_address(&self) -> StealthMetaAddress {
        StealthMetaAddress::new(self.spend_public, self.view_public)
    }
}

pub fn generate_stealth_address(
    recipient_meta: &StealthMetaAddress,
) -> NonosResult<(StealthAddress, EthAddress)> {
    let ephemeral_private = generate_private_key();
    let ephemeral_public = derive_public_key(&ephemeral_private)?;

    let shared_secret = compute_ecdh_shared_secret(&ephemeral_private, &recipient_meta.view_pubkey)?;

    let stealth_scalar = blake3_hash_domain(STEALTH_KEY_DOMAIN, &shared_secret);

    let view_tag_hash = blake3_hash_domain(VIEW_TAG_DOMAIN, &shared_secret);
    let mut view_tag = [0u8; 4];
    view_tag.copy_from_slice(&view_tag_hash.0[..4]);

    let ctx = Secp256k1::new();
    let spend_pk = PublicKey::from_slice(&recipient_meta.spend_pubkey.0)
        .map_err(|e| NonosError::InvalidKey(e.to_string()))?;

    let scalar_sk = SecretKey::from_slice(&stealth_scalar.0)
        .map_err(|e| NonosError::Crypto(e.to_string()))?;

    let scalar_pk = PublicKey::from_secret_key(&ctx, &scalar_sk);

    let stealth_pk = spend_pk.combine(&scalar_pk)
        .map_err(|e| NonosError::Crypto(e.to_string()))?;

    let stealth_pubkey = Secp256k1PublicKey::from_bytes(stealth_pk.serialize());

    let stealth_eth_address = derive_eth_address(&stealth_pubkey)?;

    let stealth_address = StealthAddress::new(
        ephemeral_public,
        view_tag,
        recipient_meta.hash(),
    );

    Ok((stealth_address, stealth_eth_address))
}

pub fn check_stealth_address(
    keypair: &StealthKeyPair,
    ephemeral_pubkey: &Secp256k1PublicKey,
    view_tag: &[u8; 4],
) -> NonosResult<bool> {
    let shared_secret = compute_ecdh_shared_secret(&keypair.view_private, ephemeral_pubkey)?;

    let view_tag_hash = blake3_hash_domain(VIEW_TAG_DOMAIN, &shared_secret);
    let expected_tag = &view_tag_hash.0[..4];

    Ok(crate::constant_time_eq(view_tag, expected_tag))
}

pub fn derive_stealth_private_key(
    keypair: &StealthKeyPair,
    ephemeral_pubkey: &Secp256k1PublicKey,
) -> NonosResult<Secp256k1PrivateKey> {
    let shared_secret = compute_ecdh_shared_secret(&keypair.view_private, ephemeral_pubkey)?;

    let stealth_scalar = blake3_hash_domain(STEALTH_KEY_DOMAIN, &shared_secret);

    let spend_sk = SecretKey::from_slice(&keypair.spend_private.0)
        .map_err(|e| NonosError::InvalidKey(e.to_string()))?;

    let scalar_sk = SecretKey::from_slice(&stealth_scalar.0)
        .map_err(|e| NonosError::Crypto(e.to_string()))?;

    let stealth_sk = spend_sk.add_tweak(&secp256k1::Scalar::from(scalar_sk))
        .map_err(|e| NonosError::Crypto(e.to_string()))?;

    Ok(Secp256k1PrivateKey::from_bytes(stealth_sk.secret_bytes()))
}

pub struct StealthScanner {
    keypair: StealthKeyPair,
    scanned_tags: Vec<[u8; 4]>,
}

impl StealthScanner {
    pub fn new(keypair: StealthKeyPair) -> Self {
        Self {
            keypair,
            scanned_tags: Vec::new(),
        }
    }

    pub fn scan_announcement(
        &mut self,
        ephemeral_pubkey: &Secp256k1PublicKey,
        view_tag: &[u8; 4],
    ) -> NonosResult<Option<Secp256k1PrivateKey>> {
        if !check_stealth_address(&self.keypair, ephemeral_pubkey, view_tag)? {
            return Ok(None);
        }

        let stealth_private = derive_stealth_private_key(&self.keypair, ephemeral_pubkey)?;

        self.scanned_tags.push(*view_tag);

        Ok(Some(stealth_private))
    }

    pub fn meta_address(&self) -> StealthMetaAddress {
        self.keypair.meta_address()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stealth_meta_address_encoding() {
        let keypair = StealthKeyPair::generate().unwrap();
        let meta = keypair.meta_address();

        let encoded = meta.encode();
        assert!(encoded.starts_with("st:eth:0x"));

        let decoded = StealthMetaAddress::decode(&encoded).unwrap();
        assert_eq!(meta.spend_pubkey, decoded.spend_pubkey);
        assert_eq!(meta.view_pubkey, decoded.view_pubkey);
    }

    #[test]
    fn test_stealth_address_generation_and_scanning() {
        let recipient_keypair = StealthKeyPair::generate().unwrap();
        let meta_address = recipient_keypair.meta_address();

        let (stealth_addr, eth_address) = generate_stealth_address(&meta_address).unwrap();

        let mut scanner = StealthScanner::new(recipient_keypair);
        let result = scanner.scan_announcement(&stealth_addr.ephemeral_pubkey, &stealth_addr.view_tag);

        assert!(result.is_ok());
        let private_key = result.unwrap();
        assert!(private_key.is_some());

        let derived_address = crate::derive_eth_address_from_private(&private_key.unwrap()).unwrap();
        assert_eq!(eth_address, derived_address);
    }

    #[test]
    fn test_stealth_address_not_ours() {
        let recipient1 = StealthKeyPair::generate().unwrap();
        let recipient2 = StealthKeyPair::generate().unwrap();

        let (stealth_addr, _) = generate_stealth_address(&recipient1.meta_address()).unwrap();

        let mut scanner = StealthScanner::new(recipient2);
        let result = scanner.scan_announcement(&stealth_addr.ephemeral_pubkey, &stealth_addr.view_tag);

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_deterministic_stealth_keys() {
        use nonos_types::Blake3Key;

        let master = Blake3Key::from_bytes([0xab; 32]);

        let keypair1 = StealthKeyPair::derive_from_master(&master).unwrap();
        let keypair2 = StealthKeyPair::derive_from_master(&master).unwrap();

        assert_eq!(keypair1.spend_public, keypair2.spend_public);
        assert_eq!(keypair1.view_public, keypair2.view_public);
    }
}
