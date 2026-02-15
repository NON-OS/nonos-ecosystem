use ark_bn254::Fr;
use ark_crypto_primitives::sponge::{
    poseidon::{find_poseidon_ark_and_mds, PoseidonConfig, PoseidonSponge},
    CryptographicSponge,
};
use ark_ff::PrimeField;
use ark_serialize::CanonicalSerialize;
use std::sync::OnceLock;

static POSEIDON_CONFIG: OnceLock<PoseidonConfig<Fr>> = OnceLock::new();

pub fn get_poseidon_config() -> &'static PoseidonConfig<Fr> {
    POSEIDON_CONFIG.get_or_init(|| {
        let rate = 2;
        let alpha = 5u64;
        let full_rounds = 8;
        let partial_rounds = 57;

        let (ark, mds) = find_poseidon_ark_and_mds::<Fr>(
            254,
            rate,
            full_rounds,
            partial_rounds,
            0,
        );

        PoseidonConfig {
            full_rounds: full_rounds as usize,
            partial_rounds: partial_rounds as usize,
            alpha,
            ark,
            mds,
            rate,
            capacity: 1,
        }
    })
}

pub fn poseidon_hash_native(inputs: &[Fr]) -> Fr {
    let config = get_poseidon_config();

    let mut sponge = PoseidonSponge::new(config);
    sponge.absorb(&inputs);

    let output: Vec<Fr> = sponge.squeeze_field_elements(1);
    output[0]
}

pub fn bytes_to_field(bytes: &[u8; 32]) -> Fr {
    Fr::from_le_bytes_mod_order(bytes)
}

pub fn field_to_bytes(field: &Fr) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    field.serialize_compressed(&mut bytes[..]).unwrap();
    bytes
}

pub fn blake3_hash_32(data: &[u8]) -> [u8; 32] {
    *blake3::hash(data).as_bytes()
}
