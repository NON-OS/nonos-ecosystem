use ark_bn254::{Bn254, Fr};
use ark_ff::{Field, PrimeField};
use ark_groth16::{Groth16, PreparedVerifyingKey, Proof, ProvingKey};
use ark_r1cs_std::{
    alloc::AllocVar,
    eq::EqGadget,
    fields::{fp::FpVar, FieldVar},
};
use ark_relations::r1cs::{
    ConstraintSynthesizer, ConstraintSystemRef, SynthesisError,
};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_snark::SNARK;
use ark_std::rand::thread_rng;
use std::sync::OnceLock;

static PROVING_KEY: OnceLock<ProvingKey<Bn254>> = OnceLock::new();
static VERIFYING_KEY: OnceLock<PreparedVerifyingKey<Bn254>> = OnceLock::new();

const MIMC_ROUNDS: usize = 91;

const MIMC_CONSTANTS: [u64; MIMC_ROUNDS] = [
    0, 7120861356467848435, 5024705281721889198, 17049961171657901334,
    9067690659255326133, 10450258261721523299, 7651805021865731224, 17119495028048486740,
    11709568770518417699, 13196915738065505387, 8249677423061994002, 1633515469962582721,
    16784442748340881121, 3033451528878493073, 3137828293920302358, 14357425728416738578,
    14316329847509215026, 7096211616547011057, 9006832650074870225, 14135068057671476579,
    6835435278967818664, 3488764820906196081, 4396803421433481609, 8715211266515380192,
    14129580808676458957, 15326677838848380666, 11181211681498256664, 15742639418975103911,
    1498364132459989403, 7925698049417562431, 16783683217117410823, 6265397681164692517,
    3908139821073616341, 246254336593267377, 10659495248951313743, 1648057961072498557,
    15327519922520968867, 4573033023237616331, 15082040906792116780, 17526701471253247795,
    13113498921708744335, 5765180028988623278, 1544179214589942198, 5184439678372465031,
    17131849407692376876, 12627834738498206789, 16561551279514757107, 9629053826315617115,
    8688576994891737966, 16816911585885485942, 13562976811195007559, 7619620176470892882,
    17457694489450936844, 5826717089558314707, 8080384868324616498, 5556894639413178404,
    10246816909539108405, 12653832972399870761, 11500938668064490955, 17451539608019164682,
    14083808311813498441, 16458049895700619582, 3651655058497573797, 5765417445419057572,
    14389478918880125365, 7275698258041998803, 13242606468816754368, 15856682807696198096,
    15854969947560766986, 2425810811685610505, 18246380640902498524, 12280932756498606990,
    12093397934187757821, 13987558442073996209, 9710925296303989440, 10869066135474417993,
    6952717714355421236, 12814605282339832054, 4609357254303353218, 1405761722193482161,
    5765814044121982137, 10044009356539295688, 7055894097839531913, 11114436024839064097,
    8960841911508618234, 15929260158942644533, 10627025778877898159, 14358340615824582486,
    18296431013091616568, 10463902484601441940, 16628803984044557786,
];

fn bytes_to_field(bytes: &[u8]) -> Fr {
    Fr::from_le_bytes_mod_order(bytes)
}

fn field_to_bytes(f: &Fr) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    f.serialize_compressed(&mut bytes[..]).expect("Serialization failed");
    bytes
}

fn mimc_hash(left: Fr, right: Fr) -> Fr {
    let mut h = Fr::ZERO;
    let k = left + right;

    for c in MIMC_CONSTANTS.iter() {
        let c_fr = Fr::from(*c);
        let t = h + k + c_fr;
        h = t * t * t * t * t;
    }

    h + k
}

fn mimc_hash_gadget(
    _cs: ConstraintSystemRef<Fr>,
    left: &FpVar<Fr>,
    right: &FpVar<Fr>,
) -> Result<FpVar<Fr>, SynthesisError> {
    let k = left + right;
    let mut h = FpVar::constant(Fr::ZERO);

    for c in MIMC_CONSTANTS.iter() {
        let c_var = FpVar::constant(Fr::from(*c));
        let t = &h + &k + &c_var;
        let t2 = &t * &t;
        let t4 = &t2 * &t2;
        h = &t4 * &t;
    }

    Ok(h + k)
}

#[derive(Clone)]
pub struct IdentityCircuit {
    pub secret: Option<Fr>,
    pub blinding: Option<Fr>,
    pub commitment: Option<Fr>,
}

impl ConstraintSynthesizer<Fr> for IdentityCircuit {
    fn generate_constraints(
        self,
        cs: ConstraintSystemRef<Fr>,
    ) -> Result<(), SynthesisError> {
        let secret_var = FpVar::new_witness(cs.clone(), || {
            self.secret.ok_or(SynthesisError::AssignmentMissing)
        })?;

        let blinding_var = FpVar::new_witness(cs.clone(), || {
            self.blinding.ok_or(SynthesisError::AssignmentMissing)
        })?;

        let commitment_var = FpVar::new_input(cs.clone(), || {
            self.commitment.ok_or(SynthesisError::AssignmentMissing)
        })?;

        let computed_commitment = mimc_hash_gadget(cs.clone(), &secret_var, &blinding_var)?;

        computed_commitment.enforce_equal(&commitment_var)?;

        Ok(())
    }
}

fn initialize_keys() {
    if PROVING_KEY.get().is_some() {
        return;
    }

    let mut rng = thread_rng();

    let dummy_circuit = IdentityCircuit {
        secret: Some(Fr::from(0u64)),
        blinding: Some(Fr::from(0u64)),
        commitment: Some(Fr::from(0u64)),
    };

    let (pk, vk) = Groth16::<Bn254>::circuit_specific_setup(dummy_circuit, &mut rng)
        .expect("Failed to generate ZK keys");

    let pvk = Groth16::<Bn254>::process_vk(&vk).expect("Failed to process verifying key");

    let _ = PROVING_KEY.set(pk);
    let _ = VERIFYING_KEY.set(pvk);
}

#[derive(Clone)]
pub struct ZkIdentityProof {
    pub proof: Proof<Bn254>,
    pub commitment: [u8; 32],
}

impl ZkIdentityProof {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        self.proof.serialize_compressed(&mut bytes).expect("Serialization failed");
        bytes.extend_from_slice(&self.commitment);
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() < 32 {
            return Err("Invalid proof bytes");
        }

        let proof_len = bytes.len() - 32;
        let proof = Proof::<Bn254>::deserialize_compressed(&bytes[..proof_len])
            .map_err(|_| "Failed to deserialize proof")?;

        let mut commitment = [0u8; 32];
        commitment.copy_from_slice(&bytes[proof_len..]);

        Ok(Self { proof, commitment })
    }

    pub fn to_base64(&self) -> String {
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, self.to_bytes())
    }

    pub fn from_base64(s: &str) -> Result<Self, &'static str> {
        let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, s)
            .map_err(|_| "Invalid base64")?;
        Self::from_bytes(&bytes)
    }
}

pub fn generate_identity_proof(
    secret: &[u8; 32],
    blinding: &[u8; 32],
) -> Result<ZkIdentityProof, &'static str> {
    initialize_keys();

    let pk = PROVING_KEY.get().ok_or("Proving key not initialized")?;

    let secret_fr = bytes_to_field(secret);
    let blinding_fr = bytes_to_field(blinding);
    let commitment_fr = mimc_hash(secret_fr, blinding_fr);

    let circuit = IdentityCircuit {
        secret: Some(secret_fr),
        blinding: Some(blinding_fr),
        commitment: Some(commitment_fr),
    };

    let mut rng = thread_rng();
    let proof = Groth16::<Bn254>::prove(pk, circuit, &mut rng)
        .map_err(|_| "Proof generation failed")?;

    Ok(ZkIdentityProof {
        proof,
        commitment: field_to_bytes(&commitment_fr),
    })
}

pub fn verify_identity_proof(proof: &ZkIdentityProof) -> Result<bool, &'static str> {
    initialize_keys();

    let pvk = VERIFYING_KEY.get().ok_or("Verifying key not initialized")?;

    let commitment_fr = bytes_to_field(&proof.commitment);
    let public_inputs = vec![commitment_fr];

    let valid = Groth16::<Bn254>::verify_with_processed_vk(pvk, &public_inputs, &proof.proof)
        .map_err(|_| "Verification failed")?;

    Ok(valid)
}

pub fn compute_zk_commitment(secret: &[u8; 32], blinding: &[u8; 32]) -> [u8; 32] {
    let secret_fr = bytes_to_field(secret);
    let blinding_fr = bytes_to_field(blinding);
    let commitment_fr = mimc_hash(secret_fr, blinding_fr);
    field_to_bytes(&commitment_fr)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::RngCore;

    #[test]
    fn test_zk_identity_proof() {
        let mut rng = thread_rng();

        let mut secret = [0u8; 32];
        let mut blinding = [0u8; 32];
        rng.fill_bytes(&mut secret);
        rng.fill_bytes(&mut blinding);

        let proof = generate_identity_proof(&secret, &blinding).unwrap();

        let valid = verify_identity_proof(&proof).unwrap();
        assert!(valid, "Proof should be valid");
    }

    #[test]
    fn test_proof_serialization() {
        let mut rng = thread_rng();

        let mut secret = [0u8; 32];
        let mut blinding = [0u8; 32];
        rng.fill_bytes(&mut secret);
        rng.fill_bytes(&mut blinding);

        let proof = generate_identity_proof(&secret, &blinding).unwrap();

        let base64 = proof.to_base64();
        let restored = ZkIdentityProof::from_base64(&base64).unwrap();

        let valid = verify_identity_proof(&restored).unwrap();
        assert!(valid, "Restored proof should be valid");
    }

    #[test]
    fn test_different_secrets_different_commitments() {
        let secret1 = [1u8; 32];
        let secret2 = [2u8; 32];
        let blinding = [0u8; 32];

        let commitment1 = compute_zk_commitment(&secret1, &blinding);
        let commitment2 = compute_zk_commitment(&secret2, &blinding);

        assert_ne!(commitment1, commitment2, "Different secrets should produce different commitments");
    }

    #[test]
    fn test_invalid_proof_rejected() {
        let mut rng = thread_rng();

        let mut secret = [0u8; 32];
        let mut blinding = [0u8; 32];
        rng.fill_bytes(&mut secret);
        rng.fill_bytes(&mut blinding);

        let mut proof = generate_identity_proof(&secret, &blinding).unwrap();

        proof.commitment[0] ^= 0xFF;

        let valid = verify_identity_proof(&proof).unwrap();
        assert!(!valid, "Tampered proof should be invalid");
    }
}
