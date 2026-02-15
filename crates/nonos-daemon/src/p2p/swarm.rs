use super::behaviour::{NonosBehaviour, NonosBehaviourEvent};
use super::network::{extract_peer_id, get_bootstrap_nodes, NetworkConfig};
use super::types::{
    BanEntry, MessageViolation, NetworkCommand, NetworkEvent, NetworkStats, PeerInfo,
    RateLimitReason, RateLimiter,
};
use futures::StreamExt;
use libp2p::{
    gossipsub, identify, kad, ping,
    swarm::SwarmEvent,
    Multiaddr, PeerId, Swarm,
};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

const DEFAULT_MESSAGES_PER_SEC: u32 = 100;
const DEFAULT_BYTES_PER_SEC: u64 = 1024 * 1024;
const MAX_MESSAGE_SIZE: usize = 64 * 1024;
const SPAM_MESSAGE_THRESHOLD: u64 = 1000;
const SPAM_TIME_WINDOW_SECS: i64 = 60;

pub(crate) async fn run_swarm(
    mut swarm: Swarm<NonosBehaviour>,
    mut command_rx: mpsc::Receiver<NetworkCommand>,
    event_tx: mpsc::Sender<NetworkEvent>,
    peers: Arc<RwLock<HashMap<PeerId, PeerInfo>>>,
    banned_peers: Arc<RwLock<HashMap<PeerId, BanEntry>>>,
    stats: Arc<NetworkStats>,
    running: Arc<AtomicBool>,
    port: u16,
    rate_limiters: Arc<RwLock<HashMap<PeerId, RateLimiter>>>,
    config: NetworkConfig,
) {
    let listen_addr: Multiaddr = format!("/ip4/0.0.0.0/tcp/{}", port)
        .parse()
        .expect("Valid listen address");

    if let Err(e) = swarm.listen_on(listen_addr.clone()) {
        error!("Failed to start listening: {}", e);
        return;
    }
    info!("Listening on {}", listen_addr);

    let bootstrap_nodes = if !config.custom_bootstrap_nodes.is_empty() {
        config.custom_bootstrap_nodes.clone()
    } else {
        get_bootstrap_nodes()
    };

    for addr_str in bootstrap_nodes {
        if let Ok(addr) = addr_str.parse::<Multiaddr>() {
            if let Some(peer_id) = extract_peer_id(&addr) {
                swarm.behaviour_mut().kademlia.add_address(&peer_id, addr);
            }
        }
    }

    let mut global_rate_limiter = RateLimiter::new(
        config.messages_per_sec,
        config.bytes_per_sec,
    );

    loop {
        tokio::select! {
            Some(cmd) = command_rx.recv() => {
                handle_command(
                    cmd,
                    &mut swarm,
                    &peers,
                    &banned_peers,
                    &stats,
                    &running,
                    &rate_limiters,
                    &mut global_rate_limiter,
                ).await;

                if !running.load(Ordering::Relaxed) {
                    break;
                }
            }

            event = swarm.select_next_some() => {
                handle_swarm_event(
                    event,
                    &event_tx,
                    &peers,
                    &banned_peers,
                    &stats,
                    &mut swarm,
                    &rate_limiters,
                    &config,
                ).await;
            }
        }
    }

    info!("P2P swarm loop ended");
}

async fn handle_command(
    cmd: NetworkCommand,
    swarm: &mut Swarm<NonosBehaviour>,
    peers: &Arc<RwLock<HashMap<PeerId, PeerInfo>>>,
    banned_peers: &Arc<RwLock<HashMap<PeerId, BanEntry>>>,
    stats: &Arc<NetworkStats>,
    running: &Arc<AtomicBool>,
    rate_limiters: &Arc<RwLock<HashMap<PeerId, RateLimiter>>>,
    global_rate_limiter: &mut RateLimiter,
) {
    match cmd {
        NetworkCommand::Connect(addr) => {
            if let Some(peer_id) = extract_peer_id(&addr) {
                if is_banned(&banned_peers, &peer_id) {
                    warn!("Refusing to connect to banned peer: {}", peer_id);
                    return;
                }
            }

            stats.connection_attempts.fetch_add(1, Ordering::Relaxed);

            if let Err(e) = swarm.dial(addr.clone()) {
                warn!("Failed to dial {}: {}", addr, e);
                stats.connection_failures.fetch_add(1, Ordering::Relaxed);
            }
        }

        NetworkCommand::Disconnect(peer) => {
            let _ = swarm.disconnect_peer_id(peer);
            peers.write().remove(&peer);
            rate_limiters.write().remove(&peer);
            stats.peer_count.fetch_sub(1, Ordering::Relaxed);
        }

        NetworkCommand::Publish { topic, data } => {
            if data.len() > MAX_MESSAGE_SIZE {
                warn!("Refusing to publish oversized message: {} bytes", data.len());
                return;
            }

            let topic_hash = gossipsub::IdentTopic::new(&topic);
            if let Err(e) = swarm.behaviour_mut().gossipsub.publish(topic_hash, data.clone()) {
                warn!("Failed to publish to {}: {:?}", topic, e);
            } else {
                stats.messages_published.fetch_add(1, Ordering::Relaxed);
                stats.bytes_sent.fetch_add(data.len() as u64, Ordering::Relaxed);
            }
        }

        NetworkCommand::Subscribe(topic) => {
            let topic_hash = gossipsub::IdentTopic::new(&topic);
            if let Err(e) = swarm.behaviour_mut().gossipsub.subscribe(&topic_hash) {
                warn!("Failed to subscribe to {}: {:?}", topic, e);
            } else {
                stats.active_topics.fetch_add(1, Ordering::Relaxed);
                debug!("Subscribed to topic: {}", topic);
            }
        }

        NetworkCommand::Unsubscribe(topic) => {
            let topic_hash = gossipsub::IdentTopic::new(&topic);
            if let Err(e) = swarm.behaviour_mut().gossipsub.unsubscribe(&topic_hash) {
                warn!("Failed to unsubscribe from {}: {:?}", topic, e);
            } else {
                stats.active_topics.fetch_sub(1, Ordering::Relaxed);
            }
        }

        NetworkCommand::AddAddress(peer, addr) => {
            if is_banned(&banned_peers, &peer) {
                return;
            }
            swarm.behaviour_mut().kademlia.add_address(&peer, addr);
        }

        NetworkCommand::Bootstrap => {
            if let Err(e) = swarm.behaviour_mut().kademlia.bootstrap() {
                warn!("Failed to bootstrap: {:?}", e);
            } else {
                debug!("Bootstrap initiated");
            }
        }

        NetworkCommand::BanPeer(peer, _duration) => {
            let _ = swarm.disconnect_peer_id(peer);
            peers.write().remove(&peer);
            rate_limiters.write().remove(&peer);
            swarm.behaviour_mut().gossipsub.blacklist_peer(&peer);
        }

        NetworkCommand::UnbanPeer(peer) => {
            swarm.behaviour_mut().gossipsub.remove_blacklisted_peer(&peer);
        }

        NetworkCommand::SetRateLimit { messages_per_sec, bytes_per_sec } => {
            global_rate_limiter.update_limits(messages_per_sec, bytes_per_sec);
            debug!("Updated global rate limit: {} msg/s, {} bytes/s", messages_per_sec, bytes_per_sec);
        }

        NetworkCommand::Shutdown => {
            info!("Received shutdown command");
            running.store(false, Ordering::Relaxed);
        }
    }
}

async fn handle_swarm_event(
    event: SwarmEvent<NonosBehaviourEvent>,
    event_tx: &mpsc::Sender<NetworkEvent>,
    peers: &Arc<RwLock<HashMap<PeerId, PeerInfo>>>,
    banned_peers: &Arc<RwLock<HashMap<PeerId, BanEntry>>>,
    stats: &Arc<NetworkStats>,
    swarm: &mut Swarm<NonosBehaviour>,
    rate_limiters: &Arc<RwLock<HashMap<PeerId, RateLimiter>>>,
    config: &NetworkConfig,
) {
    match event {
        SwarmEvent::NewListenAddr { address, .. } => {
            info!("Listening on {}", address);
        }

        SwarmEvent::ConnectionEstablished { peer_id, endpoint, num_established, .. } => {
            if is_banned(&banned_peers, &peer_id) {
                warn!("Disconnecting banned peer that connected: {}", peer_id);
                let _ = swarm.disconnect_peer_id(peer_id);
                return;
            }

            info!("Connected to peer: {} (total connections: {})", peer_id, num_established);

            let peer_info = PeerInfo::new(
                peer_id,
                vec![endpoint.get_remote_address().to_string()],
            );
            peers.write().insert(peer_id, peer_info);

            if config.enable_rate_limiting {
                rate_limiters.write().insert(
                    peer_id,
                    RateLimiter::new(DEFAULT_MESSAGES_PER_SEC, DEFAULT_BYTES_PER_SEC),
                );
            }

            stats.peer_count.fetch_add(1, Ordering::Relaxed);

            let _ = event_tx.send(NetworkEvent::PeerConnected(peer_id)).await;
        }

        SwarmEvent::ConnectionClosed { peer_id, num_established, cause, .. } => {
            if num_established == 0 {
                info!("Disconnected from peer: {} (cause: {:?})", peer_id, cause);
                peers.write().remove(&peer_id);
                rate_limiters.write().remove(&peer_id);
                stats.peer_count.fetch_sub(1, Ordering::Relaxed);
                let _ = event_tx.send(NetworkEvent::PeerDisconnected(peer_id)).await;
            }
        }

        SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
            if let Some(peer) = peer_id {
                warn!("Failed to connect to {}: {}", peer, error);
                stats.connection_failures.fetch_add(1, Ordering::Relaxed);
            } else {
                warn!("Outgoing connection failed: {}", error);
            }
        }

        SwarmEvent::IncomingConnectionError { local_addr, send_back_addr, error, .. } => {
            debug!(
                "Incoming connection error from {} to {}: {}",
                send_back_addr, local_addr, error
            );
        }

        SwarmEvent::Behaviour(NonosBehaviourEvent::Gossipsub(gossipsub::Event::Message {
            propagation_source,
            message,
            message_id,
        })) => {
            if is_banned(&banned_peers, &propagation_source) {
                debug!("Ignoring message from banned peer: {}", propagation_source);
                stats.messages_dropped.fetch_add(1, Ordering::Relaxed);
                return;
            }

            let message_size = message.data.len() as u64;

            if message.data.len() > MAX_MESSAGE_SIZE {
                warn!(
                    "Dropping oversized message from {}: {} bytes (max: {})",
                    propagation_source, message.data.len(), MAX_MESSAGE_SIZE
                );
                stats.messages_dropped.fetch_add(1, Ordering::Relaxed);

                let should_ban = {
                    let mut peers_lock = peers.write();
                    if let Some(info) = peers_lock.get_mut(&propagation_source) {
                        info.record_violation(MessageViolation::OversizedMessage);
                        info.should_ban()
                    } else {
                        false
                    }
                };

                let _ = event_tx.send(NetworkEvent::RateLimited {
                    peer: propagation_source,
                    reason: RateLimitReason::OversizedMessage,
                }).await;

                if should_ban {
                    let ban_duration = peers.read()
                        .get(&propagation_source)
                        .map(|p| p.recommended_ban_duration())
                        .unwrap_or(std::time::Duration::from_secs(300));

                    warn!("Auto-banning peer {} for repeated violations", propagation_source);
                    let ban = BanEntry::new(propagation_source, ban_duration, "repeated_violations");
                    banned_peers.write().insert(propagation_source, ban);
                    stats.banned_peers.fetch_add(1, Ordering::Relaxed);
                    let _ = swarm.disconnect_peer_id(propagation_source);
                }

                return;
            }

            if config.enable_rate_limiting {
                let rate_limit_result = {
                    let mut limiters = rate_limiters.write();
                    let rate_limiter = limiters
                        .entry(propagation_source)
                        .or_insert_with(|| RateLimiter::new(DEFAULT_MESSAGES_PER_SEC, DEFAULT_BYTES_PER_SEC));
                    rate_limiter.check_message(message_size)
                };

                if let Err(reason) = rate_limit_result {
                    stats.rate_limit_hits.fetch_add(1, Ordering::Relaxed);
                    stats.messages_dropped.fetch_add(1, Ordering::Relaxed);

                    let should_ban = {
                        let mut peers_lock = peers.write();
                        if let Some(info) = peers_lock.get_mut(&propagation_source) {
                            info.record_violation(MessageViolation::RateLimitExceeded);
                            info.should_ban()
                        } else {
                            false
                        }
                    };

                    let _ = event_tx.send(NetworkEvent::RateLimited {
                        peer: propagation_source,
                        reason,
                    }).await;

                    warn!(
                        "Rate limited message from {}: {:?}",
                        propagation_source, reason
                    );

                    if should_ban {
                        warn!("Auto-banning peer {} for repeated rate limit violations", propagation_source);
                        let ban = BanEntry::new(
                            propagation_source,
                            std::time::Duration::from_secs(600),
                            "rate_limit_violations"
                        );
                        banned_peers.write().insert(propagation_source, ban);
                        stats.banned_peers.fetch_add(1, Ordering::Relaxed);
                        let _ = swarm.disconnect_peer_id(propagation_source);
                    }

                    return;
                }
            }

            let topic = message.topic.to_string();
            debug!(
                "Received message {} on topic {} from {}",
                message_id, topic, propagation_source
            );

            stats.messages_received.fetch_add(1, Ordering::Relaxed);
            stats.bytes_received.fetch_add(message_size, Ordering::Relaxed);

            let (is_spam, should_ban) = {
                let mut peers_lock = peers.write();
                if let Some(info) = peers_lock.get_mut(&propagation_source) {
                    info.record_message(message_size);
                    info.record_success();

                    let spam = is_spam_behavior(info);
                    if spam {
                        info.record_violation(MessageViolation::SpamBehavior);
                    }
                    (spam, spam && info.should_ban())
                } else {
                    (false, false)
                }
            };

            if is_spam {
                warn!("Potential spam detected from peer: {}", propagation_source);
                let _ = event_tx.send(NetworkEvent::RateLimited {
                    peer: propagation_source,
                    reason: RateLimitReason::SpamDetected,
                }).await;

                if should_ban {
                    warn!("Auto-banning peer {} for spam behavior", propagation_source);
                    let ban = BanEntry::new(
                        propagation_source,
                        std::time::Duration::from_secs(3600),
                        "spam_behavior"
                    );
                    banned_peers.write().insert(propagation_source, ban);
                    stats.banned_peers.fetch_add(1, Ordering::Relaxed);
                    let _ = swarm.disconnect_peer_id(propagation_source);
                    return;
                }
            }

            let _ = event_tx.send(NetworkEvent::Message {
                topic,
                source: propagation_source,
                data: message.data,
            }).await;
        }

        SwarmEvent::Behaviour(NonosBehaviourEvent::Gossipsub(gossipsub::Event::Subscribed {
            peer_id,
            topic,
        })) => {
            debug!("Peer {} subscribed to {}", peer_id, topic);
        }

        SwarmEvent::Behaviour(NonosBehaviourEvent::Gossipsub(gossipsub::Event::Unsubscribed {
            peer_id,
            topic,
        })) => {
            debug!("Peer {} unsubscribed from {}", peer_id, topic);
        }

        SwarmEvent::Behaviour(NonosBehaviourEvent::Gossipsub(gossipsub::Event::GossipsubNotSupported {
            peer_id,
        })) => {
            debug!("Peer {} does not support gossipsub", peer_id);
        }

        SwarmEvent::Behaviour(NonosBehaviourEvent::Kademlia(kad::Event::RoutingUpdated {
            peer,
            addresses,
            is_new_peer,
            ..
        })) => {
            if is_banned(&banned_peers, &peer) {
                return;
            }

            debug!(
                "Kademlia routing updated for peer: {} (new: {})",
                peer, is_new_peer
            );

            let addrs: Vec<Multiaddr> = addresses.iter().cloned().collect();
            let _ = event_tx.send(NetworkEvent::PeerDiscovered(peer, addrs)).await;
        }

        SwarmEvent::Behaviour(NonosBehaviourEvent::Kademlia(kad::Event::OutboundQueryProgressed {
            result,
            ..
        })) => {
            match result {
                kad::QueryResult::Bootstrap(Ok(kad::BootstrapOk { peer, num_remaining })) => {
                    debug!("Bootstrap progress: peer={}, remaining={}", peer, num_remaining);
                }
                kad::QueryResult::Bootstrap(Err(e)) => {
                    warn!("Bootstrap failed: {:?}", e);
                }
                kad::QueryResult::GetClosestPeers(Ok(kad::GetClosestPeersOk { peers: found_peers, .. })) => {
                    debug!("Found {} closest peers", found_peers.len());
                }
                _ => {}
            }
        }

        SwarmEvent::Behaviour(NonosBehaviourEvent::Ping(ping::Event {
            peer,
            result: Ok(rtt),
            ..
        })) => {
            debug!("Ping to {} took {:?}", peer, rtt);

            if let Some(info) = peers.write().get_mut(&peer) {
                info.latency_ms = Some(rtt.as_millis() as u32);
                info.record_success();
            }

            let _ = event_tx.send(NetworkEvent::PingResult { peer, rtt }).await;
        }

        SwarmEvent::Behaviour(NonosBehaviourEvent::Ping(ping::Event {
            peer,
            result: Err(e),
            ..
        })) => {
            debug!("Ping to {} failed: {:?}", peer, e);

            if let Some(info) = peers.write().get_mut(&peer) {
                info.record_failure();
            }
        }

        SwarmEvent::Behaviour(NonosBehaviourEvent::Identify(identify::Event::Received {
            peer_id,
            info,
            ..
        })) => {
            debug!(
                "Identified peer {}: {} ({})",
                peer_id, info.agent_version, info.protocol_version
            );

            if let Some(peer_info) = peers.write().get_mut(&peer_id) {
                peer_info.protocol_version = Some(info.protocol_version.clone());
                peer_info.agent_version = Some(info.agent_version.clone());
                peer_info.addresses = info.listen_addrs.iter().map(|a| a.to_string()).collect();
            }

            for addr in info.listen_addrs {
                if !is_banned(&banned_peers, &peer_id) {
                    swarm.behaviour_mut().kademlia.add_address(&peer_id, addr);
                }
            }
        }

        SwarmEvent::Behaviour(NonosBehaviourEvent::Identify(identify::Event::Sent { peer_id, .. })) => {
            debug!("Sent identify info to {}", peer_id);
        }

        SwarmEvent::Behaviour(NonosBehaviourEvent::Identify(identify::Event::Pushed { peer_id, .. })) => {
            debug!("Pushed identify update to {}", peer_id);
        }

        SwarmEvent::Behaviour(NonosBehaviourEvent::Identify(identify::Event::Error { peer_id, error, .. })) => {
            debug!("Identify error with {}: {:?}", peer_id, error);
        }

        _ => {}
    }
}

fn is_banned(banned_peers: &Arc<RwLock<HashMap<PeerId, BanEntry>>>, peer_id: &PeerId) -> bool {
    if let Some(ban) = banned_peers.read().get(peer_id) {
        if !ban.is_expired() {
            return true;
        }
    }
    false
}

fn is_spam_behavior(info: &PeerInfo) -> bool {
    if info.message_count > SPAM_MESSAGE_THRESHOLD {
        if let Some(last_msg_ts) = info.last_message_at {
            let msg_window_secs = (last_msg_ts - info.connected_at.timestamp()).max(1);
            if msg_window_secs < SPAM_TIME_WINDOW_SECS {
                let rate = info.message_count as f64 / msg_window_secs as f64;
                return rate > 50.0;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_spam_behavior() {
        let mut info = PeerInfo::default();
        info.message_count = 100;
        assert!(!is_spam_behavior(&info));

        info.message_count = 5000;
        info.last_message_at = Some(chrono::Utc::now().timestamp());
        assert!(is_spam_behavior(&info));
    }
}
