use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_socks::tcp::Socks5Stream;
use std::io::Read;
use std::pin::Pin;
use std::future::Future;

const SOCKS_ADDR: &str = "127.0.0.1:9050";
const MAX_REDIRECTS: u8 = 10;

pub async fn fetch(url: &str) -> Result<(u16, String, Vec<u8>), String> {
    fetch_inner(url.to_string(), 0).await
}

fn fetch_inner(url: String, redirect_count: u8) -> Pin<Box<dyn Future<Output = Result<(u16, String, Vec<u8>), String>> + Send>> {
    Box::pin(async move {
        if redirect_count > MAX_REDIRECTS {
            return Err("Too many redirects".to_string());
        }

        let parsed = url::Url::parse(&url).map_err(|e| e.to_string())?;
        let host = parsed.host_str().ok_or("No host")?.to_string();
        let port = parsed.port().unwrap_or(if parsed.scheme() == "https" { 443 } else { 80 });
        let path = match parsed.query() {
            Some(q) => format!("{}?{}", parsed.path(), q),
            None => parsed.path().to_string(),
        };
        let path = if path.is_empty() { "/".to_string() } else { path };

        let stream = Socks5Stream::connect(SOCKS_ADDR, (host.as_str(), port))
            .await
            .map_err(|e| format!("SOCKS5 connection failed: {}", e))?;

        let request = format!(
            "GET {} HTTP/1.1\r\nHost: {}\r\nUser-Agent: Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36\r\nAccept: text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8\r\nAccept-Language: en-US,en;q=0.9\r\nAccept-Encoding: gzip, deflate\r\nConnection: close\r\n\r\n",
            path, host
        );

        let scheme = parsed.scheme().to_string();
        let (status, headers, body) = if scheme == "https" {
            fetch_https(stream, &host, &request).await?
        } else {
            fetch_http(stream, &request).await?
        };

        if status == 301 || status == 302 || status == 303 || status == 307 || status == 308 {
            if let Some(location) = extract_header(&headers, "location") {
                let new_url = if location.starts_with("http") {
                    location
                } else if location.starts_with("//") {
                    format!("{}:{}", scheme, location)
                } else if location.starts_with('/') {
                    format!("{}://{}{}", scheme, host, location)
                } else {
                    format!("{}://{}/{}", scheme, host, location)
                };
                return fetch_inner(new_url, redirect_count + 1).await;
            }
        }

        let content_encoding = extract_header(&headers, "content-encoding").unwrap_or_default();
        let body = decompress(&body, &content_encoding)?;

        let content_type = extract_header(&headers, "content-type")
            .unwrap_or_else(|| "text/html".to_string())
            .split(';').next().unwrap_or("text/html").trim().to_string();

        Ok((status, content_type, body))
    })
}

async fn fetch_https(stream: Socks5Stream<tokio::net::TcpStream>, host: &str, request: &str) -> Result<(u16, String, Vec<u8>), String> {
    let connector = tokio_native_tls::TlsConnector::from(
        native_tls::TlsConnector::builder()
            .danger_accept_invalid_certs(false)
            .build()
            .map_err(|e| e.to_string())?
    );

    let mut tls = connector.connect(host, stream.into_inner())
        .await
        .map_err(|e| format!("TLS handshake failed: {}", e))?;

    tls.write_all(request.as_bytes()).await.map_err(|e| e.to_string())?;
    tls.flush().await.map_err(|e| e.to_string())?;

    let mut buf = Vec::new();
    tls.read_to_end(&mut buf).await.map_err(|e| e.to_string())?;
    parse_response(&buf)
}

async fn fetch_http(stream: Socks5Stream<tokio::net::TcpStream>, request: &str) -> Result<(u16, String, Vec<u8>), String> {
    let mut tcp = stream.into_inner();
    tcp.write_all(request.as_bytes()).await.map_err(|e| e.to_string())?;
    tcp.flush().await.map_err(|e| e.to_string())?;

    let mut buf = Vec::new();
    tcp.read_to_end(&mut buf).await.map_err(|e| e.to_string())?;
    parse_response(&buf)
}

fn parse_response(data: &[u8]) -> Result<(u16, String, Vec<u8>), String> {
    let sep = data.windows(4).position(|w| w == b"\r\n\r\n").ok_or("Invalid HTTP response")?;
    let header = String::from_utf8_lossy(&data[..sep]).to_string();

    let status: u16 = header.lines().next()
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(200);

    let body = if header.to_lowercase().contains("transfer-encoding: chunked") {
        decode_chunked(&data[sep + 4..])
    } else {
        data[sep + 4..].to_vec()
    };

    Ok((status, header, body))
}

fn extract_header(headers: &str, name: &str) -> Option<String> {
    let search = format!("{}:", name);
    headers.lines()
        .find(|l| l.to_lowercase().starts_with(&search))
        .map(|l| l.splitn(2, ':').nth(1).unwrap_or("").trim().to_string())
}

fn decompress(data: &[u8], encoding: &str) -> Result<Vec<u8>, String> {
    if data.is_empty() {
        return Ok(Vec::new());
    }

    let enc = encoding.to_lowercase();
    if enc.contains("gzip") {
        let mut decoder = flate2::read::GzDecoder::new(data);
        let mut out = Vec::new();
        decoder.read_to_end(&mut out).map_err(|e| format!("gzip decompress failed: {}", e))?;
        Ok(out)
    } else if enc.contains("deflate") {
        let mut decoder = flate2::read::DeflateDecoder::new(data);
        let mut out = Vec::new();
        decoder.read_to_end(&mut out).map_err(|e| format!("deflate decompress failed: {}", e))?;
        Ok(out)
    } else {
        Ok(data.to_vec())
    }
}

fn decode_chunked(data: &[u8]) -> Vec<u8> {
    let mut result = Vec::new();
    let mut pos = 0;

    while pos < data.len() {
        let line_end = data[pos..].windows(2).position(|w| w == b"\r\n");
        if line_end.is_none() { break; }
        let line_end = pos + line_end.unwrap();

        let size_str = String::from_utf8_lossy(&data[pos..line_end]);
        let size = usize::from_str_radix(size_str.trim(), 16).unwrap_or(0);
        if size == 0 { break; }

        let chunk_start = line_end + 2;
        let chunk_end = chunk_start + size;
        if chunk_end > data.len() { break; }

        result.extend_from_slice(&data[chunk_start..chunk_end]);
        pos = chunk_end + 2;
    }

    result
}
