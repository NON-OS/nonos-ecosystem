use serde::{Deserialize, Serialize};

pub type AssetId = [u8; 8];

pub const ASSET_ETH: AssetId = [0x00; 8];
pub const ASSET_NOX: AssetId = [0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpendRequest {
    pub merkle_root: [u8; 32],
    pub nullifier: [u8; 32],
    pub recipient: [u8; 32],
    pub fee: u128,
    pub merkle_path: Vec<([u8; 32], bool)>,
    pub proof: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct SpendResult {
    pub success: bool,
    pub reason: Option<String>,
    pub tx_hash: Option<[u8; 32]>,
}
