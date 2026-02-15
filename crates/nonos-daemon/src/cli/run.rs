use super::utils::print_banner;
use nonos_daemon::{
    Node, NodeConfig, NodeStorage, ServiceManager, ServiceConfig,
    PrivacyServiceManager, ApiServer,
};
use nonos_types::NonosResult;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, error};

pub async fn run_node(
    config_path: &PathBuf,
    data_dir: &PathBuf,
    pid_file: Option<PathBuf>,
    systemd: bool,
) -> NonosResult<()> {
    print_banner();
    info!("Starting NONOS daemon v{}", env!("CARGO_PKG_VERSION"));
    info!("Data directory: {:?}", data_dir);

    if let Some(ref pid_path) = pid_file {
        let pid = std::process::id();
        std::fs::write(pid_path, pid.to_string())
            .map_err(|e| nonos_types::NonosError::Config(format!("Failed to write PID file: {}", e)))?;
        info!("PID file written: {:?}", pid_path);
    }

    std::fs::create_dir_all(data_dir)
        .map_err(|e| nonos_types::NonosError::Config(format!("Failed to create data directory: {}", e)))?;

    let config = if config_path.exists() {
        info!("Loading configuration from {:?}", config_path);
        NodeConfig::load(config_path)?
    } else {
        info!("Using default configuration");
        NodeConfig::default()
    };

    let storage_path = data_dir.join("data");
    let storage_config = nonos_daemon::StorageConfig {
        path: storage_path.clone(),
        ..Default::default()
    };
    let storage = Arc::new(NodeStorage::open(storage_config)?);
    info!("Storage initialized at {:?}", storage_path);

    let mut node = Node::new(config.clone())?;
    node.start().await?;

    let node_id = node.id();
    info!("Node ID: {}", node_id);

    let metrics_collector = node.metrics_collector();

    let mut service_manager = ServiceManager::new();
    if config.services.health_beacon || config.services.quality_oracle {
        let network = node.network().ok_or_else(|| {
            nonos_types::NonosError::Internal("Network not initialized".into())
        })?;

        let service_config = ServiceConfig {
            health_beacon: config.services.health_beacon,
            quality_oracle: config.services.quality_oracle,
            bootstrap: config.services.bootstrap,
            cache: config.services.cache,
            cache_size_mb: config.services.cache_size_mb,
            ..Default::default()
        };

        service_manager.start_all(
            node_id,
            network,
            metrics_collector.clone(),
            storage.clone(),
            service_config,
        ).await?;
        info!("Services started");
    }

    let node = Arc::new(RwLock::new(node));
    let privacy_manager = Arc::new(PrivacyServiceManager::new(node_id));
    privacy_manager.start_all().await?;
    info!("Privacy services started (ZK Identity, Cache Mixing, Tracking Blocker)");

    let api_addr: std::net::SocketAddr = format!("{}:{}", config.api.bind_address, config.api.port)
        .parse()
        .unwrap_or_else(|_| "127.0.0.1:8420".parse().unwrap());

    config.log_security_warnings();

    let api_server = ApiServer::with_privacy(
        api_addr,
        node.clone(),
        metrics_collector.clone(),
        privacy_manager.clone(),
        config.security.api_auth_token.clone(),
        config.security.api_auth_required,
        config.rate_limits.requests_per_second,
        config.rate_limits.burst_size,
    );
    tokio::spawn(async move {
        if let Err(e) = api_server.start().await {
            error!("API server error: {}", e);
        }
    });
    info!("HTTP API server started on {}", api_addr);

    if systemd {
        notify_systemd_ready();
    }

    print_ready_message(node_id, api_addr);

    wait_for_shutdown().await;

    info!("Shutting down...");
    privacy_manager.stop_all();
    service_manager.stop_all().await;
    node.write().await.stop().await?;
    storage.flush()?;

    if let Some(ref pid_path) = pid_file {
        let _ = std::fs::remove_file(pid_path);
    }

    info!("Shutdown complete");
    Ok(())
}

fn print_ready_message(node_id: nonos_types::NodeId, api_addr: std::net::SocketAddr) {
    println!();
    println!("\x1b[38;5;46m╔══════════════════════════════════════════════════════════════╗\x1b[0m");
    println!("\x1b[38;5;46m║\x1b[0m  \x1b[1;38;5;46mNONOS Daemon is now running!\x1b[0m                                \x1b[38;5;46m║\x1b[0m");
    println!("\x1b[38;5;46m╠══════════════════════════════════════════════════════════════╣\x1b[0m");
    let id_str = node_id.to_string();
    let display_id = if id_str.len() > 24 { &id_str[..24] } else { &id_str };
    println!("\x1b[38;5;46m║\x1b[0m  Node ID: \x1b[38;5;226m{:<24}\x1b[0m...               \x1b[38;5;46m║\x1b[0m", display_id);
    println!("\x1b[38;5;46m║\x1b[0m  API: \x1b[38;5;51mhttp://{:<40}\x1b[0m    \x1b[38;5;46m║\x1b[0m", api_addr);
    println!("\x1b[38;5;46m╠══════════════════════════════════════════════════════════════╣\x1b[0m");
    println!("\x1b[38;5;46m║\x1b[0m  \x1b[38;5;207mServices Active:\x1b[0m                                            \x1b[38;5;46m║\x1b[0m");
    println!("\x1b[38;5;46m║\x1b[0m    \x1b[38;5;46m[+]\x1b[0m ZK Identity Engine                                    \x1b[38;5;46m║\x1b[0m");
    println!("\x1b[38;5;46m║\x1b[0m    \x1b[38;5;46m[+]\x1b[0m Cache Mixer (Poseidon Merkle)                         \x1b[38;5;46m║\x1b[0m");
    println!("\x1b[38;5;46m║\x1b[0m    \x1b[38;5;46m[+]\x1b[0m Tracking Blocker                                      \x1b[38;5;46m║\x1b[0m");
    println!("\x1b[38;5;46m║\x1b[0m    \x1b[38;5;46m[+]\x1b[0m Stealth Scanner                                       \x1b[38;5;46m║\x1b[0m");
    println!("\x1b[38;5;46m║\x1b[0m    \x1b[38;5;46m[+]\x1b[0m P2P Network (libp2p)                                  \x1b[38;5;46m║\x1b[0m");
    println!("\x1b[38;5;46m╠══════════════════════════════════════════════════════════════╣\x1b[0m");
    println!("\x1b[38;5;46m║\x1b[0m  \x1b[38;5;245mRun '\x1b[38;5;51mnonos dash\x1b[38;5;245m' in another terminal for TUI dashboard\x1b[0m     \x1b[38;5;46m║\x1b[0m");
    println!("\x1b[38;5;46m║\x1b[0m  \x1b[38;5;245mPress Ctrl+C to stop\x1b[0m                                        \x1b[38;5;46m║\x1b[0m");
    println!("\x1b[38;5;46m╚══════════════════════════════════════════════════════════════╝\x1b[0m");
    println!();
}

async fn wait_for_shutdown() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};

        let mut sigterm = signal(SignalKind::terminate()).expect("Failed to install SIGTERM handler");
        let mut sigint = signal(SignalKind::interrupt()).expect("Failed to install SIGINT handler");
        let mut sighup = signal(SignalKind::hangup()).expect("Failed to install SIGHUP handler");

        tokio::select! {
            _ = sigterm.recv() => { info!("Received SIGTERM"); }
            _ = sigint.recv() => { info!("Received SIGINT"); }
            _ = sighup.recv() => { info!("Received SIGHUP - would reload config"); }
        }
    }

    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
        info!("Received Ctrl+C");
    }
}

fn notify_systemd_ready() {
    #[cfg(target_os = "linux")]
    {
        if let Ok(socket_path) = std::env::var("NOTIFY_SOCKET") {
            use std::os::unix::net::UnixDatagram;
            if let Ok(socket) = UnixDatagram::unbound() {
                let _ = socket.send_to(b"READY=1", &socket_path);
                tracing::debug!("Notified systemd: READY=1");
            }
        }
    }
}
