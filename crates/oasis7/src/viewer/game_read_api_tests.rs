use super::*;
use crate::world_service::{client::WorldServiceClientConfig, projection::WorldServiceProjection};
use oasis7_client_api::world_service::*;
use std::{net::TcpListener, thread};

fn client(endpoint: String) -> RemoteWorldServiceClient {
    let service = authority::sign_read_request("key", (), &hex::encode([9; 32])).unwrap();
    RemoteWorldServiceClient::new(WorldServiceClientConfig {
        endpoint,
        trusted_service_public_key: service.subject_public_key,
        expected_world: WorldIdentity {
            world_id: "world".into(),
            genesis_digest: "genesis".into(),
        },
        scope_id: "agent:a".into(),
        read_private_key_hex: hex::encode([7; 32]),
        timeout: Duration::from_secs(2),
        max_response_bytes: 2 * 1024 * 1024,
    })
    .unwrap()
}
fn proof(client: &RemoteWorldServiceClient) -> SignedReadRequest<ReadWorldViewRequest> {
    authority::sign_read_request(
        VIEW_PATH,
        ReadWorldViewRequest {
            contract_version: WORLD_SERVICE_CONTRACT_VERSION,
            world: client.config().expected_world.clone(),
            scope_id: "agent:a".into(),
            min_commit: None,
            fixed_commit: None,
            deadline_unix_ms: None,
        },
        &hex::encode([8; 32]),
    )
    .unwrap()
}
fn request(proof: Option<&SignedReadRequest<ReadWorldViewRequest>>) -> Request {
    let mut request = Request::builder().uri("/v1/game/worlds/world/agents/a/observation");
    if let Some(proof) = proof {
        request = request.header(
            "X-Oasis7-World-Read-Proof",
            hex::encode(serde_json::to_vec(proof).unwrap()),
        );
    }
    request.body(()).unwrap()
}
fn response(request: &ReadWorldViewRequest) -> ReadWorldViewResponse<WorldServiceProjection> {
    let mut state = crate::runtime::WorldState::default();
    state.agents.insert(
        "a".into(),
        crate::runtime::AgentCell::new(
            crate::models::AgentState::new("a", crate::geometry::GeoPos::new(0, 0, 0)),
            u64::MAX,
        ),
    );
    let world = crate::runtime::World::new_with_state(state);
    let mut view = WorldServiceProjection::from_world(&world, Some("a")).unwrap();
    view.runtime_binding = None;
    let subject = json!({"kind":"agent","agent_id":"a","owner_binding":"owner","generation":1});
    let presenter = json!({"presenter_id":"executor","presenter_kind":"agent_client"});
    let audience =
        json!({"world_id":"world","branch_id":"branch","finality_epoch":0,"target_kind":"world"});
    view.agent_context = Some(serde_json::from_value(json!({
        "agent_id":"a","capability_authorization_root":"root","capability_snapshot_hash":"snapshot",
        "authority_context_hash":"authority","payer_binding":{},
        "capability_catalog":{"snapshot_id":"catalog","world_id":"world","world_head":u64::MAX,
            "branch_id":"branch","finality_epoch":0,"logical_tick":world.state().time,
            "module_registry_hash":"registry","policy_hash":"policy","revocation_epoch":0,
            "subject":subject,"presenter":presenter,"audience":audience,"entries":[],"valid_until_tick":u64::MAX},
        "capability_invocation_context":{"grant_id":"grant","subject":subject,"presenter":presenter,"audience":audience,
            "catalog_snapshot_id":"catalog","module_id":"module","module_version":"1","response_nonce":"nonce"}
    })).unwrap());
    let commit = CommitRef {
        world: request.world.clone(),
        binding: ExecutionBinding {
            provider_world_id: "world".into(),
            branch_id: "branch".into(),
            finality_ref: "finality".into(),
            reorg_generation: 0,
            governing_manifest_ref: "manifest".into(),
            authority_generation: 1,
            permission_generation: 1,
        },
        position: u64::MAX,
        execution_block_hash: "block".into(),
        state_root_ref: "root".into(),
    };
    ReadWorldViewResponse {
        contract_version: WORLD_SERVICE_CONTRACT_VERSION,
        version: ProjectionVersion {
            commit: commit.clone(),
            projection_revision: "projection".into(),
            visibility_scope: "agent:a".into(),
        },
        logical_tick: world.state().time,
        continuation: EventCursor {
            stream_id: "events".into(),
            scope_id: "agent:a".into(),
            era: 0,
            sequence: 1,
            commit,
        },
        view,
    }
}
fn service(tamper: bool, rejection: bool) -> (RemoteWorldServiceClient, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let client = client(format!("http://{}", listener.local_addr().unwrap()));
    let expected = proof(&client);
    let worker = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut reader = std::io::BufReader::new(stream.try_clone().unwrap());
        let mut head = String::new();
        let mut length = 0;
        loop {
            head.clear();
            std::io::BufRead::read_line(&mut reader, &mut head).unwrap();
            if head == "\r\n" {
                break;
            }
            if let Some(value) = head.to_ascii_lowercase().strip_prefix("content-length:") {
                length = value.trim().parse::<usize>().unwrap();
            }
        }
        let mut bytes = vec![0; length];
        reader.read_exact(&mut bytes).unwrap();
        let actual: SignedReadRequest<ReadWorldViewRequest> =
            serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            actual, expected,
            "device proof must not be replaced by the configured local reader"
        );
        authority::verify_read_request(VIEW_PATH, &actual).unwrap();
        let mut signed = authority::sign_service_response(
            VIEW_PATH,
            authority::request_digest(VIEW_PATH, &actual).unwrap(),
            response(&actual.request),
            &hex::encode([9; 32]),
        )
        .unwrap();
        if tamper {
            signed.payload.logical_tick += 1;
        }
        let bytes = serde_json::to_vec(&signed).unwrap();
        let status = if rejection { 403 } else { 200 };
        write!(
            stream,
            "HTTP/1.1 {status} Test\r\nConnection: close\r\nContent-Length: {}\r\n\r\n",
            bytes.len()
        )
        .unwrap();
        stream.write_all(&bytes).unwrap();
    });
    (client, worker)
}

#[test]
fn observation_relays_actor_signature_and_exposes_only_verified_lossless_state() {
    let (client, worker) = service(false, false);
    let response = handle(&request(Some(&proof(&client))), Some(&client));
    assert_eq!(response.status(), 200);
    let value: Value = serde_json::from_str(response.body().as_ref().unwrap()).unwrap();
    assert_eq!(value["data"]["last_active"], u64::MAX.to_string());
    assert_eq!(
        value["world_binding"]["commit"]["position"],
        u64::MAX.to_string()
    );
    worker.join().unwrap();
    for (tamper, rejection, expected) in [(true, false, 503), (false, true, 403)] {
        let (client, worker) = service(tamper, rejection);
        assert_eq!(
            handle(&request(Some(&proof(&client))), Some(&client)).status(),
            expected
        );
        worker.join().unwrap();
    }
}

#[test]
fn capability_read_projects_the_authenticated_world_catalog() {
    let (client, worker) = service(false, false);
    let mut req = request(Some(&proof(&client)));
    *req.uri_mut() = "/v1/game/worlds/world/agents/a/capabilities"
        .parse()
        .unwrap();
    let response = handle(&req, Some(&client));
    assert_eq!(response.status(), 200);
    let value: Value = serde_json::from_str(response.body().as_ref().unwrap()).unwrap();
    assert_eq!(value["data"]["snapshot_id"], "catalog");
    assert_eq!(value["data"]["world_head"], u64::MAX.to_string());
    worker.join().unwrap();
}

#[test]
fn unauthorized_and_malformed_proofs_fail_before_backend_network_access() {
    let client = client("http://127.0.0.1:1".into());
    assert_eq!(handle(&request(None), Some(&client)).status(), 428);
    let mut signed = proof(&client);
    signed.request.scope_id = "agent:b".into();
    assert_eq!(handle(&request(Some(&signed)), Some(&client)).status(), 403);
    let mut req = request(Some(&proof(&client)));
    req.headers_mut().insert("Origin", "null".parse().unwrap());
    assert_eq!(handle(&req, Some(&client)).status(), 403);
    let mut value = serde_json::to_value(proof(&client)).unwrap();
    value["request"]["owner"] = json!(true);
    let req = Request::builder()
        .uri("/v1/game/worlds/world/agents/a/observation")
        .header("X-Oasis7-World-Read-Proof", hex::encode(value.to_string()))
        .body(())
        .unwrap();
    assert_eq!(handle(&req, Some(&client)).status(), 400);
    let mut req = request(Some(&proof(&client)));
    let value = req.headers()["X-Oasis7-World-Read-Proof"].clone();
    req.headers_mut().append("X-Oasis7-World-Read-Proof", value);
    assert_eq!(handle(&req, Some(&client)).status(), 400);
}

#[test]
fn plain_http_discovery_and_websocket_headers_share_the_port_without_consumption() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let worker = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        assert!(serve_if_game_request(&mut stream, None).unwrap());
    });
    let mut stream = TcpStream::connect(addr).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    stream
        .write_all(b"GET /v1/game/info HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    assert!(response.starts_with("HTTP/1.1 200"));
    assert!(response.contains("signed_read_bootstrap"));
    worker.join().unwrap();
    let request = Request::builder()
        .uri("/v1/game/info?token=secret")
        .body(())
        .unwrap();
    assert_eq!(handle(&request, None).status(), 400);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let worker = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        assert!(!serve_if_game_request(&mut stream, None).unwrap());
        let mut socket = tungstenite::accept(stream).unwrap();
        let message = socket.read().unwrap();
        socket.send(message).unwrap();
    });
    let (mut socket, _) = tungstenite::connect(format!("ws://{addr}/viewer")).unwrap();
    socket
        .send(tungstenite::Message::Text("unchanged".into()))
        .unwrap();
    assert_eq!(socket.read().unwrap().into_text().unwrap(), "unchanged");
    worker.join().unwrap();
}

#[test]
fn bootstrap_rejects_public_bind_and_plaintext_remote_backend() {
    use crate::viewer::{ViewerWebBridge, ViewerWebBridgeConfig};
    let bridge = |bind| ViewerWebBridge::new(ViewerWebBridgeConfig::new(bind, "127.0.0.1:5023"));
    assert!(
        bridge("0.0.0.0:5011")
            .with_game_read_api(client("http://127.0.0.1:1".into()))
            .is_err()
    );
    assert!(
        bridge("127.0.0.1:5011")
            .with_game_read_api(client("http://example.com".into()))
            .is_err()
    );
    assert!(
        bridge("127.0.0.1:5011")
            .with_game_read_api(client("http://127.0.0.1:1".into()))
            .is_ok()
    );
}

#[test]
fn read_errors_distinguish_transient_limits_from_invalid_requests() {
    let response = render_response(Err((429, "read_limit_exceeded")));
    assert_eq!(response.status(), 429);
    let body: Value = serde_json::from_str(response.body().as_ref().unwrap()).unwrap();
    assert_eq!(body["retryable"], true);

    let client = client("http://127.0.0.1:1".into());
    let mut payload = proof(&client).request;
    payload.contract_version = WORLD_SERVICE_CONTRACT_VERSION + 1;
    let signed = authority::sign_read_request(VIEW_PATH, payload, &hex::encode([8; 32])).unwrap();
    let response = handle(&request(Some(&signed)), Some(&client));
    assert_eq!(
        response.status(),
        400,
        "invalid signed payload must not contact backend"
    );
    let body: Value = serde_json::from_str(response.body().as_ref().unwrap()).unwrap();
    assert_eq!(body["code"], "invalid_actor_proof");
    assert_eq!(body["retryable"], false);
}
