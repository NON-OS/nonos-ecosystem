use ark_bn254::Bn254;
use ark_groth16::Proof;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use serde::{Deserialize, Serialize};

pub const MERKLE_DEPTH: usize = 20;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ZkCredentialType {
    Identity,
    AgeVerification { min_age: u8 },
    RegionVerification,
    Custom(u32),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ZkCredential {
    pub identity_secret: [u8; 32],
    pub nullifier_seed: [u8; 32],
    pub credential_type: ZkCredentialType,
    pub expires_at: u64,
    pub issuer_commitment: [u8; 32],
    pub commitment: [u8; 32],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MerkleProof {
    pub path: Vec<[u8; 32]>,
    pub indices: Vec<bool>,
    pub leaf_index: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ZkPublicInputs {
    pub merkle_root: [u8; 32],
    pub nullifier: [u8; 32],
    pub external_nullifier: [u8; 32],
    pub signal_hash: [u8; 32],
}

#[derive(Clone, Debug)]
pub struct ZkCredentialProof {
    pub proof: Proof<Bn254>,
    pub public_inputs: ZkPublicInputs,
}

impl Serialize for ZkCredentialProof {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut proof_bytes = Vec::new();
        self.proof
            .serialize_compressed(&mut proof_bytes)
            .map_err(serde::ser::Error::custom)?;

        let mut s = serializer.serialize_struct("ZkCredentialProof", 2)?;
        s.serialize_field("proof", &proof_bytes)?;
        s.serialize_field("public_inputs", &self.public_inputs)?;
        s.end()
    }
}

impl<'de> Deserialize<'de> for ZkCredentialProof {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Helper {
            proof: Vec<u8>,
            public_inputs: ZkPublicInputs,
        }

        let helper = Helper::deserialize(deserializer)?;
        let proof = Proof::deserialize_compressed(&helper.proof[..])
            .map_err(serde::de::Error::custom)?;

        Ok(ZkCredentialProof {
            proof,
            public_inputs: helper.public_inputs,
        })
    }
}
