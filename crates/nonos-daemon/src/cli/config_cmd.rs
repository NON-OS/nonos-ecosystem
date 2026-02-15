use super::commands::ConfigAction;
use nonos_daemon::NodeConfig;
use nonos_types::NonosResult;
use std::path::PathBuf;

pub fn handle_config(config_path: &PathBuf, action: Option<ConfigAction>) -> NonosResult<()> {
    match action {
        Some(ConfigAction::Show) | None => {
            if config_path.exists() {
                let content = std::fs::read_to_string(config_path)
                    .map_err(|e| nonos_types::NonosError::Config(format!("Failed to read config: {}", e)))?;
                println!("{}", content);
            } else {
                println!("\x1b[38;5;245mNo configuration file found at {:?}\x1b[0m", config_path);
                println!("Run '\x1b[38;5;51mnonos init\x1b[0m' to create one");
            }
        }
        Some(ConfigAction::Validate) => {
            if config_path.exists() {
                match NodeConfig::load(config_path) {
                    Ok(_) => println!("\x1b[38;5;46m[+]\x1b[0m Configuration is valid"),
                    Err(e) => println!("\x1b[38;5;196m[-]\x1b[0m Configuration error: {}", e),
                }
            } else {
                println!("\x1b[38;5;245mNo configuration file found at {:?}\x1b[0m", config_path);
            }
        }
        Some(ConfigAction::Set { key, value }) => {
            println!("Setting {} = {} (use config file for persistent changes)", key, value);
        }
    }
    Ok(())
}
