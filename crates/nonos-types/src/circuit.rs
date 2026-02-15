use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CircuitId(pub [u8; 16]);

impl CircuitId {
    pub fn new() -> Self {
        let mut bytes = [0u8; 16];
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        bytes[..8].copy_from_slice(&nanos.to_le_bytes()[..8]);
        let ptr = &bytes as *const _ as usize;
        bytes[8..16].copy_from_slice(&ptr.to_le_bytes());
        Self(bytes)
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
}

impl Default for CircuitId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for CircuitId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CircuitId({})", self.to_hex())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RelayInfo {
    pub fingerprint: [u8; 20],
    pub nickname: String,
    pub country: Option<String>,
    pub bandwidth: u64,
    pub is_exit: bool,
    pub is_guard: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CircuitStatus {
    Building,
    Ready,
    Active,
    Closing,
    Closed,
    Failed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CircuitInfo {
    pub id: CircuitId,
    pub path: Vec<RelayInfo>,
    pub status: CircuitStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub bytes_sent: u64,
    pub bytes_received: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionStatus {
    #[default]
    Disconnected,
    Connecting,
    Bootstrapping,
    Connected,
    Error,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkStatus {
    pub status: ConnectionStatus,
    pub bootstrap_progress: u8,
    pub active_circuits: u32,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub socks_port: Option<u16>,
}
