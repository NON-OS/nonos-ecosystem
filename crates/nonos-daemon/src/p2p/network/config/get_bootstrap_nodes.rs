pub fn get_bootstrap_nodes() -> Vec<String> {
    if let Ok(nodes) = std::env::var("NONOS_BOOTSTRAP_NODES") {
        return nodes.split(',').map(|s| s.trim().to_string()).collect();
    }

    crate::config::NetworkConfig::OFFICIAL_BOOTSTRAP_NODES
        .iter()
        .map(|s| s.to_string())
        .collect()
}
