use nonos_types::NonosResult;
use std::path::PathBuf;

pub fn generate_systemd(
    output_dir: &PathBuf,
    user: &str,
    data_dir: &PathBuf,
) -> NonosResult<()> {
    let service_content = format!(r#"[Unit]
Description=NONOS Daemon - Decentralized Node
Documentation=https://nonos.systems/docs
After=network-online.target
Wants=network-online.target

[Service]
Type=notify
User={user}
Group={user}
ExecStart=/usr/local/bin/nonos run --systemd -d {data_dir}
ExecReload=/bin/kill -HUP $MAINPID
Restart=on-failure
RestartSec=5
TimeoutStartSec=60
TimeoutStopSec=30

NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=read-only
PrivateTmp=yes
PrivateDevices=yes
ProtectKernelTunables=yes
ProtectKernelModules=yes
ProtectControlGroups=yes
ReadWritePaths={data_dir}

LimitNOFILE=65536
MemoryMax=2G

[Install]
WantedBy=multi-user.target
"#, user = user, data_dir = data_dir.display());

    println!("\x1b[38;5;46mGenerating systemd service file...\x1b[0m\n");
    println!("{}", service_content);
    println!("\nTo install, run:");
    println!("  \x1b[38;5;51msudo tee {} << 'EOF'\x1b[0m", output_dir.join("nonos.service").display());
    println!("  \x1b[38;5;51msudo systemctl daemon-reload\x1b[0m");
    println!("  \x1b[38;5;51msudo systemctl enable nonos\x1b[0m");
    println!("  \x1b[38;5;51msudo systemctl start nonos\x1b[0m");
    Ok(())
}

pub async fn stop_node(data_dir: &PathBuf, force: bool) -> NonosResult<()> {
    let pid_file = data_dir.join("nonos.pid");
    if pid_file.exists() {
        let pid_str = std::fs::read_to_string(&pid_file)
            .map_err(|e| nonos_types::NonosError::Internal(format!("Failed to read PID: {}", e)))?;
        let pid: i32 = pid_str.trim().parse()
            .map_err(|e| nonos_types::NonosError::Internal(format!("Invalid PID: {}", e)))?;

        #[cfg(unix)]
        {
            use nix::sys::signal::{kill, Signal};
            use nix::unistd::Pid;
            let signal = if force { Signal::SIGKILL } else { Signal::SIGTERM };
            match kill(Pid::from_raw(pid), signal) {
                Ok(_) => println!("\x1b[38;5;46m[+]\x1b[0m Sent {} to process {}", if force { "SIGKILL" } else { "SIGTERM" }, pid),
                Err(e) => println!("\x1b[38;5;196m[-]\x1b[0m Failed to signal process: {}", e),
            }
        }
        #[cfg(not(unix))]
        {
            let _ = (pid, force);
            println!("Stop not supported on this platform. Kill process {} manually.", pid);
        }
    } else {
        println!("\x1b[38;5;245mNo PID file found. Daemon may not be running.\x1b[0m");
    }
    Ok(())
}

pub async fn reload_node(data_dir: &PathBuf) -> NonosResult<()> {
    let pid_file = data_dir.join("nonos.pid");
    if pid_file.exists() {
        let pid_str = std::fs::read_to_string(&pid_file)
            .map_err(|e| nonos_types::NonosError::Internal(format!("Failed to read PID: {}", e)))?;
        let pid: i32 = pid_str.trim().parse()
            .map_err(|e| nonos_types::NonosError::Internal(format!("Invalid PID: {}", e)))?;

        #[cfg(unix)]
        {
            use nix::sys::signal::{kill, Signal};
            use nix::unistd::Pid;
            match kill(Pid::from_raw(pid), Signal::SIGHUP) {
                Ok(_) => println!("\x1b[38;5;46m[+]\x1b[0m Sent SIGHUP to process {} - config will be reloaded", pid),
                Err(e) => println!("\x1b[38;5;196m[-]\x1b[0m Failed to signal process: {}", e),
            }
        }
        #[cfg(not(unix))]
        {
            let _ = pid;
            println!("Reload not supported on this platform.");
        }
    } else {
        println!("\x1b[38;5;245mNo PID file found. Daemon may not be running.\x1b[0m");
    }
    Ok(())
}

pub async fn restart_node(
    _config_path: &PathBuf,
    data_dir: &PathBuf,
    force: bool,
) -> NonosResult<()> {
    let pid_file = data_dir.join("nonos.pid");

    if pid_file.exists() {
        println!("\x1b[38;5;226m[*]\x1b[0m Stopping running daemon...");
        stop_node(data_dir, force).await?;

        for i in 0..30 {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            if !pid_file.exists() {
                break;
            }
            if i == 29 {
                println!("\x1b[38;5;196m[-]\x1b[0m Timeout waiting for daemon to stop");
                return Err(nonos_types::NonosError::Internal(
                    "Daemon did not stop in time".into()
                ));
            }
        }
    }

    println!("\x1b[38;5;46m[+]\x1b[0m Starting daemon...");
    println!("Run: nonos run -d {}", data_dir.display());
    println!("\nTo restart in the same process, use systemd:");
    println!("  sudo systemctl restart nonos");

    Ok(())
}
