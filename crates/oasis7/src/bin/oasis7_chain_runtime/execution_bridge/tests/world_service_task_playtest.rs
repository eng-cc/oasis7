//! Explicit prepared-world live service for independent formal-Skill gameplay.
use super::*;
use oasis7::world_service::{
    client::{RemoteWorldServiceClient, WorldServiceClientConfig, WorldServicePort},
    private_goal_prepare::sign_goal_prepare,
    private_goal_read::publication,
    private_goal_store::PrivateGoalStore,
};
use oasis7_agent_api::game::{
    DecimalU64, GAME_EXECUTOR_CONTROL_DOMAIN, GAME_GOAL_PUBLISH_DOMAIN, GameExecutorControlV1,
    GameExecutorOperationV1, GameGoalPrepareV1, GameTaskFenceV1, GoalContentV1,
    game_goal_version_digest,
};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
fn write_private(path: &Path, bytes: &[u8]) {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .unwrap();
    file.write_all(bytes).unwrap();
    file.sync_all().unwrap();
}
fn write_public_json(path: &Path, value: &serde_json::Value) {
    let temporary = path.with_extension("pending.json");
    fs::write(&temporary, serde_json::to_vec(value).unwrap()).unwrap();
    fs::rename(temporary, path).unwrap();
}
#[test]
#[ignore = "explicit 360-second prepared-world service; independent player uses shipped task Skill"]
fn private_task_prepared_world_live_service() {
    let ready = std::path::PathBuf::from(
        std::env::var_os("OASIS7_TASK_PLAYTEST_READY").expect("explicit ready path required"),
    );
    assert!(ready.is_absolute());
    assert!(!ready.exists());
    let (dir, mut driver, world, owner, owner_public) =
        canonical_agent_chat::driver_fixture_with_verified_authority();
    let create = game_task_tests::create(&world, &owner, "context-task");
    commit(&mut driver, 3, create.clone());
    let c = game_task::current_control(&driver.execution_world, "agent-a")
        .unwrap()
        .unwrap();
    let private = dir.canonicalize().unwrap().join("private-goals");
    let store = PrivateGoalStore::open(&private, [7; 32]).unwrap();
    let content = GoalContentV1 {
        goal_text: format!(
            "Reach the landmark at coordinates (200, 0, 0) centimeters. Read the complete goal and choose a valid movement action yourself. {}",
            "The task goal is delivered privately without truncation.".repeat(24)
        ),
        soft_constraints: vec![
            "Use the currently bound executor and canonical signed task action lane.".into(),
        ],
    };
    let goal = sign_goal_prepare(
        GameGoalPrepareV1 {
            api_rev: 1,
            world_id: world.world_id.clone(),
            genesis_digest: world.genesis_digest.clone(),
            task_id: c.task_id.clone(),
            agent_id: c.agent_id.clone(),
            owner_public_key: owner_public.clone(),
            owner_binding: c.owner_binding.clone(),
            owner_generation: c.owner_generation,
            branch_id: c.branch_id.clone(),
            reorg_epoch: c.reorg_epoch,
            expected_task_revision: c.task_revision,
            expected_latest_goal_revision: c.latest_goal_revision,
            goal_revision: DecimalU64(1),
            client_operation_id: "context-goal".into(),
            client_nonce: "ab".repeat(32),
            goal_digest: game_goal_version_digest(&c.task_id, DecimalU64(1), &content).unwrap(),
            content,
        },
        &owner,
    )
    .unwrap();
    let receipt = store.prepare(&goal).unwrap();
    let publish = publication(&goal, &receipt, "context-publish".into()).unwrap();
    commit(
        &mut driver,
        4,
        request(
            &world,
            WorldServicePayloadV1::GameGoalPublish(
                sign_read_request(GAME_GOAL_PUBLISH_DOMAIN, publish, &owner).unwrap(),
            ),
        ),
    );
    let delegate = hex::encode([73; 32]);
    let delegate_public = sign_read_request("key", (), &delegate)
        .unwrap()
        .subject_public_key;
    commit(
        &mut driver,
        5,
        request(
            &world,
            WorldServicePayloadV1::Delegation(
                sign_read_request(
                    "delegation",
                    AgentSignerDelegationChangeV1 {
                        world_id: world.world_id.clone(),
                        branch_id: "main".into(),
                        agent_id: "agent-a".into(),
                        owner_binding: "owner-a".into(),
                        agent_identity_generation: 1,
                        delegate_public_key: delegate_public.clone(),
                        generation: 1,
                        revoked: false,
                        nonce: 1,
                    },
                    &owner,
                )
                .unwrap(),
            ),
        ),
    );
    let c = game_task::current_control(&driver.execution_world, "agent-a")
        .unwrap()
        .unwrap();
    commit(
        &mut driver,
        6,
        request(
            &world,
            WorldServicePayloadV1::GameExecutorControl(
                sign_read_request(
                    GAME_EXECUTOR_CONTROL_DOMAIN,
                    GameExecutorControlV1 {
                        api_rev: 1,
                        client_operation_id: "bind-context".into(),
                        fence: GameTaskFenceV1::from_control(&c),
                        operation: GameExecutorOperationV1::Bind {
                            executor_id: "context-executor".into(),
                            executor_public_key: delegate_public,
                            delegation_generation: DecimalU64(1),
                        },
                    },
                    &owner,
                )
                .unwrap(),
            ),
        ),
    );

    let root = dir.canonicalize().unwrap();
    drop(driver);
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
    let config = oasis7_node::NodeConfig::new("node-a", "w1", NodeRole::Sequencer)
        .unwrap()
        .with_tick_interval(Duration::from_secs(5))
        .unwrap();
    let mut node = oasis7_node::NodeRuntime::new(config)
        .with_execution_hook(driver)
        .with_local_execution_bootstrap(bootstrap);
    node.start().unwrap();
    let node = Arc::new(Mutex::new(node));
    let service_key = hex::encode([9; 32]);
    let service_public = sign_read_request("service", (), &service_key)
        .unwrap()
        .subject_public_key;
    let signer = crate::feedback_submit_api::FeedbackSubmitSigner {
        private_key_hex: service_key,
        public_key_hex: service_public.clone(),
    };
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let device = root.join("client-device");
    fs::create_dir(&device).unwrap();
    fs::set_permissions(&device, fs::Permissions::from_mode(0o700)).unwrap();
    let state = device.join("state");
    fs::create_dir(&state).unwrap();
    fs::set_permissions(&state, fs::Permissions::from_mode(0o700)).unwrap();
    let signer_file = device.join("executor.key");
    write_private(&signer_file, &hex::decode(&delegate).unwrap());
    let config_path = device.join("client.json");
    write_private(&config_path,&serde_json::to_vec(&serde_json::json!({"endpoint":endpoint,"world_id":world.world_id,"genesis_digest":world.genesis_digest,"agent_id":"agent-a","service_public_key":service_public,"signer_file":signer_file,"state_directory":state})).unwrap());
    let owner_client = RemoteWorldServiceClient::new(WorldServiceClientConfig {
        endpoint: endpoint.clone(),
        trusted_service_public_key: signer.public_key_hex.clone(),
        expected_world: world.clone(),
        scope_id: "agent:agent-a".into(),
        read_private_key_hex: owner.clone(),
        timeout: Duration::from_secs(5),
        max_response_bytes: 2 * 1024 * 1024,
    })
    .unwrap();
    let stop = AtomicBool::new(false);
    std::thread::scope(|scope| {
        let server = scope.spawn(|| {
            while !stop.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        stream
                            .set_read_timeout(Some(Duration::from_secs(5)))
                            .unwrap();
                        let mut bytes = Vec::new();
                        while !bytes.ends_with(b"\r\n\r\n") {
                            let mut byte = [0; 1];
                            if stream.read_exact(&mut byte).is_err() {
                                break;
                            }
                            bytes.push(byte[0]);
                            if bytes.len() > 65536 {
                                break;
                            }
                        }
                        let Ok(headers) = std::str::from_utf8(&bytes) else {
                            continue;
                        };
                        let mut first = headers.split_whitespace();
                        let Some(method) = first.next().map(str::to_owned) else {
                            continue;
                        };
                        let Some(path) = first.next().map(str::to_owned) else {
                            continue;
                        };
                        let length = headers
                            .split("\r\n")
                            .filter_map(|line| line.split_once(':'))
                            .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                            .and_then(|(_, v)| v.trim().parse::<usize>().ok())
                            .unwrap_or(0);
                        if length > 256 * 1024 {
                            continue;
                        }
                        let mut body = vec![0; length];
                        if stream.read_exact(&mut body).is_err() {
                            continue;
                        }
                        bytes.extend(body);
                        let handled = crate::world_service_api::private_goal::maybe_handle(
                            &mut stream,
                            &bytes,
                            &node,
                            &method,
                            &path,
                            "w1",
                            &root.join("world"),
                            &root.join("records"),
                            &root.join("store"),
                            &signer,
                            Some(&store),
                        )
                        .unwrap();
                        if !handled {
                            assert!(
                                crate::world_service_api::maybe_handle(
                                    &mut stream,
                                    &bytes,
                                    &node,
                                    &method,
                                    &path,
                                    "w1",
                                    &root.join("world"),
                                    &root.join("records"),
                                    &root.join("store"),
                                    &signer
                                )
                                .unwrap()
                            );
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(20))
                    }
                    Err(error) => panic!("live service listener failed: {error}"),
                }
            }
        });
        write_public_json(
            &ready,
            &serde_json::json!({"schema":"oasis7/task-prepared-world-ready/v1","pid":std::process::id(),"endpoint":endpoint,"config":config_path,"root":root,"witness":root.join("task-playtest-witness.json"),"scope":"prepared world; real native API and NodeRuntime worker; consensus membership not evaluated"}),
        );
        let deadline = Instant::now() + Duration::from_secs(360);
        let mut goal2_submission = None;
        while Instant::now() < deadline && !root.join("service-stop").exists() {
            let pinned = super::super::super::world_service_read::pin(
                &root.join("records"),
                &root.join("store"),
                &world,
                None,
            )
            .unwrap();
            let control = game_task::current_control(&pinned.world, "agent-a")
                .unwrap()
                .unwrap();
            let position = pinned.world.state().agents["agent-a"].state.pos;
            if position == oasis7::GeoPos::new(200, 0, 0)
                && control.active_goal_revision == DecimalU64(1)
                && goal2_submission.is_none()
            {
                let mut goal2 = goal.request.clone();
                goal2.goal_revision = DecimalU64(2);
                goal2.expected_task_revision = control.task_revision;
                goal2.expected_latest_goal_revision = control.latest_goal_revision;
                goal2.client_operation_id = "live-g2-prepare".into();
                goal2.client_nonce = "45".repeat(32);
                goal2.content=GoalContentV1{goal_text:"The first landmark is complete. Reach the second landmark at coordinates (-200, 0, 0) centimeters, using this new complete goal and a fresh decision context. Choose a valid movement action yourself.".into(),soft_constraints:vec!["Use the currently bound executor and canonical signed action lane.".into()]};
                goal2.goal_digest =
                    game_goal_version_digest(&goal2.task_id, goal2.goal_revision, &goal2.content)
                        .unwrap();
                let goal2 = sign_goal_prepare(goal2, &owner).unwrap();
                let prepared = owner_client.prepare_goal(&goal2).unwrap();
                let public = publication(&goal2, &prepared, "live-g2-publish".into()).unwrap();
                let submission = request(
                    &world,
                    WorldServicePayloadV1::GameGoalPublish(
                        sign_read_request(GAME_GOAL_PUBLISH_DOMAIN, public, &owner).unwrap(),
                    ),
                );
                let observed = owner_client.submit(submission.clone()).unwrap();
                assert!(!matches!(
                    observed,
                    oasis7::world_service::SubmitObservation::OutcomeUnknown(_)
                ));
                goal2_submission = Some(submission);
            }
            let effects = pinned
                .world
                .capability_revocation_state()
                .world_service_results
                .values()
                .filter_map(|value| {
                    serde_json::from_value::<CanonicalIntentResultV1>(value.clone()).ok()
                })
                .filter(|result| {
                    matches!(
                        &result.request.signed_payload,
                        WorldServicePayloadV1::Scheduler(_)
                            | WorldServicePayloadV1::Cognition(_)
                            | WorldServicePayloadV1::GameGoalPublish(_)
                            | WorldServicePayloadV1::GameExecutorControl(_)
                    )
                })
                .collect::<Vec<_>>();
            let node_snapshot = node.lock().unwrap().snapshot();
            write_public_json(
                &root.join("task-playtest-witness.json"),
                &serde_json::json!({"schema":"oasis7/task-prepared-world-witness/v1","commit":pinned.commit,"task_revision":control.task_revision,"goal_revision":control.active_goal_revision,"executor_epoch":control.executor_epoch,"executor_binding":control.executor_binding,"position":position,"canonical_effect_receipts":effects,"goal2_public_submission":goal2_submission,"node":{"node_id":node_snapshot.node_id,"running":node_snapshot.running,"tick_count":node_snapshot.tick_count,"last_error":node_snapshot.last_error},"scope":"actual immutable canonical pin; setup owner only changes goals; player actions use native submitted intents"}),
            );
            std::thread::sleep(Duration::from_millis(250));
        }
        // Revoke this fixture's execution authority through canonical operations
        // while the real service is still running, then close only our worker.
        let pinned = super::super::super::world_service_read::pin(
            &root.join("records"),
            &root.join("store"),
            &world,
            None,
        )
        .unwrap();
        let control = game_task::current_control(&pinned.world, "agent-a")
            .unwrap()
            .unwrap();
        let stop_request = request(
            &world,
            WorldServicePayloadV1::GameExecutorControl(
                sign_read_request(
                    GAME_EXECUTOR_CONTROL_DOMAIN,
                    GameExecutorControlV1 {
                        api_rev: 1,
                        client_operation_id: "playtest-cleanup-stop".into(),
                        fence: GameTaskFenceV1::from_control(&control),
                        operation: GameExecutorOperationV1::Stop,
                    },
                    &owner,
                )
                .unwrap(),
            ),
        );
        let revoke_request = request(
            &world,
            WorldServicePayloadV1::Delegation(
                sign_read_request(
                    "delegation",
                    AgentSignerDelegationChangeV1 {
                        world_id: world.world_id.clone(),
                        branch_id: "main".into(),
                        agent_id: "agent-a".into(),
                        owner_binding: "owner-a".into(),
                        agent_identity_generation: 1,
                        delegate_public_key: sign_read_request("key", (), &delegate)
                            .unwrap()
                            .subject_public_key,
                        generation: 2,
                        revoked: true,
                        nonce: 2,
                    },
                    &owner,
                )
                .unwrap(),
            ),
        );
        let mut cleanup = Vec::new();
        for original in [stop_request, revoke_request] {
            owner_client.submit(original.clone()).unwrap();
            let until = Instant::now() + Duration::from_secs(20);
            loop {
                let result = owner_client
                    .lookup(
                        LookupIntentRequest {
                            contract_version: 1,
                            key: original.correlation.key.clone(),
                        },
                        original.signed_payload.clone(),
                    )
                    .unwrap();
                if matches!(result.outcome, IntentOutcome::Committed { .. }) {
                    cleanup.push(serde_json::to_value(result).unwrap());
                    break;
                }
                assert!(!matches!(
                    result.outcome,
                    IntentOutcome::Rejected { .. } | IntentOutcome::Expired
                ));
                assert!(
                    Instant::now() < until,
                    "fixture authority cleanup did not commit"
                );
                std::thread::sleep(Duration::from_millis(100));
            }
        }
        write_public_json(
            &root.join("task-playtest-cleanup.json"),
            &serde_json::json!({
                "schema":"oasis7/task-prepared-world-cleanup/v1", "authority_operations":cleanup,
            }),
        );
        stop.store(true, Ordering::SeqCst);
        server.join().unwrap();
    });
    node.lock().unwrap().stop().unwrap();
    println!("OASIS7_TASK_PREPARED_WORLD_SERVICE_STOPPED");
}
