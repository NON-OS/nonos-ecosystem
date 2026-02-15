use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;

pub async fn fetch_json(client: &Client, api_url: &str, endpoint: &str) -> Result<serde_json::Value> {
    let resp = client
        .get(format!("{}{}", api_url, endpoint))
        .timeout(Duration::from_secs(2))
        .send()
        .await?
        .json()
        .await?;
    Ok(resp)
}

#[derive(Debug, Deserialize)]
struct GeoIpResponse {
    status: String,
    lat: Option<f64>,
    lon: Option<f64>,
    city: Option<String>,
    #[serde(rename = "countryCode")]
    country_code: Option<String>,
}

pub async fn lookup_geoip(client: &Client, ip: &str) -> Option<(f64, f64, String, String)> {
    if ip.is_empty() { return None; }

    let url = format!(
        "http://ip-api.com/json/{}?fields=status,lat,lon,city,countryCode",
        ip
    );

    let resp = match client.get(&url).timeout(Duration::from_secs(3)).send().await {
        Ok(r) => r,
        Err(_) => return None,
    };

    let data: GeoIpResponse = match resp.json().await {
        Ok(d) => d,
        Err(_) => return None,
    };

    if data.status != "success" { return None; }

    Some((
        data.lat?,
        data.lon?,
        data.city.unwrap_or_default(),
        data.country_code.unwrap_or_default(),
    ))
}
