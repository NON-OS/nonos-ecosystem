mod circuit;
mod hash;
mod merkle;
mod system;
mod types;

pub use system::ZkCredentialSystem;
pub use types::{
    MerkleProof, ZkCredential, ZkCredentialProof, ZkCredentialType, ZkPublicInputs, MERKLE_DEPTH,
};

#[cfg(test)]
mod tests {
    use super::*;
    use ark_bn254::Fr;
    use hash::poseidon_hash_native;
    use merkle::MerkleTree;

    #[test]
    fn test_poseidon_native() {
        let a = Fr::from(123u64);
        let b = Fr::from(456u64);

        let h1 = poseidon_hash_native(&[a, b]);
        let h2 = poseidon_hash_native(&[a, b]);

        assert_eq!(h1, h2, "Hash should be deterministic");

        let h3 = poseidon_hash_native(&[b, a]);
        assert_ne!(h1, h3, "Hash should be order-sensitive");
    }

    #[test]
    fn test_merkle_tree() {
        let mut tree = MerkleTree::new();

        let root1 = tree.root();

        let commitment1 = [1u8; 32];
        let commitment2 = [2u8; 32];

        tree.insert(commitment1);
        let root2 = tree.root();
        assert_ne!(root1, root2);

        tree.insert(commitment2);
        let root3 = tree.root();
        assert_ne!(root2, root3);

        let proof1 = tree.get_proof(&commitment1).unwrap();
        let proof2 = tree.get_proof(&commitment2).unwrap();

        assert_eq!(proof1.leaf_index, 0);
        assert_eq!(proof2.leaf_index, 1);
    }

    #[tokio::test]
    async fn test_credential_issuance() {
        let issuer_secret = [42u8; 32];
        let system = ZkCredentialSystem::new(issuer_secret);

        let user_secret = [123u8; 32];
        let credential = system
            .issue_credential(user_secret, ZkCredentialType::Identity, 0)
            .await
            .unwrap();

        assert_eq!(credential.identity_secret, user_secret);
        assert_eq!(system.credential_count().await, 1);
    }

    #[tokio::test]
    #[ignore]
    async fn test_full_proof() {
        let issuer_secret = [42u8; 32];
        let mut system = ZkCredentialSystem::new(issuer_secret);

        system.initialize().unwrap();

        let user_secret = [123u8; 32];
        let credential = system
            .issue_credential(user_secret, ZkCredentialType::Identity, 0)
            .await
            .unwrap();

        let external_nullifier = [1u8; 32];
        let signal = b"test signal";
        let proof = system
            .generate_proof(&credential, external_nullifier, signal)
            .await
            .unwrap();

        let valid = system.verify_and_record(&proof).await.unwrap();
        assert!(valid, "Proof should be valid");

        let valid2 = system.verify_and_record(&proof).await.unwrap();
        assert!(!valid2, "Double-use should be rejected");
    }
}
