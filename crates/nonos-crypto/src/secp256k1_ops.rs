use nonos_types::{
    EcdsaSignature, EthAddress, NonosError, NonosResult, Secp256k1PrivateKey,
    Secp256k1PublicKey, SECP256K1_PRIVATE_KEY_SIZE,
};
use secp256k1::{Message, PublicKey, Secp256k1, SecretKey};
use sha3::{Digest, Keccak256};

thread_local! {
    static SECP256K1_CTX: Secp256k1<secp256k1::All> = Secp256k1::new();
}

pub fn generate_private_key() -> Secp256k1PrivateKey {
    let bytes = crate::random_bytes::<SECP256K1_PRIVATE_KEY_SIZE>();
    Secp256k1PrivateKey::from_bytes(bytes)
}

pub fn derive_public_key(private_key: &Secp256k1PrivateKey) -> NonosResult<Secp256k1PublicKey> {
    SECP256K1_CTX.with(|ctx| {
        let secret = SecretKey::from_slice(&private_key.0)
            .map_err(|e| NonosError::InvalidKey(e.to_string()))?;
        let public = PublicKey::from_secret_key(ctx, &secret);
        let serialized = public.serialize();
        Ok(Secp256k1PublicKey::from_bytes(serialized))
    })
}

pub fn derive_eth_address(public_key: &Secp256k1PublicKey) -> NonosResult<EthAddress> {
    SECP256K1_CTX.with(|_ctx| {
        let pubkey = PublicKey::from_slice(&public_key.0)
            .map_err(|e| NonosError::InvalidKey(e.to_string()))?;

        let uncompressed = pubkey.serialize_uncompressed();

        let hash = Keccak256::digest(&uncompressed[1..]);

        let mut address = [0u8; 20];
        address.copy_from_slice(&hash[12..]);

        Ok(EthAddress::from_bytes(address))
    })
}

pub fn derive_eth_address_from_private(
    private_key: &Secp256k1PrivateKey,
) -> NonosResult<EthAddress> {
    let public_key = derive_public_key(private_key)?;
    derive_eth_address(&public_key)
}

pub fn sign_message(
    private_key: &Secp256k1PrivateKey,
    message_hash: &[u8; 32],
) -> NonosResult<EcdsaSignature> {
    SECP256K1_CTX.with(|ctx| {
        let secret = SecretKey::from_slice(&private_key.0)
            .map_err(|e| NonosError::InvalidKey(e.to_string()))?;
        let message = Message::from_digest_slice(message_hash)
            .map_err(|e| NonosError::Crypto(e.to_string()))?;

        let (recovery_id, signature) = ctx
            .sign_ecdsa_recoverable(&message, &secret)
            .serialize_compact();

        let mut r = [0u8; 32];
        let mut s = [0u8; 32];
        r.copy_from_slice(&signature[..32]);
        s.copy_from_slice(&signature[32..]);

        let v = recovery_id.to_i32() as u8 + 27;

        Ok(EcdsaSignature::new(r, s, v))
    })
}

pub fn sign_personal_message(
    private_key: &Secp256k1PrivateKey,
    message: &[u8],
) -> NonosResult<EcdsaSignature> {
    let hash = personal_message_hash(message);
    sign_message(private_key, &hash)
}

pub fn personal_message_hash(message: &[u8]) -> [u8; 32] {
    let prefix = format!("\x19Ethereum Signed Message:\n{}", message.len());
    let mut hasher = Keccak256::new();
    hasher.update(prefix.as_bytes());
    hasher.update(message);
    hasher.finalize().into()
}

pub fn recover_public_key(
    signature: &EcdsaSignature,
    message_hash: &[u8; 32],
) -> NonosResult<Secp256k1PublicKey> {
    SECP256K1_CTX.with(|ctx| {
        let v = if signature.v >= 27 {
            signature.v - 27
        } else {
            signature.v
        };

        let recovery_id = secp256k1::ecdsa::RecoveryId::from_i32(v as i32)
            .map_err(|e| NonosError::InvalidSignature(e.to_string()))?;

        let mut sig_bytes = [0u8; 64];
        sig_bytes[..32].copy_from_slice(&signature.r);
        sig_bytes[32..].copy_from_slice(&signature.s);

        let recoverable_sig =
            secp256k1::ecdsa::RecoverableSignature::from_compact(&sig_bytes, recovery_id)
                .map_err(|e| NonosError::InvalidSignature(e.to_string()))?;

        let message = Message::from_digest_slice(message_hash)
            .map_err(|e| NonosError::Crypto(e.to_string()))?;

        let public_key = ctx
            .recover_ecdsa(&message, &recoverable_sig)
            .map_err(|e| NonosError::InvalidSignature(e.to_string()))?;

        Ok(Secp256k1PublicKey::from_bytes(public_key.serialize()))
    })
}

pub fn verify_signature(
    signature: &EcdsaSignature,
    message_hash: &[u8; 32],
    expected_address: &EthAddress,
) -> NonosResult<bool> {
    let recovered_pubkey = recover_public_key(signature, message_hash)?;
    let recovered_address = derive_eth_address(&recovered_pubkey)?;
    Ok(recovered_address == *expected_address)
}

pub fn keccak256(data: &[u8]) -> [u8; 32] {
    Keccak256::digest(data).into()
}

pub fn typed_data_hash(domain_separator: &[u8; 32], struct_hash: &[u8; 32]) -> [u8; 32] {
    let mut hasher = Keccak256::new();
    hasher.update([0x19, 0x01]);
    hasher.update(domain_separator);
    hasher.update(struct_hash);
    hasher.finalize().into()
}

pub fn compute_ecdh_shared_secret(
    private_key: &Secp256k1PrivateKey,
    public_key: &Secp256k1PublicKey,
) -> NonosResult<[u8; 32]> {
    SECP256K1_CTX.with(|_ctx| {
        let secret = SecretKey::from_slice(&private_key.0)
            .map_err(|e| NonosError::InvalidKey(e.to_string()))?;
        let public = PublicKey::from_slice(&public_key.0)
            .map_err(|e| NonosError::InvalidKey(e.to_string()))?;

        let shared_point = secp256k1::ecdh::SharedSecret::new(&public, &secret);

        let hash = keccak256(shared_point.as_ref());
        Ok(hash)
    })
}

pub fn validate_private_key(key: &[u8; 32]) -> bool {
    SecretKey::from_slice(key).is_ok()
}

pub fn validate_public_key(key: &[u8; 33]) -> bool {
    PublicKey::from_slice(key).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_generation_and_derivation() {
        let private_key = generate_private_key();
        let public_key = derive_public_key(&private_key).unwrap();
        let address = derive_eth_address(&public_key).unwrap();

        let hex = address.to_hex();
        assert!(hex.starts_with("0x"));
        assert_eq!(hex.len(), 42);
    }

    #[test]
    fn test_sign_and_verify() {
        let private_key = generate_private_key();
        let address = derive_eth_address_from_private(&private_key).unwrap();

        let message = b"NONOS transaction data";
        let message_hash = keccak256(message);

        let signature = sign_message(&private_key, &message_hash).unwrap();
        assert!(verify_signature(&signature, &message_hash, &address).unwrap());

        let wrong_hash = keccak256(b"wrong message");
        assert!(!verify_signature(&signature, &wrong_hash, &address).unwrap_or(true));
    }

    #[test]
    fn test_personal_message_signing() {
        let private_key = generate_private_key();
        let address = derive_eth_address_from_private(&private_key).unwrap();

        let message = b"Sign this message to prove ownership";
        let signature = sign_personal_message(&private_key, message).unwrap();

        let message_hash = personal_message_hash(message);
        assert!(verify_signature(&signature, &message_hash, &address).unwrap());
    }

    #[test]
    fn test_ecdh_shared_secret() {
        let alice_private = generate_private_key();
        let alice_public = derive_public_key(&alice_private).unwrap();

        let bob_private = generate_private_key();
        let bob_public = derive_public_key(&bob_private).unwrap();

        let alice_secret = compute_ecdh_shared_secret(&alice_private, &bob_public).unwrap();
        let bob_secret = compute_ecdh_shared_secret(&bob_private, &alice_public).unwrap();

        assert_eq!(alice_secret, bob_secret);
    }

    #[test]
    fn test_recover_public_key() {
        let private_key = generate_private_key();
        let public_key = derive_public_key(&private_key).unwrap();

        let message_hash = keccak256(b"test message");
        let signature = sign_message(&private_key, &message_hash).unwrap();

        let recovered = recover_public_key(&signature, &message_hash).unwrap();
        assert_eq!(public_key, recovered);
    }

    #[test]
    fn test_key_validation() {
        let valid = [1u8; 32];
        assert!(validate_private_key(&valid));

        let zero = [0u8; 32];
        assert!(!validate_private_key(&zero));
    }
}
