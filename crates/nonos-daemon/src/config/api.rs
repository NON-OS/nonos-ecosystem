use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr};
use super::constants::DEFAULT_API_PORT;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ApiConfig {
    pub enabled: bool,
    pub bind_address: IpAddr,
    pub port: u16,
    pub request_timeout_secs: u64,
    pub max_body_size: usize,
    pub cors_enabled: bool,
    pub cors_origins: Vec<String>,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            bind_address: IpAddr::V4(Ipv4Addr::LOCALHOST),
            port: DEFAULT_API_PORT,
            request_timeout_secs: 30,
            max_body_size: 1024 * 1024,
            cors_enabled: true,
            cors_origins: vec![],
        }
    }
}
