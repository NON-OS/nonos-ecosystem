use crate::helpers::parse_bootstrap_progress;
use crate::proxy::set_proxy_connected;
use crate::state::{AppState, ConnectionStatus, NetworkState};
use crate::types::NetworkStatusResponse;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tauri::{State, Window};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::process::Command;
use tokio::sync::RwLock;

pub fn create_network_response(network: &NetworkState) -> NetworkStatusResponse {
    NetworkStatusResponse {
        connected: matches!(network.status, ConnectionStatus::Connected),
        status: format!("{:?}", network.status),
        bootstrap_progress: network.bootstrap_progress,
        circuits: network.circuits,
        socks_port: network.socks_addr.port(),
        error: network.error.clone(),
    }
}

pub fn emit_network_status(window: &Window, network: &NetworkState) {
    let _ = window.emit("nonos://network-status", create_network_response(network));
}

async fn write_anonrc(path: &PathBuf, network: &NetworkState) -> Result<(), String> {
    let config = format!(
        "SocksPort {}\nControlPort {}\nDataDirectory {}\nLog notice stderr\nSafeLogging 1\nAvoidDiskWrites 1\nCircuitBuildTimeout 60\n",
        network.socks_addr.port(),
        network.control_port,
        network.data_dir.display()
    );

    tokio::fs::write(path, config)
        .await
        .map_err(|e| format!("Failed to write anonrc: {}", e))
}

async fn download_anon_binary(target_dir: &PathBuf) -> Result<PathBuf, String> {
    let (os, arch) = (std::env::consts::OS, std::env::consts::ARCH);

    let download_url = match (os, arch) {
        ("macos", "aarch64") => "https://github.com/anyone-protocol/ator-protocol/releases/download/v0.4.9.11/anon-live-macos-arm64.zip",
        ("macos", "x86_64") => "https://github.com/anyone-protocol/ator-protocol/releases/download/v0.4.9.11/anon-live-macos-amd64.zip",
        ("linux", "x86_64") => "https://github.com/anyone-protocol/ator-protocol/releases/download/v0.4.9.11/anon-live-linux-amd64.zip",
        ("linux", "aarch64") => "https://github.com/anyone-protocol/ator-protocol/releases/download/v0.4.9.11/anon-live-linux-arm64.zip",
        ("windows", "x86_64") => "https://github.com/anyone-protocol/ator-protocol/releases/download/v0.4.9.11/anon-live-windows-signed-amd64.zip",
        _ => return Err(format!("Unsupported platform: {}-{}", os, arch)),
    };

    tokio::fs::create_dir_all(target_dir)
        .await
        .map_err(|e| format!("Failed to create directory: {}", e))?;

    let binary_name = if os == "windows" { "anon.exe" } else { "anon" };
    let target_path = target_dir.join(binary_name);
    let archive_path = target_dir.join("anon.zip");

    let response = reqwest::get(download_url)
        .await
        .map_err(|e| format!("Failed to download: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Download failed with status: {}", response.status()));
    }

    let bytes = response.bytes()
        .await
        .map_err(|e| format!("Failed to read download: {}", e))?;

    tokio::fs::write(&archive_path, &bytes)
        .await
        .map_err(|e| format!("Failed to write archive: {}", e))?;

    if os == "windows" {
        let output = Command::new("powershell")
            .arg("-Command")
            .arg(format!(
                "Expand-Archive -Path '{}' -DestinationPath '{}' -Force",
                archive_path.display(),
                target_dir.display()
            ))
            .output()
            .await
            .map_err(|e| format!("Failed to extract: {}", e))?;

        if !output.status.success() {
            return Err(format!("Extraction failed: {}", String::from_utf8_lossy(&output.stderr)));
        }

        let possible_paths = vec![
            target_dir.join("anon.exe"),
            target_dir.join("anon-live-windows-signed-amd64").join("anon.exe"),
        ];

        for path in possible_paths {
            if path.exists() {
                if path != target_path {
                    tokio::fs::rename(&path, &target_path)
                        .await
                        .map_err(|e| format!("Failed to move binary: {}", e))?;
                }
                break;
            }
        }
    } else {
        let output = Command::new("unzip")
            .arg("-o")
            .arg(&archive_path)
            .arg("-d")
            .arg(target_dir)
            .output()
            .await
            .map_err(|e| format!("Failed to extract: {}", e))?;

        if !output.status.success() {
            return Err(format!("Extraction failed: {}", String::from_utf8_lossy(&output.stderr)));
        }

        let possible_paths = vec![
            target_dir.join("anon"),
            target_dir.join("anon-live-macos-arm64").join("anon"),
            target_dir.join("anon-live-macos-amd64").join("anon"),
            target_dir.join("anon-live-linux-amd64").join("anon"),
            target_dir.join("anon-live-linux-arm64").join("anon"),
        ];

        for path in possible_paths {
            if path.exists() {
                if path != target_path {
                    tokio::fs::rename(&path, &target_path)
                        .await
                        .map_err(|e| format!("Failed to move binary: {}", e))?;
                }
                break;
            }
        }
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&target_path)
            .map_err(|e| format!("Failed to get permissions: {}", e))?
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&target_path, perms)
            .map_err(|e| format!("Failed to set permissions: {}", e))?;
    }

    let _ = tokio::fs::remove_file(&archive_path).await;

    if target_path.exists() {
        Ok(target_path)
    } else {
        Err("Failed to install anon binary".into())
    }
}

pub async fn find_anon_binary() -> Result<PathBuf, String> {
    let is_windows = std::env::consts::OS == "windows";
    let binary_name = if is_windows { "anon.exe" } else { "anon" };

    let mut candidates = vec![
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.join(binary_name)))
            .unwrap_or_default(),
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("nonos")
            .join("bin")
            .join(binary_name),
    ];

    if is_windows {
        candidates.extend(vec![
            PathBuf::from(format!("C:\\Program Files\\anon\\{}", binary_name)),
            PathBuf::from(format!(".\\{}", binary_name)),
            dirs::home_dir()
                .map(|h| h.join("AppData").join("Local").join("anon").join(binary_name))
                .unwrap_or_default(),
        ]);
    } else {
        candidates.extend(vec![
            PathBuf::from("/usr/bin/anon"),
            PathBuf::from("/usr/local/bin/anon"),
            PathBuf::from("/opt/anon/bin/anon"),
            PathBuf::from("/opt/homebrew/bin/anon"),
            dirs::home_dir()
                .map(|h| h.join(".local/bin/anon"))
                .unwrap_or_default(),
            PathBuf::from("./anon"),
        ]);
    }

    for path in &candidates {
        if path.exists() {
            return Ok(path.clone());
        }
    }

    let which_cmd = if is_windows { "where" } else { "which" };
    if let Ok(output) = Command::new(which_cmd).arg(binary_name).output().await {
        if output.status.success() {
            let path_str = String::from_utf8_lossy(&output.stdout);
            let path = PathBuf::from(path_str.lines().next().unwrap_or("").trim());
            if path.exists() {
                return Ok(path);
            }
        }
    }

    let download_dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("nonos")
        .join("bin");

    download_anon_binary(&download_dir).await
}

pub async fn auto_start_anon(network_state: Arc<RwLock<NetworkState>>) -> Result<(), String> {
    let mut network = network_state.write().await;

    if matches!(network.status, ConnectionStatus::Connected) || matches!(network.status, ConnectionStatus::Connecting) {
        return Ok(());
    }

    if TcpStream::connect(network.socks_addr).await.is_ok() {
        network.status = ConnectionStatus::Connected;
        network.bootstrap_progress = 100;
        network.circuits = 3;
        set_proxy_connected(true);
        return Ok(());
    }

    network.status = ConnectionStatus::Connecting;

    if !network.data_dir.exists() {
        tokio::fs::create_dir_all(&network.data_dir)
            .await
            .map_err(|e| format!("Failed to create data dir: {}", e))?;
    }

    let anon_path = match find_anon_binary().await {
        Ok(path) => path,
        Err(e) => {
            network.status = ConnectionStatus::Disconnected;
            network.error = Some("anon binary not installed. Install from https://github.com/anyone-protocol/anon-install".into());
            return Err(e);
        }
    };

    let anonrc_path = network.data_dir.join("anonrc");
    let client_config = format!(
        "SocksPort {}\nControlPort {}\nDataDirectory {}\nLog notice stderr\nSafeLogging 1\nAvoidDiskWrites 1\nCircuitBuildTimeout 60\nClientOnly 1\n",
        network.socks_addr.port(),
        network.control_port,
        network.data_dir.display()
    );

    tokio::fs::write(&anonrc_path, client_config)
        .await
        .map_err(|e| format!("Failed to write anonrc: {}", e))?;

    network.status = ConnectionStatus::Bootstrapping;

    let mut child = Command::new(&anon_path)
        .arg("-f")
        .arg(&anonrc_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to launch anon: {}", e))?;

    network.anon_pid = child.id();
    let socks_addr = network.socks_addr;

    drop(network);

    let network_state_clone = network_state.clone();
    if let Some(stderr) = child.stderr.take() {
        tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();

            while let Ok(Some(line)) = lines.next_line().await {
                if line.contains("Bootstrapped") {
                    if let Some(pct) = parse_bootstrap_progress(&line) {
                        let mut net = network_state_clone.write().await;
                        net.bootstrap_progress = pct;

                        if pct >= 100 {
                            net.status = ConnectionStatus::Connected;
                            net.circuits = 3;
                            crate::proxy::set_proxy_connected(true);
                        }
                    }
                }
            }
        });
    }

    let network_state_clone = network_state.clone();
    tokio::spawn(async move {
        for _ in 0..30 {
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

            let net = network_state_clone.read().await;
            if matches!(net.status, ConnectionStatus::Connected) {
                return;
            }
            drop(net);

            if TcpStream::connect(socks_addr).await.is_ok() {
                let mut net = network_state_clone.write().await;
                if !matches!(net.status, ConnectionStatus::Connected) {
                    net.status = ConnectionStatus::Connected;
                    net.bootstrap_progress = 100;
                    net.circuits = 3;
                    crate::proxy::set_proxy_connected(true);
                }
                return;
            }
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn network_connect(
    state: State<'_, AppState>,
    window: Window,
) -> Result<NetworkStatusResponse, String> {
    let mut network = state.network.write().await;

    if matches!(network.status, ConnectionStatus::Connected) {
        return Ok(create_network_response(&network));
    }

    network.status = ConnectionStatus::Connecting;
    network.error = None;
    emit_network_status(&window, &network);

    if !network.data_dir.exists() {
        tokio::fs::create_dir_all(&network.data_dir)
            .await
            .map_err(|e| format!("Failed to create data dir: {}", e))?;
    }

    let anon_path = find_anon_binary()
        .await
        .map_err(|e| format!("anon binary not found: {}", e))?;

    let anonrc_path = network.data_dir.join("anonrc");
    write_anonrc(&anonrc_path, &network)
        .await
        .map_err(|e| format!("Failed to write config: {}", e))?;

    network.status = ConnectionStatus::Bootstrapping;
    emit_network_status(&window, &network);

    let mut child = Command::new(&anon_path)
        .arg("-f")
        .arg(&anonrc_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to launch anon: {}", e))?;

    network.anon_pid = child.id();

    let window_clone = window.clone();
    let network_state = state.network.clone();

    if let Some(stderr) = child.stderr.take() {
        tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();

            while let Ok(Some(line)) = lines.next_line().await {
                if line.contains("Bootstrapped") {
                    if let Some(pct) = parse_bootstrap_progress(&line) {
                        let mut net = network_state.write().await;
                        net.bootstrap_progress = pct;

                        if pct >= 100 {
                            net.status = ConnectionStatus::Connected;
                            net.circuits = 3;
                            crate::proxy::set_proxy_connected(true);
                        }

                        emit_network_status(&window_clone, &net);
                    }
                }

                if line.contains("[err]") || line.contains("fatal") {
                    let mut net = network_state.write().await;
                    net.status = ConnectionStatus::Error;
                    net.error = Some(line.clone());
                    emit_network_status(&window_clone, &net);
                }
            }
        });
    }

    let socks_addr = network.socks_addr;
    drop(network);

    for _ in 0..60 {
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        let net = state.network.read().await;
        if matches!(net.status, ConnectionStatus::Connected) {
            return Ok(create_network_response(&net));
        }
        if matches!(net.status, ConnectionStatus::Error) {
            return Err(net.error.clone().unwrap_or_else(|| "Unknown error".into()));
        }
        drop(net);

        if TcpStream::connect(socks_addr).await.is_ok() {
            let mut net = state.network.write().await;
            net.status = ConnectionStatus::Connected;
            net.bootstrap_progress = 100;
            net.circuits = 3;
            set_proxy_connected(true);
            emit_network_status(&window, &net);
            return Ok(create_network_response(&net));
        }
    }

    Err("Bootstrap timeout".into())
}

#[tauri::command]
pub async fn network_disconnect(
    state: State<'_, AppState>,
    window: Window,
) -> Result<(), String> {
    let mut network = state.network.write().await;

    if let Some(pid) = network.anon_pid.take() {
        #[cfg(unix)]
        {
            use std::process::Command as StdCommand;
            let _ = StdCommand::new("kill").arg(pid.to_string()).output();
        }
        #[cfg(windows)]
        {
            use std::process::Command as StdCommand;
            let _ = StdCommand::new("taskkill")
                .args(&["/PID", &pid.to_string(), "/F"])
                .output();
        }
    }

    network.status = ConnectionStatus::Disconnected;
    network.bootstrap_progress = 0;
    network.circuits = 0;
    network.error = None;

    emit_network_status(&window, &network);
    Ok(())
}

#[tauri::command]
pub async fn network_get_status(state: State<'_, AppState>) -> Result<NetworkStatusResponse, String> {
    let network = state.network.read().await;
    Ok(create_network_response(&network))
}

#[tauri::command]
pub async fn network_new_identity(
    state: State<'_, AppState>,
    window: Window,
) -> Result<(), String> {
    let network = state.network.read().await;

    if !matches!(network.status, ConnectionStatus::Connected) {
        return Err("Not connected".into());
    }

    let control_addr = SocketAddr::from(([127, 0, 0, 1], network.control_port));

    if let Ok(mut stream) = TcpStream::connect(control_addr).await {
        let _ = stream.write_all(b"AUTHENTICATE\r\n").await;
        let _ = stream.write_all(b"SIGNAL NEWNYM\r\n").await;
        let _ = stream.write_all(b"QUIT\r\n").await;
    }

    window
        .emit("nonos://identity-changed", ())
        .map_err(|e| e.to_string())?;

    Ok(())
}
