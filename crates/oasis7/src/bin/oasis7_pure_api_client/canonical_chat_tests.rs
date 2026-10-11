use super::*;
use oasis7::runtime::{Action, World};
use oasis7::world_service::projection::WorldServiceProjection;
use oasis7::world_service::*;
use oasis7_client_api::world_service::{
    CommitRef, EventCursor, ExecutionBinding, IntentOutcome, IntentResponse, ProjectionVersion,
    ReadWorldViewResponse,
};
#[cfg(unix)]
use std::io::Read;
#[cfg(unix)]
use std::net::Shutdown;
use std::net::TcpListener;
#[cfg(unix)]
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
#[cfg(unix)]
use std::thread;

fn world_fixture() -> (World, String, WorldIdentity, CommitRef) {
    let private = hex::encode([73; 32]);
    let key = authority::sign_read_request("fixture", (), &private)
        .unwrap()
        .subject_public_key;
    let mut world = World::new();
    world.submit_action(Action::RegisterAgent {
        agent_id: "agent-0".into(),
        pos: oasis7::GeoPos::new(0, 0, 0),
    });
    world.step().unwrap();
    world.submit_action(Action::ClaimStarterOc {
        agent_id: "agent-0".into(),
        player_id: "owner".into(),
        public_key: Some(key),
    });
    world.step().unwrap();
    world
        .install_capability_agent_identity("agent-0", "owner", 1)
        .unwrap();
    world
        .bind_cognition_runtime("canonical-cli-fixture", "main", 0, None, "pending", 0)
        .unwrap();
    let identity = WorldIdentity {
        world_id: "canonical-cli-fixture".into(),
        genesis_digest: "fixture".into(),
    };
    let commit = CommitRef {
        world: identity.clone(),
        binding: ExecutionBinding {
            provider_world_id: identity.world_id.clone(),
            branch_id: "main".into(),
            finality_ref: "fixture".into(),
            reorg_generation: 0,
            governing_manifest_ref: "fixture".into(),
            authority_generation: 1,
            permission_generation: 1,
        },
        position: 2,
        execution_block_hash: "fixture".into(),
        state_root_ref: "fixture".into(),
    };
    (world, private, identity, commit)
}
#[cfg(unix)]
fn read_http(socket: &mut std::net::TcpStream) -> (String, Value, Vec<u8>) {
    socket.set_nonblocking(false).unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(4)))
        .unwrap();
    let mut bytes = Vec::new();
    loop {
        let mut part = [0; 4096];
        let n = socket.read(&mut part).unwrap();
        assert!(n > 0);
        bytes.extend_from_slice(&part[..n]);
        if let Some(i) = bytes.windows(4).position(|v| v == b"\r\n\r\n") {
            let headers = String::from_utf8_lossy(&bytes[..i]);
            let length: usize = headers
                .lines()
                .find_map(|v| {
                    v.to_lowercase()
                        .strip_prefix("content-length:")
                        .map(|v| v.trim().parse().unwrap())
                })
                .unwrap();
            if bytes.len() >= i + 4 + length {
                let path = headers
                    .lines()
                    .next()
                    .unwrap()
                    .split_whitespace()
                    .nth(1)
                    .unwrap()
                    .to_string();
                return (
                    path,
                    serde_json::from_slice(&bytes[i + 4..i + 4 + length]).unwrap(),
                    bytes[i + 4..i + 4 + length].to_vec(),
                );
            }
        }
    }
}
#[cfg(unix)]
fn reply(socket: &mut std::net::TcpStream, body: Value) {
    let bytes = serde_json::to_vec(&body).unwrap();
    write!(
        socket,
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        bytes.len()
    )
    .unwrap();
    socket.write_all(&bytes).unwrap();
}

#[cfg(unix)]
fn canonical_fixture(recover_after_timeout: bool) {
    let (world, private, identity, commit) = world_fixture();
    let canonical = Arc::new(Mutex::new(world));
    let received = Arc::new(Mutex::new(Vec::<Value>::new()));
    let service_private = hex::encode([9; 32]);
    let service_key = authority::sign_read_request("fixture", (), &service_private)
        .unwrap()
        .subject_public_key;
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let stop = Arc::new(AtomicBool::new(false));
    let stopped = stop.clone();
    let service_world = canonical.clone();
    let submitted = received.clone();
    let retry_enabled = Arc::new(AtomicBool::new(!recover_after_timeout));
    let service_retry_enabled = retry_enabled.clone();
    let (submit_observed, observed_submit) = std::sync::mpsc::channel();
    let service = thread::spawn(move || {
        let mut committed: Option<Value> = None;
        let mut first_submit_bytes = None;
        while !stopped.load(Ordering::Relaxed) {
            let (mut socket, _) = match listener.accept() {
                Ok(v) => v,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(2));
                    continue;
                }
                Err(e) => panic!("{e}"),
            };
            let (path, body, original_bytes) = read_http(&mut socket);
            let digest = authority::request_digest(&path, &body).unwrap();
            let payload = if path == VIEW_PATH {
                let signed: SignedReadRequest<ReadWorldViewRequest> =
                    serde_json::from_value(body).unwrap();
                authority::verify_read_request(VIEW_PATH, &signed).unwrap();
                let world = service_world.lock().unwrap();
                let owner = signed.request.scope_id.strip_prefix("agent:");
                if owner.is_some() {
                    assert_eq!(
                        signed.subject_public_key,
                        authority::sign_read_request("fixture", (), &hex::encode([73; 32]))
                            .unwrap()
                            .subject_public_key
                    );
                }
                let mut projection = WorldServiceProjection::from_authenticated_world(
                    &world,
                    owner,
                    &signed.subject_public_key,
                )
                .unwrap();
                projection.runtime_binding = None;
                serde_json::to_value(ReadWorldViewResponse {
                    execution_evidence_scope: None,
                    contract_version: 1,
                    version: ProjectionVersion {
                        commit: commit.clone(),
                        projection_revision: "fixture".into(),
                        visibility_scope: signed.request.scope_id.clone(),
                    },
                    logical_tick: world.state().time,
                    continuation: EventCursor {
                        stream_id: "fixture".into(),
                        scope_id: signed.request.scope_id,
                        era: 0,
                        sequence: 0,
                        commit: commit.clone(),
                    },
                    view: projection,
                })
                .unwrap()
            } else if path == CHANGES_PATH {
                let signed: SignedReadRequest<ReadWorldChangesRequest> =
                    serde_json::from_value(body).unwrap();
                authority::verify_read_request(CHANGES_PATH, &signed).unwrap();
                serde_json::to_value(ReadWorldChangesResponse::<Value> {
                    execution_evidence_scope: None,
                    contract_version: 1,
                    changes: Vec::new(),
                    next_cursor: signed.request.cursor,
                })
                .unwrap()
            } else if path == LOOKUP_PATH {
                let originals = submitted.lock().unwrap();
                assert_eq!(body["original"], originals[0]["signed_payload"]);
                assert_eq!(body["request"]["key"], originals[0]["correlation"]["key"]);
                let mut pending = committed.as_ref().unwrap().clone();
                pending["outcome"] = json!({"status":"pending"});
                pending
            } else if path == SUBMIT_PATH {
                submitted.lock().unwrap().push(body.clone());
                submit_observed.send(()).unwrap();
                if let Some(result) = &committed {
                    let originals = submitted.lock().unwrap();
                    assert_eq!(originals[0], originals[1]);
                    assert_eq!(first_submit_bytes.as_ref().unwrap(), &original_bytes);
                    if service_retry_enabled.load(Ordering::Acquire) {
                        result.clone()
                    } else {
                        let mut pending = result.clone();
                        pending["outcome"] = json!({"status":"pending"});
                        pending
                    }
                } else {
                    first_submit_bytes = Some(original_bytes);
                    let request: SubmitIntentRequest<WorldServicePayloadV1> =
                        serde_json::from_value(body).unwrap();
                    let WorldServicePayloadV1::AgentChat(chat) = &request.signed_payload else {
                        panic!("unexpected payload")
                    };
                    assert_eq!(chat.world_id.as_deref(), Some("canonical-cli-fixture"));
                    assert_eq!(
                        chat.canonical_authority
                            .as_ref()
                            .unwrap()
                            .agent_identity_generation,
                        1
                    );
                    let mut world = service_world.lock().unwrap();
                    let receipt = world.apply_authenticated_agent_chat(chat).unwrap();
                    world
                        .record_world_service_result(CanonicalIntentResultV1 {
                            request: request.clone(),
                            action_id: 1,
                            committed_height: 2,
                            receipt: receipt.clone(),
                            rejected: None,
                        })
                        .unwrap();
                    committed = Some(
                        serde_json::to_value(IntentResponse {
                            execution_evidence_scope: None,
                            contract_version: 1,
                            correlation: request.correlation.clone(),
                            outcome: IntentOutcome::Committed {
                                commit: Box::new(commit.clone()),
                                receipt,
                            },
                        })
                        .unwrap(),
                    );
                    serde_json::to_value(IntentResponse::<Value> {
                        execution_evidence_scope: None,
                        contract_version: 1,
                        correlation: request.correlation,
                        outcome: IntentOutcome::Pending,
                    })
                    .unwrap()
                }
            } else {
                panic!("unexpected service path {path}")
            };
            reply(
                &mut socket,
                serde_json::to_value(
                    authority::sign_service_response(&path, digest, payload, &service_private)
                        .unwrap(),
                )
                .unwrap(),
            );
        }
    });
    let mut config = oasis7::viewer::ViewerRuntimeLiveServerConfig::new(
        oasis7::simulator::WorldScenario::Minimal,
    );
    config.world_id = identity.world_id.clone();
    config.world_service = Some(client::WorldServiceClientConfig {
        endpoint,
        trusted_service_public_key: service_key,
        expected_world: identity,
        scope_id: "public".into(),
        read_private_key_hex: hex::encode([7; 32]),
        timeout: Duration::from_secs(3),
        max_response_bytes: 1024 * 1024,
    });
    let server = Arc::new(Mutex::new(
        oasis7::viewer::ViewerRuntimeLiveServer::new(config).unwrap(),
    ));
    let tcp = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = tcp.local_addr().unwrap().to_string();
    let serving = thread::spawn(move || {
        for _ in 0..if recover_after_timeout { 3 } else { 2 } {
            let (socket, _) = tcp.accept().unwrap();
            oasis7::viewer::ViewerRuntimeLiveServer::test_serve_shared_stream(
                server.clone(),
                socket,
            )
            .unwrap();
        }
    });
    let timeout = Duration::from_secs(10);
    let mut registration =
        ViewerConnection::connect(&addr, "fixture-registration", timeout).unwrap();
    registration.send(&ViewerRequest::RequestSnapshot).unwrap();
    registration
        .collect_until(timeout, terminal_snapshot, "prime")
        .unwrap();
    let request = build_signed_session_register_request(
        "owner",
        &private,
        None,
        Some("agent-0".into()),
        None,
    )
    .unwrap();
    registration
        .send(&ViewerRequest::AuthoritativeRecovery {
            command: AuthoritativeRecoveryCommand::RegisterSession { request },
        })
        .unwrap();
    let registered = registration
        .collect_until(timeout, terminal_recovery, "register")
        .unwrap();
    assert!(
        matches!(
            registered.last().unwrap().response,
            ViewerResponse::AuthoritativeRecoveryAck { .. }
        ),
        "{:?}",
        registered.last().unwrap().raw
    );
    registration
        .writer
        .get_ref()
        .shutdown(Shutdown::Both)
        .unwrap();
    drop(registration);
    let mut conn = ViewerConnection::connect(&addr, "fixture-chat", timeout).unwrap();
    subscribe_for_control(&mut conn, true, false).unwrap();
    let input = ChatInput {
        agent_id: "agent-0",
        player_id: "owner",
        message: "private canonical fixture goal",
        private_key_hex: &private,
        public_key_hex: None,
        intent_tick: None,
        intent_seq: None,
    };
    let first = if recover_after_timeout {
        let (chat, responses, artifact) =
            prepare_canonical(&mut conn, Instant::now() + timeout, input).unwrap();
        conn.send(&ViewerRequest::AgentChat {
            request: chat.clone(),
        })
        .unwrap();
        observed_submit
            .recv_timeout(timeout)
            .expect("service must receive original canonical Submit before timeout phase");
        submit_original(
            &mut conn,
            Instant::now() + Duration::from_millis(300),
            chat,
            responses,
            &artifact,
        )
    } else {
        exchange(&mut conn, timeout, input)
    };
    let responses = if recover_after_timeout {
        let error = match first {
            Err(error) => error,
            Ok(_) => panic!("pending is not committed success"),
        };
        let path = std::path::PathBuf::from(
            error
                .split("retained at ")
                .nth(1)
                .unwrap_or_else(|| {
                    panic!("timeout must retain persisted original request: {error}")
                })
                .split(". Retry with")
                .next()
                .unwrap(),
        );
        let saved = pending::load(&path).unwrap();
        assert_eq!(saved.endpoint, addr);
        let WorldServicePayloadV1::AgentChat(original) =
            serde_json::from_value::<WorldServicePayloadV1>(
                received.lock().unwrap()[0]["signed_payload"].clone(),
            )
            .unwrap()
        else {
            panic!("chat")
        };
        assert_eq!(saved.request, original);
        conn.writer.get_ref().shutdown(Shutdown::Both).unwrap();
        drop(conn);
        conn = ViewerConnection::connect(&addr, "fixture-retry", timeout).unwrap();
        retry_enabled.store(true, Ordering::Release);
        let result = retry(&mut conn, timeout, &path).unwrap();
        assert!(!path.exists(), "committed receipt removes pending artifact");
        result
    } else {
        first.unwrap()
    };
    assert!(
        matches!(&responses.last().unwrap().response,ViewerResponse::AgentChatAck{ack}if ack.intent_id.is_some()&&ack.auth_nonce.is_some())
    );
    assert_eq!(
        received.lock().unwrap().len(),
        if recover_after_timeout { 3 } else { 2 }
    );
    conn.writer.get_ref().shutdown(Shutdown::Both).unwrap();
    drop(conn);
    serving.join().unwrap();
    stop.store(true, Ordering::Relaxed);
    service.join().unwrap();
}

#[test]
fn owner_context_and_response_reject_foreign_stale_and_changed_authority() {
    let (world, private, identity, commit) = world_fixture();
    let view = agent_chat::owner_view(&world, "agent-0").unwrap().unwrap();
    let key = view.public_key.clone();
    let request = ReadWorldViewRequest {
        contract_version: 1,
        world: identity,
        scope_id: "agent:agent-0".into(),
        min_commit: Some(commit.clone()),
        fixed_commit: None,
        deadline_unix_ms: None,
    };
    let mut context = OwnerContext {
        signing_domain: VIEW_PATH.into(),
        signing_bytes_hex: hex::encode(authority::signing_bytes(VIEW_PATH, &request).unwrap()),
        request,
        fence: Some(CanonicalAgentOwnerFenceV1::from(&view)),
        expected: ExpectedOwner {
            agent_id: "agent-0".into(),
            player_id: "owner".into(),
            public_key: key.clone(),
            world_id: "canonical-cli-fixture".into(),
            branch_id: "main".into(),
            reorg_epoch: 0,
        },
    };
    let input = ChatInput {
        agent_id: "agent-0",
        player_id: "owner",
        message: "goal",
        private_key_hex: &private,
        public_key_hex: None,
        intent_tick: None,
        intent_seq: None,
    };
    let hello = json!({"world_id":"canonical-cli-fixture"});
    validate_context(&context, &input, &key, 0, &hello).unwrap();
    context.signing_domain = "foreign".into();
    assert!(validate_context(&context, &input, &key, 0, &hello).is_err());
    context.signing_domain = VIEW_PATH.into();
    context.signing_bytes_hex.push('0');
    assert!(validate_context(&context, &input, &key, 0, &hello).is_err());
    context.signing_bytes_hex.pop();
    assert!(validate_context(&context, &input, &key, 1, &hello).is_err());
    assert!(validate_context(&context, &input, "foreign", 0, &hello).is_err());
    let version = ProjectionVersion {
        commit,
        projection_revision: "fixture".into(),
        visibility_scope: "agent:agent-0".into(),
    };
    validate_view(&context, &version, &view).unwrap();
    let mut stale = version.clone();
    stale.commit.position = 1;
    assert!(validate_view(&context, &stale, &view).is_err());
    let mut foreign = version.clone();
    foreign.commit.world.world_id = "foreign".into();
    assert!(validate_view(&context, &foreign, &view).is_err());
    let mut wrong = view.clone();
    wrong.player_id = "foreign".into();
    assert!(validate_view(&context, &version, &wrong).is_err());
    let mut wrong = view.clone();
    wrong.canonical_authority.agent_identity_generation += 1;
    assert!(validate_view(&context, &version, &wrong).is_err());
    let mut wrong = view.clone();
    wrong.current_intent_id = Some("replacement".into());
    assert!(validate_view(&context, &version, &wrong).is_err());
}

#[test]
fn partial_response_survives_timeout_without_losing_signed_frame() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    let (mut peer, _) = listener.accept().unwrap();
    let mut conn = ViewerConnection {
        reader: BufReader::new(client.try_clone().unwrap()),
        writer: BufWriter::new(client),
        hello_ack: Value::Null,
        pending_line: String::new(),
        endpoint: "fixture".into(),
    };
    peer.write_all(b"{\"type\":\"error\",\"message\":\"preserved")
        .unwrap();
    assert!(matches!(
        conn.read_response(Duration::from_millis(20)).unwrap(),
        ReadResponseOutcome::TimedOut
    ));
    peer.write_all(b"\"}\n").unwrap();
    let ReadResponseOutcome::Response(response) =
        conn.read_response(Duration::from_secs(1)).unwrap()
    else {
        panic!("missing frame")
    };
    assert_eq!(response.raw["message"], "preserved");
}

#[test]
fn canonical_outcomes_require_original_nonce_and_request_agent() {
    let private = hex::encode([73; 32]);
    let chat = build_signed_agent_chat_request(
        "agent-0",
        "owner",
        "goal",
        &private,
        None,
        Some(2),
        Some(3),
    )
    .unwrap();
    let nonce = chat.auth.as_ref().unwrap().nonce;
    let mut ack:oasis7_proto::viewer::AgentChatAck<u64>=serde_json::from_value(json!({"agent_id":"agent-0","accepted_at_tick":2,"message_len":4,"player_id":"owner","intent_tick":2,"intent_seq":3,"auth_nonce":nonce,"intent_id":"committed","accepted_event_seq":1,"status":"accepted"})).unwrap();
    assert!(correlated(&ack, &chat, nonce));
    ack.auth_nonce = Some(nonce - 1);
    assert!(!correlated(&ack, &chat, nonce));
    ack.auth_nonce = Some(nonce);
    ack.intent_seq = Some(4);
    assert!(!correlated(&ack, &chat, nonce));
    let mut error = oasis7_proto::viewer::AgentChatError {
        code: "invalid_scope".into(),
        message: "denied".into(),
        agent_id: Some("agent-0".into()),
    };
    assert!(matching_error(&error, &chat));
    error.agent_id = Some("foreign".into());
    assert!(!matching_error(&error, &chat));
    error.agent_id = None;
    assert!(!matching_error(&error, &chat));
}

#[cfg(unix)]
#[test]
fn canonical_chat_uses_existing_session_and_actual_signed_world_service() {
    canonical_fixture(false);
}
#[cfg(unix)]
#[test]
fn canonical_timeout_recovers_original_request_on_new_connection() {
    canonical_fixture(true);
}

#[cfg(unix)]
#[test]
fn pending_artifact_is_private_bounded_and_rejects_symlink_or_unknown_fields() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let (_, private, world, _) = world_fixture();
    let chat = build_signed_agent_chat_request(
        "agent-0",
        "owner",
        "private goal",
        &private,
        None,
        Some(2),
        Some(next_u64_id()),
    )
    .unwrap();
    let path = pending::save(&chat, &world, "fixture").unwrap();
    assert_eq!(
        std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let before = std::fs::read(&path).unwrap();
    pending::load(&path).unwrap();
    let link = path.parent().unwrap().join("link");
    symlink(&path, &link).unwrap();
    assert!(pending::load(&link).is_err());
    std::fs::remove_file(link).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert!(pending::load(&path).is_err());
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    let mut raw: Value = serde_json::from_slice(&before).unwrap();
    raw["request"]["private_key_hex"] = json!("accidental secret");
    std::fs::write(&path, serde_json::to_vec(&raw).unwrap()).unwrap();
    assert!(pending::load(&path).is_err());
    std::fs::write(&path, b"\"ACCIDENTAL_PRIVATE_KEY_VALUE\"").unwrap();
    let error = pending::load(&path).err().unwrap();
    assert!(!error.contains("ACCIDENTAL_PRIVATE_KEY_VALUE"));
    std::fs::write(&path, b"{} {}").unwrap();
    assert!(pending::load(&path).is_err());
    std::fs::write(&path, vec![b' '; 1024 * 1024 + 1]).unwrap();
    assert!(pending::load(&path).is_err());
    std::fs::write(&path, before).unwrap();
    pending::complete(&path).unwrap();
}

#[cfg(unix)]
#[test]
fn retry_rejects_changed_endpoint_or_foreign_world_before_transport() {
    let (_, private, world, _) = world_fixture();
    let mut chat = build_signed_agent_chat_request(
        "agent-0",
        "owner",
        "goal",
        &private,
        None,
        Some(2),
        Some(next_u64_id()),
    )
    .unwrap();
    chat.world_id = Some(world.world_id.clone());
    chat.reorg_epoch = Some(0);
    chat.authority_scope = Some("player_agent_chat".into());
    chat.canonical_authority = Some(oasis7_proto::viewer::CanonicalAgentChatAuthorityV1 {
        branch_id: "main".into(),
        agent_identity_generation: 1,
    });
    let proof = chat.auth.as_ref().unwrap();
    chat.auth =
        Some(sign_agent_chat_auth_proof(&chat, proof.nonce, &proof.public_key, &private).unwrap());
    let path = pending::save(&chat, &world, "original-endpoint").unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    let (mut peer, _) = listener.accept().unwrap();
    let mut conn = ViewerConnection {
        reader: BufReader::new(client.try_clone().unwrap()),
        writer: BufWriter::new(client),
        hello_ack: json!({"capabilities":["canonical_agent_chat_v1"],"world_id":world.world_id}),
        pending_line: String::new(),
        endpoint: "changed-endpoint".into(),
    };
    assert!(
        retry(&mut conn, Duration::from_millis(100), &path)
            .err()
            .unwrap()
            .contains("different Viewer endpoint")
    );
    conn.endpoint = "original-endpoint".into();
    conn.hello_ack["world_id"] = json!("foreign");
    assert!(
        retry(&mut conn, Duration::from_millis(100), &path)
            .err()
            .unwrap()
            .contains("foreign")
    );
    peer.set_read_timeout(Some(Duration::from_millis(20)))
        .unwrap();
    assert!(peer.read(&mut [0; 1]).is_err());
    assert!(path.exists());
    pending::complete(&path).unwrap();
}
