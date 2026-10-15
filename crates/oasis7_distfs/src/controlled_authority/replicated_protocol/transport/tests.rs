use super::super::tests::{key, qualified, record, request, root, trust};
use super::{wire::*, *};
use std::net::TcpListener;
use std::time::Duration;

fn start(
    role: EndpointRole,
    path: std::path::PathBuf,
) -> (
    std::net::SocketAddr,
    std::sync::Arc<std::sync::atomic::AtomicBool>,
    std::thread::JoinHandle<()>,
) {
    use std::sync::atomic::{AtomicBool, Ordering};
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let stop = std::sync::Arc::new(AtomicBool::new(false));
    let worker_stop = stop.clone();
    let worker = std::thread::spawn(move || {
        let endpoint = FileEndpoint::open(
            path,
            trust(),
            role,
            key(if role == EndpointRole::Primary { 2 } else { 3 }),
            &trust().genesis_anchor().unwrap(),
        )
        .unwrap();
        let mut server = EndpointServer::new(endpoint, Duration::from_secs(2)).unwrap();
        while !worker_stop.load(Ordering::Acquire) {
            if server.serve_one(&listener).is_err() {
                std::thread::sleep(Duration::from_millis(2));
            }
        }
    });
    (address, stop, worker)
}

#[test]
#[cfg(unix)]
fn signed_rpc_submission_and_retry_use_same_evidence() {
    let directory = root();
    let (p_address, p_stop, p_worker) = start(EndpointRole::Primary, directory.join("p"));
    let (r_address, r_stop, r_worker) = start(EndpointRole::Replica, directory.join("r"));
    let genesis = trust().genesis_anchor().unwrap();
    let p = RemoteEndpoint::connect(
        p_address,
        trust(),
        EndpointRole::Primary,
        key(1),
        genesis.clone(),
        Duration::from_secs(2),
    )
    .unwrap();
    let r = RemoteEndpoint::connect(
        r_address,
        trust(),
        EndpointRole::Replica,
        key(1),
        genesis.clone(),
        Duration::from_secs(2),
    )
    .unwrap();
    let mut coordinator = ReplicatedCoordinator::new(p, r, key(1)).unwrap();
    let evidence = qualified(
        coordinator
            .submit(4, &genesis, request("rpc"), record())
            .unwrap(),
    );
    assert_eq!(
        qualified(
            coordinator
                .submit(4, &genesis, request("rpc"), record())
                .unwrap()
        ),
        evidence
    );
    assert_eq!(
        qualified(coordinator.lookup(&request("rpc")).unwrap()),
        evidence
    );
    drop(coordinator);
    p_stop.store(true, std::sync::atomic::Ordering::Release);
    r_stop.store(true, std::sync::atomic::Ordering::Release);
    p_worker.join().unwrap();
    r_worker.join().unwrap();
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
#[cfg(unix)]
fn server_rejects_unauthenticated_query_without_disclosing_status() {
    let directory = root();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let worker = std::thread::spawn(move || {
        let endpoint = FileEndpoint::open(
            directory.join("p"),
            trust(),
            EndpointRole::Primary,
            key(2),
            &trust().genesis_anchor().unwrap(),
        )
        .unwrap();
        let mut server = EndpointServer::new(endpoint, Duration::from_secs(2)).unwrap();
        assert!(server.serve_one(&listener).is_err());
        drop(server);
        std::fs::remove_dir_all(directory).unwrap();
    });
    let body = RequestBody {
        version: 1,
        trust: trust(),
        role: EndpointRole::Primary,
        challenge: "a".repeat(64),
        minimum: trust().genesis_anchor().unwrap(),
        command: Command::Head,
    };
    let signature = crypto::sign(REQUEST_DOMAIN, &body, &key(9)).unwrap();
    let mut stream = std::net::TcpStream::connect(address).unwrap();
    configure(&stream, Duration::from_secs(2)).unwrap();
    write(&mut stream, &Request { body, signature }).unwrap();
    assert!(read::<Response>(&mut stream).is_err());
    worker.join().unwrap();
}

#[test]
fn remote_transport_requires_tunnel_and_finite_timeout() {
    for (address, timeout) in [("192.0.2.1:1", 2), ("127.0.0.1:1", 0), ("127.0.0.1:1", 61)] {
        assert!(
            RemoteEndpoint::connect(
                address.parse().unwrap(),
                trust(),
                EndpointRole::Primary,
                key(1),
                trust().genesis_anchor().unwrap(),
                Duration::from_secs(timeout)
            )
            .is_err()
        );
    }
}

#[test]
#[cfg(unix)]
fn stale_or_forged_signed_response_cannot_qualify_another_request() {
    for wrong_request in [false, true] {
        let directory = root();
        let mut local = super::super::tests::coordinator(&directory);
        let genesis = trust().genesis_anchor().unwrap();
        let evidence = qualified(local.submit(4, &genesis, request("old"), record()).unwrap());
        let head = verify_evidence(&evidence, &trust(), &genesis).unwrap();
        drop(local);
        let mut workers = Vec::new();
        let mut clients = Vec::new();
        for role in [EndpointRole::Primary, EndpointRole::Replica] {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let evidence = evidence.clone();
            let head = head.clone();
            workers.push(std::thread::spawn(move || {
                for _ in 0..2 {
                    let (mut stream, _) = listener.accept().unwrap();
                    configure(&stream, Duration::from_secs(2)).unwrap();
                    let request: Request = read(&mut stream).unwrap();
                    let is_head = matches!(request.body.command, Command::Head);
                    let result = if is_head {
                        Value::Head
                    } else {
                        Value::Status(EndpointStatus::Qualified(Box::new(evidence.clone())))
                    };
                    let mut body = ResponseBody {
                        request_digest: crypto::hash(REQUEST_DOMAIN, &request.body).unwrap(),
                        trust: trust(),
                        role,
                        head: head.clone(),
                        result: Ok(result),
                    };
                    // Even a correctly signed wrapper from the endpoint cannot
                    // substitute an old challenge or another request's evidence.
                    if !is_head && !wrong_request {
                        body.request_digest = "0".repeat(64);
                    }
                    let signature = crypto::sign(
                        RESPONSE_DOMAIN,
                        &body,
                        &key(if role == EndpointRole::Primary { 2 } else { 3 }),
                    )
                    .unwrap();
                    write(&mut stream, &Response { body, signature }).unwrap();
                }
            }));
            clients.push(
                RemoteEndpoint::connect(
                    address,
                    trust(),
                    role,
                    key(1),
                    genesis.clone(),
                    Duration::from_secs(2),
                )
                .unwrap(),
            );
        }
        let r = clients.pop().unwrap();
        let p = clients.pop().unwrap();
        let mut coordinator = ReplicatedCoordinator::new(p, r, key(1)).unwrap();
        assert!(matches!(
            coordinator.lookup(&request("new")).unwrap(),
            ProtocolOutcome::Unknown { .. }
        ));
        let (p, mut r) = coordinator.into_endpoints();
        // The first invalid result stops reconciliation before R is contacted.
        r.call(Command::Head).unwrap();
        drop((p, r));
        for worker in workers {
            worker.join().unwrap();
        }
        std::fs::remove_dir_all(directory).unwrap();
    }
}

#[path = "process_tests.rs"]
mod process_tests;
