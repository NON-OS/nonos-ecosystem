use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct AnyoneNetworkConfig {
    pub enabled: bool,
    pub socks_port: u16,
    pub control_port: u16,
    pub auto_start: bool,
    pub circuit_rotation_secs: u64,
    pub use_bridges: bool,
    pub bridges: Vec<String>,
    pub security_level: SecurityLevel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SecurityLevel {
    Standard,
    Enhanced,
    Maximum,
}

impl Default for SecurityLevel {
    fn default() -> Self {
        Self::Enhanced
    }
}

impl Default for AnyoneNetworkConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            socks_port: 9050,
            control_port: 9051,
            auto_start: true,
            circuit_rotation_secs: 600,
            use_bridges: false,
            bridges: Vec::new(),
            security_level: SecurityLevel::Enhanced,
        }
    }
}

impl AnyoneNetworkConfig {
    pub fn socks_addr(&self) -> String {
        format!("socks5h://127.0.0.1:{}", self.socks_port)
    }

    pub fn to_anyone_config(&self, data_dir: &PathBuf) -> nonos_anyone::AnyoneConfig {
        nonos_anyone::AnyoneConfig {
            data_dir: data_dir.join("anyone"),
            socks_port: self.socks_port,
            circuit_length: match self.security_level {
                SecurityLevel::Standard => 3,
                SecurityLevel::Enhanced => 3,
                SecurityLevel::Maximum => 4,
            },
            circuit_rotation_secs: self.circuit_rotation_secs,
            use_bridges: self.use_bridges,
            bridges: self.bridges.clone(),
        }
    }
}
