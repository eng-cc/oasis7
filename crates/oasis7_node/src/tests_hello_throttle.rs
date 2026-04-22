use super::super::gossip_udp::GossipEndpoint;
use super::super::{NodeConfig, NodeRole, PosNodeEngine, PosValidator};
use super::gossip_config;
use std::net::UdpSocket;
use std::time::Duration;

#[test]
fn observer_reverse_path_hello_is_rate_limited_across_ticks() {
    let socket_a = UdpSocket::bind("127.0.0.1:0").expect("bind a");
    let socket_b = UdpSocket::bind("127.0.0.1:0").expect("bind b");
    let addr_a = socket_a.local_addr().expect("addr a");
    let addr_b = socket_b.local_addr().expect("addr b");
    drop(socket_a);
    drop(socket_b);

    let validators = vec![
        PosValidator {
            validator_id: "node-a".to_string(),
            stake: 60,
        },
        PosValidator {
            validator_id: "node-b".to_string(),
            stake: 40,
        },
    ];

    let config = NodeConfig::new("node-b", "world-hello-rate-limit", NodeRole::Observer)
        .expect("config")
        .with_tick_interval(Duration::from_millis(200))
        .expect("tick")
        .with_pos_validators(validators)
        .expect("validators")
        .with_gossip_optional(addr_b, vec![addr_a]);
    let mut engine = PosNodeEngine::new(&config).expect("engine");
    let endpoint = GossipEndpoint::bind(&gossip_config(addr_b, vec![addr_a])).expect("endpoint");

    for now_ms in [1_000_i64, 1_200, 1_400, 1_600] {
        engine
            .tick(
                &config.node_id,
                &config.world_id,
                now_ms,
                Some(&endpoint),
                None,
                None,
                None,
                Vec::new(),
                None,
            )
            .expect("tick within cooldown");
    }

    let first_snapshot = endpoint.traffic_metrics_snapshot();
    assert_eq!(
        first_snapshot
            .by_kind
            .get("hello")
            .map(|lane| lane.outbound.datagrams),
        Some(1),
        "observer should seed reverse path once within the cooldown window"
    );

    engine
        .tick(
            &config.node_id,
            &config.world_id,
            6_200,
            Some(&endpoint),
            None,
            None,
            None,
            Vec::new(),
            None,
        )
        .expect("tick after cooldown");

    let second_snapshot = endpoint.traffic_metrics_snapshot();
    assert_eq!(
        second_snapshot
            .by_kind
            .get("hello")
            .map(|lane| lane.outbound.datagrams),
        Some(2),
        "observer should reseed reverse path after the cooldown elapses"
    );
}
