use super::*;
use oasis7_agent_api::game::{DecimalU64, GAME_TASK_CREATE_DOMAIN, GameTaskCreateV1};
use oasis7_client_api::world_service_wire::AuthenticatedLookup;
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    thread,
};
#[derive(Clone, Copy)]
enum Fault {
    DropFirst,
    UnavailableOnce,
    WrongSignature,
    CooldownOnce,
    Overloaded,
}
struct Probe {
    client: Client,
    original: SubmitIntentRequest<WorldServicePayloadV1>,
    root: PathBuf,
    submits: Arc<AtomicUsize>,
    lookups: Arc<AtomicUsize>,
    times: Arc<Mutex<Vec<Instant>>>,
    stop: Arc<AtomicBool>,
    task: Option<thread::JoinHandle<()>>,
}
fn read_request(stream: &mut TcpStream) -> (String, Vec<u8>) {
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let mut bytes = Vec::new();
    let mut b = [0; 4096];
    let end = loop {
        let n = stream.read(&mut b).unwrap();
        assert!(n > 0);
        bytes.extend_from_slice(&b[..n]);
        if let Some(i) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
            break i + 4;
        }
    };
    let head = String::from_utf8_lossy(&bytes[..end]);
    let path = head.split_whitespace().nth(1).unwrap().to_owned();
    let length: usize = head
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse().unwrap())
        })
        .unwrap();
    while bytes.len() < end + length {
        let n = stream.read(&mut b).unwrap();
        assert!(n > 0);
        bytes.extend_from_slice(&b[..n]);
    }
    (path, bytes[end..end + length].to_vec())
}
fn committed(original: &SubmitIntentRequest<WorldServicePayloadV1>) -> IntentResponse<Value> {
    IntentResponse {
        contract_version: 1,
        execution_evidence_scope: None,
        correlation: original.correlation.clone(),
        outcome: IntentOutcome::Committed {
            commit: Box::new(CommitRef {
                world: original.correlation.key.world.clone(),
                binding: ExecutionBinding {
                    provider_world_id: "world".into(),
                    branch_id: "main".into(),
                    finality_ref: "finality".into(),
                    reorg_generation: 0,
                    governing_manifest_ref: "manifest".into(),
                    authority_generation: 1,
                    permission_generation: 1,
                },
                position: 7,
                execution_block_hash: "block".into(),
                state_root_ref: "root".into(),
            }),
            receipt: json!({"applied":true}),
        },
    }
}
impl Probe {
    fn new(fault: Fault) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let config = crate::tests::config(format!("http://{}", listener.local_addr().unwrap()));
        static NEXT_PROBE: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "oasis7-device-poll-{}-{}-{}",
            std::process::id(),
            NEXT_PROBE.fetch_add(1, Ordering::Relaxed),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        let payload = WorldServicePayloadV1::GameTaskCreate(
            authority::sign_read_request(
                GAME_TASK_CREATE_DOMAIN,
                GameTaskCreateV1 {
                    api_rev: 1,
                    client_operation_id: "retained-original".into(),
                    world_id: "world".into(),
                    genesis_digest: "genesis".into(),
                    branch_id: "main".into(),
                    reorg_epoch: DecimalU64(0),
                    agent_id: "a".into(),
                    owner_binding: "owner".into(),
                    owner_generation: DecimalU64(1),
                },
                &hex::encode([7; 32]),
            )
            .unwrap(),
        );
        let original = SubmitIntentRequest {
            contract_version: 1,
            correlation: derive_correlation(config.expected_world.clone(), &payload).unwrap(),
            deadline_unix_ms: None,
            signed_payload: payload,
        };
        let client = Client {
            world: config.expected_world.clone(),
            agent: "a".into(),
            public_key: authority::sign_read_request("key", (), &hex::encode([7; 32]))
                .unwrap()
                .subject_public_key,
            key_hex: Zeroizing::new(hex::encode([7; 32])),
            remote: crate::WorldServiceTaskClient::new(config).unwrap(),
            state: crate::operation_store::OperationStore::open(&root, [7; 32]).unwrap(),
        };
        let submits = Arc::new(AtomicUsize::new(0));
        let lookups = Arc::new(AtomicUsize::new(0));
        let times = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let (s, l, t, done, expected) = (
            submits.clone(),
            lookups.clone(),
            times.clone(),
            stop.clone(),
            original.clone(),
        );
        let task = thread::spawn(move || {
            while !done.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let (path, body) = read_request(&mut stream);
                        let (digest, response, key) = if path == SUBMIT_PATH {
                            s.fetch_add(1, Ordering::SeqCst);
                            let submitted: SubmitIntentRequest<WorldServicePayloadV1> =
                                serde_json::from_slice(&body).unwrap();
                            assert_eq!(submitted, expected);
                            (
                                authority::request_digest(SUBMIT_PATH, &submitted).unwrap(),
                                IntentResponse {
                                    contract_version: 1,
                                    execution_evidence_scope: None,
                                    correlation: expected.correlation.clone(),
                                    outcome: IntentOutcome::Received {
                                        durability: AdmissionDurability::Volatile,
                                    },
                                },
                                [9; 32],
                            )
                        } else {
                            assert_eq!(path, LOOKUP_PATH);
                            let index = l.fetch_add(1, Ordering::SeqCst);
                            t.lock().unwrap().push(Instant::now());
                            let lookup: AuthenticatedLookup =
                                serde_json::from_slice(&body).unwrap();
                            assert_eq!(lookup.request.key, expected.correlation.key);
                            assert_eq!(lookup.original, expected.signed_payload);
                            if matches!(fault, Fault::DropFirst) && index == 0 {
                                continue;
                            }
                            if matches!(fault, Fault::UnavailableOnce) && index == 0 {
                                stream.write_all(b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
                                continue;
                            }
                            if matches!(fault, Fault::Overloaded)
                                || matches!(fault, Fault::CooldownOnce) && index == 0
                            {
                                stream.write_all(b"HTTP/1.1 503 Service Unavailable\r\nRetry-After: 999999\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
                                continue;
                            }
                            (
                                authority::request_digest(LOOKUP_PATH, &lookup).unwrap(),
                                committed(&expected),
                                if matches!(fault, Fault::WrongSignature) {
                                    [8; 32]
                                } else {
                                    [9; 32]
                                },
                            )
                        };
                        let response = authority::sign_service_response(
                            &path,
                            digest,
                            response,
                            &hex::encode(key),
                        )
                        .unwrap();
                        let bytes = serde_json::to_vec(&response).unwrap();
                        write!(
                            stream,
                            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            bytes.len()
                        )
                        .unwrap();
                        stream.write_all(&bytes).unwrap();
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(2))
                    }
                    Err(e) => panic!("poll fixture: {e}"),
                }
            }
        });
        Self {
            client,
            original,
            root,
            submits,
            lookups,
            times,
            stop,
            task: Some(task),
        }
    }
}
impl Drop for Probe {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(task) = self.task.take() {
            task.join().unwrap();
        }
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
#[test]
fn dropped_or_unavailable_original_lookup_recovers_without_resigning_or_resubmitting() {
    for fault in [Fault::DropFirst, Fault::UnavailableOnce] {
        let probe = Probe::new(fault);
        let result = probe.client.submit_and_poll(&probe.original).unwrap();
        assert_eq!(result.correlation, probe.original.correlation);
        assert!(matches!(result.outcome, IntentOutcome::Committed { .. }));
        assert_eq!(probe.submits.load(Ordering::SeqCst), 1);
        assert_eq!(probe.lookups.load(Ordering::SeqCst), 2);
    }
}

#[test]
fn untrusted_original_lookup_fails_immediately_and_preserves_typed_assurance() {
    let probe = Probe::new(Fault::WrongSignature);
    let error = probe.client.submit_and_poll(&probe.original).unwrap_err();
    assert!(error.contains("response not verified"));
    assert!(!error.contains("signature_hex"));
    assert!(!error.contains("http://"));
    assert_eq!(probe.submits.load(Ordering::SeqCst), 1);
    assert_eq!(probe.lookups.load(Ordering::SeqCst), 1);
    assert!(matches!(
        probe.client.lookup_typed(&probe.original),
        Err(WorldServiceClientError::Assurance(_))
    ));
}
#[test]
fn shared_cooldown_is_respected_before_querying_the_exact_original_again() {
    let probe = Probe::new(Fault::CooldownOnce);
    let result = probe.client.submit_and_poll(&probe.original).unwrap();
    assert!(matches!(result.outcome, IntentOutcome::Committed { .. }));
    assert_eq!(probe.submits.load(Ordering::SeqCst), 1);
    assert_eq!(probe.lookups.load(Ordering::SeqCst), 2);
    let times = probe.times.lock().unwrap();
    assert!(times[1].duration_since(times[0]) >= Duration::from_millis(1900));
}
#[test]
fn repeated_cooldown_cannot_extend_the_existing_eight_second_query_window() {
    let probe = Probe::new(Fault::Overloaded);
    let start = Instant::now();
    let error = probe.client.submit_and_poll(&probe.original).unwrap_err();
    assert!(error.contains("cooldown HTTP 503"));
    assert!(start.elapsed() >= POLL_WINDOW);
    assert!(start.elapsed() < POLL_WINDOW + Duration::from_secs(5));
    assert_eq!(probe.submits.load(Ordering::SeqCst), 1);
    assert!(probe.lookups.load(Ordering::SeqCst) <= 4);
}
#[test]
fn retry_policy_is_limited_to_transport_cooldown_and_http_503() {
    assert_eq!(
        retry_delay(&WorldServiceClientError::Transport("secret".into())),
        Some(POLL_INTERVAL)
    );
    assert_eq!(
        retry_delay(&WorldServiceClientError::Rejected {
            status: 503,
            message: "secret".into()
        }),
        Some(POLL_INTERVAL)
    );
    for error in [
        WorldServiceClientError::Configuration("secret".into()),
        WorldServiceClientError::Assurance("secret".into()),
        WorldServiceClientError::Rejected {
            status: 403,
            message: "secret".into(),
        },
        WorldServiceClientError::Rejected {
            status: 429,
            message: "secret".into(),
        },
    ] {
        assert!(retry_delay(&error).is_none());
        assert!(!query_error(&error).contains("secret"));
    }
}
