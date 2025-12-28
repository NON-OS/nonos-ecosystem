// NONOS GNU AFFERO GENERAL PUBLIC LICENSE
// Version 3, 19 November 2007
// Copyright (C) 2025 NON-OS <team@nonos.systems>

use libp2p::{Multiaddr, PeerId};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

#[derive(Debug)]
pub enum NetworkCommand {
    Connect(Multiaddr),
    Disconnect(PeerId),
    Publish { topic: String, data: Vec<u8> },
    Subscribe(String),
    Unsubscribe(String),
    AddAddress(PeerId, Multiaddr),
    Bootstrap,
    Shutdown,
    BanPeer(PeerId, Duration),
    UnbanPeer(PeerId),
    SetRateLimit { messages_per_sec: u32, bytes_per_sec: u64 },
}

#[derive(Debug, Clone)]
pub enum NetworkEvent {
    PeerConnected(PeerId),
    PeerDisconnected(PeerId),
    Message { topic: String, source: PeerId, data: Vec<u8> },
    PeerDiscovered(PeerId, Vec<Multiaddr>),
    PingResult { peer: PeerId, rtt: Duration },
    Error(String),
    RateLimited { peer: PeerId, reason: RateLimitReason },
    PeerBanned { peer: PeerId, until: i64 },
    CircuitOpen { peer: PeerId },
    CircuitClosed { peer: PeerId },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitReason {
    TooManyMessages,
    TooManyBytes,
    InvalidMessage,
    SpamDetected,
    OversizedMessage,
    MalformedMessage,
    DecodeError,
    UnexpectedMessageType,
}

/// Message violation types for penalty scoring
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageViolation {
    /// Message exceeded maximum size limit
    OversizedMessage,
    /// Message failed to decode properly
    DecodeFailure,
    /// Message had invalid or malformed content
    MalformedContent,
    /// Unexpected message type for the context
    UnexpectedType,
    /// Rate limit exceeded
    RateLimitExceeded,
    /// Spam behavior detected
    SpamBehavior,
    /// Protocol violation
    ProtocolViolation,
}

impl MessageViolation {
    /// Get the base penalty score for this violation type
    pub fn penalty_score(&self) -> i32 {
        match self {
            Self::OversizedMessage => 15,
            Self::DecodeFailure => 10,
            Self::MalformedContent => 20,
            Self::UnexpectedType => 5,
            Self::RateLimitExceeded => 10,
            Self::SpamBehavior => 25,
            Self::ProtocolViolation => 30,
        }
    }

    /// Get the ban duration multiplier for repeated offenses
    pub fn ban_multiplier(&self) -> u32 {
        match self {
            Self::OversizedMessage => 2,
            Self::DecodeFailure => 1,
            Self::MalformedContent => 3,
            Self::UnexpectedType => 1,
            Self::RateLimitExceeded => 2,
            Self::SpamBehavior => 4,
            Self::ProtocolViolation => 5,
        }
    }

    /// Check if this violation should trigger an immediate ban
    pub fn immediate_ban(&self) -> bool {
        matches!(self, Self::ProtocolViolation | Self::SpamBehavior)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PeerInfo {
    pub id: String,
    pub addresses: Vec<String>,
    pub connected_at: chrono::DateTime<chrono::Utc>,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub latency_ms: Option<u32>,
    pub protocol_version: Option<String>,
    pub agent_version: Option<String>,
    pub reputation: i32,
    pub message_count: u64,
    pub last_message_at: Option<i64>,
    pub failure_count: u32,
    pub is_banned: bool,
    pub ban_expires_at: Option<i64>,
    /// Violation counts by type
    #[serde(default)]
    pub violation_counts: ViolationCounts,
    /// Total penalty score accumulated
    #[serde(default)]
    pub penalty_score: i32,
    /// Timestamp of last violation
    #[serde(default)]
    pub last_violation_at: Option<i64>,
}

/// Counts of different violation types
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ViolationCounts {
    pub oversized_messages: u32,
    pub decode_failures: u32,
    pub malformed_content: u32,
    pub unexpected_types: u32,
    pub rate_limit_exceeded: u32,
    pub spam_behavior: u32,
    pub protocol_violations: u32,
}

impl ViolationCounts {
    /// Increment count for a specific violation type
    pub fn record(&mut self, violation: MessageViolation) {
        match violation {
            MessageViolation::OversizedMessage => self.oversized_messages += 1,
            MessageViolation::DecodeFailure => self.decode_failures += 1,
            MessageViolation::MalformedContent => self.malformed_content += 1,
            MessageViolation::UnexpectedType => self.unexpected_types += 1,
            MessageViolation::RateLimitExceeded => self.rate_limit_exceeded += 1,
            MessageViolation::SpamBehavior => self.spam_behavior += 1,
            MessageViolation::ProtocolViolation => self.protocol_violations += 1,
        }
    }

    /// Get total violation count
    pub fn total(&self) -> u32 {
        self.oversized_messages
            + self.decode_failures
            + self.malformed_content
            + self.unexpected_types
            + self.rate_limit_exceeded
            + self.spam_behavior
            + self.protocol_violations
    }

    /// Check if peer has had repeated violations
    pub fn is_repeat_offender(&self) -> bool {
        self.total() >= 5
    }
}

impl Default for PeerInfo {
    fn default() -> Self {
        Self {
            id: String::new(),
            addresses: Vec::new(),
            connected_at: chrono::Utc::now(),
            bytes_sent: 0,
            bytes_received: 0,
            latency_ms: None,
            protocol_version: None,
            agent_version: None,
            reputation: 50,
            message_count: 0,
            last_message_at: None,
            failure_count: 0,
            is_banned: false,
            ban_expires_at: None,
            violation_counts: ViolationCounts::default(),
            penalty_score: 0,
            last_violation_at: None,
        }
    }
}

impl PeerInfo {
    pub fn new(peer_id: PeerId, addresses: Vec<String>) -> Self {
        Self {
            id: peer_id.to_string(),
            addresses,
            connected_at: chrono::Utc::now(),
            ..Default::default()
        }
    }

    pub fn record_message(&mut self, bytes: u64) {
        self.message_count += 1;
        self.bytes_received += bytes;
        self.last_message_at = Some(chrono::Utc::now().timestamp());
    }

    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.reputation = (self.reputation - 5).max(-100);
    }

    pub fn record_success(&mut self) {
        self.failure_count = 0;
        self.reputation = (self.reputation + 1).min(100);
    }

    pub fn is_trustworthy(&self) -> bool {
        self.reputation >= 0 && !self.is_banned
    }

    /// Record a message violation and apply penalty
    pub fn record_violation(&mut self, violation: MessageViolation) {
        self.violation_counts.record(violation);
        self.last_violation_at = Some(chrono::Utc::now().timestamp());

        // Calculate penalty with multiplier for repeat offenders
        let base_penalty = violation.penalty_score();
        let multiplier = if self.violation_counts.is_repeat_offender() {
            violation.ban_multiplier() as i32
        } else {
            1
        };

        let penalty = base_penalty * multiplier;
        self.penalty_score += penalty;
        self.reputation = (self.reputation - penalty).max(-100);

        // Record as failure too
        self.failure_count += 1;
    }

    /// Check if peer should be banned based on violations
    pub fn should_ban(&self) -> bool {
        // Ban if reputation is too low
        if self.reputation <= -50 {
            return true;
        }

        // Ban if too many violations
        if self.violation_counts.total() >= 10 {
            return true;
        }

        // Ban if high penalty score
        if self.penalty_score >= 100 {
            return true;
        }

        false
    }

    /// Get recommended ban duration based on violation history
    pub fn recommended_ban_duration(&self) -> Duration {
        let base_seconds = 300u64; // 5 minutes base

        // Increase based on violation severity
        let multiplier = if self.violation_counts.protocol_violations > 0 {
            20 // Protocol violations get 100 minute ban
        } else if self.violation_counts.spam_behavior > 0 {
            12 // Spam gets 60 minute ban
        } else if self.violation_counts.is_repeat_offender() {
            6 // Repeat offenders get 30 minute ban
        } else {
            1 // First offense gets 5 minute ban
        };

        Duration::from_secs(base_seconds * multiplier)
    }

    /// Reset violation counters (e.g., after a cooldown period)
    pub fn reset_violations(&mut self) {
        self.violation_counts = ViolationCounts::default();
        self.penalty_score = 0;
        self.last_violation_at = None;
    }
}

pub struct NetworkStats {
    pub peer_count: AtomicU64,
    pub bytes_sent: AtomicU64,
    pub bytes_received: AtomicU64,
    pub messages_published: AtomicU64,
    pub messages_received: AtomicU64,
    pub messages_dropped: AtomicU64,
    pub active_topics: AtomicU64,
    pub connection_attempts: AtomicU64,
    pub connection_failures: AtomicU64,
    pub rate_limit_hits: AtomicU64,
    pub banned_peers: AtomicU64,
    pub circuit_breaker_trips: AtomicU64,
    started_at: Instant,
}

impl Default for NetworkStats {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkStats {
    pub fn new() -> Self {
        Self {
            peer_count: AtomicU64::new(0),
            bytes_sent: AtomicU64::new(0),
            bytes_received: AtomicU64::new(0),
            messages_published: AtomicU64::new(0),
            messages_received: AtomicU64::new(0),
            messages_dropped: AtomicU64::new(0),
            active_topics: AtomicU64::new(0),
            connection_attempts: AtomicU64::new(0),
            connection_failures: AtomicU64::new(0),
            rate_limit_hits: AtomicU64::new(0),
            banned_peers: AtomicU64::new(0),
            circuit_breaker_trips: AtomicU64::new(0),
            started_at: Instant::now(),
        }
    }

    pub fn uptime(&self) -> Duration {
        self.started_at.elapsed()
    }

    pub fn snapshot(&self) -> NetworkStatsSnapshot {
        NetworkStatsSnapshot {
            peer_count: self.peer_count.load(Ordering::Relaxed),
            bytes_sent: self.bytes_sent.load(Ordering::Relaxed),
            bytes_received: self.bytes_received.load(Ordering::Relaxed),
            messages_published: self.messages_published.load(Ordering::Relaxed),
            messages_received: self.messages_received.load(Ordering::Relaxed),
            messages_dropped: self.messages_dropped.load(Ordering::Relaxed),
            active_topics: self.active_topics.load(Ordering::Relaxed),
            connection_attempts: self.connection_attempts.load(Ordering::Relaxed),
            connection_failures: self.connection_failures.load(Ordering::Relaxed),
            rate_limit_hits: self.rate_limit_hits.load(Ordering::Relaxed),
            banned_peers: self.banned_peers.load(Ordering::Relaxed),
            circuit_breaker_trips: self.circuit_breaker_trips.load(Ordering::Relaxed),
            uptime_secs: self.started_at.elapsed().as_secs(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkStatsSnapshot {
    pub peer_count: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub messages_published: u64,
    pub messages_received: u64,
    pub messages_dropped: u64,
    pub active_topics: u64,
    pub connection_attempts: u64,
    pub connection_failures: u64,
    pub rate_limit_hits: u64,
    pub banned_peers: u64,
    pub circuit_breaker_trips: u64,
    pub uptime_secs: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

pub struct CircuitBreaker {
    state: CircuitState,
    failure_count: u32,
    failure_threshold: u32,
    success_count: u32,
    success_threshold: u32,
    last_failure: Option<Instant>,
    reset_timeout: Duration,
    is_open: AtomicBool,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, success_threshold: u32, reset_timeout: Duration) -> Self {
        Self {
            state: CircuitState::Closed,
            failure_count: 0,
            failure_threshold,
            success_count: 0,
            success_threshold,
            last_failure: None,
            reset_timeout,
            is_open: AtomicBool::new(false),
        }
    }

    pub fn is_open(&self) -> bool {
        self.is_open.load(Ordering::Relaxed)
    }

    pub fn state(&self) -> CircuitState {
        self.state
    }

    pub fn should_allow(&mut self) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                if let Some(last_failure) = self.last_failure {
                    if last_failure.elapsed() >= self.reset_timeout {
                        self.state = CircuitState::HalfOpen;
                        self.is_open.store(false, Ordering::Relaxed);
                        return true;
                    }
                }
                false
            }
            CircuitState::HalfOpen => true,
        }
    }

    pub fn record_success(&mut self) {
        match self.state {
            CircuitState::Closed => {
                self.failure_count = 0;
            }
            CircuitState::HalfOpen => {
                self.success_count += 1;
                if self.success_count >= self.success_threshold {
                    self.state = CircuitState::Closed;
                    self.is_open.store(false, Ordering::Relaxed);
                    self.failure_count = 0;
                    self.success_count = 0;
                }
            }
            CircuitState::Open => {}
        }
    }

    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.last_failure = Some(Instant::now());

        match self.state {
            CircuitState::Closed => {
                if self.failure_count >= self.failure_threshold {
                    self.state = CircuitState::Open;
                    self.is_open.store(true, Ordering::Relaxed);
                }
            }
            CircuitState::HalfOpen => {
                self.state = CircuitState::Open;
                self.is_open.store(true, Ordering::Relaxed);
                self.success_count = 0;
            }
            CircuitState::Open => {}
        }
    }

    pub fn reset(&mut self) {
        self.state = CircuitState::Closed;
        self.is_open.store(false, Ordering::Relaxed);
        self.failure_count = 0;
        self.success_count = 0;
        self.last_failure = None;
    }
}

pub struct RateLimiter {
    messages_per_sec: u32,
    bytes_per_sec: u64,
    message_tokens: f64,
    byte_tokens: f64,
    max_message_burst: u32,
    max_byte_burst: u64,
    last_update: Instant,
}

impl RateLimiter {
    pub fn new(messages_per_sec: u32, bytes_per_sec: u64) -> Self {
        let max_message_burst = messages_per_sec.saturating_mul(2);
        let max_byte_burst = bytes_per_sec.saturating_mul(2);

        Self {
            messages_per_sec,
            bytes_per_sec,
            message_tokens: max_message_burst as f64,
            byte_tokens: max_byte_burst as f64,
            max_message_burst,
            max_byte_burst,
            last_update: Instant::now(),
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update).as_secs_f64();
        self.last_update = now;

        self.message_tokens = (self.message_tokens + elapsed * self.messages_per_sec as f64)
            .min(self.max_message_burst as f64);

        self.byte_tokens = (self.byte_tokens + elapsed * self.bytes_per_sec as f64)
            .min(self.max_byte_burst as f64);
    }

    pub fn check_message(&mut self, bytes: u64) -> Result<(), RateLimitReason> {
        self.refill();

        if self.message_tokens < 1.0 {
            return Err(RateLimitReason::TooManyMessages);
        }

        if (self.byte_tokens as u64) < bytes {
            return Err(RateLimitReason::TooManyBytes);
        }

        self.message_tokens -= 1.0;
        self.byte_tokens -= bytes as f64;

        Ok(())
    }

    pub fn update_limits(&mut self, messages_per_sec: u32, bytes_per_sec: u64) {
        self.messages_per_sec = messages_per_sec;
        self.bytes_per_sec = bytes_per_sec;
        self.max_message_burst = messages_per_sec.saturating_mul(2);
        self.max_byte_burst = bytes_per_sec.saturating_mul(2);
    }

    pub fn available_messages(&self) -> u32 {
        self.message_tokens as u32
    }

    pub fn available_bytes(&self) -> u64 {
        self.byte_tokens as u64
    }
}

pub struct BackoffStrategy {
    base_delay: Duration,
    max_delay: Duration,
    multiplier: f64,
    current_delay: Duration,
    attempt: u32,
    max_attempts: Option<u32>,
    jitter: bool,
}

impl BackoffStrategy {
    pub fn exponential(base: Duration, max: Duration) -> Self {
        Self {
            base_delay: base,
            max_delay: max,
            multiplier: 2.0,
            current_delay: base,
            attempt: 0,
            max_attempts: None,
            jitter: true,
        }
    }

    pub fn with_max_attempts(mut self, max: u32) -> Self {
        self.max_attempts = Some(max);
        self
    }

    pub fn with_jitter(mut self, enabled: bool) -> Self {
        self.jitter = enabled;
        self
    }

    pub fn next_delay(&mut self) -> Option<Duration> {
        if let Some(max) = self.max_attempts {
            if self.attempt >= max {
                return None;
            }
        }

        self.attempt += 1;

        let delay = if self.attempt == 1 {
            self.base_delay
        } else {
            let multiplied = self.current_delay.as_secs_f64() * self.multiplier;
            let capped = multiplied.min(self.max_delay.as_secs_f64());
            Duration::from_secs_f64(capped)
        };

        self.current_delay = delay;

        let final_delay = if self.jitter {
            let jitter_factor = 0.5 + rand::random::<f64>() * 0.5;
            Duration::from_secs_f64(delay.as_secs_f64() * jitter_factor)
        } else {
            delay
        };

        Some(final_delay)
    }

    pub fn reset(&mut self) {
        self.attempt = 0;
        self.current_delay = self.base_delay;
    }

    pub fn attempts(&self) -> u32 {
        self.attempt
    }

    pub fn is_exhausted(&self) -> bool {
        if let Some(max) = self.max_attempts {
            self.attempt >= max
        } else {
            false
        }
    }
}

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
}

impl Default for ConnectionTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BanEntry {
    pub peer_id: String,
    pub banned_at: i64,
    pub expires_at: i64,
    pub reason: String,
}

impl BanEntry {
    pub fn new(peer_id: PeerId, duration: Duration, reason: &str) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            peer_id: peer_id.to_string(),
            banned_at: now,
            expires_at: now + duration.as_secs() as i64,
            reason: reason.to_string(),
        }
    }

    pub fn is_expired(&self) -> bool {
        chrono::Utc::now().timestamp() >= self.expires_at
    }

    pub fn remaining(&self) -> Duration {
        let now = chrono::Utc::now().timestamp();
        if now >= self.expires_at {
            Duration::ZERO
        } else {
            Duration::from_secs((self.expires_at - now) as u64)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_breaker() {
        let mut cb = CircuitBreaker::new(3, 2, Duration::from_secs(5));

        assert!(cb.should_allow());
        assert_eq!(cb.state(), CircuitState::Closed);

        cb.record_failure();
        cb.record_failure();
        assert!(cb.should_allow());

        cb.record_failure();
        assert!(!cb.should_allow());
        assert_eq!(cb.state(), CircuitState::Open);
    }

    #[test]
    fn test_rate_limiter() {
        let mut rl = RateLimiter::new(10, 1000);

        // Burst capacity is 2x rate = 20 message tokens
        for _ in 0..20 {
            assert!(rl.check_message(50).is_ok());
        }

        // 21st message should fail - tokens exhausted
        assert!(matches!(
            rl.check_message(50),
            Err(RateLimitReason::TooManyMessages)
        ));
    }

    #[test]
    fn test_backoff_strategy() {
        let mut backoff = BackoffStrategy::exponential(
            Duration::from_millis(100),
            Duration::from_secs(10),
        )
        .with_max_attempts(5)
        .with_jitter(false);

        let d1 = backoff.next_delay().unwrap();
        assert_eq!(d1, Duration::from_millis(100));

        let d2 = backoff.next_delay().unwrap();
        assert_eq!(d2, Duration::from_millis(200));

        backoff.next_delay();
        backoff.next_delay();
        let d5 = backoff.next_delay();
        assert!(d5.is_some());

        let d6 = backoff.next_delay();
        assert!(d6.is_none());
        assert!(backoff.is_exhausted());
    }

    #[test]
    fn test_connection_tracker() {
        let mut tracker = ConnectionTracker::new();

        assert_eq!(tracker.state(), ConnectionState::Disconnected);

        tracker.on_connecting();
        assert_eq!(tracker.state(), ConnectionState::Connecting);

        tracker.on_connected();
        assert_eq!(tracker.state(), ConnectionState::Connected);
        assert!(tracker.connection_duration().is_some());

        tracker.on_disconnected();
        assert_eq!(tracker.state(), ConnectionState::Disconnected);
        assert_eq!(tracker.total_connections, 1);
        assert_eq!(tracker.total_disconnections, 1);
    }

    #[test]
    fn test_peer_info_reputation() {
        let mut peer = PeerInfo::default();
        assert_eq!(peer.reputation, 50);
        assert!(peer.is_trustworthy());

        // 15 failures at -5 each: 50 - 75 = -25
        for _ in 0..15 {
            peer.record_failure();
        }
        assert_eq!(peer.reputation, -25);
        assert!(!peer.is_trustworthy());

        // 125 successes at +1 each: -25 + 125 = 100 (capped)
        for _ in 0..125 {
            peer.record_success();
        }
        assert!(peer.is_trustworthy());
        assert_eq!(peer.reputation, 100);
    }

    #[test]
    fn test_ban_entry() {
        let peer_id = PeerId::random();
        let ban = BanEntry::new(peer_id, Duration::from_secs(60), "spam");

        assert!(!ban.is_expired());
        assert!(ban.remaining() <= Duration::from_secs(60));
    }
}
