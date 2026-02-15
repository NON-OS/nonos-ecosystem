use super::commands::{RewardsAction, OutputFormat};
use super::utils::load_contract_config;
use nonos_daemon::ContractClient;
use nonos_types::{EthAddress, NonosResult};
use tracing::error;

pub async fn handle_rewards(action: RewardsAction, format: &OutputFormat) -> NonosResult<()> {
    let contract_config = load_contract_config()?;
    let mut client = ContractClient::new(contract_config);

    if let Err(e) = client.connect().await {
        error!("Failed to connect to network: {}", e);
        println!("\x1b[38;5;196m[-]\x1b[0m Cannot connect to blockchain");
        return Ok(());
    }

    let wallet_key = std::env::var("NONOS_WALLET_KEY").ok();
    let wallet_address = if let Some(ref key) = wallet_key {
        client.set_wallet(key).await.ok().map(|a| EthAddress(a.0))
    } else {
        None
    };

    match action {
        RewardsAction::Status => {
            if let Some(addr) = &wallet_address {
                let pending = client.get_pending_rewards(addr).await.ok();
                match format {
                    OutputFormat::Json => {
                        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
                            "pending_nox": pending.map(|p| p.to_decimal())
                        })).unwrap());
                    }
                    OutputFormat::Text => {
                        println!("\x1b[38;5;46mReward Status\x1b[0m");
                        println!("\x1b[38;5;245m{}\x1b[0m", "═".repeat(50));
                        if let Some(p) = pending {
                            println!("Pending:   \x1b[38;5;46m{} NOX\x1b[0m", p.to_decimal());
                        }
                    }
                }
            } else {
                println!("\x1b[38;5;245mNo wallet configured. Set NONOS_WALLET_KEY environment variable.\x1b[0m");
            }
        }
        RewardsAction::Claim => {
            if wallet_address.is_none() {
                println!("\x1b[38;5;196m[-]\x1b[0m No wallet configured");
                return Ok(());
            }
            println!("Claiming rewards...");
            match client.claim_rewards().await {
                Ok((tx, amount)) => {
                    println!("  Tx: \x1b[38;5;245m{:?}\x1b[0m", tx);
                    println!("\x1b[38;5;46m[+]\x1b[0m Successfully claimed {} NOX!", amount.to_decimal());
                }
                Err(e) => println!("\x1b[38;5;196m[-]\x1b[0m Claim failed: {}", e),
            }
        }
        RewardsAction::Auto { threshold } => {
            println!("\x1b[38;5;46mAuto-claim Configuration\x1b[0m");
            println!("\x1b[38;5;245m{}\x1b[0m", "═".repeat(50));
            println!("Threshold: \x1b[38;5;51m{} NOX\x1b[0m", threshold);
            println!("\nSet \x1b[38;5;51mNONOS_AUTOCLAIM_THRESHOLD={}\x1b[0m in your environment", threshold);
        }
        RewardsAction::History { limit } => {
            println!("\x1b[38;5;46mReward History\x1b[0m (last {} epochs)", limit);
            println!("\x1b[38;5;245m(Reward history requires indexer - coming soon)\x1b[0m");
        }
        RewardsAction::Debug { epoch } => {
            println!("\x1b[38;5;46mReward Debug\x1b[0m: epoch {}", epoch);
            println!("\x1b[38;5;245m(Debug info requires connection to epoch oracle)\x1b[0m");
        }
    }
    Ok(())
}
