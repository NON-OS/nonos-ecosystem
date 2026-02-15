use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use super::types::LogLevel;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct LoggingConfig {
    pub level: LogLevel,
    pub file: Option<PathBuf>,
    pub json: bool,
    pub timestamps: bool,
    pub source_location: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            file: None,
            json: false,
            timestamps: true,
            source_location: false,
        }
    }
}
