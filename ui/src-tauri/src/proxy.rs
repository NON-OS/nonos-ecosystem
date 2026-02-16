use http_body_util::Full;
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, body::Incoming};
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::net::TcpListener;

pub const LOCAL_PROXY_PORT: u16 = 9060;

static PROXY_SOCKS_ADDR: std::sync::OnceLock<std::sync::Mutex<SocketAddr>> = std::sync::OnceLock::new();
static PROXY_CONNECTED: std::sync::OnceLock<AtomicBool> = std::sync::OnceLock::new();

fn get_proxy_socks_addr() -> &'static std::sync::Mutex<SocketAddr> {
    PROXY_SOCKS_ADDR.get_or_init(|| std::sync::Mutex::new(SocketAddr::from(([127, 0, 0, 1], 9050))))
}

pub fn get_proxy_connected() -> &'static AtomicBool {
    PROXY_CONNECTED.get_or_init(|| AtomicBool::new(false))
}

pub fn set_proxy_connected(connected: bool) {
    get_proxy_connected().store(connected, Ordering::Relaxed);
}

fn build_proxy_client() -> reqwest::Client {
    let socks_addr = *get_proxy_socks_addr().lock().unwrap();
    let is_connected = get_proxy_connected().load(Ordering::Relaxed);

    if is_connected {
        match reqwest::Proxy::all(format!("socks5h://{}", socks_addr)) {
            Ok(proxy) => {
                reqwest::Client::builder()
                    .proxy(proxy)
                    .timeout(std::time::Duration::from_secs(60))
                    .redirect(reqwest::redirect::Policy::limited(10))
                    .build()
                    .unwrap_or_else(|_| reqwest::Client::new())
            }
            Err(_) => reqwest::Client::new(),
        }
    } else {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new())
    }
}

fn extract_base_url(url: &str) -> String {
    if let Ok(parsed) = url::Url::parse(url) {
        if let Some(port) = parsed.port() {
            format!("{}://{}:{}", parsed.scheme(), parsed.host_str().unwrap_or(""), port)
        } else {
            format!("{}://{}", parsed.scheme(), parsed.host_str().unwrap_or(""))
        }
    } else {
        String::new()
    }
}

fn resolve_url(base_url: &str, href: &str) -> String {
    if href.starts_with("http://") || href.starts_with("https://") {
        return href.to_string();
    }

    if let Ok(base) = url::Url::parse(base_url) {
        if href.starts_with("//") {
            return format!("{}:{}", base.scheme(), href);
        }

        if let Ok(resolved) = base.join(href) {
            return resolved.to_string();
        }
    }

    href.to_string()
}

fn rewrite_html_urls(html: &str, page_url: &str) -> Vec<u8> {
    let proxy_base = format!("http://localhost:{}/proxy?url=", LOCAL_PROXY_PORT);

    let base_url = if let Ok(parsed) = url::Url::parse(page_url) {
        let mut base = parsed.clone();
        if let Some(path) = parsed.path().rfind('/') {
            let _ = base.set_path(&parsed.path()[..=path]);
        }
        base.to_string()
    } else {
        page_url.to_string()
    };

    let origin = extract_base_url(page_url);

    let mut result = html.to_string();

    let base_tag = format!(r#"<base href="{}">"#, base_url);

    let interceptor_script = format!(r#"
<script>
(function() {{
    const PROXY_BASE = '{}';
    const ORIGIN = '{}';

    const originalToDataURL = HTMLCanvasElement.prototype.toDataURL;
    HTMLCanvasElement.prototype.toDataURL = function(...args) {{
        const ctx = this.getContext('2d');
        if (ctx) {{
            const imageData = ctx.getImageData(0, 0, this.width, this.height);
            const data = imageData.data;
            for (let i = 0; i < data.length; i += 4) {{
                data[i] = data[i] ^ (Math.random() > 0.99 ? 1 : 0);
                data[i+1] = data[i+1] ^ (Math.random() > 0.99 ? 1 : 0);
                data[i+2] = data[i+2] ^ (Math.random() > 0.99 ? 1 : 0);
            }}
            ctx.putImageData(imageData, 0, 0);
        }}
        return originalToDataURL.apply(this, args);
    }};

    const originalGetImageData = CanvasRenderingContext2D.prototype.getImageData;
    CanvasRenderingContext2D.prototype.getImageData = function(...args) {{
        const imageData = originalGetImageData.apply(this, args);
        for (let i = 0; i < imageData.data.length; i += 4) {{
            imageData.data[i] = imageData.data[i] ^ (Math.random() > 0.99 ? 1 : 0);
            imageData.data[i+1] = imageData.data[i+1] ^ (Math.random() > 0.99 ? 1 : 0);
            imageData.data[i+2] = imageData.data[i+2] ^ (Math.random() > 0.99 ? 1 : 0);
        }}
        return imageData;
    }};

    const getParameterOriginal = WebGLRenderingContext.prototype.getParameter;
    WebGLRenderingContext.prototype.getParameter = function(parameter) {{
        if (parameter === 37445) return 'Intel Inc.';
        if (parameter === 37446) return 'Intel Iris OpenGL Engine';
        return getParameterOriginal.apply(this, arguments);
    }};

    if (typeof WebGL2RenderingContext !== 'undefined') {{
        const getParameter2Original = WebGL2RenderingContext.prototype.getParameter;
        WebGL2RenderingContext.prototype.getParameter = function(parameter) {{
            if (parameter === 37445) return 'Intel Inc.';
            if (parameter === 37446) return 'Intel Iris OpenGL Engine';
            return getParameter2Original.apply(this, arguments);
        }};
    }}

    const originalGetChannelData = AudioBuffer.prototype.getChannelData;
    AudioBuffer.prototype.getChannelData = function(channel) {{
        const data = originalGetChannelData.apply(this, arguments);
        for (let i = 0; i < data.length; i++) {{
            if (Math.random() > 0.999) {{
                data[i] += (Math.random() - 0.5) * 0.0001;
            }}
        }}
        return data;
    }};

    Object.defineProperty(navigator, 'hardwareConcurrency', {{ get: () => 4 }});
    Object.defineProperty(navigator, 'deviceMemory', {{ get: () => 8 }});
    Object.defineProperty(screen, 'colorDepth', {{ get: () => 24 }});
    Object.defineProperty(screen, 'pixelDepth', {{ get: () => 24 }});

    Date.prototype.getTimezoneOffset = function() {{ return 0; }};

    const RTCPeerConnectionOriginal = window.RTCPeerConnection;
    window.RTCPeerConnection = function(...args) {{
        const pc = new RTCPeerConnectionOriginal(...args);
        const originalCreateOffer = pc.createOffer.bind(pc);
        pc.createOffer = function(options) {{
            if (!options) options = {{}};
            options.iceServers = [];
            return originalCreateOffer(options);
        }};
        return pc;
    }};
    window.RTCPeerConnection.prototype = RTCPeerConnectionOriginal.prototype;

    function getTargetUrl(url) {{
        if (url.includes('localhost:9060/proxy?url=') || url.includes('127.0.0.1:9060/proxy?url=')) {{
            const match = url.match(/proxy\?url=(.+)$/);
            if (match) return decodeURIComponent(match[1]);
        }}
        return url;
    }}

    document.addEventListener('click', function(e) {{
        let target = e.target;
        while (target && target.tagName !== 'A') {{
            target = target.parentElement;
        }}
        if (target && target.href && !target.href.startsWith('javascript:') && !target.href.startsWith('#')) {{
            e.preventDefault();
            window.location.href = PROXY_BASE + encodeURIComponent(getTargetUrl(target.href));
        }}
    }}, true);

    document.addEventListener('submit', function(e) {{
        const form = e.target;
        if (form.method.toLowerCase() === 'get') {{
            e.preventDefault();
            const formData = new FormData(form);
            const params = new URLSearchParams(formData).toString();
            const action = getTargetUrl(form.action || window.location.href);
            const url = action + (action.includes('?') ? '&' : '?') + params;
            window.location.href = PROXY_BASE + encodeURIComponent(url);
        }}
    }}, true);
}})();
</script>
"#, proxy_base, origin);

    let inject_point = if let Some(pos) = result.to_lowercase().find("<head") {
        if let Some(end) = result[pos..].find('>') {
            pos + end + 1
        } else {
            0
        }
    } else if let Some(pos) = result.to_lowercase().find("<html") {
        if let Some(end) = result[pos..].find('>') {
            pos + end + 1
        } else {
            0
        }
    } else {
        0
    };

    result.insert_str(inject_point, &format!("{}{}", base_tag, interceptor_script));

    result = result.replace("src=\"https://", &format!("src=\"{}https://", proxy_base));
    result = result.replace("src='https://", &format!("src='{}https://", proxy_base));
    result = result.replace("src=\"http://", &format!("src=\"{}http://", proxy_base));
    result = result.replace("src='http://", &format!("src='{}http://", proxy_base));
    result = result.replace("src=\"//", &format!("src=\"{}https://", proxy_base));
    result = result.replace("src='//", &format!("src='{}https://", proxy_base));

    result = rewrite_root_relative_urls(&result, &origin, &proxy_base);

    result = result.replace("href=\"https://", &format!("href=\"{}https://", proxy_base));
    result = result.replace("href='https://", &format!("href='{}https://", proxy_base));
    result = result.replace("href=\"http://", &format!("href=\"{}http://", proxy_base));
    result = result.replace("href='http://", &format!("href='{}http://", proxy_base));
    result = result.replace("href=\"//", &format!("href=\"{}https://", proxy_base));
    result = result.replace("href='//", &format!("href='{}https://", proxy_base));

    result = rewrite_srcset(&result, &origin, &proxy_base);
    result = rewrite_inline_css_urls(&result, page_url);

    result.into_bytes()
}

fn rewrite_root_relative_urls(html: &str, origin: &str, proxy_base: &str) -> String {
    let mut result = String::new();
    let mut remaining = html;

    while let Some(pos) = remaining.find("src=\"/") {
        if remaining[pos..].starts_with("src=\"//") {
            result.push_str(&remaining[..pos + 7]);
            remaining = &remaining[pos + 7..];
            continue;
        }

        result.push_str(&remaining[..pos]);
        result.push_str("src=\"");
        result.push_str(proxy_base);
        result.push_str(&urlencoding::encode(origin));

        remaining = &remaining[pos + 5..];
    }
    result.push_str(remaining);

    let html = result;
    let mut result = String::new();
    let mut remaining = html.as_str();

    while let Some(pos) = remaining.find("src='/") {
        if remaining[pos..].starts_with("src='//") {
            result.push_str(&remaining[..pos + 7]);
            remaining = &remaining[pos + 7..];
            continue;
        }

        result.push_str(&remaining[..pos]);
        result.push_str("src='");
        result.push_str(proxy_base);
        result.push_str(&urlencoding::encode(origin));

        remaining = &remaining[pos + 5..];
    }
    result.push_str(remaining);

    result
}

fn rewrite_srcset(html: &str, base_url: &str, proxy_base: &str) -> String {
    let mut result = html.to_string();
    result = rewrite_srcset_pattern(&result, "srcset=\"", '"', base_url, proxy_base);
    result = rewrite_srcset_pattern(&result, "srcset='", '\'', base_url, proxy_base);
    result
}

fn rewrite_srcset_pattern(html: &str, pattern: &str, quote: char, base_url: &str, proxy_base: &str) -> String {
    let mut result = String::new();
    let mut remaining = html;

    while let Some(start) = remaining.find(pattern) {
        result.push_str(&remaining[..start]);

        let after = &remaining[start + pattern.len()..];

        if let Some(end) = after.find(quote) {
            let srcset_content = &after[..end];
            let rewritten = rewrite_srcset_content(srcset_content, base_url, proxy_base);
            result.push_str(&format!("{}{}{}", pattern, rewritten, quote));
            remaining = &after[end + 1..];
        } else {
            result.push_str(pattern);
            remaining = after;
        }
    }

    result.push_str(remaining);
    result
}

fn rewrite_srcset_content(srcset: &str, base_url: &str, proxy_base: &str) -> String {
    srcset
        .split(',')
        .map(|part| {
            let trimmed = part.trim();
            let parts: Vec<&str> = trimmed.splitn(2, ' ').collect();
            if let Some(url) = parts.first() {
                let resolved = resolve_url(base_url, url);
                let proxied = format!("{}{}", proxy_base, urlencoding::encode(&resolved));
                if parts.len() > 1 {
                    format!("{} {}", proxied, parts[1])
                } else {
                    proxied
                }
            } else {
                trimmed.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn rewrite_css_urls(css: &str, page_url: &str) -> Vec<u8> {
    let proxy_base = format!("http://localhost:{}/proxy?url=", LOCAL_PROXY_PORT);
    let base_url = extract_base_url(page_url);

    let mut result = css.to_string();

    result = rewrite_css_url_pattern(&result, "url(\"/", "url(\"", &base_url, &proxy_base);
    result = rewrite_css_url_pattern(&result, "url('/", "url('", &base_url, &proxy_base);
    result = rewrite_css_url_pattern(&result, "url(/", "url(", &base_url, &proxy_base);

    result = result.replace("url(\"https://", &format!("url(\"{}https://", proxy_base));
    result = result.replace("url('https://", &format!("url('{}https://", proxy_base));
    result = result.replace("url(https://", &format!("url({}https://", proxy_base));
    result = result.replace("url(\"http://", &format!("url(\"{}http://", proxy_base));
    result = result.replace("url('http://", &format!("url('{}http://", proxy_base));
    result = result.replace("url(http://", &format!("url({}http://", proxy_base));

    result = result.replace("url(\"//", &format!("url(\"{}https://", proxy_base));
    result = result.replace("url('//", &format!("url('{}https://", proxy_base));
    result = result.replace("url(//", &format!("url({}https://", proxy_base));

    result = rewrite_css_imports(&result, &proxy_base);

    result.into_bytes()
}

fn rewrite_css_url_pattern(css: &str, pattern: &str, prefix: &str, base_url: &str, proxy_base: &str) -> String {
    let mut result = String::new();
    let mut remaining = css;

    while let Some(start) = remaining.find(pattern) {
        result.push_str(&remaining[..start]);

        let after_pattern = &remaining[start + pattern.len()..];

        let end_char = if pattern.contains('"') {
            '"'
        } else if pattern.contains('\'') {
            '\''
        } else {
            ')'
        };

        if let Some(end) = after_pattern.find(end_char) {
            let path = &after_pattern[..end];
            if !path.starts_with("data:") {
                let full_url = resolve_url(base_url, &format!("/{}", path.trim_start_matches('/')));
                result.push_str(&format!("{}{}{}{}",
                    prefix,
                    proxy_base,
                    urlencoding::encode(&full_url),
                    end_char
                ));
            } else {
                result.push_str(pattern);
                result.push_str(&after_pattern[..end + 1]);
            }
            remaining = &after_pattern[end + 1..];
        } else {
            result.push_str(pattern);
            remaining = after_pattern;
        }
    }

    result.push_str(remaining);
    result
}

fn rewrite_css_imports(css: &str, proxy_base: &str) -> String {
    let mut result = css.to_string();

    result = result.replace("@import url(\"https://", &format!("@import url(\"{}https://", proxy_base));
    result = result.replace("@import url('https://", &format!("@import url('{}https://", proxy_base));
    result = result.replace("@import \"https://", &format!("@import \"{}https://", proxy_base));
    result = result.replace("@import 'https://", &format!("@import '{}https://", proxy_base));

    result
}

fn rewrite_inline_css_urls(html: &str, page_url: &str) -> String {
    let proxy_base = format!("http://localhost:{}/proxy?url=", LOCAL_PROXY_PORT);
    let base_url = extract_base_url(page_url);

    let mut result = html.to_string();

    result = result.replace("url(\"/", &format!("url(\"{}{}/", proxy_base, urlencoding::encode(&base_url)));
    result = result.replace("url('/", &format!("url('{}{}/", proxy_base, urlencoding::encode(&base_url)));

    result
}

async fn handle_proxy_request(
    req: Request<Incoming>,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let uri = req.uri();
    let query = uri.query().unwrap_or("");

    if req.method() == hyper::Method::OPTIONS {
        return Ok(Response::builder()
            .status(200)
            .header("Access-Control-Allow-Origin", "*")
            .header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
            .header("Access-Control-Allow-Headers", "*")
            .header("Access-Control-Max-Age", "86400")
            .body(Full::new(Bytes::new()))
            .unwrap());
    }

    let target_url = query
        .split('&')
        .find_map(|param| {
            let parts: Vec<&str> = param.splitn(2, '=').collect();
            if parts.len() == 2 && parts[0] == "url" {
                urlencoding::decode(parts[1]).ok().map(|s| s.into_owned())
            } else {
                None
            }
        });

    let target_url = match target_url {
        Some(url) => url,
        None => {
            let body = r#"<!DOCTYPE html>
<html><head><title>NONOS Proxy</title></head>
<body style="font-family: sans-serif; padding: 40px; background: #0a0a0f; color: #e0e0e0;">
<h1 style="color: #66ffff;">NONOS Privacy Proxy</h1>
<p>Missing 'url' parameter. Usage: /proxy?url=https://example.com</p>
</body></html>"#;
            return Ok(Response::builder()
                .status(400)
                .header("Content-Type", "text/html")
                .header("Access-Control-Allow-Origin", "*")
                .body(Full::new(Bytes::from(body)))
                .unwrap());
        }
    };

    let client = build_proxy_client();

    match client.get(&target_url).send().await {
        Ok(response) => {
            let status = response.status();
            let final_url = response.url().to_string();
            let content_type = response.headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("application/octet-stream")
                .to_string();

            match response.bytes().await {
                Ok(body_bytes) => {
                    let final_body = if content_type.contains("text/html") {
                        let html = String::from_utf8_lossy(&body_bytes);
                        rewrite_html_urls(&html, &final_url)
                    } else if content_type.contains("text/css") {
                        let css = String::from_utf8_lossy(&body_bytes);
                        rewrite_css_urls(&css, &final_url)
                    } else {
                        body_bytes.to_vec()
                    };

                    Ok(Response::builder()
                        .status(status)
                        .header("Content-Type", &content_type)
                        .header("Access-Control-Allow-Origin", "*")
                        .header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
                        .header("Access-Control-Allow-Headers", "*")
                        .body(Full::new(Bytes::from(final_body)))
                        .unwrap())
                }
                Err(e) => {
                    let body = format!(r#"<!DOCTYPE html>
<html><head><title>Error</title></head>
<body style="font-family: sans-serif; padding: 40px; background: #0a0a0f; color: #e0e0e0;">
<h1 style="color: #ff6666;">Error Reading Response</h1>
<p>{}</p>
<p><a href="javascript:history.back()" style="color: #66ffff;">Go Back</a></p>
</body></html>"#, e);
                    Ok(Response::builder()
                        .status(502)
                        .header("Content-Type", "text/html")
                        .header("Access-Control-Allow-Origin", "*")
                        .body(Full::new(Bytes::from(body)))
                        .unwrap())
                }
            }
        }
        Err(e) => {
            let body = format!(r#"<!DOCTYPE html>
<html><head><title>Error</title></head>
<body style="font-family: sans-serif; padding: 40px; background: #0a0a0f; color: #e0e0e0;">
<h1 style="color: #ff6666;">Connection Error</h1>
<p>Failed to fetch: {}</p>
<p style="color: #888;">URL: {}</p>
<p><a href="javascript:history.back()" style="color: #66ffff;">Go Back</a></p>
</body></html>"#, e, target_url);
            Ok(Response::builder()
                .status(502)
                .header("Content-Type", "text/html")
                .header("Access-Control-Allow-Origin", "*")
                .body(Full::new(Bytes::from(body)))
                .unwrap())
        }
    }
}

pub async fn start_local_proxy_server() {
    let addr = SocketAddr::from(([127, 0, 0, 1], LOCAL_PROXY_PORT));

    let listener = match TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(_) => return,
    };

    loop {
        let (stream, _) = match listener.accept().await {
            Ok(conn) => conn,
            Err(_) => continue,
        };

        let io = TokioIo::new(stream);

        tokio::spawn(async move {
            let _ = http1::Builder::new()
                .serve_connection(io, service_fn(handle_proxy_request))
                .await;
        });
    }
}
