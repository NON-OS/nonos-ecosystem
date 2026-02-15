mod backoff;
mod ban;
mod circuit;
mod commands;
mod connection;
mod peer_info;
mod rate_limit;
mod stats;
mod violations;

pub use backoff::BackoffStrategy;
pub use ban::BanEntry;
pub use circuit::{CircuitBreaker, CircuitState};
pub use commands::{NetworkCommand, NetworkEvent};
pub use connection::{ConnectionState, ConnectionTracker};
pub use peer_info::PeerInfo;
pub use rate_limit::{RateLimitReason, RateLimiter};
pub use stats::{NetworkStats, NetworkStatsSnapshot};
pub use violations::MessageViolation;

#[cfg(test)]
mod tests {
    use super::*;
    use libp2p::PeerId;
    use std::time::Duration;

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

        for _ in 0..20 {
            assert!(rl.check_message(50).is_ok());
        }

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
        assert_eq!(tracker.total_connections(), 1);
        assert_eq!(tracker.total_disconnections(), 1);
    }

    #[test]
    fn test_peer_info_reputation() {
        let mut peer = PeerInfo::default();
        assert_eq!(peer.reputation, 50);
        assert!(peer.is_trustworthy());

        for _ in 0..15 {
            peer.record_failure();
        }
        assert_eq!(peer.reputation, -25);
        assert!(!peer.is_trustworthy());

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
