//! Actual SDK device files and signed HTTP reads; no provider transport is invoked.
use super::*;
use crate::hosted_identity::{IdentityService, model::PlayerBinding};
use oasis7_agent_api::game::GameTaskControlV1;
use oasis7_client_api::world_service::*;
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
    time::Duration,
};

struct SignedWorld {
    endpoint: String,
    control: Arc<Mutex<GameTaskControlV1>>,
    stop: Arc<AtomicBool>,
    reads: Arc<AtomicUsize>,
    worker: Option<std::thread::JoinHandle<()>>,
}
impl SignedWorld {
    fn new(mut f: GameTaskFenceV1, public: &str) -> Self {
        f.executor_binding.as_mut().unwrap().executor_public_key = public.into();
        let mut value = serde_json::to_value(f).unwrap();
        value["schema_revision"] = serde_json::json!(1);
        value["executor_id"] = serde_json::json!("executor");
        value["created_operation_id"] = serde_json::json!("create");
        let control: GameTaskControlV1 = serde_json::from_value(value).unwrap();
        let control = Arc::new(Mutex::new(control));
        let stop = Arc::new(AtomicBool::new(false));
        let reads = Arc::new(AtomicUsize::new(0));
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let (c, s, r) = (control.clone(), stop.clone(), reads.clone());
        let expected_public = public.to_owned();
        let worker = std::thread::spawn(move || {
            while !s.load(Ordering::SeqCst) {
                let mut stream = match listener.accept() {
                    Ok((stream, _)) => stream,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(2));
                        continue;
                    }
                    Err(e) => panic!("signed fixture accept: {e}"),
                };
                let proof = read_request(&mut stream);
                verify_read_request(VIEW_JSON_PATH, &proof).unwrap();
                assert_eq!(proof.subject_public_key, expected_public);
                assert_eq!(proof.request.scope_id, "agent:agent");
                let control = c.lock().unwrap().clone();
                let world = WorldIdentity {
                    world_id: control.world_id.clone(),
                    genesis_digest: control.genesis_digest.clone(),
                };
                assert_eq!(proof.request.world, world);
                let commit = CommitRef {
                    world,
                    binding: ExecutionBinding {
                        provider_world_id: control.world_id.clone(),
                        branch_id: control.branch_id.clone(),
                        finality_ref: "finality".into(),
                        reorg_generation: control.reorg_epoch.0,
                        governing_manifest_ref: "manifest".into(),
                        authority_generation: 1,
                        permission_generation: 1,
                    },
                    position: 7,
                    execution_block_hash: "block".into(),
                    state_root_ref: "root".into(),
                };
                let payload = ReadWorldViewResponse {
                    formal_profile: None,
                    formal_proof: None,
                    contract_version: 1,
                    execution_evidence_scope: None,
                    version: ProjectionVersion {
                        commit: commit.clone(),
                        projection_revision: "v1".into(),
                        visibility_scope: "agent:agent".into(),
                    },
                    logical_tick: 4,
                    continuation: EventCursor {
                        stream_id: "events".into(),
                        scope_id: "agent:agent".into(),
                        era: 0,
                        sequence: 0,
                        commit,
                    },
                    view: serde_json::json!({"state":{"time":4},"runtime_binding":null,"agent_context":null,"scheduler_wakes":[],"continuations":[],
                        "cognition_leases":[],"continuation_contexts":{},"events":[],"game_task_control":control}),
                };
                let signed = sign_service_response(
                    VIEW_JSON_PATH,
                    request_digest(VIEW_JSON_PATH, &proof).unwrap(),
                    payload,
                    &hex::encode([9; 32]),
                )
                .unwrap();
                let bytes = serde_json::to_vec(&signed).unwrap();
                write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",bytes.len()).unwrap();
                stream.write_all(&bytes).unwrap();
                r.fetch_add(1, Ordering::SeqCst);
            }
        });
        Self {
            endpoint,
            control,
            stop,
            reads,
            worker: Some(worker),
        }
    }
}
impl Drop for SignedWorld {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let result = self.worker.take().unwrap().join();
        if !std::thread::panicking() {
            result.unwrap();
        }
    }
}
fn read_request(stream: &mut TcpStream) -> SignedReadRequest<ReadWorldViewRequest> {
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let mut header = Vec::new();
    while !header.ends_with(b"\r\n\r\n") {
        let mut b = [0];
        stream.read_exact(&mut b).unwrap();
        header.push(b[0]);
        assert!(header.len() < 16384);
    }
    let header = String::from_utf8(header).unwrap();
    assert_eq!(
        header.split_whitespace().take(2).collect::<Vec<_>>(),
        ["POST", VIEW_JSON_PATH]
    );
    let length = header
        .lines()
        .find_map(|line| {
            let (n, v) = line.split_once(':')?;
            n.eq_ignore_ascii_case("content-length")
                .then(|| v.trim().parse::<usize>().unwrap())
        })
        .unwrap();
    assert!(length < 65536);
    let mut body = vec![0; length];
    stream.read_exact(&mut body).unwrap();
    serde_json::from_slice(&body).unwrap()
}
#[test]
fn real_device_signed_registry_resolves_owner_and_rechecks_fence_before_activation() {
    use std::os::unix::fs::OpenOptionsExt;
    let mut identity_fixture = IdentityService::model_executor_test_fixture();
    let scope = identity_fixture.service.config.world_scope.clone();
    identity_fixture
        .service
        .store
        .transaction(|tx| {
            tx.create(
                &format!("binding:a:{scope}"),
                "binding",
                Some("a"),
                None,
                &PlayerBinding {
                    account_id: "a".into(),
                    world_identity_scope: scope,
                    player_id: "player".into(),
                    source: "identity_v2".into(),
                    binding_version: 1,
                },
            )
        })
        .unwrap();
    let headers = format!(
        "GET / HTTP/1.1\r\nCookie: __Host-oasis7_sid={}\r\n\r\n",
        identity_fixture.token
    );
    let principal = identity_fixture
        .service
        .model_principal(&headers, false)
        .unwrap();
    let directory = identity_fixture
        .service
        .config
        .sqlite_path
        .parent()
        .unwrap()
        .canonicalize()
        .unwrap();
    let identity = Arc::new(Mutex::new(Some(identity_fixture.service)));
    let public = sign_read_request("key", (), &hex::encode([7; 32]))
        .unwrap()
        .subject_public_key;
    let server = SignedWorld::new(fence(), &public);
    let key_path = directory.join("executor.key");
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&key_path)
        .unwrap();
    file.write_all(&[7; 32]).unwrap();
    drop(file);
    let device_path = directory.join("device.json");
    let service_public = sign_read_request("key", (), &hex::encode([9; 32]))
        .unwrap()
        .subject_public_key;
    std::fs::write(&device_path,serde_json::to_vec(&serde_json::json!({"endpoint":server.endpoint,"world_id":"world","genesis_digest":"genesis","agent_id":"agent",
        "service_public_key":service_public,"signer_file":key_path,"state_directory":directory.join("sdk-state")})).unwrap()).unwrap();
    let config = vec![ExecutorConfig {
        account_id: "a".into(),
        device_config_path: device_path,
    }];
    let registry = Arc::new(Executors::load(&config, identity.clone()).unwrap());
    let executor = registry
        .resolve(&principal, "agent")
        .expect("legal configured owner must resolve");
    assert!(registry.available(&principal));
    assert!(registry.authorize(&principal, &target(&executor.fence)));
    let mut other = principal.clone();
    other.account_id = "other".into();
    assert!(registry.resolve(&other, "agent").is_none());
    assert!(!registry.authorize(&other, &target(&executor.fence)));
    let claim = registry.claim(&executor, "connection-one").unwrap();
    assert!(registry.claim(&executor, "connection-two").is_err());
    drop(claim);
    assert!(registry.claim(&executor, "connection-two").is_ok());
    // Use the actual host binding callback with the core's deterministic probe
    // lifecycle only; these permits never invoke a model/provider transport.
    let authority = identity.clone();
    let bindings = registry.clone();
    let manager = oasis7_model_provider::connection::ConnectionManager::new(
        Arc::new(move |p: &Principal| {
            authority
                .lock()
                .unwrap()
                .as_mut()
                .unwrap()
                .model_session_current(p)
        }),
        Arc::new(move |p: &Principal, t: &BindingTarget| bindings.authorize(p, t)),
        Arc::new(oasis7_model_provider::connection::MonotonicClock::default()),
        oasis7_model_provider::connection::Limits::default(),
    )
    .unwrap();
    let preset = oasis7_model_provider::preset::ProviderPreset {
        id: "test".into(),
        provider: oasis7_model_provider::preset::ProviderId::DeepSeek,
        protocol: oasis7_model_provider::preset::Protocol::ChatCompletions,
        model: "deepseek-chat".into(),
        routes: vec![],
        revision: 1,
        limits: Default::default(),
    };
    let connection = manager
        .create(
            &principal,
            preset,
            "fixture-provider-key".as_bytes().to_vec().into(),
            true,
        )
        .unwrap();
    let probe = match manager
        .begin_probe(
            &principal,
            &connection.id,
            connection.version,
            "fixture-probe",
        )
        .unwrap()
    {
        oasis7_model_provider::connection::ProbeStart::Operation(p) => p,
        _ => panic!("new probe expected"),
    };
    for phase in 0..2 {
        let permit = probe.reserve_phase(phase, 10).unwrap();
        permit.before_send().unwrap();
        permit.finish(Some(10), true).unwrap();
    }
    let ready = manager.snapshot(&principal, &connection.id).unwrap();
    let active = manager
        .activate(
            &principal,
            &ready.id,
            ready.version,
            target(&executor.fence),
        )
        .expect("legal owner/fence must activate");
    let provider = oasis7_model_provider::harness_adapter::SessionModelProvider::new(
        manager.clone(),
        principal.clone(),
        active.id.clone(),
        active.active_version.unwrap(),
        active.generation,
    )
    .unwrap();
    oasis7_agent_harness::ModelProvider::check_execution(&provider).unwrap();
    // Ownership is read from the current DB, not cached at registry startup.
    let binding_key = format!(
        "binding:a:{}",
        identity
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .config
            .world_scope
    );
    for (player, allowed) in [("different-player", false), ("player", true)] {
        identity
            .lock()
            .unwrap()
            .as_mut()
            .unwrap()
            .store
            .transaction(|tx| {
                let mut row = tx.get::<PlayerBinding>(&binding_key)?.unwrap();
                row.value.player_id = player.into();
                tx.replace(
                    &binding_key,
                    row.row_version,
                    "binding",
                    Some("a"),
                    None,
                    &row.value,
                )
            })
            .unwrap();
        assert_eq!(
            registry.authorize(&principal, &target(&executor.fence)),
            allowed
        );
    }
    server.control.lock().unwrap().task_revision = DecimalU64(2);
    assert!(!registry.authorize(&principal, &target(&executor.fence)));
    assert!(oasis7_agent_harness::ModelProvider::check_execution(&provider).is_err());
    assert!(
        manager
            .activate(
                &principal,
                &active.id,
                active.version,
                target(&executor.fence)
            )
            .is_err()
    );
    assert!(server.reads.load(Ordering::SeqCst) >= 7);
    drop(provider);
    drop(manager);
    drop(executor);
    drop(registry);
    drop(identity);
}
