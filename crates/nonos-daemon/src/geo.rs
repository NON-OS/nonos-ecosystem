use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, warn};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GeoLocation {
    pub lat: f64,
    pub lon: f64,
    pub city: String,
    pub country: String,
    pub country_code: String,
}

impl Default for GeoLocation {
    fn default() -> Self {
        Self {
            lat: 0.0,
            lon: 0.0,
            city: "Unknown".to_string(),
            country: "Unknown".to_string(),
            country_code: "XX".to_string(),
        }
    }
}

pub struct GeoCache {
    cache: Arc<RwLock<HashMap<String, GeoLocation>>>,
    client: reqwest::Client,
}

impl GeoCache {
    pub fn new() -> Self {
        let mut cache = HashMap::new();

        cache.insert("bootstrap-amsterdam".to_string(), GeoLocation {
            lat: 52.37,
            lon: 4.90,
            city: "Amsterdam".to_string(),
            country: "Netherlands".to_string(),
            country_code: "NL".to_string(),
        });

        cache.insert("bootstrap-sofia".to_string(), GeoLocation {
            lat: 42.70,
            lon: 23.32,
            city: "Sofia".to_string(),
            country: "Bulgaria".to_string(),
            country_code: "BG".to_string(),
        });

        cache.insert("bootstrap-capetown".to_string(), GeoLocation {
            lat: -33.92,
            lon: 18.42,
            city: "Cape Town".to_string(),
            country: "South Africa".to_string(),
            country_code: "ZA".to_string(),
        });

        cache.insert("bootstrap-budapest".to_string(), GeoLocation {
            lat: 47.50,
            lon: 19.04,
            city: "Budapest".to_string(),
            country: "Hungary".to_string(),
            country_code: "HU".to_string(),
        });

        Self {
            cache: Arc::new(RwLock::new(cache)),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .unwrap_or_default(),
        }
    }

    pub fn extract_ip(multiaddr: &str) -> Option<String> {
        let parts: Vec<&str> = multiaddr.split('/').collect();
        for (i, part) in parts.iter().enumerate() {
            if *part == "ip4" || *part == "ip6" {
                if let Some(ip) = parts.get(i + 1) {
                    return Some(ip.to_string());
                }
            }
        }
        None
    }

    pub async fn lookup(&self, ip: &str) -> Option<GeoLocation> {
        {
            let cache = self.cache.read().await;
            if let Some(loc) = cache.get(ip) {
                return Some(loc.clone());
            }
        }

        if ip.starts_with("127.") || ip.starts_with("10.") ||
           ip.starts_with("192.168.") || ip.starts_with("172.") ||
           ip == "0.0.0.0" || ip == "localhost" {
            return None;
        }

        let url = format!("http://ip-api.com/json/{}?fields=status,lat,lon,city,country,countryCode", ip);

        match self.client.get(&url).send().await {
            Ok(resp) => {
                if let Ok(data) = resp.json::<IpApiResponse>().await {
                    if data.status == "success" {
                        let loc = GeoLocation {
                            lat: data.lat.unwrap_or(0.0),
                            lon: data.lon.unwrap_or(0.0),
                            city: data.city.unwrap_or_else(|| "Unknown".to_string()),
                            country: data.country.unwrap_or_else(|| "Unknown".to_string()),
                            country_code: data.country_code.unwrap_or_else(|| "XX".to_string()),
                        };

                        let mut cache = self.cache.write().await;
                        cache.insert(ip.to_string(), loc.clone());

                        debug!("Geo lookup for {}: {} ({})", ip, loc.city, loc.country_code);
                        return Some(loc);
                    }
                }
            }
            Err(e) => {
                warn!("Geo lookup failed for {}: {}", ip, e);
            }
        }

        None
    }

    pub async fn lookup_multiaddr(&self, multiaddr: &str) -> Option<GeoLocation> {
        if let Some(ip) = Self::extract_ip(multiaddr) {
            self.lookup(&ip).await
        } else {
            None
        }
    }
}

#[derive(Deserialize)]
struct IpApiResponse {
    status: String,
    lat: Option<f64>,
    lon: Option<f64>,
    city: Option<String>,
    country: Option<String>,
    #[serde(rename = "countryCode")]
    country_code: Option<String>,
}

impl Default for GeoCache {
    fn default() -> Self {
        Self::new()
    }
}
