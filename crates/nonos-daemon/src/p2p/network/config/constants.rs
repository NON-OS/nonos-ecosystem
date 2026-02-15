use std::time::Duration;

pub const PROTOCOL_VERSION: &str = "/nonos/1.0.0";
pub const DEFAULT_MESSAGES_PER_SEC: u32 = 100;
pub const DEFAULT_BYTES_PER_SEC: u64 = 1024 * 1024;
pub const CIRCUIT_BREAKER_FAILURE_THRESHOLD: u32 = 5;
pub const CIRCUIT_BREAKER_SUCCESS_THRESHOLD: u32 = 3;
pub const CIRCUIT_BREAKER_RESET_TIMEOUT: Duration = Duration::from_secs(30);
pub const MAX_MESSAGE_SIZE: usize = 64 * 1024;
pub const MAX_PEERS: usize = 50;
pub const MIN_PEERS: usize = 3;
