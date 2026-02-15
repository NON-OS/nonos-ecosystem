use super::network::P2pNetwork;
use crate::p2p::types::NetworkEvent;

impl P2pNetwork {
    pub async fn recv_event(&self) -> Option<NetworkEvent> {
        let rx_opt = self.event_rx.write().take();
        if let Some(mut rx) = rx_opt {
            let result = rx.recv().await;
            *self.event_rx.write() = Some(rx);
            result
        } else {
            None
        }
    }

    pub fn try_recv_event(&self) -> Option<NetworkEvent> {
        if let Some(rx) = &mut *self.event_rx.write() {
            rx.try_recv().ok()
        } else {
            None
        }
    }
}
