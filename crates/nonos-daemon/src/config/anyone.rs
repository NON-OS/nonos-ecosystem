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
    /// Bind address for SOCKS proxy. Default: 127.0.0.1 (loopback only).
    pub bind_address: String,
    /// Require SOCKS5 authentication.
    pub require_auth: bool,
    /// SOCKS5 authentication username.
    pub auth_username: Option<String>,
    /// SOCKS5 authentication password (hashed).
    pub auth_password_hash: Option<String>,
    /// Maximum concurrent connections.
    pub max_connections: u32,
    /// Rate limit: requests per second per IP.
    pub rate_limit_rps: u32,
    /// Refuse non-loopback connections unless auth is enabled.
    pub strict_loopback: bool,
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
            // Security hardening defaults
            bind_address: "127.0.0.1".to_string(), // Loopback only by default
            require_auth: false,
            auth_username: None,
            auth_password_hash: None,
            max_connections: 100,
            rate_limit_rps: 50,
            strict_loopback: true, // Refuse non-loopback unless auth enabled
        }
    }
}

impl AnyoneNetworkConfig {
    /// Get SOCKS proxy address string.
    pub fn socks_addr(&self) -> String {
        format!("socks5h://{}:{}", self.bind_address, self.socks_port)
    }

    /// Check if bind address is loopback.
    pub fn is_loopback(&self) -> bool {
        self.bind_address == "127.0.0.1" || self.bind_address == "::1" || self.bind_address == "localhost"
    }

    /// Validate configuration security.
    pub fn validate_security(&self) -> Result<(), String> {
        // If strict loopback is enabled and not binding to loopback
        if self.strict_loopback && !self.is_loopback() {
            if !self.require_auth {
                return Err(
                    "Non-loopback bind requires authentication. Set require_auth=true or use bind_address=127.0.0.1".into()
                );
            }
            if self.auth_username.is_none() || self.auth_password_hash.is_none() {
                return Err(
                    "Authentication enabled but credentials not configured".into()
                );
            }
        }

        // Rate limit sanity check
        if self.rate_limit_rps == 0 {
            return Err("rate_limit_rps must be > 0".into());
        }

        // Max connections sanity check
        if self.max_connections == 0 {
            return Err("max_connections must be > 0".into());
        }

        Ok(())
    }

    /// Hash a password for storage.
    pub fn hash_password(password: &str) -> String {
        let hash = blake3::hash(password.as_bytes());
        hex::encode(hash.as_bytes())
    }

    /// Verify a password against stored hash.
    pub fn verify_password(&self, password: &str) -> bool {
        if let Some(stored_hash) = &self.auth_password_hash {
            let input_hash = Self::hash_password(password);
            input_hash == *stored_hash
        } else {
            false
        }
    }

    /// Check if a connection from given address should be allowed.
    pub fn should_allow_connection(&self, remote_addr: &str, username: Option<&str>, password: Option<&str>) -> bool {
        // Always allow loopback
        if remote_addr.starts_with("127.") || remote_addr == "::1" {
            return true;
        }

        // Non-loopback requires auth if strict mode
        if self.strict_loopback {
            if !self.require_auth {
                return false;
            }

            // Verify credentials
            match (username, password, &self.auth_username) {
                (Some(u), Some(p), Some(expected_user)) => {
                    u == expected_user && self.verify_password(p)
                }
                _ => false,
            }
        } else {
            // Not strict mode, allow if auth passes or not required
            if !self.require_auth {
                return true;
            }

            match (username, password, &self.auth_username) {
                (Some(u), Some(p), Some(expected_user)) => {
                    u == expected_user && self.verify_password(p)
                }
                _ => false,
            }
        }
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
            max_circuits: 10,
            use_bridges: self.use_bridges,
            bridges: self.bridges.clone(),
            directory_authorities: vec![
                "dirauth1.anyone.io:9030".to_string(),
                "dirauth2.anyone.io:9030".to_string(),
                "dirauth3.anyone.io:9030".to_string(),
            ],
            bootstrap_on_start: true,
            connection_timeout_secs: 30,
            stream_timeout_secs: 60,
        }
    }
}
