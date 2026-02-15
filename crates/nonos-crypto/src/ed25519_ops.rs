use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use nonos_types::{
    Ed25519PrivateKey, Ed25519PublicKey, NonosError, NonosResult, ED25519_SIGNATURE_SIZE,
};
use rand::rngs::OsRng;

pub fn generate_ed25519_keypair() -> (Ed25519PrivateKey, Ed25519PublicKey) {
    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();

    (
        Ed25519PrivateKey::from_bytes(signing_key.to_bytes()),
        Ed25519PublicKey::from_bytes(verifying_key.to_bytes()),
    )
}

pub fn derive_ed25519_from_seed(seed: &[u8; 32]) -> (Ed25519PrivateKey, Ed25519PublicKey) {
    let signing_key = SigningKey::from_bytes(seed);
    let verifying_key = signing_key.verifying_key();

    (
        Ed25519PrivateKey::from_bytes(signing_key.to_bytes()),
        Ed25519PublicKey::from_bytes(verifying_key.to_bytes()),
    )
}

pub fn ed25519_derive_public(private_key: &Ed25519PrivateKey) -> Ed25519PublicKey {
    let signing_key = SigningKey::from_bytes(&private_key.0);
    let verifying_key = signing_key.verifying_key();
    Ed25519PublicKey::from_bytes(verifying_key.to_bytes())
}

pub fn ed25519_sign(private_key: &Ed25519PrivateKey, message: &[u8]) -> [u8; ED25519_SIGNATURE_SIZE] {
    let signing_key = SigningKey::from_bytes(&private_key.0);
    let signature = signing_key.sign(message);
    signature.to_bytes()
}

pub fn ed25519_verify(
    public_key: &Ed25519PublicKey,
    message: &[u8],
    signature: &[u8; ED25519_SIGNATURE_SIZE],
) -> NonosResult<bool> {
    let verifying_key = VerifyingKey::from_bytes(&public_key.0)
        .map_err(|e| NonosError::InvalidKey(e.to_string()))?;

    let sig = Signature::from_bytes(signature);

    Ok(verifying_key.verify(message, &sig).is_ok())
}

#[derive(Clone)]
pub struct Ed25519Signature {
    pub bytes: [u8; ED25519_SIGNATURE_SIZE],
}

impl Ed25519Signature {
    pub fn from_bytes(bytes: [u8; ED25519_SIGNATURE_SIZE]) -> Self {
        Self { bytes }
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.bytes)
    }

    pub fn from_hex(s: &str) -> NonosResult<Self> {
        let bytes = hex::decode(s).map_err(|e| NonosError::InvalidSignature(e.to_string()))?;
        if bytes.len() != ED25519_SIGNATURE_SIZE {
            return Err(NonosError::InvalidSignature(
                "Invalid signature length".into(),
            ));
        }
        let mut arr = [0u8; ED25519_SIGNATURE_SIZE];
        arr.copy_from_slice(&bytes);
        Ok(Self { bytes: arr })
    }
}

impl std::fmt::Debug for Ed25519Signature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Ed25519Signature({}...)", &self.to_hex()[..16])
    }
}

pub struct NodeIdentity {
    private_key: Ed25519PrivateKey,
    public_key: Ed25519PublicKey,
}

impl NodeIdentity {
    pub fn generate() -> Self {
        let (private_key, public_key) = generate_ed25519_keypair();
        Self {
            private_key,
            public_key,
        }
    }

    pub fn from_private_key(private_key: Ed25519PrivateKey) -> Self {
        let public_key = ed25519_derive_public(&private_key);
        Self {
            private_key,
            public_key,
        }
    }

    pub fn public_key(&self) -> &Ed25519PublicKey {
        &self.public_key
    }

    pub fn sign(&self, message: &[u8]) -> Ed25519Signature {
        let bytes = ed25519_sign(&self.private_key, message);
        Ed25519Signature::from_bytes(bytes)
    }

    pub fn verify(&self, message: &[u8], signature: &Ed25519Signature) -> bool {
        ed25519_verify(&self.public_key, message, &signature.bytes).unwrap_or(false)
    }

    pub fn node_id(&self) -> nonos_types::NodeId {
        let hash = crate::blake3_hash(&self.public_key.0);
        nonos_types::NodeId::from_bytes(hash.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nonos_types::{ED25519_PRIVATE_KEY_SIZE, ED25519_PUBLIC_KEY_SIZE};

    #[test]
    fn test_keypair_generation() {
        let (private, public) = generate_ed25519_keypair();
        assert_eq!(private.0.len(), ED25519_PRIVATE_KEY_SIZE);
        assert_eq!(public.0.len(), ED25519_PUBLIC_KEY_SIZE);
    }

    #[test]
    fn test_sign_and_verify() {
        let (private, public) = generate_ed25519_keypair();
        let message = b"NONOS node authentication message";

        let signature = ed25519_sign(&private, message);
        assert!(ed25519_verify(&public, message, &signature).unwrap());

        assert!(!ed25519_verify(&public, b"wrong message", &signature).unwrap());
    }

    #[test]
    fn test_deterministic_derivation() {
        let seed = [0xab; 32];

        let (private1, public1) = derive_ed25519_from_seed(&seed);
        let (private2, public2) = derive_ed25519_from_seed(&seed);

        assert_eq!(private1.0, private2.0);
        assert_eq!(public1.0, public2.0);
    }

    #[test]
    fn test_node_identity() {
        let identity = NodeIdentity::generate();
        let message = b"node authentication";

        let signature = identity.sign(message);
        assert!(identity.verify(message, &signature));

        let node_id1 = identity.node_id();
        let node_id2 = identity.node_id();
        assert_eq!(node_id1.0, node_id2.0);
    }
}
