use crate::constants::*;
use crate::error::{NonosError, NonosResult};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::fmt;
use zeroize::Zeroize;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Blake3Hash(pub [u8; BLAKE3_HASH_SIZE]);

impl Blake3Hash {
    pub fn from_bytes(bytes: [u8; BLAKE3_HASH_SIZE]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; BLAKE3_HASH_SIZE] {
        &self.0
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    pub fn from_hex(s: &str) -> NonosResult<Self> {
        let bytes = hex::decode(s).map_err(|e| NonosError::Crypto(e.to_string()))?;
        if bytes.len() != BLAKE3_HASH_SIZE {
            return Err(NonosError::Crypto("Invalid hash length".into()));
        }
        let mut arr = [0u8; BLAKE3_HASH_SIZE];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }

    pub fn zero() -> Self {
        Self([0u8; BLAKE3_HASH_SIZE])
    }
}

impl fmt::Debug for Blake3Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Blake3Hash({})", self.to_hex())
    }
}

impl fmt::Display for Blake3Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl Default for Blake3Hash {
    fn default() -> Self {
        Self::zero()
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Blake3Key(pub [u8; BLAKE3_KEY_SIZE]);

impl Blake3Key {
    pub fn from_bytes(bytes: [u8; BLAKE3_KEY_SIZE]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; BLAKE3_KEY_SIZE] {
        &self.0
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    pub fn from_hex(s: &str) -> NonosResult<Self> {
        let bytes = hex::decode(s).map_err(|e| NonosError::Crypto(e.to_string()))?;
        if bytes.len() != BLAKE3_KEY_SIZE {
            return Err(NonosError::Crypto("Invalid key length".into()));
        }
        let mut arr = [0u8; BLAKE3_KEY_SIZE];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }
}

impl fmt::Debug for Blake3Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Blake3Key({}...)", &self.to_hex()[..8])
    }
}

impl Drop for Blake3Key {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Secp256k1PrivateKey(pub [u8; SECP256K1_PRIVATE_KEY_SIZE]);

impl Secp256k1PrivateKey {
    pub fn from_bytes(bytes: [u8; SECP256K1_PRIVATE_KEY_SIZE]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; SECP256K1_PRIVATE_KEY_SIZE] {
        &self.0
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    pub fn from_hex(s: &str) -> NonosResult<Self> {
        let bytes = hex::decode(s).map_err(|e| NonosError::InvalidKey(e.to_string()))?;
        if bytes.len() != SECP256K1_PRIVATE_KEY_SIZE {
            return Err(NonosError::InvalidKey("Invalid private key length".into()));
        }
        let mut arr = [0u8; SECP256K1_PRIVATE_KEY_SIZE];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }
}

impl fmt::Debug for Secp256k1PrivateKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Secp256k1PrivateKey([REDACTED])")
    }
}

impl Drop for Secp256k1PrivateKey {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

#[serde_as]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Secp256k1PublicKey(#[serde_as(as = "serde_with::Bytes")] pub [u8; SECP256K1_PUBLIC_KEY_SIZE]);

impl Secp256k1PublicKey {
    pub fn from_bytes(bytes: [u8; SECP256K1_PUBLIC_KEY_SIZE]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; SECP256K1_PUBLIC_KEY_SIZE] {
        &self.0
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    pub fn from_hex(s: &str) -> NonosResult<Self> {
        let bytes = hex::decode(s).map_err(|e| NonosError::InvalidKey(e.to_string()))?;
        if bytes.len() != SECP256K1_PUBLIC_KEY_SIZE {
            return Err(NonosError::InvalidKey("Invalid public key length".into()));
        }
        let mut arr = [0u8; SECP256K1_PUBLIC_KEY_SIZE];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }
}

impl fmt::Debug for Secp256k1PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Secp256k1PublicKey({})", self.to_hex())
    }
}

impl fmt::Display for Secp256k1PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Ed25519PrivateKey(pub [u8; ED25519_PRIVATE_KEY_SIZE]);

impl Ed25519PrivateKey {
    pub fn from_bytes(bytes: [u8; ED25519_PRIVATE_KEY_SIZE]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; ED25519_PRIVATE_KEY_SIZE] {
        &self.0
    }
}

impl fmt::Debug for Ed25519PrivateKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Ed25519PrivateKey([REDACTED])")
    }
}

impl Drop for Ed25519PrivateKey {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Ed25519PublicKey(pub [u8; ED25519_PUBLIC_KEY_SIZE]);

impl Ed25519PublicKey {
    pub fn from_bytes(bytes: [u8; ED25519_PUBLIC_KEY_SIZE]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; ED25519_PUBLIC_KEY_SIZE] {
        &self.0
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
}

impl fmt::Debug for Ed25519PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Ed25519PublicKey({})", self.to_hex())
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct EcdsaSignature {
    pub r: [u8; 32],
    pub s: [u8; 32],
    pub v: u8,
}

impl EcdsaSignature {
    pub fn new(r: [u8; 32], s: [u8; 32], v: u8) -> Self {
        Self { r, s, v }
    }

    pub fn to_bytes(&self) -> [u8; 65] {
        let mut bytes = [0u8; 65];
        bytes[..32].copy_from_slice(&self.r);
        bytes[32..64].copy_from_slice(&self.s);
        bytes[64] = self.v;
        bytes
    }

    pub fn from_bytes(bytes: &[u8; 65]) -> Self {
        let mut r = [0u8; 32];
        let mut s = [0u8; 32];
        r.copy_from_slice(&bytes[..32]);
        s.copy_from_slice(&bytes[32..64]);
        Self {
            r,
            s,
            v: bytes[64],
        }
    }
}

impl fmt::Debug for EcdsaSignature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EcdsaSignature(v={})", self.v)
    }
}
