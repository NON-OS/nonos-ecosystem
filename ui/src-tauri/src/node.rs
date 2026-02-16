use crate::state::{AppState, NodeInfo};
use crate::types::NodeStatusResponse;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::atomic::Ordering;
use tauri::{State, Window};

fn find_nonos_node_binary() -> Option<std::path::PathBuf> {
    let locations = [
        std::path::PathBuf::from("../../target/release/nonos-node"),
        std::path::PathBuf::from("/usr/local/bin/nonos-node"),
        std::path::PathBuf::from("/usr/bin/nonos-node"),
        dirs::data_local_dir()
            .unwrap_or_default()
            .join("nonos")
            .join("bin")
            .join("nonos-node"),
        dirs::home_dir()
            .unwrap_or_default()
            .join(".local")
            .join("bin")
            .join("nonos-node"),
    ];

    for path in locations {
        if path.exists() {
            return Some(path);
        }
    }

    if let Ok(output) = Command::new("which")
        .arg("nonos-node")
        .output()
    {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Some(std::path::PathBuf::from(path));
            }
        }
    }

    None
}

#[tauri::command]
pub async fn node_get_status(state: State<'_, AppState>) -> Result<NodeStatusResponse, String> {
    let nodes = state.nodes.read().await;
    Ok(NodeStatusResponse {
        running: nodes.embedded_running,
        connected_nodes: nodes.nodes.len(),
        quality: nodes.embedded_quality,
        total_requests: nodes.total_requests.load(Ordering::Relaxed),
    })
}

#[tauri::command]
pub async fn node_start_embedded(
    state: State<'_, AppState>,
    window: Window,
) -> Result<(), String> {
    let mut nodes = state.nodes.write().await;

    if nodes.embedded_running {
        return Ok(());
    }

    let node_binary = find_nonos_node_binary()
        .ok_or_else(|| "NONOS node binary not found. Please install it first.".to_string())?;

    let data_dir = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("nonos")
        .join("node");

    std::fs::create_dir_all(&data_dir)
        .map_err(|e| format!("Failed to create node data directory: {}", e))?;

    let config_path = data_dir.join("config.toml");
    if !config_path.exists() {
        let init_output = Command::new(&node_binary)
            .args(["init", "-d", data_dir.to_str().unwrap(), "--non-interactive"])
            .output()
            .map_err(|e| format!("Failed to initialize node: {}", e))?;

        if !init_output.status.success() {
            let stderr = String::from_utf8_lossy(&init_output.stderr);
            return Err(format!("Node initialization failed: {}", stderr));
        }
    }

    let mut child = Command::new(&node_binary)
        .args(["run", "-d", data_dir.to_str().unwrap()])
        .env("RUST_LOG", "info")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start NONOS node: {}", e))?;

    let pid = child.id();
    nodes.embedded_pid = Some(pid);
    nodes.embedded_running = true;
    nodes.embedded_quality = 0.95;
    nodes.api_addr = "127.0.0.1:8080".to_string();
    nodes.p2p_port = 9432;

    if let Some(stderr) = child.stderr.take() {
        let reader = BufReader::new(stderr);
        let window_clone = window.clone();

        std::thread::spawn(move || {
            for line in reader.lines().map_while(Result::ok) {
                if line.contains("Node ID:") {
                    if let Some(id) = line.split("Node ID:").nth(1) {
                        let node_id = id.trim().to_string();
                        let _ = window_clone.emit("nonos://node-id", &node_id);
                    }
                }
                if line.contains("NONOS node started successfully") || line.contains("API server listening") {
                    let _ = window_clone.emit("nonos://node-ready", ());
                }
            }
        });
    }

    window
        .emit("nonos://node-started", serde_json::json!({
            "pid": pid,
            "api_addr": "http://127.0.0.1:8080",
            "p2p_port": 9432
        }))
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn node_stop_embedded(
    state: State<'_, AppState>,
    window: Window,
) -> Result<(), String> {
    let mut nodes = state.nodes.write().await;

    if let Some(pid) = nodes.embedded_pid.take() {
        #[cfg(unix)]
        {
            let _ = Command::new("kill")
                .args(["-15", &pid.to_string()])
                .output();

            std::thread::sleep(std::time::Duration::from_millis(500));

            let _ = Command::new("kill")
                .args(["-9", &pid.to_string()])
                .output();
        }

        #[cfg(windows)]
        {
            let _ = Command::new("taskkill")
                .args(["/F", "/PID", &pid.to_string()])
                .output();
        }
    }

    nodes.embedded_running = false;
    nodes.embedded_quality = 0.0;
    nodes.node_id = None;

    window
        .emit("nonos://node-stopped", ())
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn node_get_connected(state: State<'_, AppState>) -> Result<Vec<NodeInfo>, String> {
    let nodes = state.nodes.read().await;
    Ok(nodes.nodes.clone())
}
