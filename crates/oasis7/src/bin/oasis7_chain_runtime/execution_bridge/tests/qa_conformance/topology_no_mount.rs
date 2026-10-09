//! Linux container entrypoints. The controller never mounts the service volume.
use super::*;
use std::io::{BufRead, BufReader, Write};

fn copy_tree(source: &Path, target: &Path) {
    fs::create_dir_all(target).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let destination = target.join(entry.file_name());
        assert!(!entry.file_type().unwrap().is_symlink());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &destination);
        } else {
            fs::copy(entry.path(), destination).unwrap();
        }
    }
}

#[test]
#[ignore = "explicit Linux no-mount fixture preparation; service-only volume"]
fn no_mount_prepare_service() {
    let destination = std::path::PathBuf::from(std::env::var("PRE2_SERVER_ROOT").unwrap());
    assert!(!destination.join("world").exists(), "fresh volume required");
    let fixture = Fixture::with_options(true, true);
    fixture.finish_http_workers().unwrap();
    copy_tree(&fixture.root, &destination);
    let node = fixture.node.lock().unwrap().snapshot();
    let topology = serde_json::json!({"node_id":node.node_id,"world_id":node.world_id,
        "role":format!("{:?}",node.role),"running":node.running,
        "replication_enabled":node.replication_enabled,"tick_count":node.tick_count});
    let public_config = serde_json::json!({"service_public_key":fixture.client.config().trusted_service_public_key,
        "world":fixture.client.config().expected_world,"topology_scope":"real NodeRuntime execution fixture; consensus membership not evaluated",
        "node_snapshot":topology,"genesis":"fixture-genesis-v1"});
    fs::write(
        destination.join("app-public-config.json"),
        serde_json::to_vec(&public_config).unwrap(),
    )
    .unwrap();
    println!("PRE2_NO_MOUNT_REAL_SERVICE_PREPARED");
}

#[test]
#[ignore = "bounded deterministic provider in application network namespace"]
fn no_mount_provider_entry() {
    let root = std::path::PathBuf::from(std::env::var("PRE2_APP_PRIVATE").unwrap());
    let provider = provider_metadata::MetadataServer::start(None, root.clone(), None);
    fs::write(root.join("provider-endpoint"), provider.endpoint.as_bytes()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(120);
    while !root.join("provider-stop").exists() {
        assert!(
            Instant::now() < deadline,
            "provider bounded lifetime exhausted"
        );
        thread::sleep(Duration::from_millis(20));
    }
    let count = provider.decision_count.load(Ordering::SeqCst);
    drop(provider);
    fs::write(root.join("provider-count"), count.to_string()).unwrap();
}

#[test]
#[ignore = "real service process; operator witness stays on service-only volume"]
fn no_mount_service_entry() {
    let root = std::path::PathBuf::from(std::env::var("PRE2_SERVER_ROOT").unwrap());
    let endpoint = std::env::var("PRE2_SERVER_ENDPOINT").unwrap();
    let _lock =
        crate::world_writer_lock::acquire_live_world_writer_lock(&root.join("world")).unwrap();
    let bootstrap = super::super::super::local_bootstrap::derive_service_execution_bootstrap(
        &root.join("world"),
        &root.join("records"),
        &root.join("store"),
        "w1",
    )
    .unwrap();
    let driver = NodeRuntimeExecutionDriver::new_with_local_bootstrap(
        root.join("state.json"),
        root.join("world"),
        root.join("records"),
        root.join("store"),
        &oasis7_proto::storage_profile::StorageProfileConfig::default(),
        bootstrap.clone(),
    )
    .unwrap();
    let config = NodeConfig::new("node-a", "w1", NodeRole::Sequencer)
        .unwrap()
        .with_tick_interval(Duration::from_millis(500))
        .unwrap();
    let mut node = NodeRuntime::new(config)
        .with_execution_hook(driver)
        .with_local_execution_bootstrap(bootstrap);
    node.start().unwrap();
    let node = Arc::new(Mutex::new(node));
    let private_key_hex = hex::encode([9u8; 32]);
    let signer = crate::feedback_submit_api::FeedbackSubmitSigner {
        public_key_hex: sign_read_request("service", (), &private_key_hex)
            .unwrap()
            .subject_public_key,
        private_key_hex,
    };
    let listener = TcpListener::bind(endpoint.strip_prefix("http://").unwrap()).unwrap();
    let mut requests = std::collections::BTreeMap::<String, u64>::new();
    loop {
        let (mut stream, _) = listener.accept().unwrap();
        http_fixture::configure_accepted_stream(&stream);
        let bytes = read_request(&mut stream);
        if bytes.is_empty() {
            continue;
        }
        let first = std::str::from_utf8(&bytes).unwrap().lines().next().unwrap();
        let mut parts = first.split_whitespace();
        let method = parts.next().unwrap();
        let path = parts.next().unwrap();
        let handled = crate::world_service_api::maybe_handle(
            &mut stream,
            &bytes,
            &node,
            method,
            path,
            "w1",
            &root.join("world"),
            &root.join("records"),
            &root.join("store"),
            &signer,
        )
        .unwrap();
        assert!(handled, "unexpected service route");
        *requests.entry(format!("{method} {path}")).or_default() += 1;
        let snapshot = node.lock().unwrap().snapshot();
        // Operator evidence reads the immutable execution record/CAS, never the redacted public projection.
        let identity =
            super::super::super::world_service_read::identity(&root.join("world"), "w1").unwrap();
        let pinned = super::super::super::world_service_read::pin(
            &root.join("records"),
            &root.join("store"),
            &identity,
            None,
        )
        .unwrap();
        let owner_public = sign_read_request("owner", (), &hex::encode([7u8; 32]))
            .unwrap()
            .subject_public_key;
        let actual_nonce = pinned
            .world
            .state()
            .authenticated_collect_data_last_nonces
            .get("owner-a")
            .and_then(|nonces| nonces.get(&owner_public))
            .copied();
        let matching_events=pinned.world.journal().events.iter().filter(|event|matches!(&event.body,
            oasis7::runtime::WorldEventBody::Domain(oasis7::runtime::DomainEvent::DataCollectedAuthenticated {
                collector_agent_id,player_id,nonce:551,..}) if collector_agent_id=="agent-a" && player_id=="owner-a")).count();
        let witness = serde_json::json!({"schema":"pre2.actual-service-instance/v1",
            "pid":std::process::id(),"node_id":snapshot.node_id,"world_id":snapshot.world_id,
            "role":format!("{:?}",snapshot.role),"running":snapshot.running,
            "replication_enabled":snapshot.replication_enabled,"tick_count":snapshot.tick_count,
            "last_error":snapshot.last_error,"http_requests":requests,
            "canonical_gameplay_effect":{"commit":pinned.commit,"last_nonce":actual_nonce,"matching_nonce551_event_count":matching_events},
            "scope":"single real NodeRuntime instance, local execution bootstrap; consensus membership not evaluated"});
        let temporary = root.join("actual-service-witness.pending.json");
        fs::write(&temporary, serde_json::to_vec(&witness).unwrap()).unwrap();
        fs::rename(temporary, root.join("actual-service-witness.json")).unwrap();
    }
}

fn client() -> RemoteWorldServiceClient {
    RemoteWorldServiceClient::new(WorldServiceClientConfig {
        endpoint: std::env::var("OASIS7_WORLD_SERVICE_ENDPOINT").unwrap(),
        trusted_service_public_key: sign_read_request("service", (), &hex::encode([9u8; 32]))
            .unwrap()
            .subject_public_key,
        expected_world: WorldIdentity {
            world_id: "w1".into(),
            genesis_digest: "fixture-genesis-v1".into(),
        },
        scope_id: "agent:agent-a".into(),
        read_private_key_hex: hex::encode([7u8; 32]),
        timeout: Duration::from_secs(2),
        max_response_bytes: 1_048_576,
    })
    .unwrap()
}
fn view_request(
    client: &RemoteWorldServiceClient,
    min_commit: Option<CommitRef>,
) -> ReadWorldViewRequest {
    ReadWorldViewRequest {
        contract_version: 1,
        world: client.config().expected_world.clone(),
        scope_id: client.config().scope_id.clone(),
        min_commit,
        fixed_commit: None,
        deadline_unix_ms: None,
    }
}
fn send(stream: &mut BufReader<TcpStream>, value: serde_json::Value) {
    let mut bytes = serde_json::to_vec(&value).unwrap();
    bytes.push(b'\n');
    stream.get_mut().write_all(&bytes).unwrap();
}
fn receive(stream: &mut BufReader<TcpStream>, kind: &str) -> serde_json::Value {
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        let mut line = String::new();
        assert!(stream.read_line(&mut line).unwrap() > 0);
        let value: serde_json::Value = serde_json::from_str(&line).unwrap();
        assert!(
            !value["type"]
                .as_str()
                .is_some_and(|v| v.ends_with("_error")),
            "protocol error type={}",
            value["type"]
        );
        if value["type"] == kind {
            return value;
        }
        assert!(Instant::now() < deadline, "missing protocol message {kind}");
    }
}
#[test]
#[ignore = "real shipped Viewer and signed remote authority, no service mount"]
fn no_mount_application_acceptance() {
    assert!(!Path::new("/node").exists());
    let mounts = fs::read_to_string("/proc/self/mountinfo").unwrap();
    assert!(
        !mounts
            .lines()
            .any(|line| line.split_whitespace().nth(4) == Some("/node"))
    );
    let client = client();
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        if client.describe().is_ok() {
            break;
        }
        assert!(Instant::now() < deadline, "real service unavailable");
        thread::sleep(Duration::from_millis(50));
    }
    let initial = client.read_view(view_request(&client, None)).unwrap();
    let deadline = Instant::now() + Duration::from_secs(30);
    let socket = loop {
        if let Ok(socket) = TcpStream::connect("127.0.0.1:4100") {
            break socket;
        }
        assert!(
            Instant::now() < deadline,
            "shipped protocol listener unavailable"
        );
        thread::sleep(Duration::from_millis(50));
    };
    socket
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let mut stream = BufReader::new(socket);
    send(
        &mut stream,
        serde_json::json!({"type":"hello_v2","client":"PRE2 no-mount","version":2,"capabilities":[]}),
    );
    receive(&mut stream, "hello_ack");
    send(
        &mut stream,
        serde_json::json!({"type":"subscribe","streams":["snapshot","events"],"event_kinds":[]}),
    );
    send(&mut stream, serde_json::json!({"type":"request_snapshot"}));
    receive(&mut stream, "snapshot");
    use oasis7::viewer::{CollectDataCommand, CollectDataRequest, sign_collect_data_auth_proof};
    let owner = hex::encode([7u8; 32]);
    let public = sign_read_request("owner", (), &owner)
        .unwrap()
        .subject_public_key;
    let mut command = CollectDataCommand::Submit {
        request: CollectDataRequest {
            electricity_cost: 7,
            data_amount: 11,
            player_id: "owner-a".into(),
            public_key: Some(public.clone()),
            auth: None,
        },
    };
    let auth = sign_collect_data_auth_proof(&command, 551, &public, &owner).unwrap();
    if let CollectDataCommand::Submit { request } = &mut command {
        request.auth = Some(auth);
    }
    let payload = WorldServicePayloadV1::GameplayJson(serde_json::to_vec(&command).unwrap());
    let correlation =
        oasis7::world_service::derive_correlation(client.config().expected_world.clone(), &payload)
            .unwrap();
    send(
        &mut stream,
        serde_json::json!({"type":"collect_data","command":command}),
    );
    receive(&mut stream, "gameplay_action_ack");
    let deadline = Instant::now() + Duration::from_secs(30);
    let original_result = loop {
        let response = client
            .lookup(
                LookupIntentRequest {
                    contract_version: 1,
                    key: correlation.key.clone(),
                },
                payload.clone(),
            )
            .unwrap();
        if matches!(&response.outcome, IntentOutcome::Committed { .. }) {
            break response;
        }
        assert!(
            Instant::now() < deadline,
            "gameplay original lookup unresolved"
        );
        thread::sleep(Duration::from_millis(20));
    };
    let IntentOutcome::Committed {
        commit: committed, ..
    } = &original_result.outcome
    else {
        unreachable!()
    };
    fs::write("/app-private/endpoint-original.json",serde_json::to_vec(&serde_json::json!({"payload":payload,"correlation":correlation,"result":original_result,"command":command})).unwrap()).unwrap();
    let current = client
        .read_view(view_request(&client, Some(committed.clone())))
        .unwrap();
    let changes = client
        .read_changes(ReadWorldChangesRequest {
            contract_version: 1,
            cursor: initial.continuation().clone(),
            max_items: 128,
            max_bytes: 1_048_576,
        })
        .unwrap();
    assert!(!changes.changes.is_empty());
    send(
        &mut stream,
        serde_json::json!({"type":"live_control","mode":{"mode":"play"},"request_id":701}),
    );
    let deadline = Instant::now() + Duration::from_secs(35);
    loop {
        let view = client
            .read_view(view_request(
                &client,
                Some(current.version().commit.clone()),
            ))
            .unwrap();
        if view.projection().state.agents["agent-a"].state.pos == oasis7::GeoPos::new(2, 2, 0)
            && view.projection().cognition_leases.iter().any(|lease| {
                lease.status == oasis7::runtime::CognitionLeaseStatusV1::Settled
                    && lease.settled_amount > 0
            })
        {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "shipped Hosted provider did not produce canonical action and settled lease"
        );
        thread::sleep(Duration::from_millis(50));
    }
    send(
        &mut stream,
        serde_json::json!({"type":"live_control","mode":{"mode":"pause"},"request_id":702}),
    );
    send(&mut stream, serde_json::json!({"type":"request_snapshot"}));
    receive(&mut stream, "snapshot");
    let checkpoint: serde_json::Value =
        serde_json::from_slice(&fs::read("/app-private/lineage.json").unwrap()).unwrap();
    assert_eq!(
        checkpoint["provider_terminal_states"]["agent-a"]["status"],
        "committed"
    );
    let entries = checkpoint["provider_scheduler_pending"]
        .as_object()
        .unwrap();
    let mut committed_agent_operations = 0;
    for entry in entries.values() {
        let payload: WorldServicePayloadV1 =
            serde_json::from_value(entry["payload"].clone()).unwrap();
        let correlation: oasis7_client_api::world_service::RequestCorrelation =
            serde_json::from_value(entry["correlation"].clone()).unwrap();
        let result = client
            .lookup(
                LookupIntentRequest {
                    contract_version: 1,
                    key: correlation.key,
                },
                payload,
            )
            .unwrap();
        if let IntentOutcome::Committed { commit, .. } = result.outcome {
            client
                .read_view(view_request(&client, Some(commit)))
                .unwrap();
            committed_agent_operations += 1;
        }
    }
    assert!(
        committed_agent_operations >= 3,
        "actual original reserve/prefix/settle lookups absent"
    );
    fs::write(
        Path::new("/app-private").join("provider-stop"),
        b"actual application acceptance complete",
    )
    .unwrap();
    client.describe().unwrap(); // actual final request refreshes the operator-only service witness.
    println!("PRE2_NO_MOUNT_SHIPPED_GAMEPLAY_AGENT_FIVE_OPS_PASSED");
}

#[test]
#[ignore = "same shipped Viewer artifact restarted with endpoint alias; same actual Node"]
fn no_mount_endpoint_switch_acceptance() {
    assert!(!Path::new("/node").exists());
    let mounts = fs::read_to_string("/proc/self/mountinfo").unwrap();
    assert!(
        !mounts
            .lines()
            .any(|line| line.split_whitespace().nth(4) == Some("/node"))
    );
    let original: serde_json::Value =
        serde_json::from_slice(&fs::read("/app-private/endpoint-original.json").unwrap()).unwrap();
    let payload: WorldServicePayloadV1 =
        serde_json::from_value(original["payload"].clone()).unwrap();
    let correlation: oasis7_client_api::world_service::RequestCorrelation =
        serde_json::from_value(original["correlation"].clone()).unwrap();
    let mut config = client().config().clone();
    assert_eq!(config.endpoint, "http://world-service-alt:4200");
    config.endpoint = "http://world-service:4200".into();
    let first = RemoteWorldServiceClient::new(config).unwrap();
    let second = client();
    let describe_first = first.describe().unwrap();
    let describe_second = second.describe().unwrap();
    assert_eq!(describe_first.world, describe_second.world);
    let original_response = second
        .lookup(
            LookupIntentRequest {
                contract_version: 1,
                key: correlation.key.clone(),
            },
            payload.clone(),
        )
        .unwrap();
    assert_eq!(
        serde_json::to_value(&original_response).unwrap(),
        original["result"]
    );
    let IntentOutcome::Committed { commit, .. } = &original_response.outcome else {
        panic!("original result lost after endpoint change")
    };
    let before = second
        .read_view(view_request(&second, Some(commit.clone())))
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(30);
    let socket = loop {
        if let Ok(socket) = TcpStream::connect("127.0.0.1:4100") {
            break socket;
        }
        assert!(
            Instant::now() < deadline,
            "same-artifact switched Viewer unavailable"
        );
        thread::sleep(Duration::from_millis(50));
    };
    socket
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let mut stream = BufReader::new(socket);
    send(
        &mut stream,
        serde_json::json!({"type":"hello_v2","client":"PRE2 endpoint switch","version":2,"capabilities":[]}),
    );
    assert_eq!(receive(&mut stream, "hello_ack")["world_id"], "w1");
    send(
        &mut stream,
        serde_json::json!({"type":"subscribe","streams":["snapshot","events"],"event_kinds":[]}),
    );
    send(&mut stream, serde_json::json!({"type":"request_snapshot"}));
    receive(&mut stream, "snapshot");
    let recovery = receive(&mut stream, "authoritative_recovery_ack");
    assert!(recovery["ack"]["snapshot_height"].as_u64().unwrap() >= commit.position);
    send(
        &mut stream,
        serde_json::json!({"type":"collect_data","command":original["command"]}),
    );
    receive(&mut stream, "gameplay_action_ack");
    let replay = second
        .lookup(
            LookupIntentRequest {
                contract_version: 1,
                key: correlation.key.clone(),
            },
            payload.clone(),
        )
        .unwrap();
    assert_eq!(serde_json::to_value(replay).unwrap(), original["result"]);
    let duplicate = second
        .submit(SubmitIntentRequest {
            contract_version: 1,
            correlation,
            deadline_unix_ms: None,
            signed_payload: payload,
        })
        .unwrap();
    let SubmitObservation::Response(duplicate) = duplicate else {
        panic!("duplicate original outcome unknown")
    };
    assert_eq!(serde_json::to_value(duplicate).unwrap(), original["result"]);
    second
        .read_view(view_request(&second, Some(commit.clone())))
        .unwrap();
    let changes = second
        .read_changes(ReadWorldChangesRequest {
            contract_version: 1,
            cursor: before.continuation().clone(),
            max_items: 128,
            max_bytes: 1_048_576,
        })
        .unwrap();
    for change in changes.changes {
        let event: oasis7::runtime::WorldEvent = serde_json::from_value(change.change).unwrap();
        assert!(
            !matches!(
                event.body,
                oasis7::runtime::WorldEventBody::Domain(
                    oasis7::runtime::DomainEvent::DataCollectedAuthenticated { nonce: 551, .. }
                )
            ),
            "duplicate gameplay effect after endpoint switch"
        );
    }
    println!("PRE2_NO_MOUNT_SAME_VIEWER_ARTIFACT_ENDPOINT_ALIAS_PASSED");
}
