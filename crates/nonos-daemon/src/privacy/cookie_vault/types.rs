use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecretShare {
    pub index: u8,
    pub value: Vec<u8>,
    pub node_id: [u8; 32],
}
