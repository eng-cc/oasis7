use futures::future::Either;
use libp2p::core::muxing::StreamMuxerBox;
use libp2p::core::transport::OrTransport;
use libp2p::gossipsub::{self, MessageAuthenticity};
use libp2p::identity::Keypair;
use libp2p::kad::{self, store::MemoryStore};
use libp2p::multiaddr::Protocol;
use libp2p::noise;
use libp2p::rendezvous::{client as rendezvous_client, server as rendezvous_server};
use libp2p::request_response::{self, ProtocolSupport};
use libp2p::swarm::{NetworkBehaviour, Swarm};
use libp2p::{Multiaddr, PeerId, StreamProtocol, Transport as _};

use oasis7_proto::distributed::RR_PROTOCOL_PREFIX;
use oasis7_proto::distributed_net::{NetworkRequest, NetworkResponse};

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "BehaviourEvent")]
pub(super) struct Behaviour {
    pub(super) gossipsub: gossipsub::Behaviour,
    pub(super) request_response: request_response::cbor::Behaviour<NetworkRequest, NetworkResponse>,
    pub(super) kademlia: kad::Behaviour<MemoryStore>,
    pub(super) rendezvous_client: rendezvous_client::Behaviour,
    pub(super) rendezvous_server: rendezvous_server::Behaviour,
}

#[derive(Debug)]
pub(super) enum BehaviourEvent {
    Gossipsub(gossipsub::Event),
    RequestResponse(request_response::Event<NetworkRequest, NetworkResponse>),
    Kademlia(kad::Event),
    RendezvousClient(rendezvous_client::Event),
    RendezvousServer(rendezvous_server::Event),
}

impl From<gossipsub::Event> for BehaviourEvent {
    fn from(event: gossipsub::Event) -> Self {
        BehaviourEvent::Gossipsub(event)
    }
}

impl From<request_response::Event<NetworkRequest, NetworkResponse>> for BehaviourEvent {
    fn from(event: request_response::Event<NetworkRequest, NetworkResponse>) -> Self {
        BehaviourEvent::RequestResponse(event)
    }
}

impl From<kad::Event> for BehaviourEvent {
    fn from(event: kad::Event) -> Self {
        BehaviourEvent::Kademlia(event)
    }
}

impl From<rendezvous_client::Event> for BehaviourEvent {
    fn from(event: rendezvous_client::Event) -> Self {
        BehaviourEvent::RendezvousClient(event)
    }
}

impl From<rendezvous_server::Event> for BehaviourEvent {
    fn from(event: rendezvous_server::Event) -> Self {
        BehaviourEvent::RendezvousServer(event)
    }
}

pub(super) fn build_swarm(keypair: &Keypair) -> Swarm<Behaviour> {
    let swarm_config = libp2p::swarm::Config::with_async_std_executor()
        .with_idle_connection_timeout(std::time::Duration::from_secs(30));

    let peer_id = PeerId::from(keypair.public());
    let gossipsub = gossipsub::Behaviour::new(
        MessageAuthenticity::Signed(keypair.clone()),
        gossipsub::Config::default(),
    )
    .expect("gossipsub config");

    let protocols = vec![(
        StreamProtocol::new(RR_PROTOCOL_PREFIX),
        ProtocolSupport::Full,
    )];
    let request_response =
        request_response::cbor::Behaviour::new(protocols, request_response::Config::default());

    let store = MemoryStore::new(peer_id);
    let kademlia = kad::Behaviour::new(peer_id, store);

    let behaviour = Behaviour {
        gossipsub,
        request_response,
        kademlia,
        rendezvous_client: rendezvous_client::Behaviour::new(keypair.clone()),
        rendezvous_server: rendezvous_server::Behaviour::new(Default::default()),
    };

    let quic_transport =
        libp2p::quic::async_std::Transport::new(libp2p::quic::Config::new(keypair));
    let tcp_transport = libp2p::tcp::async_io::Transport::new(libp2p::tcp::Config::default())
        .upgrade(libp2p::core::upgrade::Version::V1)
        .authenticate(noise::Config::new(keypair).expect("noise config"))
        .multiplex(libp2p::yamux::Config::default())
        .map(|(peer_id, muxer), _| (peer_id, StreamMuxerBox::new(muxer)));
    let transport = OrTransport::new(quic_transport, tcp_transport)
        .map(|either_output, _| match either_output {
            Either::Left((peer_id, muxer)) => (peer_id, StreamMuxerBox::new(muxer)),
            Either::Right((peer_id, muxer)) => (peer_id, muxer),
        })
        .boxed();

    Swarm::new(transport, behaviour, peer_id, swarm_config)
}

pub(super) fn dial_addr_with_optional_peer_id(
    swarm: &mut Swarm<Behaviour>,
    addr: Multiaddr,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (peer_id, dial_addr) = split_peer_id(addr);
    if let Some(peer_id) = peer_id {
        swarm
            .behaviour_mut()
            .kademlia
            .add_address(&peer_id, dial_addr.clone());
        let opts = libp2p::swarm::dial_opts::DialOpts::peer_id(peer_id)
            .addresses(vec![dial_addr])
            .build();
        swarm.dial(opts)?;
    } else {
        swarm.dial(dial_addr)?;
    }
    Ok(())
}

pub(super) fn split_peer_id(mut addr: Multiaddr) -> (Option<PeerId>, Multiaddr) {
    let peer_id = match addr.pop() {
        Some(Protocol::P2p(peer)) => Some(peer),
        Some(protocol) => {
            addr.push(protocol);
            None
        }
        None => None,
    };
    (peer_id, addr)
}

pub(super) fn ensure_peer_id(mut addr: Multiaddr, peer_id: PeerId) -> Multiaddr {
    let needs_peer_id = !matches!(addr.iter().last(), Some(Protocol::P2p(_)));
    if needs_peer_id {
        addr.push(Protocol::P2p(peer_id.into()));
    }
    addr
}
