mod cli;

use clap::Parser;
use cli::{
    Cli, Commands, init_logging, run_node, init_node,
    handle_identity, handle_mixer, handle_stake, handle_rewards,
    show_info, show_status, handle_config, run_checks, show_stats,
    handle_peers, generate_systemd, stop_node, restart_node, reload_node,
    show_version, launch_dashboard,
};
use nonos_types::NonosResult;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> NonosResult<()> {
    let cli = Cli::parse();

    init_logging(&cli);

    let data_dir = cli.data_dir.clone().unwrap_or_else(|| {
        dirs::home_dir()
            .map(|h| h.join(".nonos"))
            .unwrap_or_else(|| PathBuf::from("/var/lib/nonos"))
    });

    let config_path = cli.config.clone().unwrap_or_else(|| data_dir.join("config.toml"));

    match cli.command {
        Commands::Run { foreground: _, pid_file, systemd } => {
            run_node(&config_path, &data_dir, pid_file, systemd).await?;
        }
        Commands::Init { force, non_interactive, reward_address, nickname } => {
            init_node(&config_path, &data_dir, force, non_interactive, reward_address, nickname)?;
        }
        Commands::Info => {
            show_info(&config_path, &data_dir, &cli.format).await?;
        }
        Commands::Status => {
            show_status(&data_dir, &cli.format).await?;
        }
        Commands::Config { action } => {
            handle_config(&config_path, action)?;
        }
        Commands::Check { full } => {
            run_checks(&config_path, &data_dir, full).await?;
        }
        Commands::Identity { action } => {
            handle_identity(action, &data_dir, &cli.format).await?;
        }
        Commands::Mixer { action } => {
            handle_mixer(action, &data_dir, &cli.format).await?;
        }
        Commands::Dash { theme } => {
            launch_dashboard(&data_dir, &theme).await?;
        }
        Commands::Stake { action } => {
            handle_stake(action, &cli.format).await?;
        }
        Commands::Rewards { action } => {
            handle_rewards(action, &cli.format).await?;
        }
        Commands::Stats => {
            show_stats(&data_dir, &cli.format).await?;
        }
        Commands::Peers { action } => {
            handle_peers(action, &data_dir, &cli.format).await?;
        }
        Commands::Systemd { output_dir, user } => {
            generate_systemd(&output_dir, &user, &data_dir)?;
        }
        Commands::Stop { force } => {
            stop_node(&data_dir, force).await?;
        }
        Commands::Restart { force } => {
            restart_node(&config_path, &data_dir, force).await?;
        }
        Commands::Reload => {
            reload_node(&data_dir).await?;
        }
        Commands::Version => {
            show_version();
        }
    }

    Ok(())
}
