use bip39::{Mnemonic, Language};
use k256::ecdsa::SigningKey;
use rand::RngCore;
use tiny_keccak::{Hasher, Keccak};

pub fn format_wei(wei: u128) -> String {
    let eth = wei as f64 / 1e18;
    if eth >= 1.0 {
        format!("{:.4}", eth)
    } else if eth >= 0.0001 {
        format!("{:.6}", eth)
    } else {
        format!("{:.8}", eth)
    }
}

pub fn generate_mnemonic() -> String {
    let mut entropy = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut entropy);

    Mnemonic::from_entropy_in(Language::English, &entropy)
        .expect("Failed to generate mnemonic")
        .to_string()
}

pub fn derive_keys_from_mnemonic(mnemonic: &str) -> (String, String) {
    let zero_addr = "0x0000000000000000000000000000000000000000".to_string();
    let zero_key = "0".repeat(64);

    let mnemonic = match Mnemonic::parse_normalized(mnemonic) {
        Ok(m) => m,
        Err(_) => return (zero_addr, zero_key),
    };

    let seed = mnemonic.to_seed("");

    let mut derived = [0u8; 32];
    let mut hasher = Keccak::v256();
    hasher.update(&seed);
    hasher.update(b"m/44'/60'/0'/0/0");
    hasher.finalize(&mut derived);

    let signing_key = match SigningKey::from_slice(&derived) {
        Ok(key) => key,
        Err(_) => return (zero_addr, zero_key),
    };

    let private_key_hex = hex::encode(derived);

    let verifying_key = signing_key.verifying_key();
    let public_key_bytes = verifying_key.to_encoded_point(false);
    let public_key_uncompressed = &public_key_bytes.as_bytes()[1..];

    let mut address_hash = [0u8; 32];
    let mut keccak = Keccak::v256();
    keccak.update(public_key_uncompressed);
    keccak.finalize(&mut address_hash);

    let address = format!("0x{}", hex::encode(&address_hash[12..]));

    (address, private_key_hex)
}

pub fn parse_bootstrap_progress(line: &str) -> Option<u8> {
    if let Some(start) = line.find("Bootstrapped ") {
        let rest = &line[start + 13..];
        if let Some(end) = rest.find('%') {
            if let Ok(pct) = rest[..end].trim().parse::<u8>() {
                return Some(pct);
            }
        }
    }
    None
}
