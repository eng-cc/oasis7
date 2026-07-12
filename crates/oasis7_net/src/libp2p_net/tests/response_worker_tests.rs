use super::*;
use std::sync::mpsc;
use std::time::{Duration, Instant};

const FETCH_BLOB: &str = "/aw/node/replication/fetch-blob/1.0.0";
const GET_HEAD: &str = "/aw/rr/1.0.0/get_world_head";
const FETCH_COMMIT: &str = "/aw/node/replication/fetch-commit/1.0.0";

fn wait_until(what: &str, deadline: Instant, mut condition: impl FnMut() -> bool) {
    while Instant::now() < deadline {
        if condition() {
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("timed out waiting for {what}");
}

#[test]
fn slow_blob_handler_does_not_block_head_or_commit_responses() {
    let server = Libp2pNetwork::new(Libp2pNetworkConfig {
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".parse().expect("listen addr")],
        ..Libp2pNetworkConfig::default()
    });
    wait_until(
        "server listen",
        Instant::now() + Duration::from_secs(5),
        || !server.listening_addrs().is_empty(),
    );
    let server_addr = server
        .listening_addrs()
        .into_iter()
        .next()
        .expect("server address")
        .with(libp2p::multiaddr::Protocol::P2p(server.peer_id().into()));
    let (started_tx, started_rx) = mpsc::channel();
    server
        .register_handler(
            FETCH_BLOB,
            Box::new(move |_| {
                started_tx.send(()).expect("report slow blob start");
                std::thread::sleep(Duration::from_millis(400));
                Ok(b"blob".to_vec())
            }),
        )
        .expect("register blob handler");
    server
        .register_handler(GET_HEAD, Box::new(|_| Ok(b"head".to_vec())))
        .expect("register head handler");
    server
        .register_handler(FETCH_COMMIT, Box::new(|_| Ok(b"commit".to_vec())))
        .expect("register commit handler");

    let client = Libp2pNetwork::new(Libp2pNetworkConfig {
        bootstrap_peers: vec![server_addr],
        ..Libp2pNetworkConfig::default()
    });
    wait_until(
        "client connect",
        Instant::now() + Duration::from_secs(5),
        || client.connected_peers().contains(&server.peer_id()),
    );

    let blob_client = client.clone();
    let server_peer = server.peer_id();
    let blob =
        std::thread::spawn(move || blob_client.request_to_peer(FETCH_BLOB, b"blob", server_peer));
    started_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("slow blob handler started");

    let deadline = Instant::now() + Duration::from_millis(300);
    assert_eq!(
        client
            .request_to_peer(GET_HEAD, b"head", server.peer_id())
            .expect("head response"),
        b"head"
    );
    assert_eq!(
        client
            .request_to_peer(FETCH_COMMIT, b"commit", server.peer_id())
            .expect("commit response"),
        b"commit"
    );
    assert!(
        Instant::now() < deadline,
        "control responses waited for the slow blob handler"
    );
    assert_eq!(
        blob.join()
            .expect("blob request thread")
            .expect("blob response"),
        b"blob"
    );
}

#[test]
fn timed_out_slow_response_surfaces_as_retryable_transport_failure() {
    let server = Libp2pNetwork::new(Libp2pNetworkConfig {
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".parse().expect("listen addr")],
        ..Libp2pNetworkConfig::default()
    });
    wait_until(
        "server listen",
        Instant::now() + Duration::from_secs(5),
        || !server.listening_addrs().is_empty(),
    );
    let server_addr = server
        .listening_addrs()
        .into_iter()
        .next()
        .expect("server address")
        .with(libp2p::multiaddr::Protocol::P2p(server.peer_id().into()));
    server
        .register_handler(
            FETCH_BLOB,
            Box::new(|_| {
                std::thread::sleep(Duration::from_millis(200));
                Ok(b"late blob".to_vec())
            }),
        )
        .expect("register blob handler");

    let client = Libp2pNetwork::new(Libp2pNetworkConfig {
        bootstrap_peers: vec![server_addr],
        request_response_timeout: Duration::from_millis(50),
        ..Libp2pNetworkConfig::default()
    });
    wait_until(
        "client connect",
        Instant::now() + Duration::from_secs(5),
        || client.connected_peers().contains(&server.peer_id()),
    );

    let error = client
        .request_to_peer(FETCH_BLOB, b"blob", server.peer_id())
        .expect_err("late response must time out");
    assert!(super::super::error_mapping::world_error_is_retryable_connection_gap(&error));
    assert!(matches!(
        error,
        WorldError::NetworkRequestFailed {
            retryable: true,
            ..
        }
    ));
}
