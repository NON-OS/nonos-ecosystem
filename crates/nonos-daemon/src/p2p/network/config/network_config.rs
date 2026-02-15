use std::time::Duration;
use super::constants::{DEFAULT_MESSAGES_PER_SEC, DEFAULT_BYTES_PER_SEC, MAX_MESSAGE_SIZE, MAX_PEERS};

#[derive(Debug, Clone)]
pub struct NetworkConfig {
    pub port: u16,
    pub max_connections: u32,
    pub messages_per_sec: u32,
    pub bytes_per_sec: u64,
    pub enable_rate_limiting: bool,
    pub enable_circuit_breaker: bool,
    pub max_message_size: usize,
    pub idle_timeout: Duration,
    pub dial_timeout: Duration,
    pub bootstrap_on_start: bool,
    pub custom_bootstrap_nodes: Vec<String>,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            port: 9432,
            max_connections: MAX_PEERS as u32,
            messages_per_sec: DEFAULT_MESSAGES_PER_SEC,
            bytes_per_sec: DEFAULT_BYTES_PER_SEC,
            enable_rate_limiting: true,
            enable_circuit_breaker: true,
            max_message_size: MAX_MESSAGE_SIZE,
            idle_timeout: Duration::from_secs(60),
            dial_timeout: Duration::from_secs(10),
            bootstrap_on_start: true,
            custom_bootstrap_nodes: Vec::new(),
        }
    }
}
