use super::commands::{StakeAction, OutputFormat};
use super::utils::load_contract_config;
use nonos_daemon::ContractClient;
use nonos_types::{EthAddress, NodeTier, TokenAmount, NOX_DECIMALS};
use nonos_types::NonosResult;
use tracing::error;

pub async fn handle_stake(action: StakeAction, format: &OutputFormat) -> NonosResult<()> {
    let contract_config = load_contract_config()?;
    let mut client = ContractClient::new(contract_config);

    if let Err(e) = client.connect().await {
        error!("Failed to connect to network: {}", e);
        println!("\x1b[38;5;196m[-]\x1b[0m Cannot connect to blockchain");
        println!("    Check your RPC endpoint and network connection.");
        return Ok(());
    }

    let wallet_key = std::env::var("NONOS_WALLET_KEY").ok();
    let wallet_address = if let Some(ref key) = wallet_key {
        client.set_wallet(key).await.ok().map(|a| EthAddress(a.0))
    } else {
        None
    };

    match action {
        StakeAction::Status => {
            if let Some(addr) = &wallet_address {
                let stake = client.get_stake(addr).await.ok();
                let tier = client.get_tier(addr).await.ok();
                let balance = client.get_balance(addr).await.ok();

                match format {
                    OutputFormat::Json => {
                        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
                            "address": format!("0x{}", hex::encode(addr.0)),
                            "staked_nox": stake.map(|s| s.to_decimal()),
                            "tier": tier.map(|t| format!("{:?}", t)),
                            "available_nox": balance.map(|b| b.to_decimal()),
                        })).unwrap());
                    }
                    OutputFormat::Text => {
                        println!("\x1b[38;5;46mStake Status\x1b[0m");
                        println!("\x1b[38;5;245m{}\x1b[0m", "═".repeat(50));
                        println!("Address:   \x1b[38;5;51m0x{}\x1b[0m", hex::encode(addr.0));
                        if let Some(s) = stake { println!("Staked:    \x1b[38;5;46m{} NOX\x1b[0m", s.to_decimal()); }
                        if let Some(t) = tier { println!("Tier:      \x1b[38;5;226m{:?}\x1b[0m ({}x multiplier)", t, t.multiplier()); }
                        if let Some(b) = balance { println!("Available: \x1b[38;5;51m{} NOX\x1b[0m", b.to_decimal()); }
                    }
                }
            } else {
                println!("\x1b[38;5;245mNo wallet configured. Set NONOS_WALLET_KEY environment variable.\x1b[0m");
            }
        }
        StakeAction::Deposit { amount } => {
            if wallet_address.is_none() {
                println!("\x1b[38;5;196m[-]\x1b[0m No wallet configured");
                return Ok(());
            }
            let amount_wei = TokenAmount::from_raw(
                (amount * 10u128.pow(NOX_DECIMALS as u32) as f64) as u128,
                NOX_DECIMALS
            );
            println!("Staking \x1b[38;5;46m{} NOX\x1b[0m...", amount);
            println!("Step 1/2: Approving tokens...");
            match client.approve(&amount_wei).await {
                Ok(tx) => println!("  Approval tx: \x1b[38;5;245m{:?}\x1b[0m", tx),
                Err(e) => {
                    println!("\x1b[38;5;196m[-]\x1b[0m Approval failed: {}", e);
                    return Ok(());
                }
            }
            println!("Step 2/2: Staking tokens...");
            match client.stake(&amount_wei).await {
                Ok(tx) => {
                    println!("  Stake tx: \x1b[38;5;245m{:?}\x1b[0m", tx);
                    println!("\x1b[38;5;46m[+]\x1b[0m Successfully staked {} NOX!", amount);
                }
                Err(e) => println!("\x1b[38;5;196m[-]\x1b[0m Staking failed: {}", e),
            }
        }
        StakeAction::Tier { tier } => {
            if wallet_address.is_none() {
                println!("\x1b[38;5;196m[-]\x1b[0m No wallet configured");
                return Ok(());
            }
            let node_tier = match tier.to_lowercase().as_str() {
                "bronze" => NodeTier::Bronze,
                "silver" => NodeTier::Silver,
                "gold" => NodeTier::Gold,
                "platinum" => NodeTier::Platinum,
                "diamond" => NodeTier::Diamond,
                _ => {
                    println!("\x1b[38;5;196m[-]\x1b[0m Invalid tier. Run 'nonos stake tiers' to see options.");
                    return Ok(());
                }
            };
            println!("Setting tier to \x1b[38;5;226m{:?}\x1b[0m...", node_tier);
            match client.set_tier(node_tier).await {
                Ok(tx) => {
                    println!("  Tx: \x1b[38;5;245m{:?}\x1b[0m", tx);
                    println!("\x1b[38;5;46m[+]\x1b[0m Successfully set tier to {:?}!", node_tier);
                }
                Err(e) => println!("\x1b[38;5;196m[-]\x1b[0m Tier change failed: {}", e),
            }
        }
        StakeAction::Withdraw { amount } => {
            if wallet_address.is_none() {
                println!("\x1b[38;5;196m[-]\x1b[0m No wallet configured");
                return Ok(());
            }
            let amount_wei = TokenAmount::from_raw(
                (amount * 10u128.pow(NOX_DECIMALS as u32) as f64) as u128,
                NOX_DECIMALS
            );
            println!("Withdrawing \x1b[38;5;226m{} NOX\x1b[0m...", amount);
            match client.unstake(&amount_wei).await {
                Ok(tx) => {
                    println!("  Tx: \x1b[38;5;245m{:?}\x1b[0m", tx);
                    println!("\x1b[38;5;46m[+]\x1b[0m Successfully withdrew {} NOX!", amount);
                }
                Err(e) => println!("\x1b[38;5;196m[-]\x1b[0m Withdrawal failed: {}", e),
            }
        }
        StakeAction::Tiers => {
            println!("\x1b[38;5;46mNONOS Node Tiers\x1b[0m");
            println!("\x1b[38;5;245m{}\x1b[0m", "═".repeat(65));
            println!("{:<12} {:<14} {:<12} {}", "Tier", "Min Stake", "Multiplier", "Benefits");
            println!("{}", "-".repeat(65));
            println!("\x1b[38;5;208m{:<12}\x1b[0m {:<14} {:<12} {}", "Bronze", "100 NOX", "1.0x", "Basic rewards");
            println!("\x1b[38;5;250m{:<12}\x1b[0m {:<14} {:<12} {}", "Silver", "1,000 NOX", "1.5x", "+50% rewards");
            println!("\x1b[38;5;226m{:<12}\x1b[0m {:<14} {:<12} {}", "Gold", "10,000 NOX", "2.0x", "+100% rewards");
            println!("\x1b[38;5;51m{:<12}\x1b[0m {:<14} {:<12} {}", "Platinum", "50,000 NOX", "3.0x", "+200% rewards");
            println!("\x1b[38;5;207m{:<12}\x1b[0m {:<14} {:<12} {}", "Diamond", "100,000 NOX", "5.0x", "+400% rewards");
            println!("\n\x1b[38;5;245mHigher tiers provide priority in node selection for services.\x1b[0m");
        }
    }
    Ok(())
}
