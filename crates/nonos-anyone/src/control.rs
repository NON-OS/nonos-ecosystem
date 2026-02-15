use nonos_types::{NonosError, NonosResult};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

pub struct ControlConnection {
    stream: TcpStream,
    authenticated: bool,
}

impl ControlConnection {
    pub async fn connect(addr: SocketAddr) -> NonosResult<Self> {
        let stream = TcpStream::connect(addr).await.map_err(|e| {
            NonosError::Network(format!("Failed to connect to control port: {}", e))
        })?;

        Ok(Self {
            stream,
            authenticated: false,
        })
    }

    pub async fn authenticate(&mut self, password: Option<&str>) -> NonosResult<()> {
        let auth_cmd = match password {
            Some(pw) => format!("AUTHENTICATE \"{}\"\r\n", pw),
            None => "AUTHENTICATE\r\n".to_string(),
        };

        self.send_command(&auth_cmd).await?;
        let response = self.read_response().await?;

        if response.starts_with("250") {
            self.authenticated = true;
            Ok(())
        } else {
            Err(NonosError::Network(format!(
                "Authentication failed: {}",
                response
            )))
        }
    }

    pub async fn authenticate_cookie(&mut self, cookie_path: &PathBuf) -> NonosResult<()> {
        let cookie = tokio::fs::read(cookie_path).await.map_err(|e| {
            NonosError::Network(format!("Failed to read cookie file: {}", e))
        })?;

        let hex_cookie = hex::encode(&cookie);
        let auth_cmd = format!("AUTHENTICATE {}\r\n", hex_cookie);

        self.send_command(&auth_cmd).await?;
        let response = self.read_response().await?;

        if response.starts_with("250") {
            self.authenticated = true;
            Ok(())
        } else {
            Err(NonosError::Network(format!(
                "Cookie authentication failed: {}",
                response
            )))
        }
    }

    pub async fn signal_newnym(&mut self) -> NonosResult<()> {
        self.ensure_authenticated()?;
        self.send_command("SIGNAL NEWNYM\r\n").await?;
        let response = self.read_response().await?;

        if response.starts_with("250") {
            Ok(())
        } else {
            Err(NonosError::Network(format!("NEWNYM failed: {}", response)))
        }
    }

    pub async fn get_info(&mut self, key: &str) -> NonosResult<String> {
        self.ensure_authenticated()?;
        self.send_command(&format!("GETINFO {}\r\n", key)).await?;
        self.read_response().await
    }

    pub async fn get_circuit_status(&mut self) -> NonosResult<Vec<CircuitEntry>> {
        let response = self.get_info("circuit-status").await?;
        parse_circuit_status(&response)
    }

    pub async fn get_stream_status(&mut self) -> NonosResult<Vec<StreamEntry>> {
        let response = self.get_info("stream-status").await?;
        parse_stream_status(&response)
    }

    pub async fn get_bootstrap_status(&mut self) -> NonosResult<BootstrapStatus> {
        let response = self.get_info("status/bootstrap-phase").await?;
        parse_bootstrap_status(&response)
    }

    pub async fn get_version(&mut self) -> NonosResult<String> {
        let response = self.get_info("version").await?;
        Ok(response
            .lines()
            .find(|l| l.starts_with("version="))
            .map(|l| l.trim_start_matches("version=").to_string())
            .unwrap_or_else(|| "unknown".to_string()))
    }

    pub async fn get_traffic_stats(&mut self) -> NonosResult<TrafficStats> {
        let read = self.get_info("traffic/read").await?;
        let written = self.get_info("traffic/written").await?;

        let bytes_read = read
            .lines()
            .find(|l| l.starts_with("traffic/read="))
            .and_then(|l| l.trim_start_matches("traffic/read=").parse().ok())
            .unwrap_or(0);

        let bytes_written = written
            .lines()
            .find(|l| l.starts_with("traffic/written="))
            .and_then(|l| l.trim_start_matches("traffic/written=").parse().ok())
            .unwrap_or(0);

        Ok(TrafficStats {
            bytes_read,
            bytes_written,
        })
    }

    pub async fn close_circuit(&mut self, circuit_id: u32) -> NonosResult<()> {
        self.ensure_authenticated()?;
        self.send_command(&format!("CLOSECIRCUIT {}\r\n", circuit_id))
            .await?;
        let response = self.read_response().await?;

        if response.starts_with("250") {
            Ok(())
        } else {
            Err(NonosError::Network(format!(
                "Failed to close circuit: {}",
                response
            )))
        }
    }

    pub async fn close_stream(&mut self, stream_id: u32) -> NonosResult<()> {
        self.ensure_authenticated()?;
        self.send_command(&format!("CLOSESTREAM {} 1\r\n", stream_id))
            .await?;
        let response = self.read_response().await?;

        if response.starts_with("250") {
            Ok(())
        } else {
            Err(NonosError::Network(format!(
                "Failed to close stream: {}",
                response
            )))
        }
    }

    pub async fn set_conf(&mut self, key: &str, value: &str) -> NonosResult<()> {
        self.ensure_authenticated()?;
        self.send_command(&format!("SETCONF {}=\"{}\"\r\n", key, value))
            .await?;
        let response = self.read_response().await?;

        if response.starts_with("250") {
            Ok(())
        } else {
            Err(NonosError::Network(format!(
                "SETCONF failed: {}",
                response
            )))
        }
    }

    pub async fn get_conf(&mut self, key: &str) -> NonosResult<String> {
        self.ensure_authenticated()?;
        self.send_command(&format!("GETCONF {}\r\n", key)).await?;
        self.read_response().await
    }

    pub async fn quit(&mut self) -> NonosResult<()> {
        let _ = self.send_command("QUIT\r\n").await;
        Ok(())
    }

    fn ensure_authenticated(&self) -> NonosResult<()> {
        if !self.authenticated {
            Err(NonosError::Network("Not authenticated".into()))
        } else {
            Ok(())
        }
    }

    async fn send_command(&mut self, cmd: &str) -> NonosResult<()> {
        self.stream.write_all(cmd.as_bytes()).await.map_err(|e| {
            NonosError::Network(format!("Failed to send command: {}", e))
        })?;
        self.stream.flush().await.map_err(|e| {
            NonosError::Network(format!("Failed to flush: {}", e))
        })?;
        Ok(())
    }

    async fn read_response(&mut self) -> NonosResult<String> {
        let mut reader = BufReader::new(&mut self.stream);
        let mut response = String::new();

        loop {
            let mut line = String::new();
            reader.read_line(&mut line).await.map_err(|e| {
                NonosError::Network(format!("Failed to read response: {}", e))
            })?;

            if line.is_empty() {
                break;
            }

            response.push_str(&line);

            if line.len() >= 4 {
                let status = &line[..3];
                let separator = line.chars().nth(3).unwrap_or(' ');
                if separator == ' ' && (status.starts_with("2") || status.starts_with("5")) {
                    break;
                }
            }
        }

        Ok(response)
    }
}

#[derive(Clone, Debug)]
pub struct CircuitEntry {
    pub id: u32,
    pub status: String,
    pub path: Vec<RelayInfo>,
    pub purpose: String,
}

#[derive(Clone, Debug)]
pub struct RelayInfo {
    pub fingerprint: String,
    pub nickname: Option<String>,
}

#[derive(Clone, Debug)]
pub struct StreamEntry {
    pub id: u32,
    pub status: String,
    pub circuit_id: u32,
    pub target: String,
}

#[derive(Clone, Debug)]
pub struct BootstrapStatus {
    pub progress: u8,
    pub tag: String,
    pub summary: String,
}

#[derive(Clone, Debug, Default)]
pub struct TrafficStats {
    pub bytes_read: u64,
    pub bytes_written: u64,
}

fn parse_circuit_status(response: &str) -> NonosResult<Vec<CircuitEntry>> {
    let mut circuits = Vec::new();

    for line in response.lines() {
        if line.starts_with("250+circuit-status=") || line.starts_with("250-circuit-status=") {
            continue;
        }
        if line == "250 OK" || line == "." {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }

        let id = match parts[0].parse::<u32>() {
            Ok(id) => id,
            Err(_) => continue,
        };

        let status = parts[1].to_string();

        let mut path = Vec::new();
        let mut purpose = "GENERAL".to_string();

        for part in &parts[2..] {
            if part.starts_with("PURPOSE=") {
                purpose = part.trim_start_matches("PURPOSE=").to_string();
            } else if part.contains('$') || part.contains('~') {
                for relay in part.split(',') {
                    let relay = relay.trim();
                    if relay.is_empty() {
                        continue;
                    }

                    let (fingerprint, nickname) = if relay.contains('~') {
                        let mut parts = relay.splitn(2, '~');
                        let fp = parts.next().unwrap_or("").trim_start_matches('$');
                        let nick = parts.next().map(|s| s.to_string());
                        (fp.to_string(), nick)
                    } else {
                        (relay.trim_start_matches('$').to_string(), None)
                    };

                    if !fingerprint.is_empty() {
                        path.push(RelayInfo {
                            fingerprint,
                            nickname,
                        });
                    }
                }
            }
        }

        circuits.push(CircuitEntry {
            id,
            status,
            path,
            purpose,
        });
    }

    Ok(circuits)
}

fn parse_stream_status(response: &str) -> NonosResult<Vec<StreamEntry>> {
    let mut streams = Vec::new();

    for line in response.lines() {
        if line.starts_with("250+stream-status=") || line.starts_with("250-stream-status=") {
            continue;
        }
        if line == "250 OK" || line == "." {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            continue;
        }

        let id = match parts[0].parse::<u32>() {
            Ok(id) => id,
            Err(_) => continue,
        };

        let status = parts[1].to_string();

        let circuit_id: u32 = parts[2].parse().unwrap_or_default();

        let target = parts[3].to_string();

        streams.push(StreamEntry {
            id,
            status,
            circuit_id,
            target,
        });
    }

    Ok(streams)
}

fn parse_bootstrap_status(response: &str) -> NonosResult<BootstrapStatus> {
    let mut progress = 0u8;
    let mut tag = String::new();
    let mut summary = String::new();

    for line in response.lines() {
        if line.contains("BOOTSTRAP") {
            let params: HashMap<String, String> = line
                .split_whitespace()
                .filter_map(|p| {
                    let mut parts = p.splitn(2, '=');
                    let key = parts.next()?;
                    let value = parts.next()?.trim_matches('"');
                    Some((key.to_string(), value.to_string()))
                })
                .collect();

            if let Some(p) = params.get("PROGRESS") {
                progress = p.parse().unwrap_or(0);
            }
            if let Some(t) = params.get("TAG") {
                tag = t.clone();
            }
            if let Some(s) = params.get("SUMMARY") {
                summary = s.clone();
            }
        }
    }

    Ok(BootstrapStatus {
        progress,
        tag,
        summary,
    })
}

pub struct ControlClient {
    addr: SocketAddr,
    cookie_path: Option<PathBuf>,
    password: Option<String>,
}

impl ControlClient {
    pub fn new(port: u16) -> Self {
        Self {
            addr: SocketAddr::from(([127, 0, 0, 1], port)),
            cookie_path: None,
            password: None,
        }
    }

    pub fn with_cookie(mut self, path: PathBuf) -> Self {
        self.cookie_path = Some(path);
        self
    }

    pub fn with_password(mut self, password: String) -> Self {
        self.password = Some(password);
        self
    }

    pub async fn connect(&self) -> NonosResult<ControlConnection> {
        let mut conn = ControlConnection::connect(self.addr).await?;

        if let Some(ref cookie_path) = self.cookie_path {
            conn.authenticate_cookie(cookie_path).await?;
        } else if let Some(ref password) = self.password {
            conn.authenticate(Some(password)).await?;
        } else {
            conn.authenticate(None).await?;
        }

        Ok(conn)
    }

    pub async fn signal_newnym(&self) -> NonosResult<()> {
        let mut conn = self.connect().await?;
        conn.signal_newnym().await
    }

    pub async fn get_circuits(&self) -> NonosResult<Vec<CircuitEntry>> {
        let mut conn = self.connect().await?;
        conn.get_circuit_status().await
    }

    pub async fn get_bootstrap_progress(&self) -> NonosResult<u8> {
        match self.connect().await {
            Ok(mut conn) => {
                let status = conn.get_bootstrap_status().await?;
                Ok(status.progress)
            }
            Err(_) => Ok(0),
        }
    }

    pub async fn get_traffic(&self) -> NonosResult<TrafficStats> {
        let mut conn = self.connect().await?;
        conn.get_traffic_stats().await
    }

    pub async fn is_ready(&self) -> bool {
        match self.connect().await {
            Ok(mut conn) => {
                if let Ok(status) = conn.get_bootstrap_status().await {
                    status.progress >= 100
                } else {
                    false
                }
            }
            Err(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_circuit_status() {
        let response = "250+circuit-status=\n\
            1 BUILT $AAAA~Guard,$BBBB~Middle,$CCCC~Exit PURPOSE=GENERAL\n\
            2 EXTENDED $DDDD PURPOSE=HS_SERVICE_INTRO\n\
            .\n\
            250 OK";

        let circuits = parse_circuit_status(response).unwrap();
        assert_eq!(circuits.len(), 2);
        assert_eq!(circuits[0].id, 1);
        assert_eq!(circuits[0].status, "BUILT");
        assert_eq!(circuits[0].path.len(), 3);
        assert_eq!(circuits[0].purpose, "GENERAL");
    }

    #[test]
    fn test_parse_bootstrap_status() {
        let response = "250-status/bootstrap-phase=NOTICE BOOTSTRAP PROGRESS=100 TAG=done SUMMARY=\"Done\"";

        let status = parse_bootstrap_status(response).unwrap();
        assert_eq!(status.progress, 100);
        assert_eq!(status.tag, "done");
    }

    #[test]
    fn test_control_client_creation() {
        let client = ControlClient::new(9051);
        assert_eq!(client.addr.port(), 9051);
    }
}
