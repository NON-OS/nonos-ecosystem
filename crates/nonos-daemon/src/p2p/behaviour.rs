use libp2p::{
    gossipsub, identify, kad, ping,
    swarm::NetworkBehaviour,
};

#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "NonosBehaviourEvent")]
pub struct NonosBehaviour {
    pub kademlia: kad::Behaviour<kad::store::MemoryStore>,
    pub gossipsub: gossipsub::Behaviour,
    pub identify: identify::Behaviour,
    pub ping: ping::Behaviour,
}

#[derive(Debug)]
pub enum NonosBehaviourEvent {
    Kademlia(kad::Event),
    Gossipsub(gossipsub::Event),
    Identify(identify::Event),
    Ping(ping::Event),
}

impl From<kad::Event> for NonosBehaviourEvent {
    fn from(event: kad::Event) -> Self {
        NonosBehaviourEvent::Kademlia(event)
    }
}

impl From<gossipsub::Event> for NonosBehaviourEvent {
    fn from(event: gossipsub::Event) -> Self {
        NonosBehaviourEvent::Gossipsub(event)
    }
}

impl From<identify::Event> for NonosBehaviourEvent {
    fn from(event: identify::Event) -> Self {
        NonosBehaviourEvent::Identify(event)
    }
}

impl From<ping::Event> for NonosBehaviourEvent {
    fn from(event: ping::Event) -> Self {
        NonosBehaviourEvent::Ping(event)
    }
}
