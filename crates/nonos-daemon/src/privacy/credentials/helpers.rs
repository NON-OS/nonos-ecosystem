use nonos_crypto::{blake3_derive_key, poseidon_hash};

pub fn compute_commitment(value: &[u8], salt: &[u8; 32]) -> [u8; 32] {
    let mut input = Vec::with_capacity(value.len() + 32);
    input.extend_from_slice(value);
    input.extend_from_slice(salt);
    poseidon_hash(&input)
}

pub fn compute_proof_mac(
    master_secret: &[u8; 32],
    salt: &[u8; 32],
    value: &[u8],
    challenge: &[u8; 32],
) -> [u8; 32] {
    let mut key_input = Vec::with_capacity(64);
    key_input.extend_from_slice(master_secret);
    key_input.extend_from_slice(challenge);
    let key = blake3_derive_key("nonos-credential-proof", &key_input);

    let mut mac_input = Vec::with_capacity(32 + value.len());
    mac_input.extend_from_slice(salt);
    mac_input.extend_from_slice(value);

    blake3_derive_key("nonos-credential-mac", &[&key.0[..], &mac_input[..]].concat()).0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commitment_determinism() {
        let value = b"test value";
        let salt = [0xab; 32];

        let c1 = compute_commitment(value, &salt);
        let c2 = compute_commitment(value, &salt);

        assert_eq!(c1, c2);
    }

    #[test]
    fn test_different_salt_different_commitment() {
        let value = b"test value";
        let salt1 = [0xab; 32];
        let salt2 = [0xcd; 32];

        let c1 = compute_commitment(value, &salt1);
        let c2 = compute_commitment(value, &salt2);

        assert_ne!(c1, c2);
    }
}
