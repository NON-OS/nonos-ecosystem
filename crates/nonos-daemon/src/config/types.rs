use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum NodeRole {
    #[default]
    Local,
    Relay,
    Backbone,
}

impl NodeRole {
    pub fn max_peers(&self) -> u32 {
        match self {
            NodeRole::Local => 25,
            NodeRole::Relay => 100,
            NodeRole::Backbone => 500,
        }
    }

    pub fn min_peers(&self) -> u32 {
        match self {
            NodeRole::Local => 3,
            NodeRole::Relay => 10,
            NodeRole::Backbone => 25,
        }
    }

    pub fn serves_bootstrap(&self) -> bool {
        matches!(self, NodeRole::Relay | NodeRole::Backbone)
    }

    pub fn high_availability(&self) -> bool {
        matches!(self, NodeRole::Backbone)
    }
}

impl std::fmt::Display for NodeRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeRole::Local => write!(f, "local"),
            NodeRole::Relay => write!(f, "relay"),
            NodeRole::Backbone => write!(f, "backbone"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum BootstrapMode {
    #[default]
    Official,
    Custom,
    None,
}

impl std::fmt::Display for BootstrapMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BootstrapMode::Official => write!(f, "official"),
            BootstrapMode::Custom => write!(f, "custom"),
            BootstrapMode::None => write!(f, "none"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum UpdateChannel {
    #[default]
    Stable,
    Beta,
    Manual,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LogLevel {
    Error,
    Warn,
    #[default]
    Info,
    Debug,
    Trace,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Error => write!(f, "error"),
            LogLevel::Warn => write!(f, "warn"),
            LogLevel::Info => write!(f, "info"),
            LogLevel::Debug => write!(f, "debug"),
            LogLevel::Trace => write!(f, "trace"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WarningSeverity {
    High,
    Medium,
    Low,
}

#[derive(Clone, Debug)]
pub struct SecurityWarning {
    pub severity: WarningSeverity,
    pub message: String,
    pub recommendation: String,
}
