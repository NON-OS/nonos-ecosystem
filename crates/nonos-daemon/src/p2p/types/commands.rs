use libp2p::{Multiaddr, PeerId};
use std::time::Duration;

use super::rate_limit::RateLimitReason;

#[derive(Debug)]
pub enum NetworkCommand {
    Connect(Multiaddr),
    Disconnect(PeerId),
    Publish { topic: String, data: Vec<u8> },
    Subscribe(String),
    Unsubscribe(String),
    AddAddress(PeerId, Multiaddr),
    Bootstrap,
    Shutdown,
    BanPeer(PeerId, Duration),
    UnbanPeer(PeerId),
    SetRateLimit { messages_per_sec: u32, bytes_per_sec: u64 },
}

#[derive(Debug, Clone)]
pub enum NetworkEvent {
    PeerConnected(PeerId),
    PeerDisconnected(PeerId),
    Message { topic: String, source: PeerId, data: Vec<u8> },
    PeerDiscovered(PeerId, Vec<Multiaddr>),
    PingResult { peer: PeerId, rtt: Duration },
    Error(String),
    RateLimited { peer: PeerId, reason: RateLimitReason },
    PeerBanned { peer: PeerId, until: i64 },
    CircuitOpen { peer: PeerId },
    CircuitClosed { peer: PeerId },
}
