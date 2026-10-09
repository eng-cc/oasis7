use futures::StreamExt;
use libp2p::{
    gossipsub, identity, noise, request_response,
    swarm::{NetworkBehaviour, SwarmEvent},
    yamux,
};
use std::time::Duration;

#[derive(NetworkBehaviour)]
struct Behaviour {
    rpc: request_response::cbor::Behaviour<Vec<u8>, Vec<u8>>,
    gossip: gossipsub::Behaviour,
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args[1] == "identity" {
        let key = identity::Keypair::ed25519_from_bytes([42; 32]).unwrap();
        println!("{}", key.public().to_peer_id());
        let signature = key.sign(b"oasis7-identity-compatibility").unwrap();
        assert!(
            key.public()
                .verify(b"oasis7-identity-compatibility", &signature)
        );
        return;
    }
    if args[1] == "address" {
        for address in &args[2..] {
            println!("{}", address.parse::<libp2p::Multiaddr>().unwrap());
        }
        return;
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        tokio::time::timeout(Duration::from_secs(25), run(&args))
            .await
            .expect("peer completes protocol exchange");
    });
}

async fn run(args: &[String]) {
    let server = args[1] == "server";
    let key = identity::Keypair::generate_ed25519();
    let mut swarm = libp2p::SwarmBuilder::with_existing_identity(key)
        .with_tokio()
        .with_tcp(
            libp2p::tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )
        .unwrap()
        .with_quic()
        .with_behaviour(|key| Behaviour {
            rpc: request_response::cbor::Behaviour::new(
                [(
                    libp2p::StreamProtocol::new("/oasis7/compat/1"),
                    request_response::ProtocolSupport::Full,
                )],
                request_response::Config::default(),
            ),
            gossip: gossipsub::Behaviour::new(
                gossipsub::MessageAuthenticity::Signed(key.clone()),
                gossipsub::ConfigBuilder::default()
                    .heartbeat_interval(Duration::from_millis(100))
                    .flood_publish(true)
                    .build()
                    .unwrap(),
            )
            .unwrap(),
        })
        .unwrap()
        .build();
    let topic = gossipsub::IdentTopic::new("oasis7-compatibility");
    swarm.behaviour_mut().gossip.subscribe(&topic).unwrap();
    if server {
        let address = if args[2] == "quic" {
            "/ip4/127.0.0.1/udp/0/quic-v1"
        } else {
            "/ip4/127.0.0.1/tcp/0"
        };
        swarm.listen_on(address.parse().unwrap()).unwrap();
    } else {
        swarm
            .dial(args[2].parse::<libp2p::Multiaddr>().unwrap())
            .unwrap();
    }
    let mut rpc_done = false;
    let mut gossip_done = false;
    let mut published = false;
    let mut connected = None;
    loop {
        let event = swarm.select_next_some().await;
        match event {
            SwarmEvent::NewListenAddr { address, .. } if server => {
                println!("READY {address}/p2p/{}", swarm.local_peer_id());
            }
            SwarmEvent::ConnectionEstablished { peer_id, .. } if !server => {
                connected = Some(peer_id);
                swarm
                    .behaviour_mut()
                    .rpc
                    .send_request(&peer_id, b"request".to_vec());
            }
            SwarmEvent::Behaviour(BehaviourEvent::Rpc(request_response::Event::Message {
                message,
                ..
            })) => match message {
                request_response::Message::Request {
                    request, channel, ..
                } if server => {
                    assert_eq!(request, b"request");
                    swarm
                        .behaviour_mut()
                        .rpc
                        .send_response(channel, b"response".to_vec())
                        .unwrap();
                }
                request_response::Message::Response { response, .. } if !server => {
                    assert_eq!(response, b"response");
                    rpc_done = true;
                }
                _ => {}
            },
            SwarmEvent::Behaviour(BehaviourEvent::Gossip(gossipsub::Event::Subscribed {
                peer_id,
                ..
            })) if !server => {
                assert_eq!(Some(peer_id), connected);
                published = swarm
                    .behaviour_mut()
                    .gossip
                    .publish(topic.clone(), b"gossip".to_vec())
                    .is_ok();
            }
            SwarmEvent::Behaviour(BehaviourEvent::Gossip(gossipsub::Event::Message {
                message,
                ..
            })) => {
                if server {
                    assert_eq!(message.data, b"gossip");
                    swarm
                        .behaviour_mut()
                        .gossip
                        .publish(topic.clone(), b"gossip-response".to_vec())
                        .unwrap();
                } else {
                    assert_eq!(message.data, b"gossip-response");
                    gossip_done = true;
                }
            }
            SwarmEvent::Behaviour(BehaviourEvent::Rpc(
                request_response::Event::OutboundFailure { error, .. },
            )) => panic!("RPC failure: {error}"),
            _ => {}
        }
        if !server && published && rpc_done && gossip_done {
            println!("PASS rpc gossip");
            return;
        }
    }
}
