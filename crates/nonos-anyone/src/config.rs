use nonos_types::{
    ANYONE_CIRCUIT_LENGTH, ANYONE_CIRCUIT_ROTATION_SECS,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnyoneConfig {
    pub data_dir: PathBuf,

    pub socks_port: u16,

    pub circuit_length: usize,

    pub circuit_rotation_secs: u64,

    pub max_circuits: usize,

    pub use_bridges: bool,

    pub bridges: Vec<String>,

    pub directory_authorities: Vec<String>,

    pub bootstrap_on_start: bool,

    pub connection_timeout_secs: u64,

    pub stream_timeout_secs: u64,
}

impl Default for AnyoneConfig {
    fn default() -> Self {
        let data_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".nonos")
            .join("anyone");

        Self {
            data_dir,
            socks_port: 9150,
            circuit_length: ANYONE_CIRCUIT_LENGTH,
            circuit_rotation_secs: ANYONE_CIRCUIT_ROTATION_SECS,
            max_circuits: 10,
            use_bridges: false,
            bridges: Vec::new(),
            directory_authorities: default_directory_authorities(),
            bootstrap_on_start: true,
            connection_timeout_secs: 30,
            stream_timeout_secs: 60,
        }
    }
}

impl AnyoneConfig {
    pub fn with_data_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.data_dir = dir.into();
        self
    }

    pub fn with_socks_port(mut self, port: u16) -> Self {
        self.socks_port = port;
        self
    }

    pub fn with_bridges(mut self, bridges: Vec<String>) -> Self {
        self.use_bridges = true;
        self.bridges = bridges;
        self
    }

    pub fn with_rotation_interval(mut self, secs: u64) -> Self {
        self.circuit_rotation_secs = secs;
        self
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.socks_port == 0 {
            return Err("SOCKS port cannot be 0".into());
        }

        if self.circuit_length < 1 || self.circuit_length > 8 {
            return Err("Circuit length must be between 1 and 8".into());
        }

        if self.use_bridges && self.bridges.is_empty() {
            return Err("Bridges enabled but no bridge addresses provided".into());
        }

        Ok(())
    }
}

fn default_directory_authorities() -> Vec<String> {
    vec![
        "dirauth1.anyone.io:9030".to_string(),
        "dirauth2.anyone.io:9030".to_string(),
        "dirauth3.anyone.io:9030".to_string(),
    ]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecurityPreset {
    Standard,
    Enhanced,
    Maximum,
}

impl Default for SecurityPreset {
    fn default() -> Self {
        Self::Enhanced
    }
}

impl SecurityPreset {
    pub fn circuit_length(&self) -> usize {
        match self {
            SecurityPreset::Standard => 3,
            SecurityPreset::Enhanced => 3,
            SecurityPreset::Maximum => 4,
        }
    }

    pub fn rotation_interval(&self) -> u64 {
        match self {
            SecurityPreset::Standard => 600,
            SecurityPreset::Enhanced => 300,
            SecurityPreset::Maximum => 120,
        }
    }

    pub fn use_guards(&self) -> bool {
        true
    }

    pub fn allow_single_hop(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AnyoneConfig::default();
        assert!(config.validate().is_ok());
        assert_eq!(config.circuit_length, 3);
    }

    #[test]
    fn test_config_validation() {
        let mut config = AnyoneConfig::default();
        config.circuit_length = 0;
        assert!(config.validate().is_err());

        config.circuit_length = 3;
        config.use_bridges = true;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_security_presets() {
        assert_eq!(SecurityPreset::Standard.circuit_length(), 3);
        assert_eq!(SecurityPreset::Maximum.circuit_length(), 4);
        assert!(!SecurityPreset::Maximum.allow_single_hop());
    }
}
