pub fn format_wei(wei: u128) -> String {
    let eth = wei as f64 / 1e18;
    if eth >= 1.0 {
        format!("{:.4}", eth)
    } else if eth >= 0.0001 {
        format!("{:.6}", eth)
    } else {
        format!("{:.8}", eth)
    }
}

pub fn parse_bootstrap_progress(line: &str) -> Option<u8> {
    if let Some(start) = line.find("Bootstrapped ") {
        let rest = &line[start + 13..];
        if let Some(end) = rest.find('%') {
            if let Ok(pct) = rest[..end].trim().parse::<u8>() {
                return Some(pct);
            }
        }
    }
    None
}
