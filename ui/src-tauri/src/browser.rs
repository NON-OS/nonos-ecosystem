use crate::proxy::LOCAL_PROXY_PORT;
use crate::state::{AppState, ConnectionStatus};
use tauri::Manager;
use crate::types::ProxyFetchResponse;
use std::sync::atomic::Ordering;
use tauri::{State, Window};

static BROWSER_WINDOWS: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<u32, String>>> = std::sync::OnceLock::new();

fn get_browser_windows() -> &'static std::sync::Mutex<std::collections::HashMap<u32, String>> {
    BROWSER_WINDOWS.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}

#[tauri::command]
pub async fn proxy_fetch(
    state: State<'_, AppState>,
    url: String,
    method: Option<String>,
    headers: Option<std::collections::HashMap<String, String>>,
    body: Option<String>,
) -> Result<ProxyFetchResponse, String> {
    let network = state.network.read().await;
    let socks_addr = network.socks_addr;
    let is_connected = matches!(network.status, ConnectionStatus::Connected);
    drop(network);

    {
        let nodes = state.nodes.read().await;
        nodes.total_requests.fetch_add(1, Ordering::Relaxed);
    }

    let client = if is_connected {
        let proxy = reqwest::Proxy::all(format!("socks5h://{}", socks_addr))
            .map_err(|e| format!("Failed to create proxy: {}", e))?;

        reqwest::Client::builder()
            .proxy(proxy)
            .danger_accept_invalid_certs(false)
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| format!("Failed to build proxy client: {}", e))?
    } else {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| format!("Failed to build client: {}", e))?
    };

    let method_str = method.unwrap_or_else(|| "GET".to_string());
    let method = reqwest::Method::from_bytes(method_str.as_bytes())
        .map_err(|_| "Invalid HTTP method")?;

    let mut request = client.request(method, &url);

    if let Some(hdrs) = headers {
        for (key, value) in hdrs {
            request = request.header(&key, &value);
        }
    }

    if let Some(b) = body {
        request = request.body(b);
    }

    let response = request.send().await
        .map_err(|e| format!("Request failed: {}", e))?;

    let status_code = response.status().as_u16();
    let content_type = response.headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("text/html")
        .to_string();

    let mut resp_headers = std::collections::HashMap::new();
    for (key, value) in response.headers() {
        if let Ok(v) = value.to_str() {
            resp_headers.insert(key.to_string(), v.to_string());
        }
    }

    let body_text = response.text().await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    Ok(ProxyFetchResponse {
        success: status_code >= 200 && status_code < 400,
        status_code,
        headers: resp_headers,
        body: body_text,
        content_type,
        via_proxy: is_connected,
        circuit_id: if is_connected { Some("anon-circuit-1".to_string()) } else { None },
    })
}

#[tauri::command]
pub async fn browser_navigate(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    url: String,
    window: Window,
) -> Result<String, String> {
    let target_url = if url.starts_with("http://") || url.starts_with("https://") {
        url.clone()
    } else if url.contains('.') {
        format!("https://{}", url)
    } else {
        format!("https://html.duckduckgo.com/html/?q={}", urlencoding::encode(&url))
    };

    {
        let mut browser = state.browser.write().await;
        browser.history.push(target_url.clone());
    }

    let network = state.network.read().await;
    let socks_addr = network.socks_addr;
    let is_connected = matches!(network.status, ConnectionStatus::Connected);
    drop(network);

    {
        let nodes = state.nodes.read().await;
        nodes.total_requests.fetch_add(1, Ordering::Relaxed);
    }

    let tab_id = {
        let mut browser = state.browser.write().await;
        browser.next_tab_id += 1;
        browser.next_tab_id
    };

    let window_label = format!("browser-{}", tab_id);

    let browser_url = if is_connected {
        format!("http://localhost:{}/proxy?url={}", LOCAL_PROXY_PORT, urlencoding::encode(&target_url))
    } else {
        target_url.clone()
    };

    let _browser_window = tauri::WindowBuilder::new(
        &app_handle,
        &window_label,
        tauri::WindowUrl::External(browser_url.parse().map_err(|e| format!("Invalid URL: {}", e))?)
    )
    .title(format!("NONOS - {}", if is_connected { "Secure" } else { "Direct" }))
    .inner_size(1200.0, 800.0)
    .min_inner_size(800.0, 600.0)
    .center()
    .visible(true)
    .build()
    .map_err(|e| format!("Failed to create browser window: {}", e))?;

    {
        let mut windows = get_browser_windows().lock().unwrap();
        windows.insert(tab_id, target_url.clone());
    }

    window
        .emit("nonos://tab-opened", serde_json::json!({
            "tab_id": tab_id,
            "url": target_url,
            "secure": is_connected,
            "socks_proxy": if is_connected { Some(socks_addr.to_string()) } else { None }
        }))
        .ok();

    Ok(format!(
        "Opened {} in tab {} {}",
        target_url,
        tab_id,
        if is_connected { format!("(via Anyone Network SOCKS5: {})", socks_addr) } else { "(direct connection)".to_string() }
    ))
}

#[tauri::command]
pub async fn browser_close_tab(
    app_handle: tauri::AppHandle,
    tab_id: u32,
) -> Result<(), String> {
    let window_label = format!("browser-{}", tab_id);
    if let Some(window) = app_handle.get_window(&window_label) {
        window.close().map_err(|e| e.to_string())?;
    }

    let mut windows = get_browser_windows().lock().unwrap();
    windows.remove(&tab_id);

    Ok(())
}

#[tauri::command]
pub async fn browser_get_tabs() -> Result<Vec<serde_json::Value>, String> {
    let windows = get_browser_windows().lock().unwrap();
    let tabs: Vec<_> = windows.iter().map(|(id, url)| {
        serde_json::json!({
            "id": id,
            "url": url
        })
    }).collect();
    Ok(tabs)
}

#[tauri::command]
pub async fn browser_get_socks_proxy(state: State<'_, AppState>) -> Result<String, String> {
    let network = state.network.read().await;
    Ok(network.socks_addr.to_string())
}

#[tauri::command]
pub fn get_proxy_url(target_url: String) -> String {
    format!("http://localhost:{}/proxy?url={}", LOCAL_PROXY_PORT, urlencoding::encode(&target_url))
}
