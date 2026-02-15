use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Failed,
    Banned,
}

pub struct ConnectionTracker {
    state: ConnectionState,
    connected_at: Option<Instant>,
    disconnected_at: Option<Instant>,
    reconnect_attempts: u32,
    total_connections: u64,
    total_disconnections: u64,
    bytes_sent: u64,
    bytes_received: u64,
}

impl ConnectionTracker {
    pub fn new() -> Self {
        Self {
            state: ConnectionState::Disconnected,
            connected_at: None,
            disconnected_at: None,
            reconnect_attempts: 0,
            total_connections: 0,
            total_disconnections: 0,
            bytes_sent: 0,
            bytes_received: 0,
        }
    }

    pub fn state(&self) -> ConnectionState {
        self.state
    }

    pub fn on_connecting(&mut self) {
        self.state = ConnectionState::Connecting;
    }

    pub fn on_connected(&mut self) {
        self.state = ConnectionState::Connected;
        self.connected_at = Some(Instant::now());
        self.reconnect_attempts = 0;
        self.total_connections += 1;
    }

    pub fn on_disconnected(&mut self) {
        self.state = ConnectionState::Disconnected;
        self.disconnected_at = Some(Instant::now());
        self.total_disconnections += 1;
    }

    pub fn on_reconnecting(&mut self) {
        self.state = ConnectionState::Reconnecting;
        self.reconnect_attempts += 1;
    }

    pub fn on_failed(&mut self) {
        self.state = ConnectionState::Failed;
    }

    pub fn on_banned(&mut self) {
        self.state = ConnectionState::Banned;
    }

    pub fn connection_duration(&self) -> Option<Duration> {
        match (self.state, self.connected_at) {
            (ConnectionState::Connected, Some(at)) => Some(at.elapsed()),
            _ => None,
        }
    }

    pub fn record_bytes(&mut self, sent: u64, received: u64) {
        self.bytes_sent += sent;
        self.bytes_received += received;
    }

    pub fn total_connections(&self) -> u64 {
        self.total_connections
    }

    pub fn total_disconnections(&self) -> u64 {
        self.total_disconnections
    }
}

impl Default for ConnectionTracker {
    fn default() -> Self {
        Self::new()
    }
}
