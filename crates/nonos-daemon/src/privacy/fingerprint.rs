use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NormalizedRequest {
    pub user_agent: String,
    pub accept: String,
    pub accept_language: String,
    pub accept_encoding: String,
    pub dnt: String,
    pub sec_fetch_dest: String,
    pub sec_fetch_mode: String,
    pub sec_fetch_site: String,
    pub cache_control: String,
}

impl Default for NormalizedRequest {
    fn default() -> Self {
        Self {
            user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".into(),
            accept: "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8".into(),
            accept_language: "en-US,en;q=0.5".into(),
            accept_encoding: "gzip, deflate, br".into(),
            dnt: "1".into(),
            sec_fetch_dest: "document".into(),
            sec_fetch_mode: "navigate".into(),
            sec_fetch_site: "none".into(),
            cache_control: "no-cache".into(),
        }
    }
}

impl NormalizedRequest {
    pub fn to_headers(&self) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        headers.insert("User-Agent".into(), self.user_agent.clone());
        headers.insert("Accept".into(), self.accept.clone());
        headers.insert("Accept-Language".into(), self.accept_language.clone());
        headers.insert("Accept-Encoding".into(), self.accept_encoding.clone());
        headers.insert("DNT".into(), self.dnt.clone());
        headers.insert("Sec-Fetch-Dest".into(), self.sec_fetch_dest.clone());
        headers.insert("Sec-Fetch-Mode".into(), self.sec_fetch_mode.clone());
        headers.insert("Sec-Fetch-Site".into(), self.sec_fetch_site.clone());
        headers.insert("Cache-Control".into(), self.cache_control.clone());
        headers
    }
}

pub struct FingerprintNormalizer {
    standard_request: NormalizedRequest,
    fingerprint_patches: Vec<String>,
    tracking_headers_to_remove: Vec<String>,
}

impl FingerprintNormalizer {
    pub fn new() -> Self {
        Self {
            standard_request: NormalizedRequest::default(),
            fingerprint_patches: Self::default_patches(),
            tracking_headers_to_remove: Self::default_tracking_headers(),
        }
    }

    fn default_patches() -> Vec<String> {
        vec![
            r#"(function(){const o=HTMLCanvasElement.prototype.toDataURL;HTMLCanvasElement.prototype.toDataURL=function(){const c=this.getContext('2d');if(c){const n=Math.random()*0.01;const d=c.getImageData(0,0,this.width,this.height);for(let i=0;i<d.data.length;i+=4)d.data[i]+=n;c.putImageData(d,0,0)}return o.apply(this,arguments)}})();"#.into(),
            r#"(function(){const o=AudioBuffer.prototype.getChannelData;AudioBuffer.prototype.getChannelData=function(c){const d=o.call(this,c);for(let i=0;i<d.length;i++)d[i]+=(Math.random()-0.5)*0.0001;return d}})();"#.into(),
            r#"(function(){const g=WebGLRenderingContext.prototype.getParameter;WebGLRenderingContext.prototype.getParameter=function(p){if(p===37445)return'Intel Inc.';if(p===37446)return'Intel Iris OpenGL Engine';return g.apply(this,arguments)}})();"#.into(),
            r#"(function(){Object.defineProperty(navigator,'plugins',{get:function(){return[]}})})();"#.into(),
            r#"(function(){Object.defineProperty(screen,'width',{get:()=>1920});Object.defineProperty(screen,'height',{get:()=>1080});Object.defineProperty(screen,'availWidth',{get:()=>1920});Object.defineProperty(screen,'availHeight',{get:()=>1040})})();"#.into(),
        ]
    }

    fn default_tracking_headers() -> Vec<String> {
        vec![
            "X-Forwarded-For".into(),
            "X-Real-IP".into(),
            "X-Client-IP".into(),
            "CF-Connecting-IP".into(),
            "True-Client-IP".into(),
            "X-Cluster-Client-IP".into(),
            "Forwarded".into(),
            "Via".into(),
        ]
    }

    pub fn normalize_headers(&self, headers: &mut HashMap<String, String>) {
        headers.insert("User-Agent".into(), self.standard_request.user_agent.clone());
        headers.insert("Accept".into(), self.standard_request.accept.clone());
        headers.insert("Accept-Language".into(), self.standard_request.accept_language.clone());
        headers.insert("Accept-Encoding".into(), self.standard_request.accept_encoding.clone());
        headers.insert("DNT".into(), self.standard_request.dnt.clone());
        headers.insert("Sec-Fetch-Dest".into(), self.standard_request.sec_fetch_dest.clone());
        headers.insert("Sec-Fetch-Mode".into(), self.standard_request.sec_fetch_mode.clone());
        headers.insert("Sec-Fetch-Site".into(), self.standard_request.sec_fetch_site.clone());
        headers.insert("Cache-Control".into(), self.standard_request.cache_control.clone());

        for header in &self.tracking_headers_to_remove {
            headers.remove(header);
        }
    }

    pub fn get_fingerprint_patches(&self) -> &[String] {
        &self.fingerprint_patches
    }

    pub fn get_standard_request(&self) -> &NormalizedRequest {
        &self.standard_request
    }

    pub fn patch_count(&self) -> usize {
        self.fingerprint_patches.len()
    }

    pub fn set_user_agent(&mut self, user_agent: String) {
        self.standard_request.user_agent = user_agent;
    }

    pub fn set_accept_language(&mut self, lang: String) {
        self.standard_request.accept_language = lang;
    }
}

impl Default for FingerprintNormalizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fingerprint_normalizer_headers() {
        let normalizer = FingerprintNormalizer::new();
        let mut headers = HashMap::new();
        headers.insert("User-Agent".into(), "My Custom Browser".into());
        headers.insert("X-Forwarded-For".into(), "1.2.3.4".into());

        normalizer.normalize_headers(&mut headers);

        assert!(headers.get("User-Agent").unwrap().contains("Chrome"));
        assert!(!headers.contains_key("X-Forwarded-For"));
    }

    #[test]
    fn test_fingerprint_patches_present() {
        let normalizer = FingerprintNormalizer::new();
        assert!(normalizer.patch_count() > 0);
        assert!(normalizer.get_fingerprint_patches()[0].contains("Canvas"));
    }

    #[test]
    fn test_normalized_request_to_headers() {
        let request = NormalizedRequest::default();
        let headers = request.to_headers();
        assert!(headers.contains_key("User-Agent"));
        assert!(headers.contains_key("DNT"));
    }
}
