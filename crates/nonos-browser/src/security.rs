use nonos_types::SecurityLevel;
use std::sync::RwLock;
use tracing::info;

pub struct SecurityManager {
    level: RwLock<SecurityLevel>,
    javascript_enabled: RwLock<bool>,
    webrtc_enabled: RwLock<bool>,
    block_fingerprinting: RwLock<bool>,
    block_third_party_cookies: RwLock<bool>,
}

impl SecurityManager {
    pub fn new() -> Self {
        Self {
            level: RwLock::new(SecurityLevel::Safer),
            javascript_enabled: RwLock::new(true),
            webrtc_enabled: RwLock::new(false),
            block_fingerprinting: RwLock::new(true),
            block_third_party_cookies: RwLock::new(true),
        }
    }

    pub fn level(&self) -> SecurityLevel {
        *self.level.read().unwrap()
    }

    pub fn set_level(&self, level: SecurityLevel) {
        info!("Setting security level to {:?}", level);

        *self.level.write().unwrap() = level;

        match level {
            SecurityLevel::Standard => {
                *self.javascript_enabled.write().unwrap() = true;
                *self.webrtc_enabled.write().unwrap() = false;
                *self.block_fingerprinting.write().unwrap() = true;
            }
            SecurityLevel::Safer => {
                *self.javascript_enabled.write().unwrap() = true;
                *self.webrtc_enabled.write().unwrap() = false;
                *self.block_fingerprinting.write().unwrap() = true;
            }
            SecurityLevel::Safest => {
                *self.javascript_enabled.write().unwrap() = false;
                *self.webrtc_enabled.write().unwrap() = false;
                *self.block_fingerprinting.write().unwrap() = true;
            }
        }
    }

    pub fn javascript_enabled(&self) -> bool {
        *self.javascript_enabled.read().unwrap()
    }

    pub fn webrtc_enabled(&self) -> bool {
        *self.webrtc_enabled.read().unwrap()
    }

    pub fn fingerprinting_blocked(&self) -> bool {
        *self.block_fingerprinting.read().unwrap()
    }

    pub fn summary(&self) -> SecuritySummary {
        SecuritySummary {
            level: self.level(),
            javascript_enabled: self.javascript_enabled(),
            webrtc_enabled: self.webrtc_enabled(),
            fingerprinting_blocked: self.fingerprinting_blocked(),
            third_party_cookies_blocked: *self.block_third_party_cookies.read().unwrap(),
        }
    }
}

impl Default for SecurityManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug)]
pub struct SecuritySummary {
    pub level: SecurityLevel,
    pub javascript_enabled: bool,
    pub webrtc_enabled: bool,
    pub fingerprinting_blocked: bool,
    pub third_party_cookies_blocked: bool,
}

pub struct CspBuilder {
    directives: Vec<(String, Vec<String>)>,
}

impl CspBuilder {
    pub fn new() -> Self {
        Self {
            directives: Vec::new(),
        }
    }

    pub fn directive(mut self, name: &str, values: Vec<&str>) -> Self {
        self.directives
            .push((name.to_string(), values.iter().map(|s| s.to_string()).collect()));
        self
    }

    pub fn default_src(self, values: Vec<&str>) -> Self {
        self.directive("default-src", values)
    }

    pub fn script_src(self, values: Vec<&str>) -> Self {
        self.directive("script-src", values)
    }

    pub fn style_src(self, values: Vec<&str>) -> Self {
        self.directive("style-src", values)
    }

    pub fn img_src(self, values: Vec<&str>) -> Self {
        self.directive("img-src", values)
    }

    pub fn connect_src(self, values: Vec<&str>) -> Self {
        self.directive("connect-src", values)
    }

    pub fn frame_src(self, values: Vec<&str>) -> Self {
        self.directive("frame-src", values)
    }

    pub fn build(&self) -> String {
        self.directives
            .iter()
            .map(|(name, values)| format!("{} {}", name, values.join(" ")))
            .collect::<Vec<_>>()
            .join("; ")
    }

    pub fn strict() -> Self {
        Self::new()
            .default_src(vec!["'self'"])
            .script_src(vec!["'self'"])
            .style_src(vec!["'self'", "'unsafe-inline'"])
            .img_src(vec!["'self'", "data:"])
            .connect_src(vec!["'self'"])
            .frame_src(vec!["'none'"])
    }
}

impl Default for CspBuilder {
    fn default() -> Self {
        Self::new()
    }
}

pub fn privacy_headers() -> Vec<(&'static str, &'static str)> {
    vec![
        ("DNT", "1"),
        ("Sec-GPC", "1"),
        ("Permissions-Policy", "interest-cohort=()"),
        ("X-Content-Type-Options", "nosniff"),
        ("X-Frame-Options", "DENY"),
        ("Referrer-Policy", "no-referrer"),
    ]
}

pub fn is_safe_url(url: &str) -> bool {
    let dangerous_protocols = ["javascript:", "data:", "vbscript:", "file:"];

    let url_lower = url.to_lowercase();
    for protocol in &dangerous_protocols {
        if url_lower.starts_with(protocol) {
            return false;
        }
    }

    let safe_protocols = ["https://", "http://", "about:"];
    for protocol in &safe_protocols {
        if url_lower.starts_with(protocol) {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_levels() {
        let manager = SecurityManager::new();

        assert!(manager.javascript_enabled());

        manager.set_level(SecurityLevel::Safest);
        assert!(!manager.javascript_enabled());
        assert!(!manager.webrtc_enabled());

        manager.set_level(SecurityLevel::Standard);
        assert!(manager.javascript_enabled());
    }

    #[test]
    fn test_csp_builder() {
        let csp = CspBuilder::strict().build();
        assert!(csp.contains("default-src 'self'"));
        assert!(csp.contains("frame-src 'none'"));
    }

    #[test]
    fn test_safe_url() {
        assert!(is_safe_url("https://example.com"));
        assert!(is_safe_url("http://example.com"));
        assert!(is_safe_url("about:blank"));

        assert!(!is_safe_url("javascript:alert(1)"));
        assert!(!is_safe_url("data:text/html,<script>"));
        assert!(!is_safe_url("file:///etc/passwd"));
    }
}
