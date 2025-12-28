// NONOS GNU AFFERO GENERAL PUBLIC LICENSE
// Version 3, 19 November 2007
// Copyright (C) 2025 NON-OS <team@nonos.systems>

//! API Middleware
//!
//! Provides security and rate limiting middleware for the HTTP API:
//! - Bearer token authentication
//! - Per-IP rate limiting with token bucket algorithm
//! - Request validation

use parking_lot::RwLock;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Instant;

/// Rate limiter for API requests using token bucket algorithm
pub struct ApiRateLimiter {
    /// Tokens per second
    tokens_per_sec: f64,
    /// Maximum burst size
    burst_size: u32,
    /// Per-IP state
    state: RwLock<HashMap<IpAddr, TokenBucket>>,
    /// Global limiter for all requests
    global: RwLock<TokenBucket>,
}

struct TokenBucket {
    tokens: f64,
    last_update: Instant,
}

impl TokenBucket {
    fn new(initial: f64) -> Self {
        Self {
            tokens: initial,
            last_update: Instant::now(),
        }
    }

    fn refill(&mut self, rate: f64, max: f64) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update).as_secs_f64();
        self.last_update = now;
        self.tokens = (self.tokens + elapsed * rate).min(max);
    }

    fn try_consume(&mut self, rate: f64, max: f64) -> bool {
        self.refill(rate, max);
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

impl ApiRateLimiter {
    /// Create a new API rate limiter
    pub fn new(requests_per_second: u32, burst_size: u32) -> Self {
        Self {
            tokens_per_sec: requests_per_second as f64,
            burst_size,
            state: RwLock::new(HashMap::new()),
            global: RwLock::new(TokenBucket::new(burst_size as f64)),
        }
    }

    /// Check if a request from the given IP is allowed
    pub fn check_request(&self, ip: IpAddr) -> RateLimitResult {
        // Check global limit first
        {
            let mut global = self.global.write();
            if !global.try_consume(self.tokens_per_sec * 10.0, self.burst_size as f64 * 10.0) {
                return RateLimitResult::GlobalLimitExceeded;
            }
        }

        // Check per-IP limit
        let mut state = self.state.write();
        let bucket = state
            .entry(ip)
            .or_insert_with(|| TokenBucket::new(self.burst_size as f64));

        if bucket.try_consume(self.tokens_per_sec, self.burst_size as f64) {
            RateLimitResult::Allowed
        } else {
            RateLimitResult::IpLimitExceeded
        }
    }

    /// Clean up stale entries (call periodically)
    pub fn cleanup(&self) {
        let mut state = self.state.write();
        let now = Instant::now();
        state.retain(|_, bucket| {
            // Remove entries older than 5 minutes
            now.duration_since(bucket.last_update).as_secs() < 300
        });
    }

    /// Get current stats
    pub fn stats(&self) -> RateLimiterStats {
        let state = self.state.read();
        RateLimiterStats {
            tracked_ips: state.len(),
            global_tokens_available: self.global.read().tokens as u32,
        }
    }
}

impl Default for ApiRateLimiter {
    fn default() -> Self {
        Self::new(100, 200)
    }
}

/// Result of rate limit check
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitResult {
    /// Request is allowed
    Allowed,
    /// Per-IP limit exceeded
    IpLimitExceeded,
    /// Global limit exceeded
    GlobalLimitExceeded,
}

/// Rate limiter statistics
#[derive(Debug, Clone)]
pub struct RateLimiterStats {
    pub tracked_ips: usize,
    pub global_tokens_available: u32,
}

/// Authentication result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthResult {
    /// Authentication succeeded
    Authenticated,
    /// No authentication required for this endpoint
    NotRequired,
    /// Missing Authorization header
    MissingToken,
    /// Invalid token format
    InvalidFormat,
    /// Token doesn't match
    InvalidToken,
}

/// API authentication middleware
pub struct ApiAuthenticator {
    /// Expected token (if authentication is enabled)
    token: Option<String>,
    /// Paths that don't require authentication
    public_paths: Vec<&'static str>,
}

impl ApiAuthenticator {
    /// Create a new authenticator
    pub fn new(token: Option<String>) -> Self {
        Self {
            token,
            public_paths: vec![
                "/api/health",
                "/api/metrics/prometheus",
                "/",
            ],
        }
    }

    /// Check if authentication is required and valid
    pub fn authenticate(&self, path: &str, auth_header: Option<&str>) -> AuthResult {
        // Check if authentication is enabled
        let expected_token = match &self.token {
            Some(t) if !t.is_empty() => t,
            _ => return AuthResult::NotRequired,
        };

        // Check if path is public
        if self.is_public_path(path) {
            return AuthResult::NotRequired;
        }

        // Check for Authorization header
        let auth_header = match auth_header {
            Some(h) => h,
            None => return AuthResult::MissingToken,
        };

        // Parse Bearer token
        if !auth_header.starts_with("Bearer ") {
            return AuthResult::InvalidFormat;
        }

        let provided_token = &auth_header[7..];

        // Constant-time comparison to prevent timing attacks
        if constant_time_compare(provided_token, expected_token) {
            AuthResult::Authenticated
        } else {
            AuthResult::InvalidToken
        }
    }

    /// Check if a path is public (no auth required)
    fn is_public_path(&self, path: &str) -> bool {
        self.public_paths.iter().any(|&p| path == p || path.starts_with(p))
    }

    /// Add a public path
    pub fn add_public_path(&mut self, path: &'static str) {
        self.public_paths.push(path);
    }
}

/// Constant-time string comparison to prevent timing attacks
fn constant_time_compare(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result = 0u8;
    for (x, y) in a.bytes().zip(b.bytes()) {
        result |= x ^ y;
    }
    result == 0
}

/// HTTP headers extracted from request
#[derive(Debug, Default)]
pub struct RequestHeaders {
    pub authorization: Option<String>,
    pub content_length: Option<usize>,
    pub content_type: Option<String>,
    pub x_forwarded_for: Option<String>,
    pub x_real_ip: Option<String>,
}

impl RequestHeaders {
    /// Parse headers from raw header lines
    pub fn parse(lines: &[String]) -> Self {
        let mut headers = Self::default();

        for line in lines {
            let line = line.trim();
            if let Some(colon_pos) = line.find(':') {
                let name = line[..colon_pos].trim().to_lowercase();
                let value = line[colon_pos + 1..].trim().to_string();

                match name.as_str() {
                    "authorization" => headers.authorization = Some(value),
                    "content-length" => headers.content_length = value.parse().ok(),
                    "content-type" => headers.content_type = Some(value),
                    "x-forwarded-for" => headers.x_forwarded_for = Some(value),
                    "x-real-ip" => headers.x_real_ip = Some(value),
                    _ => {}
                }
            }
        }

        headers
    }

    /// Get the real client IP (considering proxies)
    pub fn real_ip(&self, socket_ip: IpAddr) -> IpAddr {
        // Try X-Forwarded-For first (first IP in chain)
        if let Some(ref xff) = self.x_forwarded_for {
            if let Some(first_ip) = xff.split(',').next() {
                if let Ok(ip) = first_ip.trim().parse() {
                    return ip;
                }
            }
        }

        // Try X-Real-IP
        if let Some(ref xri) = self.x_real_ip {
            if let Ok(ip) = xri.parse() {
                return ip;
            }
        }

        socket_ip
    }
}

/// API middleware context passed to handlers
#[derive(Clone)]
pub struct ApiContext {
    /// Rate limiter
    pub rate_limiter: Arc<ApiRateLimiter>,
    /// Authenticator
    pub authenticator: Arc<ApiAuthenticator>,
}

impl ApiContext {
    /// Create a new API context
    pub fn new(auth_token: Option<String>, requests_per_second: u32, burst_size: u32) -> Self {
        Self {
            rate_limiter: Arc::new(ApiRateLimiter::new(requests_per_second, burst_size)),
            authenticator: Arc::new(ApiAuthenticator::new(auth_token)),
        }
    }

    /// Create with defaults (no auth, default rate limits)
    pub fn default_without_auth() -> Self {
        Self::new(None, 100, 200)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn test_rate_limiter() {
        let limiter = ApiRateLimiter::new(10, 20);
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

        // Should allow burst
        for _ in 0..20 {
            assert_eq!(limiter.check_request(ip), RateLimitResult::Allowed);
        }

        // Should deny after burst exhausted
        assert_eq!(limiter.check_request(ip), RateLimitResult::IpLimitExceeded);
    }

    #[test]
    fn test_authenticator_no_token() {
        let auth = ApiAuthenticator::new(None);
        assert_eq!(
            auth.authenticate("/api/status", None),
            AuthResult::NotRequired
        );
    }

    #[test]
    fn test_authenticator_with_token() {
        let auth = ApiAuthenticator::new(Some("secret123".to_string()));

        // Public path should not require auth
        assert_eq!(
            auth.authenticate("/api/health", None),
            AuthResult::NotRequired
        );

        // Protected path without token
        assert_eq!(
            auth.authenticate("/api/status", None),
            AuthResult::MissingToken
        );

        // Protected path with wrong token
        assert_eq!(
            auth.authenticate("/api/status", Some("Bearer wrong")),
            AuthResult::InvalidToken
        );

        // Protected path with correct token
        assert_eq!(
            auth.authenticate("/api/status", Some("Bearer secret123")),
            AuthResult::Authenticated
        );
    }

    #[test]
    fn test_header_parsing() {
        let lines = vec![
            "Authorization: Bearer token123".to_string(),
            "Content-Length: 42".to_string(),
            "X-Forwarded-For: 1.2.3.4, 5.6.7.8".to_string(),
        ];

        let headers = RequestHeaders::parse(&lines);
        assert_eq!(headers.authorization, Some("Bearer token123".to_string()));
        assert_eq!(headers.content_length, Some(42));
        assert_eq!(
            headers.x_forwarded_for,
            Some("1.2.3.4, 5.6.7.8".to_string())
        );
    }

    #[test]
    fn test_real_ip_extraction() {
        let socket_ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

        let headers = RequestHeaders {
            x_forwarded_for: Some("1.2.3.4, 5.6.7.8".to_string()),
            ..Default::default()
        };
        assert_eq!(
            headers.real_ip(socket_ip),
            IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4))
        );

        let headers = RequestHeaders::default();
        assert_eq!(headers.real_ip(socket_ip), socket_ip);
    }
}
