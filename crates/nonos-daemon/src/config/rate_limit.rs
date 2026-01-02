use serde::{Deserialize, Serialize};
use super::constants::DEFAULT_RATE_LIMIT_RPS;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct RateLimitConfig {
    pub enabled: bool,
    pub requests_per_second: u32,
    pub burst_size: u32,
    pub p2p_messages_per_second: u32,
    pub p2p_burst_size: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            requests_per_second: DEFAULT_RATE_LIMIT_RPS,
            burst_size: 200,
            p2p_messages_per_second: 50,
            p2p_burst_size: 100,
        }
    }
}
