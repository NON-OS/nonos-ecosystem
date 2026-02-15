use serde::{Deserialize, Serialize};
use super::types::UpdateChannel;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct SecurityConfig {
    pub auto_update: bool,
    pub update_channel: UpdateChannel,
    pub encrypted_storage: bool,
    pub api_auth_required: bool,
    #[serde(skip_serializing)]
    pub api_auth_token: Option<String>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            auto_update: false,
            update_channel: UpdateChannel::Stable,
            encrypted_storage: false,
            api_auth_required: true,
            api_auth_token: None,
        }
    }
}
