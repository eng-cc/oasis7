use super::*;
use oasis7_proto::distributed::DistributedErrorCode;
use oasis7_proto::distributed_dht::{PeerDeploymentMode, PeerNodeRole, PeerReachabilityClass};
use std::net::TcpListener;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

fn wait_until(what: &str, deadline: Instant, mut condition: impl FnMut() -> bool) {
    while Instant::now() < deadline {
        if condition() {
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("timed out waiting for condition: {what}");
}

fn test_peer_record(node_id: &str) -> PeerRecord {
    PeerRecord {
        peer_id: String::new(),
        node_id: node_id.to_string(),
        world_id: "world-a".to_string(),
        network_id: "world-a".to_string(),
        node_role: PeerNodeRole::FullStorage.as_str().to_string(),
        deployment_mode: PeerDeploymentMode::Private,
        reachability_class: PeerReachabilityClass::Private,
        direct_addrs: Vec::new(),
        hole_punch_addrs: Vec::new(),
        relay_addrs: Vec::new(),
        discovery_sources: vec![
            PeerDiscoverySource::StaticBootstrap,
            PeerDiscoverySource::Dht,
        ],
        capability_lanes: PeerNodeRole::FullStorage.default_capability_lanes(),
        source_operator: None,
        source_asn: None,
        published_at_ms: 0,
        ttl_ms: 60_000,
    }
}

fn listening_addr_with_peer_id(network: &Libp2pReplicationNetwork) -> Multiaddr {
    network
        .listening_addrs()
        .into_iter()
        .find(|addr| addr.to_string().contains("127.0.0.1"))
        .expect("listener visible addr")
        .with(libp2p::multiaddr::Protocol::P2p(network.peer_id().into()))
}

#[test]
fn libp2p_replication_network_generates_peer_id() {
    let network = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig::default());
    assert!(!network.peer_id().to_string().is_empty());
}

#[test]
fn libp2p_replication_network_request_rejects_without_connected_peers_by_default() {
    let network = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig::default());
    let result = network.request("/aw/node/replication/ping", b"hello");
    match result {
        Err(WorldError::NetworkProtocolUnavailable { protocol }) => {
            assert!(protocol.contains("no connected peers"));
        }
        other => panic!("expected NetworkProtocolUnavailable, got {other:?}"),
    }
}

#[test]
fn libp2p_replication_network_request_falls_back_to_local_handler_when_enabled() {
    let network = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig {
        allow_local_handler_fallback_when_no_peers: true,
        ..Libp2pReplicationNetworkConfig::default()
    });
    network
        .register_handler(
            "/aw/node/replication/ping",
            Box::new(|payload| {
                let mut out = payload.to_vec();
                out.extend_from_slice(b"-ok");
                Ok(out)
            }),
        )
        .expect("register local handler");

    let response = network
        .request("/aw/node/replication/ping", b"hello")
        .expect("local request");
    assert_eq!(response, b"hello-ok".to_vec());
}

#[test]
fn libp2p_replication_network_request_response_between_peers() {
    let listener = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig {
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".parse().expect("listener addr")],
        peer_record: Some(test_peer_record("listener")),
        ..Libp2pReplicationNetworkConfig::default()
    });
    let listen_deadline = Instant::now() + Duration::from_secs(10);
    wait_until("listener bind", listen_deadline, || {
        !listener.listening_addrs().is_empty()
    });

    let dial_addr = listening_addr_with_peer_id(&listener);
    listener
        .register_handler(
            "/aw/node/replication/ping",
            Box::new(|payload| {
                let mut out = payload.to_vec();
                out.extend_from_slice(b"-pong");
                Ok(out)
            }),
        )
        .expect("register listener handler");

    let dialer = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig {
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".parse().expect("dialer addr")],
        bootstrap_peers: vec![dial_addr],
        ..Libp2pReplicationNetworkConfig::default()
    });
    let connect_deadline = Instant::now() + Duration::from_secs(10);
    wait_until("dialer connection", connect_deadline, || {
        !dialer.connected_peers().is_empty()
    });

    let request_deadline = Instant::now() + Duration::from_secs(10);
    let mut last_result = None;
    while Instant::now() < request_deadline {
        let result = dialer.request("/aw/node/replication/ping", b"node");
        match &result {
            Ok(payload) if *payload == b"node-pong".to_vec() => return,
            _ => {}
        }
        last_result = Some(result);
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!(
        "request response timed out: last_result={last_result:?}; dialer_snapshot={:?}; listener_snapshot={:?}; dialer_errors={:?}; listener_errors={:?}",
        dialer.debug_snapshot(),
        listener.debug_snapshot(),
        dialer.debug_errors(),
        listener.debug_errors(),
    );
}

#[test]
fn libp2p_replication_network_redials_bootstrap_peer_until_listener_is_ready() {
    let reserved_listener =
        TcpListener::bind("127.0.0.1:0").expect("reserve bootstrap listener port");
    let bootstrap_port = reserved_listener
        .local_addr()
        .expect("reserved bootstrap addr")
        .port();
    drop(reserved_listener);

    let bootstrap_addr = format!("/ip4/127.0.0.1/tcp/{bootstrap_port}")
        .parse()
        .expect("bootstrap addr");
    let dialer = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig {
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".parse().expect("dialer addr")],
        bootstrap_peers: vec![bootstrap_addr],
        ..Libp2pReplicationNetworkConfig::default()
    });

    std::thread::sleep(Duration::from_millis(200));

    let listener = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig {
        listen_addrs: vec![format!("/ip4/127.0.0.1/tcp/{bootstrap_port}")
            .parse()
            .expect("listener addr")],
        peer_record: Some(test_peer_record("late-listener")),
        ..Libp2pReplicationNetworkConfig::default()
    });
    listener
        .register_handler(
            "/aw/node/replication/ping",
            Box::new(|payload| {
                let mut out = payload.to_vec();
                out.extend_from_slice(b"-late");
                Ok(out)
            }),
        )
        .expect("register listener handler");

    let connect_deadline = Instant::now() + Duration::from_secs(10);
    wait_until(
        "dialer reconnects to late listener",
        connect_deadline,
        || !dialer.connected_peers().is_empty(),
    );

    let listener_deadline = Instant::now() + Duration::from_secs(10);
    wait_until(
        "late listener sees connected peer",
        listener_deadline,
        || !listener.connected_peers().is_empty(),
    );
}

#[test]
fn libp2p_replication_network_request_waits_for_delayed_bootstrap_connection() {
    let reserved_listener =
        TcpListener::bind("127.0.0.1:0").expect("reserve bootstrap listener port");
    let bootstrap_port = reserved_listener
        .local_addr()
        .expect("reserved bootstrap addr")
        .port();
    drop(reserved_listener);

    let bootstrap_addr = format!("/ip4/127.0.0.1/tcp/{bootstrap_port}")
        .parse()
        .expect("bootstrap addr");
    let dialer = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig {
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".parse().expect("dialer addr")],
        bootstrap_peers: vec![bootstrap_addr],
        ..Libp2pReplicationNetworkConfig::default()
    });

    let listener_thread = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(250));
        let listener = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig {
            listen_addrs: vec![format!("/ip4/127.0.0.1/tcp/{bootstrap_port}")
                .parse()
                .expect("listener addr")],
            peer_record: Some(test_peer_record("delayed-listener")),
            ..Libp2pReplicationNetworkConfig::default()
        });
        listener
            .register_handler(
                "/aw/node/replication/ping",
                Box::new(|payload| {
                    let mut out = payload.to_vec();
                    out.extend_from_slice(b"-delayed");
                    Ok(out)
                }),
            )
            .expect("register delayed listener handler");
        std::thread::sleep(Duration::from_secs(3));
    });

    let response = dialer
        .request("/aw/node/replication/ping", b"node")
        .expect("request should wait for delayed connection");
    assert_eq!(response, b"node-delayed".to_vec());

    listener_thread
        .join()
        .expect("join delayed listener thread");
}

#[test]
fn filtered_request_peers_excludes_protocol_retry_cooldown_peers_without_fallback() {
    let network = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig::default());
    let observer_peer = PeerId::random();
    let sequencer_peer = PeerId::random();
    network.mark_peer_for_protocol_retry_cooldown(
        "/aw/node/replication/fetch-commit/1.0.0",
        observer_peer,
    );

    let filtered = network.filtered_request_peers(
        "/aw/node/replication/fetch-commit/1.0.0",
        vec![observer_peer, sequencer_peer],
    );
    assert_eq!(filtered, vec![sequencer_peer]);

    let filtered_only_cooldown = network.filtered_request_peers(
        "/aw/node/replication/fetch-commit/1.0.0",
        vec![observer_peer],
    );
    assert!(filtered_only_cooldown.is_empty());
}

#[test]
fn filtered_request_peers_excludes_transport_retry_cooldown_peers_across_protocols() {
    let network = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig::default());
    let observer_peer = PeerId::random();
    let sequencer_peer = PeerId::random();
    network.mark_peer_for_transport_retry_cooldown(observer_peer);

    let filtered = network.filtered_request_peers(
        "/aw/node/replication/fetch-commit/1.0.0",
        vec![observer_peer, sequencer_peer],
    );
    assert_eq!(filtered, vec![sequencer_peer]);

    let cross_protocol_filtered = network.filtered_request_peers(
        "/aw/node/replication/ping",
        vec![observer_peer, sequencer_peer],
    );
    assert_eq!(cross_protocol_filtered, vec![sequencer_peer]);
}

#[test]
fn filtered_request_peers_retries_protocol_retry_cooldown_peer_after_retry_window() {
    let network = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig {
        protocol_retry_cooldown_after: Duration::from_millis(5),
        ..Libp2pReplicationNetworkConfig::default()
    });
    let sequencer_peer = PeerId::random();
    network.mark_peer_for_protocol_retry_cooldown(
        "/aw/node/replication/fetch-commit/1.0.0",
        sequencer_peer,
    );

    let filtered_initial = network.filtered_request_peers(
        "/aw/node/replication/fetch-commit/1.0.0",
        vec![sequencer_peer],
    );
    assert!(filtered_initial.is_empty());

    std::thread::sleep(Duration::from_millis(15));

    let filtered_after_retry_window = network.filtered_request_peers(
        "/aw/node/replication/fetch-commit/1.0.0",
        vec![sequencer_peer],
    );
    assert_eq!(filtered_after_retry_window, vec![sequencer_peer]);
}

#[test]
fn filtered_request_peers_retries_transport_retry_cooldown_peer_after_retry_window() {
    let network = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig {
        protocol_retry_cooldown_after: Duration::from_millis(5),
        ..Libp2pReplicationNetworkConfig::default()
    });
    let sequencer_peer = PeerId::random();
    network.mark_peer_for_transport_retry_cooldown(sequencer_peer);

    let filtered_initial = network.filtered_request_peers(
        "/aw/node/replication/fetch-commit/1.0.0",
        vec![sequencer_peer],
    );
    assert!(filtered_initial.is_empty());

    std::thread::sleep(Duration::from_millis(15));

    let filtered_after_retry_window =
        network.filtered_request_peers("/aw/node/replication/ping", vec![sequencer_peer]);
    assert_eq!(filtered_after_retry_window, vec![sequencer_peer]);
}

#[test]
fn libp2p_replication_network_request_round_robins_across_connected_peers() {
    let listener_a = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig {
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".parse().expect("listener a addr")],
        peer_record: Some(test_peer_record("listener-a")),
        ..Libp2pReplicationNetworkConfig::default()
    });
    let listener_b = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig {
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".parse().expect("listener b addr")],
        peer_record: Some(test_peer_record("listener-b")),
        ..Libp2pReplicationNetworkConfig::default()
    });
    let listen_deadline = Instant::now() + Duration::from_secs(10);
    wait_until("listener a bind", listen_deadline, || {
        !listener_a.listening_addrs().is_empty()
    });
    wait_until("listener b bind", listen_deadline, || {
        !listener_b.listening_addrs().is_empty()
    });

    listener_a
        .register_handler(
            "/aw/node/replication/ping",
            Box::new(|payload| {
                let mut out = payload.to_vec();
                out.extend_from_slice(b"-a");
                Ok(out)
            }),
        )
        .expect("register listener a handler");
    listener_b
        .register_handler(
            "/aw/node/replication/ping",
            Box::new(|payload| {
                let mut out = payload.to_vec();
                out.extend_from_slice(b"-b");
                Ok(out)
            }),
        )
        .expect("register listener b handler");

    let dialer = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig {
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".parse().expect("dialer addr")],
        bootstrap_peers: vec![
            listening_addr_with_peer_id(&listener_a),
            listening_addr_with_peer_id(&listener_b),
        ],
        ..Libp2pReplicationNetworkConfig::default()
    });
    let connect_deadline = Instant::now() + Duration::from_secs(10);
    wait_until("dialer connects to two peers", connect_deadline, || {
        dialer.connected_peers().len() >= 2
    });

    let first = dialer
        .request("/aw/node/replication/ping", b"node")
        .expect("first request");
    let second = dialer
        .request("/aw/node/replication/ping", b"node")
        .expect("second request");

    assert_ne!(
        first, second,
        "expected round-robin request targets to differ"
    );
    let mut responses = vec![first, second];
    responses.sort();
    assert_eq!(responses, vec![b"node-a".to_vec(), b"node-b".to_vec()]);
}

#[test]
fn libp2p_replication_network_request_retries_next_peer_when_remote_handler_fails() {
    let listener_fail = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig {
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".parse().expect("listener fail addr")],
        peer_record: Some(test_peer_record("listener-fail")),
        ..Libp2pReplicationNetworkConfig::default()
    });
    let listener_ok = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig {
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".parse().expect("listener ok addr")],
        peer_record: Some(test_peer_record("listener-ok")),
        ..Libp2pReplicationNetworkConfig::default()
    });
    let listen_deadline = Instant::now() + Duration::from_secs(10);
    wait_until("listener fail bind", listen_deadline, || {
        !listener_fail.listening_addrs().is_empty()
    });
    wait_until("listener ok bind", listen_deadline, || {
        !listener_ok.listening_addrs().is_empty()
    });

    listener_fail
        .register_handler(
            "/aw/node/replication/ping",
            Box::new(|_payload| {
                Err(WorldError::NetworkRequestFailed {
                    code: DistributedErrorCode::ErrUnsupported,
                    message: "forced failure".to_string(),
                    retryable: false,
                })
            }),
        )
        .expect("register listener fail handler");
    listener_ok
        .register_handler(
            "/aw/node/replication/ping",
            Box::new(|payload| {
                let mut out = payload.to_vec();
                out.extend_from_slice(b"-ok");
                Ok(out)
            }),
        )
        .expect("register listener ok handler");

    let dialer = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig {
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".parse().expect("dialer addr")],
        bootstrap_peers: vec![
            listening_addr_with_peer_id(&listener_fail),
            listening_addr_with_peer_id(&listener_ok),
        ],
        ..Libp2pReplicationNetworkConfig::default()
    });
    let connect_deadline = Instant::now() + Duration::from_secs(10);
    wait_until("dialer connects to two peers", connect_deadline, || {
        dialer.connected_peers().len() >= 2
    });

    let first = dialer
        .request("/aw/node/replication/ping", b"node")
        .expect("first request should succeed via retry");
    let mut second = None;
    let second_deadline = Instant::now() + Duration::from_secs(2);
    wait_until(
        "second request succeeds via retry",
        second_deadline,
        || match dialer.request("/aw/node/replication/ping", b"node") {
            Ok(reply) => {
                second = Some(reply);
                true
            }
            Err(_) => false,
        },
    );
    let second = second.expect("second request should eventually succeed via retry");

    assert_eq!(first, b"node-ok".to_vec());
    assert_eq!(second, b"node-ok".to_vec());
}

#[test]
fn libp2p_replication_network_retries_previously_cooled_down_single_peer_after_retry_window() {
    let listener = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig {
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".parse().expect("listener addr")],
        peer_record: Some(test_peer_record("listener-not-found")),
        ..Libp2pReplicationNetworkConfig::default()
    });
    let listen_deadline = Instant::now() + Duration::from_secs(10);
    wait_until("listener bind", listen_deadline, || {
        !listener.listening_addrs().is_empty()
    });

    listener
        .register_handler(
            "/aw/node/replication/ping",
            Box::new(move |payload| {
                let mut out = payload.to_vec();
                out.extend_from_slice(b"-recovered");
                Ok(out)
            }),
        )
        .expect("register listener handler");

    let dialer = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig {
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".parse().expect("dialer addr")],
        bootstrap_peers: vec![listening_addr_with_peer_id(&listener)],
        protocol_retry_cooldown_after: Duration::from_millis(250),
        ..Libp2pReplicationNetworkConfig::default()
    });
    let connect_deadline = Instant::now() + Duration::from_secs(10);
    wait_until("dialer connection", connect_deadline, || {
        !dialer.connected_peers().is_empty()
    });

    let listener_peer_id = listener.peer_id();
    dialer.mark_peer_for_protocol_retry_cooldown("/aw/node/replication/ping", listener_peer_id);

    let immediate_retry = dialer.request("/aw/node/replication/ping", b"node");
    if let Ok(payload) = &immediate_retry {
        assert_eq!(payload, &b"node-recovered".to_vec());
    } else {
        assert!(matches!(
            immediate_retry,
            Err(WorldError::NetworkProtocolUnavailable { .. })
        ));
    }

    std::thread::sleep(Duration::from_millis(300));

    let recovered = dialer
        .request("/aw/node/replication/ping", b"node")
        .expect("request after retry window");
    assert_eq!(recovered, b"node-recovered".to_vec());
}

#[test]
fn libp2p_replication_network_does_not_quarantine_not_found_response_as_unsupported() {
    let listener = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig {
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".parse().expect("listener addr")],
        peer_record: Some(test_peer_record("listener-unsupported")),
        ..Libp2pReplicationNetworkConfig::default()
    });
    let listen_deadline = Instant::now() + Duration::from_secs(10);
    wait_until("listener bind", listen_deadline, || {
        !listener.listening_addrs().is_empty()
    });

    listener
        .register_handler(
            "/aw/node/replication/ping",
            Box::new(|_payload| {
                Err(WorldError::NetworkRequestFailed {
                    code: DistributedErrorCode::ErrNotFound,
                    message: "missing content".to_string(),
                    retryable: false,
                })
            }),
        )
        .expect("register listener handler");

    let dialer = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig {
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".parse().expect("dialer addr")],
        bootstrap_peers: vec![listening_addr_with_peer_id(&listener)],
        protocol_retry_cooldown_after: Duration::from_millis(250),
        ..Libp2pReplicationNetworkConfig::default()
    });
    let connect_deadline = Instant::now() + Duration::from_secs(10);
    wait_until("dialer connection", connect_deadline, || {
        !dialer.connected_peers().is_empty()
    });

    let first = dialer.request("/aw/node/replication/ping", b"node");
    assert!(matches!(
        first,
        Err(WorldError::NetworkRequestFailed {
            code: DistributedErrorCode::ErrNotFound,
            ..
        })
    ));

    let second = dialer.request("/aw/node/replication/ping", b"node");
    assert!(matches!(
        second,
        Err(WorldError::NetworkRequestFailed {
            code: DistributedErrorCode::ErrNotFound,
            ..
        })
    ));
}

#[test]
fn fetch_commit_not_found_retry_cooldown_is_protocol_scoped_but_connection_gaps_are_not() {
    let not_found = WorldError::NetworkRequestFailed {
        code: DistributedErrorCode::ErrNotFound,
        message: "missing content".to_string(),
        retryable: false,
    };
    assert!(peer_error_indicates_protocol_retry_cooldown(
        crate::replication::REPLICATION_FETCH_COMMIT_PROTOCOL,
        &not_found,
    ));
    assert!(!peer_error_indicates_protocol_retry_cooldown(
        "/aw/node/replication/ping",
        &not_found,
    ));

    let timeout = WorldError::NetworkProtocolUnavailable {
        protocol: "libp2p-replication outbound request failed: request failed: Timeout".to_string(),
    };
    assert!(!peer_error_indicates_protocol_retry_cooldown(
        crate::replication::REPLICATION_FETCH_COMMIT_PROTOCOL,
        &timeout,
    ));
    assert!(!peer_error_indicates_protocol_retry_cooldown(
        "/aw/node/replication/ping",
        &timeout,
    ));
    assert!(peer_error_indicates_transport_retry_cooldown(&timeout));

    let business_unsupported = WorldError::NetworkRequestFailed {
        code: DistributedErrorCode::ErrUnsupported,
        message: "forced unsupported".to_string(),
        retryable: false,
    };
    assert!(!peer_error_indicates_protocol_retry_cooldown(
        crate::replication::REPLICATION_FETCH_COMMIT_PROTOCOL,
        &business_unsupported,
    ));
}

#[test]
fn libp2p_replication_network_connection_gap_cools_peer_across_protocols() {
    let listener = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig {
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".parse().expect("listener addr")],
        peer_record: Some(test_peer_record("listener-cross-protocol-timeout")),
        ..Libp2pReplicationNetworkConfig::default()
    });
    let listen_deadline = Instant::now() + Duration::from_secs(10);
    wait_until("listener bind", listen_deadline, || {
        !listener.listening_addrs().is_empty()
    });

    let ping_request_count = Arc::new(AtomicUsize::new(0));
    listener
        .register_handler(
            "/aw/node/replication/ping",
            Box::new({
                let ping_request_count = Arc::clone(&ping_request_count);
                move |_payload| {
                    ping_request_count.fetch_add(1, Ordering::SeqCst);
                    Err(WorldError::NetworkProtocolUnavailable {
                        protocol:
                            "libp2p-replication outbound request failed: request failed: Timeout"
                                .to_string(),
                    })
                }
            }),
        )
        .expect("register ping listener handler");

    let fetch_commit_request_count = Arc::new(AtomicUsize::new(0));
    listener
        .register_handler(
            crate::replication::REPLICATION_FETCH_COMMIT_PROTOCOL,
            Box::new({
                let fetch_commit_request_count = Arc::clone(&fetch_commit_request_count);
                move |_payload| {
                    fetch_commit_request_count.fetch_add(1, Ordering::SeqCst);
                    Ok(b"commit-ok".to_vec())
                }
            }),
        )
        .expect("register fetch-commit listener handler");

    let dialer = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig {
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".parse().expect("dialer addr")],
        bootstrap_peers: vec![listening_addr_with_peer_id(&listener)],
        protocol_retry_cooldown_after: Duration::from_millis(250),
        ..Libp2pReplicationNetworkConfig::default()
    });
    let connect_deadline = Instant::now() + Duration::from_secs(10);
    wait_until("dialer connection", connect_deadline, || {
        !dialer.connected_peers().is_empty()
    });

    let first = dialer.request("/aw/node/replication/ping", b"node");
    assert!(matches!(
        first,
        Err(WorldError::NetworkRequestFailed {
            code: DistributedErrorCode::ErrNotAvailable,
            ..
        })
    ));
    assert!(ping_request_count.load(Ordering::SeqCst) > 0);

    let second = dialer.request(
        crate::replication::REPLICATION_FETCH_COMMIT_PROTOCOL,
        b"node",
    );
    assert!(matches!(
        second,
        Err(WorldError::NetworkProtocolUnavailable { .. })
    ));
    assert_eq!(
        fetch_commit_request_count.load(Ordering::SeqCst),
        0,
        "transport cooldown should suppress immediate cross-protocol reuse of the same peer"
    );

    std::thread::sleep(Duration::from_millis(300));

    let third = dialer
        .request(
            crate::replication::REPLICATION_FETCH_COMMIT_PROTOCOL,
            b"node",
        )
        .expect("fetch commit should recover after transport cooldown");
    assert_eq!(third, b"commit-ok".to_vec());
    assert_eq!(fetch_commit_request_count.load(Ordering::SeqCst), 1);
}

#[test]
fn libp2p_replication_network_fetch_commit_not_found_enters_short_cooldown() {
    let listener = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig {
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".parse().expect("listener addr")],
        peer_record: Some(test_peer_record("listener-fetch-commit-not-found")),
        ..Libp2pReplicationNetworkConfig::default()
    });
    let listen_deadline = Instant::now() + Duration::from_secs(10);
    wait_until("listener bind", listen_deadline, || {
        !listener.listening_addrs().is_empty()
    });

    let request_count = Arc::new(AtomicUsize::new(0));
    listener
        .register_handler(
            crate::replication::REPLICATION_FETCH_COMMIT_PROTOCOL,
            Box::new({
                let request_count = Arc::clone(&request_count);
                move |_payload| {
                    request_count.fetch_add(1, Ordering::SeqCst);
                    Err(WorldError::NetworkRequestFailed {
                        code: DistributedErrorCode::ErrNotFound,
                        message: "missing commit".to_string(),
                        retryable: false,
                    })
                }
            }),
        )
        .expect("register listener handler");

    let dialer = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig {
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".parse().expect("dialer addr")],
        bootstrap_peers: vec![listening_addr_with_peer_id(&listener)],
        protocol_retry_cooldown_after: Duration::from_millis(250),
        ..Libp2pReplicationNetworkConfig::default()
    });
    let connect_deadline = Instant::now() + Duration::from_secs(10);
    wait_until("dialer connection", connect_deadline, || {
        !dialer.connected_peers().is_empty()
    });

    let first = dialer.request(
        crate::replication::REPLICATION_FETCH_COMMIT_PROTOCOL,
        b"node",
    );
    assert!(matches!(
        first,
        Err(WorldError::NetworkRequestFailed {
            code: DistributedErrorCode::ErrNotFound,
            ..
        })
    ));
    assert_eq!(request_count.load(Ordering::SeqCst), 1);

    let second = dialer.request(
        crate::replication::REPLICATION_FETCH_COMMIT_PROTOCOL,
        b"node",
    );
    assert!(matches!(
        second,
        Err(WorldError::NetworkProtocolUnavailable { .. })
    ));
    assert_eq!(
        request_count.load(Ordering::SeqCst),
        1,
        "cooldown should suppress an immediate second fetch-commit request"
    );

    std::thread::sleep(Duration::from_millis(300));

    let third = dialer.request(
        crate::replication::REPLICATION_FETCH_COMMIT_PROTOCOL,
        b"node",
    );
    assert!(matches!(
        third,
        Err(WorldError::NetworkRequestFailed {
            code: DistributedErrorCode::ErrNotFound,
            ..
        })
    ));
    assert_eq!(request_count.load(Ordering::SeqCst), 2);
}

#[test]
fn libp2p_replication_network_connection_gap_on_ping_enters_short_cooldown() {
    let listener = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig {
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".parse().expect("listener addr")],
        peer_record: Some(test_peer_record("listener-ping-timeout")),
        ..Libp2pReplicationNetworkConfig::default()
    });
    let listen_deadline = Instant::now() + Duration::from_secs(10);
    wait_until("listener bind", listen_deadline, || {
        !listener.listening_addrs().is_empty()
    });

    let request_count = Arc::new(AtomicUsize::new(0));
    listener
        .register_handler(
            "/aw/node/replication/ping",
            Box::new({
                let request_count = Arc::clone(&request_count);
                move |_payload| {
                    request_count.fetch_add(1, Ordering::SeqCst);
                    Err(WorldError::NetworkProtocolUnavailable {
                        protocol:
                            "libp2p-replication outbound request failed: request failed: Timeout"
                                .to_string(),
                    })
                }
            }),
        )
        .expect("register listener handler");

    let dialer = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig {
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".parse().expect("dialer addr")],
        bootstrap_peers: vec![listening_addr_with_peer_id(&listener)],
        protocol_retry_cooldown_after: Duration::from_millis(250),
        ..Libp2pReplicationNetworkConfig::default()
    });
    let connect_deadline = Instant::now() + Duration::from_secs(10);
    wait_until("dialer connection", connect_deadline, || {
        !dialer.connected_peers().is_empty()
    });

    let first = dialer.request("/aw/node/replication/ping", b"node");
    assert!(matches!(
        first,
        Err(WorldError::NetworkRequestFailed {
            code: DistributedErrorCode::ErrNotAvailable,
            ..
        })
    ));
    let request_count_after_first = request_count.load(Ordering::SeqCst);
    assert!(
        request_count_after_first > 0,
        "first ping request should reach the remote handler at least once"
    );

    let second = dialer.request("/aw/node/replication/ping", b"node");
    assert!(matches!(
        second,
        Err(WorldError::NetworkProtocolUnavailable { .. })
    ));
    assert_eq!(
        request_count.load(Ordering::SeqCst),
        request_count_after_first,
        "cooldown should suppress an immediate second ping request after a connection gap"
    );

    std::thread::sleep(Duration::from_millis(300));

    let third = dialer.request("/aw/node/replication/ping", b"node");
    assert!(matches!(
        third,
        Err(WorldError::NetworkRequestFailed {
            code: DistributedErrorCode::ErrNotAvailable,
            ..
        })
    ));
    assert!(
        request_count.load(Ordering::SeqCst) > request_count_after_first,
        "request count should grow again after the short cooldown expires"
    );
}

#[test]
fn retryable_connection_gap_detection_matches_request_to_peer_disconnects() {
    let err = WorldError::NetworkProtocolUnavailable {
        protocol: "libp2p-replication outbound request failed: NetworkProtocolUnavailable { protocol: \"peer 12D3KooW... is not connected for protocol /aw/node/replication/fetch-commit/1.0.0\" }".to_string(),
    };
    assert!(peer_error_indicates_retryable_connection_gap(&err));

    let connection_closed = WorldError::NetworkProtocolUnavailable {
        protocol: "libp2p-replication outbound request failed: request failed: ConnectionClosed"
            .to_string(),
    };
    assert!(peer_error_indicates_retryable_connection_gap(
        &connection_closed
    ));

    let legacy_remote_gap = WorldError::NetworkRequestFailed {
        code: DistributedErrorCode::ErrUnsupported,
        message: "no connected providers for protocol /aw/node/replication/fetch-commit/1.0.0"
            .to_string(),
        retryable: false,
    };
    assert!(peer_error_indicates_retryable_connection_gap(
        &legacy_remote_gap
    ));

    let admissible_gap = WorldError::NetworkProtocolUnavailable {
        protocol:
            "libp2p-replication no admissible connected peers for protocol /aw/node/replication/fetch-commit/1.0.0"
                .to_string(),
    };
    assert!(peer_error_indicates_retryable_connection_gap(
        &admissible_gap
    ));

    let healthy_provider_gap = WorldError::NetworkRequestFailed {
        code: DistributedErrorCode::ErrUnsupported,
        message:
            "no healthy connected providers for protocol /aw/node/replication/fetch-commit/1.0.0"
                .to_string(),
        retryable: false,
    };
    assert!(peer_error_indicates_retryable_connection_gap(
        &healthy_provider_gap
    ));

    let not_retryable = WorldError::NetworkProtocolUnavailable {
        protocol: "libp2p-replication handler missing: /aw/node/replication/fetch-commit/1.0.0"
            .to_string(),
    };
    assert!(!peer_error_indicates_retryable_connection_gap(
        &not_retryable
    ));
}

#[test]
fn unsupported_protocol_detection_ignores_generic_availability_failures() {
    let transient = WorldError::NetworkRequestFailed {
        code: DistributedErrorCode::ErrUnsupported,
        message: "no connected providers for protocol /aw/node/replication/fetch-commit/1.0.0"
            .to_string(),
        retryable: false,
    };
    assert!(!peer_error_indicates_unsupported_protocol(&transient));

    let missing_handler = WorldError::NetworkRequestFailed {
        code: DistributedErrorCode::ErrUnsupported,
        message: "/aw/node/replication/fetch-commit/1.0.0".to_string(),
        retryable: false,
    };
    assert!(peer_error_indicates_unsupported_protocol(&missing_handler));

    let forced_unsupported = WorldError::NetworkRequestFailed {
        code: DistributedErrorCode::ErrUnsupported,
        message: "forced unsupported".to_string(),
        retryable: false,
    };
    assert!(!peer_error_indicates_unsupported_protocol(
        &forced_unsupported
    ));

    let transient_internal = WorldError::NetworkRequestFailed {
        code: DistributedErrorCode::ErrNotAvailable,
        message: "remote temporarily unavailable".to_string(),
        retryable: true,
    };
    assert!(!peer_error_indicates_retryable_connection_gap(
        &transient_internal
    ));
}

#[test]
fn libp2p_replication_network_preserves_remote_unsupported_error_code() {
    let listener = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig {
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".parse().expect("listener addr")],
        peer_record: Some(test_peer_record("listener-unsupported")),
        ..Libp2pReplicationNetworkConfig::default()
    });
    let listen_deadline = Instant::now() + Duration::from_secs(10);
    wait_until("listener bind", listen_deadline, || {
        !listener.listening_addrs().is_empty()
    });

    listener
        .register_handler(
            "/aw/node/replication/ping",
            Box::new(|_payload| {
                Err(WorldError::NetworkRequestFailed {
                    code: DistributedErrorCode::ErrUnsupported,
                    message: "forced unsupported".to_string(),
                    retryable: false,
                })
            }),
        )
        .expect("register listener handler");

    let dialer = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig {
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".parse().expect("dialer addr")],
        bootstrap_peers: vec![listening_addr_with_peer_id(&listener)],
        protocol_retry_cooldown_after: Duration::from_millis(250),
        ..Libp2pReplicationNetworkConfig::default()
    });
    let connect_deadline = Instant::now() + Duration::from_secs(10);
    wait_until("dialer connection", connect_deadline, || {
        !dialer.connected_peers().is_empty()
    });

    let err = dialer
        .request("/aw/node/replication/ping", b"node")
        .expect_err("unsupported remote handler must bubble its code");
    assert!(matches!(
        err,
        WorldError::NetworkRequestFailed {
            code: DistributedErrorCode::ErrUnsupported,
            ..
        }
    ));

    let second_err = dialer
        .request("/aw/node/replication/ping", b"node")
        .expect_err("business unsupported must not quarantine the only peer");
    assert!(matches!(
        second_err,
        WorldError::NetworkRequestFailed {
            code: DistributedErrorCode::ErrUnsupported,
            ..
        }
    ));
}
