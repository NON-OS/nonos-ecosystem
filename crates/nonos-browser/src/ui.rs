use nonos_types::{ConnectionStatus, SecurityLevel, TabInfo};

#[derive(Clone, Debug, Default)]
pub struct UiState {
    pub address_bar: String,
    pub address_bar_focused: bool,
    pub tabs: Vec<TabInfo>,
    pub active_tab_index: usize,
    pub sidebar_visible: bool,
    pub wallet_panel_visible: bool,
    pub settings_visible: bool,
    pub connection_status: ConnectionStatus,
    pub security_level: SecurityLevel,
}

#[derive(Clone, Debug, Default)]
pub struct WalletUiState {
    pub unlocked: bool,
    pub address: String,
    pub eth_balance: String,
    pub nox_balance: String,
    pub staked_nox: String,
    pub pending_rewards: String,
    pub view: WalletView,
    pub send_form: SendFormState,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WalletView {
    #[default]
    Overview,
    Send,
    Receive,
    Staking,
    History,
    Settings,
}

#[derive(Clone, Debug, Default)]
pub struct SendFormState {
    pub recipient: String,
    pub amount: String,
    pub token: String,
    pub is_private: bool,
    pub error: Option<String>,
    pub submitting: bool,
}

#[derive(Clone, Debug)]
pub struct ConnectionIndicator {
    pub status: ConnectionStatus,
    pub progress: u8,
    pub circuits: u32,
    pub tooltip: String,
}

impl ConnectionIndicator {
    pub fn from_status(status: ConnectionStatus, progress: u8, circuits: u32) -> Self {
        let tooltip = match status {
            ConnectionStatus::Disconnected => "Disconnected - Not private".to_string(),
            ConnectionStatus::Connecting => "Connecting to Anyone network...".to_string(),
            ConnectionStatus::Bootstrapping => format!("Bootstrapping... {}%", progress),
            ConnectionStatus::Connected => format!("Connected - {} active circuits", circuits),
            ConnectionStatus::Error => "Connection error".to_string(),
        };

        Self {
            status,
            progress,
            circuits,
            tooltip,
        }
    }

    pub fn color(&self) -> &'static str {
        match self.status {
            ConnectionStatus::Disconnected => "#ef4444",
            ConnectionStatus::Connecting => "#eab308",
            ConnectionStatus::Bootstrapping => "#eab308",
            ConnectionStatus::Connected => "#00ffff",
            ConnectionStatus::Error => "#ef4444",
        }
    }
}

#[derive(Clone, Debug)]
pub struct SecuritySelector {
    pub current: SecurityLevel,
    pub levels: Vec<SecurityLevelInfo>,
}

#[derive(Clone, Debug)]
pub struct SecurityLevelInfo {
    pub level: SecurityLevel,
    pub name: String,
    pub description: String,
    pub warning: String,
}

impl SecuritySelector {
    pub fn new(current: SecurityLevel) -> Self {
        Self {
            current,
            levels: vec![
                SecurityLevelInfo {
                    level: SecurityLevel::Standard,
                    name: "Standard".to_string(),
                    description: "Most websites work normally".to_string(),
                    warning: "Some fingerprinting possible".to_string(),
                },
                SecurityLevelInfo {
                    level: SecurityLevel::Safer,
                    name: "Safer".to_string(),
                    description: "JavaScript restricted on non-HTTPS".to_string(),
                    warning: "Some sites may break".to_string(),
                },
                SecurityLevelInfo {
                    level: SecurityLevel::Safest,
                    name: "Safest".to_string(),
                    description: "JavaScript disabled, fonts blocked".to_string(),
                    warning: "Many sites will break".to_string(),
                },
            ],
        }
    }
}

#[derive(Clone, Debug)]
pub struct CircuitDisplay {
    pub id: String,
    pub path: Vec<RelayDisplay>,
    pub age_secs: u64,
    pub is_current: bool,
}

#[derive(Clone, Debug)]
pub struct RelayDisplay {
    pub name: String,
    pub country: Option<String>,
    pub flag: Option<String>,
    pub role: String,
}

#[derive(Clone, Debug)]
pub struct Theme {
    pub background: String,
    pub surface: String,
    pub text: String,
    pub accent: String,
    pub error: String,
    pub success: String,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            background: "#0a0a0a".to_string(),
            surface: "#1a1a1a".to_string(),
            text: "#ffffff".to_string(),
            accent: "#00ffff".to_string(),
            error: "#ef4444".to_string(),
            success: "#22c55e".to_string(),
        }
    }
}

impl Theme {
    pub fn light() -> Self {
        Self {
            background: "#ffffff".to_string(),
            surface: "#f5f5f5".to_string(),
            text: "#000000".to_string(),
            accent: "#0891b2".to_string(),
            error: "#dc2626".to_string(),
            success: "#16a34a".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_indicator() {
        let indicator = ConnectionIndicator::from_status(ConnectionStatus::Connected, 100, 3);
        assert_eq!(indicator.color(), "#00ffff");
        assert!(indicator.tooltip.contains("3"));
    }

    #[test]
    fn test_security_selector() {
        let selector = SecuritySelector::new(SecurityLevel::Safer);
        assert_eq!(selector.levels.len(), 3);
    }

    #[test]
    fn test_theme() {
        let dark = Theme::default();
        let light = Theme::light();

        assert_ne!(dark.background, light.background);
    }
}
