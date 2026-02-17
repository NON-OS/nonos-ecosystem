use super::utils::parse_eth_address;
use libp2p::identity::Keypair;
use nonos_daemon::NodeConfig;
use nonos_types::NonosResult;
use std::path::PathBuf;

pub fn init_node(
    config_path: &PathBuf,
    data_dir: &PathBuf,
    force: bool,
    _non_interactive: bool,
    reward_address: Option<String>,
    nickname: Option<String>,
) -> NonosResult<()> {
    println!("\x1b[38;5;46mInitializing NONOS daemon...\x1b[0m");
    println!();

    if config_path.exists() && !force {
        println!("\x1b[38;5;226mConfiguration already exists at {:?}\x1b[0m", config_path);
        println!("Use --force to overwrite");
        return Ok(());
    }

    std::fs::create_dir_all(data_dir)
        .map_err(|e| nonos_types::NonosError::Config(format!("Failed to create data directory: {}", e)))?;

    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| nonos_types::NonosError::Config(format!("Failed to create config directory: {}", e)))?;
    }

    let mut config = NodeConfig::default();

    if let Some(addr) = reward_address {
        if addr.starts_with("0x") && addr.len() == 42 {
            config.rewards.reward_address = parse_eth_address(&addr)?;
        } else {
            return Err(nonos_types::NonosError::Config(
                "Invalid reward address format. Expected 0x followed by 40 hex characters.".into()
            ));
        }
    }

    if let Some(name) = &nickname {
        let nickname_path = data_dir.join("nickname");
        std::fs::write(&nickname_path, name)
            .map_err(|e| nonos_types::NonosError::Config(format!("Failed to write nickname: {}", e)))?;
    }

    config.save(config_path)?;

    // Generate libp2p P2P identity (protobuf-encoded)
    let p2p_identity_path = data_dir.join("p2p_identity.key");
    let peer_id = if !p2p_identity_path.exists() || force {
        let keypair = Keypair::generate_ed25519();
        let key_bytes = keypair.to_protobuf_encoding()
            .map_err(|e| nonos_types::NonosError::Config(format!("Failed to encode P2P identity: {}", e)))?;
        std::fs::write(&p2p_identity_path, &key_bytes)
            .map_err(|e| nonos_types::NonosError::Config(format!("Failed to save P2P identity: {}", e)))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&p2p_identity_path)?.permissions();
            perms.set_mode(0o600);
            std::fs::set_permissions(&p2p_identity_path, perms)?;
        }

        let peer_id = keypair.public().to_peer_id();
        println!("\x1b[38;5;46m[+]\x1b[0m Generated P2P identity: \x1b[38;5;226m{}\x1b[0m", peer_id);
        peer_id.to_string()
    } else {
        let key_bytes = std::fs::read(&p2p_identity_path)
            .map_err(|e| nonos_types::NonosError::Config(format!("Failed to read P2P identity: {}", e)))?;
        let keypair = Keypair::from_protobuf_encoding(&key_bytes)
            .map_err(|e| nonos_types::NonosError::Config(format!("Invalid P2P identity: {}", e)))?;
        keypair.public().to_peer_id().to_string()
    };

    let identities_dir = data_dir.join("identities");
    std::fs::create_dir_all(&identities_dir)
        .map_err(|e| nonos_types::NonosError::Config(format!("Failed to create identities dir: {}", e)))?;

    // Print success message
    let config_str = config_path.display().to_string();
    let data_str = data_dir.display().to_string();
    let nickname_display = nickname.as_deref().unwrap_or("(not set)");

    println!();
    println!("\x1b[38;5;46m══════════════════════════════════════════════════════════════════\x1b[0m");
    println!("\x1b[1;38;5;46m  NONOS Daemon Initialized Successfully!\x1b[0m");
    println!("\x1b[38;5;46m══════════════════════════════════════════════════════════════════\x1b[0m");
    println!();
    println!("  \x1b[38;5;245mConfig:\x1b[0m    \x1b[38;5;51m{}\x1b[0m", config_str);
    println!("  \x1b[38;5;245mData:\x1b[0m      \x1b[38;5;51m{}\x1b[0m", data_str);
    println!("  \x1b[38;5;245mNickname:\x1b[0m  \x1b[38;5;51m{}\x1b[0m", nickname_display);
    println!("  \x1b[38;5;245mPeer ID:\x1b[0m   \x1b[38;5;226m{}\x1b[0m", peer_id);
    println!();
    println!("\x1b[38;5;46m══════════════════════════════════════════════════════════════════\x1b[0m");
    println!("  \x1b[38;5;226mNext steps:\x1b[0m");
    println!();
    println!("  1. Set reward address in config.toml (if not set)");
    println!("  2. Start the daemon:  \x1b[38;5;51mnonos run\x1b[0m");
    println!("  3. Check status:      \x1b[38;5;51mnonos info\x1b[0m");
    println!("  4. Launch dashboard:  \x1b[38;5;51mnonos dash\x1b[0m");
    println!();
    println!("\x1b[38;5;46m══════════════════════════════════════════════════════════════════\x1b[0m");

    Ok(())
}
