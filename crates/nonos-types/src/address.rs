use crate::constants::ETH_ADDRESS_SIZE;
use crate::crypto::{Blake3Hash, Secp256k1PublicKey};
use crate::error::{NonosError, NonosResult};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EthAddress(pub [u8; ETH_ADDRESS_SIZE]);

impl EthAddress {
    pub fn from_bytes(bytes: [u8; ETH_ADDRESS_SIZE]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; ETH_ADDRESS_SIZE] {
        &self.0
    }

    pub fn to_checksum(&self) -> String {
        let hex_addr = hex::encode(self.0);
        let hash = blake3::hash(hex_addr.as_bytes());
        let hash_hex = hex::encode(hash.as_bytes());

        let mut checksummed = String::with_capacity(42);
        checksummed.push_str("0x");

        for (i, c) in hex_addr.chars().enumerate() {
            if c.is_ascii_alphabetic() {
                let hash_char = hash_hex.chars().nth(i).unwrap_or('0');
                if hash_char >= '8' {
                    checksummed.push(c.to_ascii_uppercase());
                } else {
                    checksummed.push(c.to_ascii_lowercase());
                }
            } else {
                checksummed.push(c);
            }
        }
        checksummed
    }

    pub fn to_hex(&self) -> String {
        format!("0x{}", hex::encode(self.0))
    }

    pub fn from_hex(s: &str) -> NonosResult<Self> {
        let s = s.strip_prefix("0x").unwrap_or(s);
        let bytes = hex::decode(s).map_err(|e| NonosError::InvalidAddress(e.to_string()))?;
        if bytes.len() != ETH_ADDRESS_SIZE {
            return Err(NonosError::InvalidAddress("Invalid address length".into()));
        }
        let mut arr = [0u8; ETH_ADDRESS_SIZE];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }

    pub fn zero() -> Self {
        Self([0u8; ETH_ADDRESS_SIZE])
    }
}

impl fmt::Debug for EthAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EthAddress({})", self.to_checksum())
    }
}

impl fmt::Display for EthAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_checksum())
    }
}

impl Default for EthAddress {
    fn default() -> Self {
        Self::zero()
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct StealthAddress {
    pub ephemeral_pubkey: Secp256k1PublicKey,
    pub view_tag: [u8; 4],
    pub meta_hash: Blake3Hash,
}

impl StealthAddress {
    pub fn new(
        ephemeral_pubkey: Secp256k1PublicKey,
        view_tag: [u8; 4],
        meta_hash: Blake3Hash,
    ) -> Self {
        Self {
            ephemeral_pubkey,
            view_tag,
            meta_hash,
        }
    }

    pub fn encode(&self) -> String {
        format!(
            "st:{}:{}:{}",
            self.ephemeral_pubkey.to_hex(),
            hex::encode(self.view_tag),
            self.meta_hash.to_hex()
        )
    }

    pub fn decode(s: &str) -> NonosResult<Self> {
        let s = s.strip_prefix("st:").ok_or_else(|| {
            NonosError::InvalidAddress("Invalid stealth address prefix".into())
        })?;

        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 3 {
            return Err(NonosError::InvalidAddress(
                "Invalid stealth address format".into(),
            ));
        }

        let ephemeral_pubkey = Secp256k1PublicKey::from_hex(parts[0])?;
        let view_tag_bytes =
            hex::decode(parts[1]).map_err(|e| NonosError::InvalidAddress(e.to_string()))?;
        if view_tag_bytes.len() != 4 {
            return Err(NonosError::InvalidAddress("Invalid view tag length".into()));
        }
        let mut view_tag = [0u8; 4];
        view_tag.copy_from_slice(&view_tag_bytes);
        let meta_hash = Blake3Hash::from_hex(parts[2])?;

        Ok(Self {
            ephemeral_pubkey,
            view_tag,
            meta_hash,
        })
    }
}

impl fmt::Debug for StealthAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "StealthAddress({}...)", &self.encode()[..20])
    }
}
