use crate::config::BootstrapMode;

pub fn get_bootstrap_nodes_for_mode(
    mode: &BootstrapMode,
    custom_peers: &[String],
) -> Vec<String> {
    match mode {
        BootstrapMode::Official => {
            crate::config::NetworkConfig::OFFICIAL_BOOTSTRAP_NODES
                .iter()
                .map(|s| s.to_string())
                .collect()
        }
        BootstrapMode::Custom => custom_peers.to_vec(),
        BootstrapMode::None => Vec::new(),
    }
}
