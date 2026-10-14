use super::*;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Instant;

const HELPER: &str =
    "controlled_authority::replicated_protocol::transport::tests::process_tests::endpoint_process";

#[test]
#[ignore = "endpoint subprocess helper"]
fn endpoint_process() {
    let path = PathBuf::from(std::env::var_os("O7_RPC_PATH").unwrap());
    let role: EndpointRole = serde_json::from_str(&std::env::var("O7_RPC_ROLE").unwrap()).unwrap();
    let secret: [u8; 32] = hex::decode(std::env::var("O7_RPC_ENDPOINT_SECRET").unwrap())
        .unwrap()
        .try_into()
        .unwrap();
    let signer = ed25519_dalek::SigningKey::from_bytes(&secret);
    let endpoint = FileEndpoint::open(
        path.join("data"),
        trust(),
        role,
        signer,
        &trust().genesis_anchor().unwrap(),
    )
    .unwrap();
    let mut server = EndpointServer::new(endpoint, Duration::from_secs(2)).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    std::fs::write(
        path.join("ready.tmp"),
        listener.local_addr().unwrap().to_string(),
    )
    .unwrap();
    std::fs::rename(path.join("ready.tmp"), path.join("ready")).unwrap();
    loop {
        let _ = server.serve_one(&listener);
    }
}

struct Process(Child);
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn spawn(path: &Path, role: EndpointRole) -> (Process, std::net::SocketAddr) {
    std::fs::create_dir_all(path).unwrap();
    match std::fs::remove_file(path.join("ready")) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => panic!("{e}"),
    }
    let mut process = Process(
        Command::new(std::env::current_exe().unwrap())
            .args(["--exact", HELPER, "--ignored", "--nocapture"])
            .env("O7_RPC_PATH", path)
            .env("O7_RPC_ROLE", serde_json::to_string(&role).unwrap())
            .env(
                "O7_RPC_ENDPOINT_SECRET",
                hex::encode(
                    [if role == EndpointRole::Primary {
                        2u8
                    } else {
                        3u8
                    }; 32],
                ),
            )
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Ok(ready) = std::fs::read_to_string(path.join("ready")) {
            return (process, ready.parse().unwrap());
        }
        assert!(
            process.0.try_wait().unwrap().is_none(),
            "endpoint exited before ready"
        );
        assert!(Instant::now() < deadline, "endpoint readiness timeout");
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
#[cfg(unix)]
fn separate_endpoint_processes_reconcile_reply_loss_and_outage() {
    let directory = root();
    let (p_process, p_address) = spawn(&directory.join("p"), EndpointRole::Primary);
    let (r_process, r_address) = spawn(&directory.join("r"), EndpointRole::Replica);
    let genesis = trust().genesis_anchor().unwrap();
    let connect = |address, role, minimum| {
        RemoteEndpoint::connect(
            address,
            trust(),
            role,
            key(1),
            minimum,
            Duration::from_secs(2),
        )
        .unwrap()
    };
    let p = connect(p_address, EndpointRole::Primary, genesis.clone());
    let r = connect(r_address, EndpointRole::Replica, genesis.clone());
    let mut coordinator = ReplicatedCoordinator::new(p, r, key(1)).unwrap();
    let evidence = qualified(
        coordinator
            .submit(4, &genesis, request("original"), record())
            .unwrap(),
    );
    let head = verify_evidence(&evidence, &trust(), &genesis).unwrap();
    drop(coordinator); // Client lost its reply/projection; endpoints retain it.
    let mut coordinator = ReplicatedCoordinator::new(
        connect(p_address, EndpointRole::Primary, head.clone()),
        connect(r_address, EndpointRole::Replica, head.clone()),
        key(1),
    )
    .unwrap();
    assert_eq!(
        qualified(
            coordinator
                .submit(4, &genesis, request("original"), record())
                .unwrap()
        ),
        evidence
    );
    drop(r_process);
    assert!(matches!(
        coordinator
            .submit(4, &head, request("next"), record())
            .unwrap(),
        ProtocolOutcome::Unknown { .. }
    ));
    // An endpoint outage cannot produce a qualified reply or free the request identity.
    assert!(matches!(
        coordinator.lookup(&request("next")).unwrap(),
        ProtocolOutcome::Unknown { .. }
    ));
    drop((coordinator, p_process));
    let (p_restarted, p_address) = spawn(&directory.join("p"), EndpointRole::Primary);
    let (r_restarted, r_address) = spawn(&directory.join("r"), EndpointRole::Replica);
    let mut recovered = ReplicatedCoordinator::new(
        connect(p_address, EndpointRole::Primary, head.clone()),
        connect(r_address, EndpointRole::Replica, head),
        key(1),
    )
    .unwrap();
    assert_eq!(
        qualified(recovered.lookup(&request("original")).unwrap()),
        evidence
    );
    drop((recovered, p_restarted, r_restarted));
    std::fs::remove_dir_all(directory).unwrap();
}
