use parking_lot::RwLock;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Instant;

pub struct ApiRateLimiter {
    tokens_per_sec: f64,
    burst_size: u32,
    state: RwLock<HashMap<IpAddr, TokenBucket>>,
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
    pub fn new(requests_per_second: u32, burst_size: u32) -> Self {
        Self {
            tokens_per_sec: requests_per_second as f64,
            burst_size,
            state: RwLock::new(HashMap::new()),
            global: RwLock::new(TokenBucket::new(burst_size as f64 * 10.0)),
        }
    }

    pub fn check_request(&self, ip: IpAddr) -> RateLimitResult {
        {
            let mut global = self.global.write();
            if !global.try_consume(self.tokens_per_sec * 10.0, self.burst_size as f64 * 10.0) {
                return RateLimitResult::GlobalLimitExceeded;
            }
        }

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

    pub fn cleanup(&self) {
        let mut state = self.state.write();
        let now = Instant::now();
        state.retain(|_, bucket| {
            now.duration_since(bucket.last_update).as_secs() < 300
        });
    }

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitResult {
    Allowed,
    IpLimitExceeded,
    GlobalLimitExceeded,
}

#[derive(Debug, Clone)]
pub struct RateLimiterStats {
    pub tracked_ips: usize,
    pub global_tokens_available: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthResult {
    Authenticated,
    NotRequired,
    MissingToken,
    InvalidFormat,
    InvalidToken,
}

pub struct ApiAuthenticator {
    token: Option<String>,
    public_paths: Vec<&'static str>,
}

impl ApiAuthenticator {
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

    pub fn authenticate(&self, path: &str, auth_header: Option<&str>) -> AuthResult {
        let expected_token = match &self.token {
            Some(t) if !t.is_empty() => t,
            _ => return AuthResult::NotRequired,
        };

        if self.is_public_path(path) {
            return AuthResult::NotRequired;
        }

        let auth_header = match auth_header {
            Some(h) => h,
            None => return AuthResult::MissingToken,
        };

        if !auth_header.starts_with("Bearer ") {
            return AuthResult::InvalidFormat;
        }

        let provided_token = &auth_header[7..];

        if constant_time_compare(provided_token, expected_token) {
            AuthResult::Authenticated
        } else {
            AuthResult::InvalidToken
        }
    }

    fn is_public_path(&self, path: &str) -> bool {
        self.public_paths.iter().any(|&p| {
            if p == "/" {
                path == "/"
            } else {
                path == p || path.starts_with(&format!("{}/", p))
            }
        })
    }

    pub fn add_public_path(&mut self, path: &'static str) {
        self.public_paths.push(path);
    }
}

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

#[derive(Debug, Clone, Default)]
pub struct TrustedProxies {
    pub proxies: Vec<IpAddr>,
}

impl TrustedProxies {
    pub fn none() -> Self {
        Self { proxies: vec![] }
    }

    pub fn localhost() -> Self {
        use std::net::{Ipv4Addr, Ipv6Addr};
        Self {
            proxies: vec![
                IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)),
            ],
        }
    }

    pub fn is_trusted(&self, ip: &IpAddr) -> bool {
        self.proxies.contains(ip)
    }
}

#[derive(Debug, Default)]
pub struct RequestHeaders {
    pub authorization: Option<String>,
    pub content_length: Option<usize>,
    pub content_type: Option<String>,
    pub x_forwarded_for: Option<String>,
    pub x_real_ip: Option<String>,
}

impl RequestHeaders {
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

    pub fn real_ip(&self, socket_ip: IpAddr, trusted_proxies: &TrustedProxies) -> IpAddr {
        if !trusted_proxies.is_trusted(&socket_ip) {
            return socket_ip;
        }

        if let Some(ref xff) = self.x_forwarded_for {
            if let Some(first_ip) = xff.split(',').next() {
                if let Ok(ip) = first_ip.trim().parse() {
                    return ip;
                }
            }
        }

        if let Some(ref xri) = self.x_real_ip {
            if let Ok(ip) = xri.parse() {
                return ip;
            }
        }

        socket_ip
    }

    pub fn real_ip_unsafe(&self, socket_ip: IpAddr) -> IpAddr {
        if let Some(ref xff) = self.x_forwarded_for {
            if let Some(first_ip) = xff.split(',').next() {
                if let Ok(ip) = first_ip.trim().parse() {
                    return ip;
                }
            }
        }

        if let Some(ref xri) = self.x_real_ip {
            if let Ok(ip) = xri.parse() {
                return ip;
            }
        }

        socket_ip
    }
}

#[derive(Clone)]
pub struct ApiContext {
    pub rate_limiter: Arc<ApiRateLimiter>,
    pub authenticator: Arc<ApiAuthenticator>,
    pub trusted_proxies: Arc<TrustedProxies>,
    pub auth_explicitly_disabled: bool,
}

impl ApiContext {
    pub fn new(auth_token: Option<String>, requests_per_second: u32, burst_size: u32) -> Self {
        Self {
            rate_limiter: Arc::new(ApiRateLimiter::new(requests_per_second, burst_size)),
            authenticator: Arc::new(ApiAuthenticator::new(auth_token)),
            trusted_proxies: Arc::new(TrustedProxies::none()),
            auth_explicitly_disabled: false,
        }
    }

    pub fn with_trusted_proxies(
        auth_token: Option<String>,
        requests_per_second: u32,
        burst_size: u32,
        trusted_proxies: TrustedProxies,
    ) -> Self {
        Self {
            rate_limiter: Arc::new(ApiRateLimiter::new(requests_per_second, burst_size)),
            authenticator: Arc::new(ApiAuthenticator::new(auth_token)),
            trusted_proxies: Arc::new(trusted_proxies),
            auth_explicitly_disabled: false,
        }
    }

    pub fn insecure_without_auth() -> Self {
        Self {
            rate_limiter: Arc::new(ApiRateLimiter::new(100, 200)),
            authenticator: Arc::new(ApiAuthenticator::new(None)),
            trusted_proxies: Arc::new(TrustedProxies::none()),
            auth_explicitly_disabled: true,
        }
    }

    pub fn with_generated_token(requests_per_second: u32, burst_size: u32) -> (Self, String) {
        let token = generate_random_token();
        let ctx = Self {
            rate_limiter: Arc::new(ApiRateLimiter::new(requests_per_second, burst_size)),
            authenticator: Arc::new(ApiAuthenticator::new(Some(token.clone()))),
            trusted_proxies: Arc::new(TrustedProxies::none()),
            auth_explicitly_disabled: false,
        };
        (ctx, token)
    }

    pub fn is_auth_enabled(&self) -> bool {
        !self.auth_explicitly_disabled && self.authenticator.token.is_some()
    }

    #[deprecated(note = "Use insecure_without_auth() to make security implications explicit")]
    pub fn default_without_auth() -> Self {
        Self::insecure_without_auth()
    }
}

fn generate_random_token() -> String {
    use nonos_crypto::random_bytes;
    let bytes: [u8; 32] = random_bytes();
    hex::encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn test_rate_limiter() {
        let limiter = ApiRateLimiter::new(10, 20);
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

        for _ in 0..20 {
            assert_eq!(limiter.check_request(ip), RateLimitResult::Allowed);
        }

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

        assert_eq!(
            auth.authenticate("/api/health", None),
            AuthResult::NotRequired
        );

        assert_eq!(
            auth.authenticate("/api/status", None),
            AuthResult::MissingToken
        );

        assert_eq!(
            auth.authenticate("/api/status", Some("Bearer wrong")),
            AuthResult::InvalidToken
        );

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
    fn test_real_ip_extraction_with_trusted_proxy() {
        let localhost = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        let untrusted_ip = IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8));
        let trusted = TrustedProxies::localhost();
        let no_trust = TrustedProxies::none();

        let headers = RequestHeaders {
            x_forwarded_for: Some("1.2.3.4, 5.6.7.8".to_string()),
            ..Default::default()
        };

        assert_eq!(
            headers.real_ip(localhost, &trusted),
            IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4))
        );

        assert_eq!(
            headers.real_ip(untrusted_ip, &trusted),
            untrusted_ip
        );

        assert_eq!(
            headers.real_ip(localhost, &no_trust),
            localhost
        );
    }

    #[test]
    fn test_real_ip_unsafe_legacy() {
        let socket_ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

        let headers = RequestHeaders {
            x_forwarded_for: Some("1.2.3.4, 5.6.7.8".to_string()),
            ..Default::default()
        };

        assert_eq!(
            headers.real_ip_unsafe(socket_ip),
            IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4))
        );

        let headers = RequestHeaders::default();
        assert_eq!(headers.real_ip_unsafe(socket_ip), socket_ip);
    }
}
