use serde::{Deserialize, Serialize};
use super::constants::DEFAULT_BOOTSTRAP_PORT;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ServicesConfig {
    pub health_beacon: bool,
    pub health_beacon_interval_secs: u64,
    pub quality_oracle: bool,
    pub quality_oracle_interval_secs: u64,
    pub bootstrap: bool,
    pub bootstrap_port: u16,
    pub cache: bool,
    pub cache_size_mb: u32,
    pub cache_max_age_secs: u64,
}

impl Default for ServicesConfig {
    fn default() -> Self {
        Self {
            health_beacon: true,
            health_beacon_interval_secs: 60,
            quality_oracle: true,
            quality_oracle_interval_secs: 300,
            bootstrap: false,
            bootstrap_port: DEFAULT_BOOTSTRAP_PORT,
            cache: false,
            cache_size_mb: 1024,
            cache_max_age_secs: 86400,
        }
    }
}
