use super::utils::parse_eth_address;
use nonos_daemon::NodeConfig;
use nonos_crypto::generate_ed25519_keypair;
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

    if let Some(name) = nickname {
        let nickname_path = data_dir.join("nickname");
        std::fs::write(&nickname_path, name)
            .map_err(|e| nonos_types::NonosError::Config(format!("Failed to write nickname: {}", e)))?;
    }

    config.save(config_path)?;

    let identity_path = data_dir.join("identity");
    if !identity_path.exists() {
        let keypair = generate_ed25519_keypair();
        let private_key = keypair.0;
        std::fs::write(&identity_path, private_key.as_bytes())
            .map_err(|e| nonos_types::NonosError::Config(format!("Failed to write identity: {}", e)))?;
        println!("\x1b[38;5;46m[+]\x1b[0m Generated new node identity");
    }

    let identities_dir = data_dir.join("identities");
    std::fs::create_dir_all(&identities_dir)
        .map_err(|e| nonos_types::NonosError::Config(format!("Failed to create identities dir: {}", e)))?;

    println!();
    println!("\x1b[38;5;46m╔══════════════════════════════════════════════════════════════╗\x1b[0m");
    println!("\x1b[38;5;46m║\x1b[0m  \x1b[1;38;5;46mNONOS Daemon Initialized Successfully!\x1b[0m                      \x1b[38;5;46m║\x1b[0m");
    println!("\x1b[38;5;46m╠══════════════════════════════════════════════════════════════╣\x1b[0m");
    println!("\x1b[38;5;46m║\x1b[0m  Config: \x1b[38;5;51m{:50}\x1b[0m \x1b[38;5;46m║\x1b[0m", format!("{:?}", config_path));
    println!("\x1b[38;5;46m║\x1b[0m  Data:   \x1b[38;5;51m{:50}\x1b[0m \x1b[38;5;46m║\x1b[0m", format!("{:?}", data_dir));
    println!("\x1b[38;5;46m╠══════════════════════════════════════════════════════════════╣\x1b[0m");
    println!("\x1b[38;5;46m║\x1b[0m  \x1b[38;5;226mNext steps:\x1b[0m                                                 \x1b[38;5;46m║\x1b[0m");
    println!("\x1b[38;5;46m║\x1b[0m  1. Edit config.toml to set your reward address              \x1b[38;5;46m║\x1b[0m");
    println!("\x1b[38;5;46m║\x1b[0m  2. Generate a ZK identity: \x1b[38;5;51mnonos identity generate\x1b[0m          \x1b[38;5;46m║\x1b[0m");
    println!("\x1b[38;5;46m║\x1b[0m  3. Start the daemon: \x1b[38;5;51mnonos run\x1b[0m                             \x1b[38;5;46m║\x1b[0m");
    println!("\x1b[38;5;46m║\x1b[0m  4. Launch dashboard: \x1b[38;5;51mnonos dash\x1b[0m                            \x1b[38;5;46m║\x1b[0m");
    println!("\x1b[38;5;46m╚══════════════════════════════════════════════════════════════╝\x1b[0m");

    Ok(())
}
