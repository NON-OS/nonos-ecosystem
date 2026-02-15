use super::commands::{IdentityAction, OutputFormat};
use nonos_crypto::{
    compute_zk_commitment, generate_identity_proof, verify_identity_proof,
    ZkIdentityProof, compute_nullifier,
};
use nonos_types::NonosResult;
use std::path::PathBuf;

pub async fn handle_identity(
    action: IdentityAction,
    data_dir: &PathBuf,
    format: &OutputFormat,
) -> NonosResult<()> {
    let identities_dir = data_dir.join("identities");
    std::fs::create_dir_all(&identities_dir)
        .map_err(|e| nonos_types::NonosError::Config(format!("Failed to create identities dir: {}", e)))?;

    match action {
        IdentityAction::Generate { label } => generate_identity(&identities_dir, label, format)?,
        IdentityAction::List => list_identities(&identities_dir, format)?,
        IdentityAction::Show { id } => show_identity(&identities_dir, &id, format)?,
        IdentityAction::Export { id, output } => export_identity(&identities_dir, &id, output, format)?,
        IdentityAction::Import { file } => import_identity(&identities_dir, &file, format)?,
        IdentityAction::Prove { id, challenge } => generate_proof(&identities_dir, &id, challenge, format)?,
        IdentityAction::Verify { proof } => verify_proof(&proof, format)?,
        IdentityAction::Register { id } => register_identity(&identities_dir, &id, format).await?,
    }

    Ok(())
}

fn generate_identity(identities_dir: &PathBuf, label: Option<String>, format: &OutputFormat) -> NonosResult<()> {
    use rand::RngCore;

    let mut secret = [0u8; 32];
    let mut blinding = [0u8; 32];
    let mut spending_key = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut secret);
    rand::thread_rng().fill_bytes(&mut blinding);
    rand::thread_rng().fill_bytes(&mut spending_key);

    let commitment = compute_zk_commitment(&secret, &blinding);
    let nullifier = compute_nullifier(&spending_key, &commitment);
    let id = hex::encode(&commitment[..8]);

    let identity = serde_json::json!({
        "version": 1,
        "id": id,
        "label": label.clone().unwrap_or_else(|| format!("identity-{}", &id[..6])),
        "commitment": hex::encode(commitment),
        "nullifier": hex::encode(nullifier),
        "secret": hex::encode(secret),
        "blinding": hex::encode(blinding),
        "spending_key": hex::encode(spending_key),
        "created_at": chrono::Utc::now().to_rfc3339(),
        "registered": false,
    });

    let identity_file = identities_dir.join(format!("{}.json", id));
    let content = serde_json::to_string_pretty(&identity)
        .map_err(|e| nonos_types::NonosError::Internal(format!("Failed to serialize: {}", e)))?;
    std::fs::write(&identity_file, &content)
        .map_err(|e| nonos_types::NonosError::Internal(format!("Failed to write identity: {}", e)))?;

    match format {
        OutputFormat::Json => {
            let output = serde_json::json!({
                "id": id,
                "label": label.unwrap_or_else(|| format!("identity-{}", &id[..6])),
                "commitment": hex::encode(commitment),
                "file": identity_file.to_string_lossy(),
            });
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
        OutputFormat::Text => {
            println!("\x1b[38;5;46m╔══════════════════════════════════════════════════════════════╗\x1b[0m");
            println!("\x1b[38;5;46m║\x1b[0m  \x1b[1;38;5;46mZK Identity Generated!\x1b[0m                                      \x1b[38;5;46m║\x1b[0m");
            println!("\x1b[38;5;46m╠══════════════════════════════════════════════════════════════╣\x1b[0m");
            println!("\x1b[38;5;46m║\x1b[0m  ID:         \x1b[38;5;226m{:<46}\x1b[0m \x1b[38;5;46m║\x1b[0m", id);
            if let Some(ref lbl) = label {
                println!("\x1b[38;5;46m║\x1b[0m  Label:      \x1b[38;5;51m{:<46}\x1b[0m \x1b[38;5;46m║\x1b[0m", lbl);
            }
            println!("\x1b[38;5;46m║\x1b[0m  Commitment: \x1b[38;5;245m{:<46}\x1b[0m \x1b[38;5;46m║\x1b[0m", &hex::encode(commitment)[..46]);
            println!("\x1b[38;5;46m╠══════════════════════════════════════════════════════════════╣\x1b[0m");
            println!("\x1b[38;5;46m║\x1b[0m  \x1b[38;5;196mIMPORTANT:\x1b[0m Your secret keys are stored in:                  \x1b[38;5;46m║\x1b[0m");
            println!("\x1b[38;5;46m║\x1b[0m  \x1b[38;5;51m{:<58}\x1b[0m \x1b[38;5;46m║\x1b[0m", identity_file.to_string_lossy());
            println!("\x1b[38;5;46m║\x1b[0m  \x1b[38;5;196mBack up this file securely!\x1b[0m                                \x1b[38;5;46m║\x1b[0m");
            println!("\x1b[38;5;46m╠══════════════════════════════════════════════════════════════╣\x1b[0m");
            println!("\x1b[38;5;46m║\x1b[0m  Next: \x1b[38;5;51mnonos identity prove {}\x1b[0m                       \x1b[38;5;46m║\x1b[0m", id);
            println!("\x1b[38;5;46m╚══════════════════════════════════════════════════════════════╝\x1b[0m");
        }
    }

    Ok(())
}

fn list_identities(identities_dir: &PathBuf, format: &OutputFormat) -> NonosResult<()> {
    let mut identities = Vec::new();

    if identities_dir.exists() {
        for entry in std::fs::read_dir(identities_dir)
            .map_err(|e| nonos_types::NonosError::Internal(format!("Failed to read dir: {}", e)))?
        {
            let entry = entry.map_err(|e| nonos_types::NonosError::Internal(format!("Failed to read entry: {}", e)))?;
            let path = entry.path();

            if path.extension().map(|e| e == "json").unwrap_or(false) {
                let content = std::fs::read_to_string(&path)
                    .map_err(|e| nonos_types::NonosError::Internal(format!("Failed to read: {}", e)))?;
                let identity: serde_json::Value = serde_json::from_str(&content)
                    .map_err(|e| nonos_types::NonosError::Internal(format!("Failed to parse: {}", e)))?;

                identities.push(serde_json::json!({
                    "id": identity["id"],
                    "label": identity["label"],
                    "created_at": identity["created_at"],
                    "registered": identity["registered"],
                }));
            }
        }
    }

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&identities).unwrap());
        }
        OutputFormat::Text => {
            if identities.is_empty() {
                println!("\x1b[38;5;245mNo identities found. Generate one with:\x1b[0m");
                println!("  \x1b[38;5;51mnonos identity generate --label \"My Identity\"\x1b[0m");
            } else {
                println!("\x1b[38;5;46mZK Identities\x1b[0m");
                println!("\x1b[38;5;245m{}\x1b[0m", "═".repeat(70));
                println!("{:<18} {:<25} {:<20} {:<10}", "ID", "Label", "Created", "Registered");
                println!("{}", "-".repeat(70));

                for id in &identities {
                    let created = id["created_at"].as_str().unwrap_or("-");
                    let short_created = if created.len() > 19 { &created[..19] } else { created };
                    let registered = if id["registered"].as_bool().unwrap_or(false) { "\x1b[38;5;46mYes\x1b[0m" } else { "\x1b[38;5;245mNo\x1b[0m" };

                    println!("{:<18} {:<25} {:<20} {}",
                        id["id"].as_str().unwrap_or("-"),
                        id["label"].as_str().unwrap_or("-"),
                        short_created,
                        registered
                    );
                }

                println!("\x1b[38;5;245m{}\x1b[0m", "═".repeat(70));
                println!("Total: {} identities", identities.len());
            }
        }
    }

    Ok(())
}

fn show_identity(identities_dir: &PathBuf, id: &str, format: &OutputFormat) -> NonosResult<()> {
    let identity_file = identities_dir.join(format!("{}.json", id));

    if !identity_file.exists() {
        return Err(nonos_types::NonosError::Config(format!("Identity '{}' not found", id)));
    }

    let content = std::fs::read_to_string(&identity_file)
        .map_err(|e| nonos_types::NonosError::Internal(format!("Failed to read: {}", e)))?;
    let identity: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| nonos_types::NonosError::Internal(format!("Failed to parse: {}", e)))?;

    match format {
        OutputFormat::Json => {
            let safe_output = serde_json::json!({
                "id": identity["id"],
                "label": identity["label"],
                "commitment": identity["commitment"],
                "nullifier": identity["nullifier"],
                "created_at": identity["created_at"],
                "registered": identity["registered"],
            });
            println!("{}", serde_json::to_string_pretty(&safe_output).unwrap());
        }
        OutputFormat::Text => {
            println!("\x1b[38;5;46mZK Identity Details\x1b[0m");
            println!("\x1b[38;5;245m{}\x1b[0m", "═".repeat(70));
            println!("ID:          \x1b[38;5;226m{}\x1b[0m", identity["id"].as_str().unwrap_or("-"));
            println!("Label:       \x1b[38;5;51m{}\x1b[0m", identity["label"].as_str().unwrap_or("-"));
            println!("Commitment:  \x1b[38;5;245m{}\x1b[0m", identity["commitment"].as_str().unwrap_or("-"));
            println!("Nullifier:   \x1b[38;5;245m{}\x1b[0m", identity["nullifier"].as_str().unwrap_or("-"));
            println!("Created:     {}", identity["created_at"].as_str().unwrap_or("-"));
            let registered = if identity["registered"].as_bool().unwrap_or(false) { "\x1b[38;5;46mYes\x1b[0m" } else { "\x1b[38;5;245mNo\x1b[0m" };
            println!("Registered:  {}", registered);
            println!("\x1b[38;5;245m{}\x1b[0m", "═".repeat(70));
        }
    }

    Ok(())
}

fn export_identity(
    identities_dir: &PathBuf,
    id: &str,
    output: Option<PathBuf>,
    format: &OutputFormat,
) -> NonosResult<()> {
    let identity_file = identities_dir.join(format!("{}.json", id));

    if !identity_file.exists() {
        return Err(nonos_types::NonosError::Config(format!("Identity '{}' not found", id)));
    }

    let content = std::fs::read_to_string(&identity_file)
        .map_err(|e| nonos_types::NonosError::Internal(format!("Failed to read: {}", e)))?;

    let output_path = output.unwrap_or_else(|| PathBuf::from(format!("{}-backup.json", id)));
    std::fs::write(&output_path, &content)
        .map_err(|e| nonos_types::NonosError::Internal(format!("Failed to write: {}", e)))?;

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::json!({ "exported_to": output_path.to_string_lossy() }));
        }
        OutputFormat::Text => {
            println!("\x1b[38;5;46m[+]\x1b[0m Identity exported to {:?}", output_path);
            println!("\x1b[38;5;196mStore this backup securely!\x1b[0m");
        }
    }

    Ok(())
}

fn import_identity(identities_dir: &PathBuf, file: &PathBuf, format: &OutputFormat) -> NonosResult<()> {
    if !file.exists() {
        return Err(nonos_types::NonosError::Config(format!("File not found: {:?}", file)));
    }

    let content = std::fs::read_to_string(file)
        .map_err(|e| nonos_types::NonosError::Internal(format!("Failed to read: {}", e)))?;
    let identity: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| nonos_types::NonosError::Internal(format!("Invalid identity file: {}", e)))?;

    let id = identity["id"].as_str()
        .ok_or_else(|| nonos_types::NonosError::Config("Missing id field".into()))?;

    let identity_file = identities_dir.join(format!("{}.json", id));
    std::fs::write(&identity_file, &content)
        .map_err(|e| nonos_types::NonosError::Internal(format!("Failed to write: {}", e)))?;

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::json!({ "imported": id }));
        }
        OutputFormat::Text => {
            println!("\x1b[38;5;46m[+]\x1b[0m Identity '{}' imported successfully", id);
        }
    }

    Ok(())
}

fn generate_proof(
    identities_dir: &PathBuf,
    id: &str,
    _challenge: Option<String>,
    format: &OutputFormat,
) -> NonosResult<()> {
    let identity_file = identities_dir.join(format!("{}.json", id));

    if !identity_file.exists() {
        return Err(nonos_types::NonosError::Config(format!("Identity '{}' not found", id)));
    }

    let content = std::fs::read_to_string(&identity_file)
        .map_err(|e| nonos_types::NonosError::Internal(format!("Failed to read: {}", e)))?;
    let identity: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| nonos_types::NonosError::Internal(format!("Failed to parse: {}", e)))?;

    let secret_hex = identity["secret"].as_str()
        .ok_or_else(|| nonos_types::NonosError::Config("Missing secret in identity".into()))?;
    let blinding_hex = identity["blinding"].as_str()
        .ok_or_else(|| nonos_types::NonosError::Config("Missing blinding in identity".into()))?;

    let secret_bytes = hex::decode(secret_hex)
        .map_err(|e| nonos_types::NonosError::Internal(format!("Invalid secret hex: {}", e)))?;
    let blinding_bytes = hex::decode(blinding_hex)
        .map_err(|e| nonos_types::NonosError::Internal(format!("Invalid blinding hex: {}", e)))?;

    let mut secret = [0u8; 32];
    let mut blinding = [0u8; 32];
    secret.copy_from_slice(&secret_bytes);
    blinding.copy_from_slice(&blinding_bytes);

    println!("\x1b[38;5;245mGenerating Groth16 proof (this may take a moment)...\x1b[0m");

    let zk_proof = generate_identity_proof(&secret, &blinding)
        .map_err(|e| nonos_types::NonosError::Internal(format!("Proof generation failed: {}", e)))?;

    let proof_b64 = zk_proof.to_base64();

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&serde_json::json!({
                "proof": proof_b64,
                "commitment": hex::encode(zk_proof.commitment),
                "proof_system": "groth16",
                "curve": "bn254"
            })).unwrap());
        }
        OutputFormat::Text => {
            println!("\x1b[38;5;46mZK Proof Generated (Groth16/BN254)\x1b[0m");
            println!("\x1b[38;5;245m{}\x1b[0m", "═".repeat(70));
            println!("Commitment: {}", hex::encode(zk_proof.commitment));
            println!("Proof: {}", proof_b64);
            println!("\n\x1b[38;5;245mThis is a cryptographically valid ZK proof.\x1b[0m");
            println!("\x1b[38;5;245mVerify with: nonos identity verify <proof>\x1b[0m");
        }
    }

    Ok(())
}

fn verify_proof(proof: &str, format: &OutputFormat) -> NonosResult<()> {
    let zk_proof = ZkIdentityProof::from_base64(proof)
        .map_err(|e| nonos_types::NonosError::Config(format!("Invalid proof format: {}", e)))?;

    println!("\x1b[38;5;245mVerifying Groth16 proof...\x1b[0m");

    let valid = verify_identity_proof(&zk_proof)
        .map_err(|e| nonos_types::NonosError::Internal(format!("Verification error: {}", e)))?;

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::json!({
                "valid": valid,
                "commitment": hex::encode(zk_proof.commitment),
                "proof_system": "groth16",
                "curve": "bn254"
            }));
        }
        OutputFormat::Text => {
            if valid {
                println!("\x1b[38;5;46m[+]\x1b[0m Proof verified successfully");
                println!("\x1b[38;5;245mCommitment: {}\x1b[0m", hex::encode(zk_proof.commitment));
                println!("\x1b[38;5;245mThe prover knows the secret preimage.\x1b[0m");
            } else {
                println!("\x1b[38;5;196m[X]\x1b[0m Proof verification FAILED");
                println!("\x1b[38;5;196mThis proof is invalid or has been tampered with.\x1b[0m");
            }
        }
    }

    Ok(())
}

async fn register_identity(
    _identities_dir: &PathBuf,
    id: &str,
    format: &OutputFormat,
) -> NonosResult<()> {
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::json!({ "id": id, "status": "pending", "message": "On-chain registration not yet implemented" }));
        }
        OutputFormat::Text => {
            println!("\x1b[38;5;226m[!]\x1b[0m On-chain registration for identity '{}' pending", id);
            println!("\x1b[38;5;245mThis feature requires staking contract deployment.\x1b[0m");
        }
    }

    Ok(())
}
