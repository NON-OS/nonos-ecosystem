use http_body_util::Full;
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, body::Incoming};
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use tokio::net::TcpListener;

use super::{socks, rewrite};

pub const LOCAL_PROXY_PORT: u16 = 9060;

pub async fn start_local_proxy_server() {
    let addr = SocketAddr::from(([127, 0, 0, 1], LOCAL_PROXY_PORT));
    let listener = match TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(_) => return,
    };

    loop {
        if let Ok((stream, _)) = listener.accept().await {
            let io = TokioIo::new(stream);
            tokio::spawn(async move {
                let _ = http1::Builder::new()
                    .serve_connection(io, service_fn(handle))
                    .await;
            });
        }
    }
}

async fn handle(req: Request<Incoming>) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let origin = req.headers().get("origin").and_then(|v| v.to_str().ok()).unwrap_or("");
    let allowed = origin.is_empty() || origin.starts_with("tauri://") || origin.starts_with("http://localhost") || origin.starts_with("https://tauri.localhost");

    if !allowed {
        return Ok(error_response(403, "Forbidden"));
    }

    if req.method() == hyper::Method::OPTIONS {
        return Ok(cors_response(200, ""));
    }

    let url = match extract_url(req.uri().query()) {
        Some(u) => u,
        None => return Ok(error_response(400, "Missing url parameter")),
    };

    if is_private_url(&url) {
        return Ok(error_response(403, "Access to private networks blocked"));
    }

    match socks::fetch(&url).await {
        Ok((status, content_type, body)) => {
            let final_body = if content_type.contains("text/html") {
                let html = String::from_utf8_lossy(&body);
                rewrite::html(&html, &url)
            } else if content_type.contains("text/css") {
                let css = String::from_utf8_lossy(&body);
                rewrite::css(&css, &url)
            } else {
                body
            };

            Ok(Response::builder()
                .status(status)
                .header("Content-Type", content_type)
                .header("Access-Control-Allow-Origin", "*")
                .header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
                .header("Access-Control-Allow-Headers", "*")
                .body(Full::new(Bytes::from(final_body)))
                .unwrap())
        }
        Err(e) => Ok(error_response(502, &format!("Connection failed: {}", e))),
    }
}

fn extract_url(query: Option<&str>) -> Option<String> {
    query?.split('&')
        .find_map(|p| {
            let mut parts = p.splitn(2, '=');
            if parts.next()? == "url" {
                urlencoding::decode(parts.next()?).ok().map(|s| s.into_owned())
            } else {
                None
            }
        })
}

fn is_private_url(url: &str) -> bool {
    if let Ok(parsed) = url::Url::parse(url) {
        if let Some(host) = parsed.host_str() {
            if host == "localhost" || host.ends_with(".localhost") {
                return true;
            }
            if host.starts_with("127.") || host == "::1" || host == "[::1]" {
                return true;
            }
            if host.starts_with("10.") {
                return true;
            }
            if host.starts_with("192.168.") {
                return true;
            }
            if host.starts_with("172.") {
                if let Some(second) = host.split('.').nth(1) {
                    if let Ok(n) = second.parse::<u8>() {
                        if (16..=31).contains(&n) {
                            return true;
                        }
                    }
                }
            }
            if host.starts_with("169.254.") {
                return true;
            }
            if host == "0.0.0.0" || host.starts_with("0.") {
                return true;
            }
        }
        if parsed.scheme() == "file" {
            return true;
        }
    }
    false
}

fn cors_response(status: u16, body: &str) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
        .header("Access-Control-Allow-Headers", "*")
        .body(Full::new(Bytes::from(body.to_string())))
        .unwrap()
}

fn error_response(status: u16, msg: &str) -> Response<Full<Bytes>> {
    let body = format!(
        r#"<!DOCTYPE html><html><head><title>Error</title></head>
<body style="font-family:sans-serif;padding:40px;background:#0a0a0f;color:#e0e0e0;">
<h1 style="color:#ff6666;">Error</h1><p>{}</p>
<p><a href="javascript:history.back()" style="color:#66ffff;">Go Back</a></p>
</body></html>"#, msg
    );
    Response::builder()
        .status(status)
        .header("Content-Type", "text/html")
        .header("Access-Control-Allow-Origin", "*")
        .body(Full::new(Bytes::from(body)))
        .unwrap()
}
