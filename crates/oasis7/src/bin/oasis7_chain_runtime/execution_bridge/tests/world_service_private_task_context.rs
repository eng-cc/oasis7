use super::private_goal_tests::{consumer, http, http_with_hook};
use super::*;
use oasis7::world_service::{
    private_goal_prepare::sign_goal_prepare, private_goal_read::publication,
    private_goal_store::PrivateGoalStore, private_task_context::*,
};
use oasis7_agent_api::game::{
    DecimalU64, GAME_EXECUTOR_CONTROL_DOMAIN, GAME_GOAL_PUBLISH_DOMAIN, GameExecutorControlV1,
    GameExecutorOperationV1, GameGoalPrepareV1, GameTaskFenceV1, GoalContentV1,
    game_goal_version_digest,
};
use std::sync::{Arc, Mutex};

#[test]
fn private_task_context_native_complete_prepare_capture_read_action_and_restart() {
    const AGE: u64 = 34;
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
        goal_text: format!("完整任务正文{}", "x".repeat(1800)),
        soft_constraints: vec!["actual full constraint".into()],
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
    let c = game_task::current_control(&driver.execution_world, "agent-a")
        .unwrap()
        .unwrap();
    let fence = GameTaskFenceV1::from_control(&c);
    let signer = crate::feedback_submit_api::FeedbackSubmitSigner {
        private_key_hex: owner.clone(),
        public_key_hex: owner_public,
    };
    let runtime = Arc::new(Mutex::new(oasis7_node::NodeRuntime::new(
        oasis7_node::NodeConfig::new("node-a", "w1", NodeRole::Sequencer).unwrap(),
    )));
    let proof = sign_read_request(
        TASK_CONTEXT_PREPARE_PATH,
        TaskContextPrepareV1 {
            schema_revision: 1,
            fence: fence.clone(),
            client_operation_id: "context-operation".into(),
            client_nonce: "cd".repeat(32),
        },
        &delegate,
    )
    .unwrap();
    // Active goal evidence survives both the normal 32-block window and full reconciliation.
    for height in 7..40 {
        commit(&mut driver, height, create.clone());
    }
    super::super::super::checkpoint::task_evidence::reset_protected_height_validation_count();
    commit(&mut driver, 40, create.clone());
    assert_eq!(
        super::super::super::checkpoint::task_evidence::protected_height_validation_count(),
        1,
        "one retention transaction must fully validate existing task-pin shards once"
    );
    let cas = LocalCasStore::new(dir.join("store"));
    assert_eq!(
        cas.list_pin_scope_shards("execution_bridge_live_tasks_v1")
            .unwrap(),
        vec![format!("record-{:020}", 4)],
        "published task evidence shards must equal the current live dependency set"
    );
    // An unchanged, fully verified task dependency must not be re-published.
    // A historical mtime distinguishes reuse from atomic rewriting without
    // depending on filesystem timestamp granularity or execution speed.
    let shard = dir
        .join("store/pin_scopes/execution_bridge_live_tasks_v1/record-00000000000000000004.json");
    let sentinel = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1);
    std::fs::File::options()
        .write(true)
        .open(&shard)
        .unwrap()
        .set_times(std::fs::FileTimes::new().set_modified(sentinel))
        .unwrap();
    let retained_bytes = std::fs::read(&shard).unwrap();
    for _ in 0..3 {
        super::super::super::checkpoint::refresh_live_task_read_evidence(
            &dir.join("records"),
            &cas,
            &driver.execution_world,
            2,
        )
        .unwrap();
        assert_eq!(std::fs::read(&shard).unwrap(), retained_bytes);
        assert_eq!(
            std::fs::metadata(&shard).unwrap().modified().unwrap(),
            sentinel,
            "unchanged validated task shard was rewritten"
        );
    }
    // Reuse must still reject a parseable shard with the wrong semantic set.
    std::fs::write(&shard, br#"{"pins":[]}"#).unwrap();
    assert!(
        super::super::super::checkpoint::refresh_live_task_read_evidence(
            &dir.join("records"),
            &cas,
            &driver.execution_world,
            2,
        )
        .is_err()
    );
    let latest_record_path =
        super::super::super::checkpoint::execution_bridge_record_path(&dir.join("records"), 40);
    let latest_record =
        super::super::super::checkpoint::load_execution_bridge_record(latest_record_path.as_path())
            .unwrap();
    assert!(
        super::super::super::checkpoint::run_execution_bridge_incremental_retention_maintenance(
            &dir.join("records"),
            &cas,
            &latest_record,
            2,
            32,
            3,
        )
        .is_err(),
        "standalone incremental maintenance must revalidate a corrupt task-pin shard"
    );
    assert!(
        super::super::super::checkpoint::run_execution_bridge_retention_maintenance(
            &dir.join("records"),
            &cas,
            2,
        )
        .is_err(),
        "full reconciliation must revalidate a corrupt task-pin shard"
    );
    std::fs::write(&shard, &retained_bytes).unwrap();
    let validated = super::super::super::checkpoint::refresh_live_task_read_evidence(
        &dir.join("records"),
        &cas,
        &driver.execution_world,
        2,
    )
    .unwrap();
    let mut replacement_record = latest_record.clone();
    replacement_record.node_block_hash = Some("replacement-head-hash".into());
    assert!(
        super::super::super::checkpoint::run_execution_bridge_incremental_retention_maintenance_after_task_refresh(
            &dir.join("records"),
            &cas,
            &replacement_record,
            validated,
            2,
            32,
            3,
        )
        .is_err(),
        "validated task-pin set must not be reused for a replaced same-height record"
    );
    super::super::super::checkpoint::run_execution_bridge_retention_maintenance(
        &dir.join("records"),
        &cas,
        2,
    )
    .unwrap();
    assert!(
        super::super::super::world_service_read::pin_at_height(
            &dir.join("records"),
            &dir.join("store"),
            &world,
            4
        )
        .is_ok()
    );
    // The live-task cache is derived: remove it and restart from the real current head.
    cas.clear_pin_scope("execution_bridge_live_tasks_v1")
        .unwrap();
    drop(driver);
    let bootstrap = super::super::super::local_bootstrap::derive_service_execution_bootstrap(
        &dir.join("world"),
        &dir.join("records"),
        &dir.join("store"),
        "w1",
    )
    .unwrap();
    let mut driver = NodeRuntimeExecutionDriver::new_with_local_bootstrap(
        dir.join("state.json"),
        dir.join("world"),
        dir.join("records"),
        dir.join("store"),
        &oasis7_proto::storage_profile::StorageProfileConfig::default(),
        bootstrap,
    )
    .unwrap();
    let prepared = consumer(&dir, &runtime, &signer, &store, &world, |client| {
        client.prepare_task_context(&proof).unwrap()
    });
    assert_eq!(
        prepared.snapshot.active_goal.owner_proof.request.content,
        goal.request.content
    );
    let original_context_file = std::fs::read_dir(&private)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("context-")
        })
        .unwrap();
    drop(store);
    let store = PrivateGoalStore::open(&private, [7; 32]).unwrap();
    let recovered = store.lookup_task_context(&proof).unwrap().unwrap();
    assert_eq!(prepared.identity, recovered.identity);
    assert_eq!(prepared.reference_salt, recovered.reference_salt);
    let mut changed = proof.request.clone();
    changed.client_nonce = "ef".repeat(32);
    let changed = sign_read_request(TASK_CONTEXT_PREPARE_PATH, changed, &delegate).unwrap();
    assert!(matches!(
        store.lookup_task_context(&changed),
        Err(oasis7::world_service::private_goal_store::PrivateTaskContextError::Conflict)
    ));
    let get = sign_read_request(
        TASK_CONTEXT_READ_PATH,
        TaskContextGetV1 {
            schema_revision: 1,
            context: prepared.identity.clone(),
            prepare_operation_id: proof.request.client_operation_id.clone(),
        },
        &delegate,
    )
    .unwrap();
    assert_eq!(
        http(
            &dir,
            &runtime,
            &signer,
            Some(&store),
            TASK_CONTEXT_READ_PATH,
            &get,
            ""
        )
        .0,
        409
    );
    let prefix = public_prefix(&prepared).unwrap();
    // A valid bound executor cannot extend a public context past the lease
    // simply by bypassing encrypted preparation and signing another prefix.
    let mut overlong = prefix.clone();
    let SchedulerOperationV1::TaskProviderPrefix { context } = &mut overlong.operation else {
        unreachable!()
    };
    context.valid_until_tick.0 += 1;
    let overlong = sign_read_request("scheduler", overlong, &delegate).unwrap();
    assert!(
        derive_correlation(
            world.clone(),
            &WorldServicePayloadV1::Scheduler(overlong.clone())
        )
        .is_err(),
        "correctly signed overlong public prefix must fail correlation validation"
    );
    let before_overlong = driver.execution_world.snapshot();
    assert!(
        driver
            .execution_world
            .apply_authenticated_scheduler(overlong)
            .is_err()
    );
    assert_eq!(driver.execution_world.snapshot(), before_overlong);

    let public = serde_json::to_string(&prefix).unwrap();
    for secret in [
        &goal.request.content.goal_text,
        &goal.request.goal_digest.0,
        &prepared.reference_salt,
        &proof.request.client_nonce,
    ] {
        assert!(!public.contains(secret));
    }
    let submit = request(
        &world,
        WorldServicePayloadV1::Scheduler(
            sign_read_request("scheduler", prefix, &delegate).unwrap(),
        ),
    );
    commit(&mut driver, AGE + 7, submit.clone());
    assert!(
        game_task_tests::recorded(&driver, &submit)
            .rejected
            .is_none(),
        "canonical action rejection: {:?}",
        game_task_tests::recorded(&driver, &submit).rejected
    );
    // A distinct preparation uses a fresh public base, but cannot open a second live turn.
    let mut competing = proof.request.clone();
    competing.client_operation_id = "competing-context".into();
    let competing = sign_read_request(TASK_CONTEXT_PREPARE_PATH, competing, &delegate).unwrap();
    let competing = consumer(&dir, &runtime, &signer, &store, &world, |client| {
        client.prepare_task_context(&competing).unwrap()
    });
    let competing_submit = request(
        &world,
        WorldServicePayloadV1::Scheduler(
            sign_read_request("scheduler", public_prefix(&competing).unwrap(), &delegate).unwrap(),
        ),
    );
    commit(&mut driver, AGE + 8, competing_submit.clone());
    assert!(
        game_task_tests::recorded(&driver, &competing_submit)
            .rejected
            .is_some()
    );
    // A pending capture remains usable even when a full maintenance window excludes its height.
    super::super::super::checkpoint::run_execution_bridge_retention_maintenance(
        &dir.join("records"),
        &cas,
        1,
    )
    .unwrap();
    assert!(
        super::super::super::world_service_read::pin_at_height(
            &dir.join("records"),
            &dir.join("store"),
            &world,
            AGE + 7
        )
        .is_ok()
    );
    let captured = consumer(&dir, &runtime, &signer, &store, &world, |client| {
        client.read_task_context(&get).unwrap()
    });
    assert_eq!(captured.capture_commit.position, AGE + 7);
    assert_eq!(
        captured
            .prepared
            .snapshot
            .active_goal
            .owner_proof
            .request
            .content,
        goal.request.content
    );
    // Missing historical capture evidence fails before decrypting/delivering context.
    let capture_path = super::super::super::checkpoint::execution_bridge_record_path(
        &dir.join("records"),
        AGE + 7,
    );
    let capture_bytes = std::fs::read(&capture_path).unwrap();
    let mut missing: serde_json::Value = serde_json::from_slice(&capture_bytes).unwrap();
    missing["snapshot_ref"] = serde_json::Value::Null;
    std::fs::write(&capture_path, serde_json::to_vec(&missing).unwrap()).unwrap();
    let (status, error) = http(
        &dir,
        &runtime,
        &signer,
        Some(&store),
        TASK_CONTEXT_READ_PATH,
        &get,
        "",
    );
    assert_eq!(status, 503);
    assert_eq!(error["error"], "private_goal_recovery_required");
    assert!(error.get("payload").is_none());
    std::fs::write(&capture_path, capture_bytes).unwrap();
    // Authenticated capture cannot conceal a missing or corrupted encrypted record.
    let context_file = original_context_file;
    let encrypted = std::fs::read(&context_file).unwrap();
    std::fs::write(&context_file, b"corrupt").unwrap();
    assert_eq!(
        http(
            &dir,
            &runtime,
            &signer,
            Some(&store),
            TASK_CONTEXT_READ_PATH,
            &get,
            ""
        )
        .0,
        503
    );
    std::fs::write(&context_file, &encrypted).unwrap();
    for field in 0..4 {
        let mut bad = prepared.clone();
        match field {
            0 => bad
                .snapshot
                .active_goal
                .owner_proof
                .request
                .content
                .goal_text
                .push('!'),
            1 => bad.reference_salt = "00".repeat(32),
            2 => {
                bad.identity.provider_invocation_commitment =
                    "blake3:".to_owned() + &"00".repeat(32)
            }
            _ => bad.prepare_proof.request.client_nonce = "12".repeat(32),
        };
        assert!(bad.validate_against(&proof).is_err());
    }
    let action = cognition_intent(
        &captured,
        Action::MoveAgent {
            agent_id: "agent-a".into(),
            to: oasis7::GeoPos::new(1, 0, 0),
        },
    )
    .unwrap();
    let submit = request(
        &world,
        WorldServicePayloadV1::Cognition(
            sign_read_request("cognition", action, &delegate).unwrap(),
        ),
    );
    commit(&mut driver, AGE + 9, submit.clone());
    assert!(
        game_task_tests::recorded(&driver, &submit)
            .rejected
            .is_none(),
        "canonical action rejection: {:?}",
        game_task_tests::recorded(&driver, &submit).rejected
    );
    assert_eq!(
        driver.execution_world.state().agents["agent-a"].state.pos,
        oasis7::GeoPos::new(1, 0, 0)
    );
    assert_eq!(
        http(
            &dir,
            &runtime,
            &signer,
            Some(&store),
            TASK_CONTEXT_READ_PATH,
            &get,
            ""
        )
        .0,
        409
    );
    let canonical = serde_json::to_string(&driver.execution_world.snapshot()).unwrap();
    for secret in [
        &goal.request.content.goal_text,
        &goal.request.goal_digest.0,
        &prepared.reference_salt,
        &proof.request.client_nonce,
    ] {
        assert!(!canonical.contains(secret));
    }
    // Publish G2 through the canonical owner lane, retire G1, bind, and consume G2 in full.
    let before_goal = game_task::current_control(&driver.execution_world, "agent-a")
        .unwrap()
        .unwrap();
    let mut g2 = goal.request.clone();
    g2.expected_task_revision = before_goal.task_revision;
    g2.expected_latest_goal_revision = before_goal.latest_goal_revision;
    g2.goal_revision = DecimalU64(2);
    g2.client_operation_id = "g2-prepare".into();
    g2.client_nonce = "56".repeat(32);
    g2.content.goal_text = "G2 move two cells with new complete goal".into();
    g2.goal_digest = game_goal_version_digest(&g2.task_id, g2.goal_revision, &g2.content).unwrap();
    let g2 = sign_goal_prepare(g2, &owner).unwrap();
    let g2_receipt = store.prepare(&g2).unwrap();
    commit(
        &mut driver,
        AGE + 10,
        request(
            &world,
            WorldServicePayloadV1::GameGoalPublish(
                sign_read_request(
                    GAME_GOAL_PUBLISH_DOMAIN,
                    publication(&g2, &g2_receipt, "g2-publish".into()).unwrap(),
                    &owner,
                )
                .unwrap(),
            ),
        ),
    );
    assert_eq!(
        http(
            &dir,
            &runtime,
            &signer,
            Some(&store),
            TASK_CONTEXT_READ_PATH,
            &get,
            ""
        )
        .0,
        403
    );
    let retired = super::super::super::checkpoint::load_execution_bridge_record(
        &super::super::super::checkpoint::execution_bridge_record_path(&dir.join("records"), 4),
    )
    .unwrap();
    assert!(
        retired.snapshot_ref.is_none() && retired.journal_ref.is_none(),
        "replaced goal must release out-of-window full record pins"
    );
    assert!(
        !cas.list_pin_scope_shards("execution_bridge_live_tasks_v1")
            .unwrap()
            .contains(&format!("record-{:020}", 4))
    );
    assert_eq!(
        cas.list_pin_scope_shards("execution_bridge_live_tasks_v1")
            .unwrap(),
        vec![format!("record-{:020}", AGE + 10)],
        "retained task evidence shards must equal the live dependency set after goal replacement"
    );
    let current = game_task::current_control(&driver.execution_world, "agent-a")
        .unwrap()
        .unwrap();
    assert_eq!(
        current.executor_epoch,
        prepared.identity.fence.executor_epoch
    );
    assert_eq!(
        current.executor_binding,
        prepared.identity.fence.executor_binding
    );
    let fence = GameTaskFenceV1::from_control(
        &game_task::current_control(&driver.execution_world, "agent-a")
            .unwrap()
            .unwrap(),
    );
    let g2_prepare = sign_read_request(
        TASK_CONTEXT_PREPARE_PATH,
        TaskContextPrepareV1 {
            schema_revision: 1,
            fence: fence.clone(),
            client_operation_id: "g2-context".into(),
            client_nonce: "78".repeat(32),
        },
        &delegate,
    )
    .unwrap();
    let g2_context = consumer(&dir, &runtime, &signer, &store, &world, |client| {
        client.prepare_task_context(&g2_prepare).unwrap()
    });
    assert_eq!(
        g2_context.snapshot.active_goal.owner_proof.request.content,
        g2.request.content
    );
    assert_ne!(
        g2_context.identity.context_commitment,
        prepared.identity.context_commitment
    );
    commit(
        &mut driver,
        AGE + 11,
        request(
            &world,
            WorldServicePayloadV1::Scheduler(
                sign_read_request("scheduler", public_prefix(&g2_context).unwrap(), &delegate)
                    .unwrap(),
            ),
        ),
    );
    let g2_get = sign_read_request(
        TASK_CONTEXT_READ_PATH,
        TaskContextGetV1 {
            schema_revision: 1,
            context: g2_context.identity.clone(),
            prepare_operation_id: g2_prepare.request.client_operation_id.clone(),
        },
        &delegate,
    )
    .unwrap();
    let g2_captured = consumer(&dir, &runtime, &signer, &store, &world, |client| {
        client.read_task_context(&g2_get).unwrap()
    });
    let g2_action = request(
        &world,
        WorldServicePayloadV1::Cognition(
            sign_read_request(
                "cognition",
                cognition_intent(
                    &g2_captured,
                    Action::MoveAgent {
                        agent_id: "agent-a".into(),
                        to: oasis7::GeoPos::new(2, 0, 0),
                    },
                )
                .unwrap(),
                &delegate,
            )
            .unwrap(),
        ),
    );
    commit(&mut driver, AGE + 12, g2_action.clone());
    assert!(
        game_task_tests::recorded(&driver, &g2_action)
            .rejected
            .is_none()
    );
    assert_eq!(
        driver.execution_world.state().agents["agent-a"].state.pos,
        oasis7::GeoPos::new(2, 0, 0)
    );
    // A context lease expires before the still-valid grant. Cleanup is entirely public.
    let mut expiry_request = g2_prepare.request.clone();
    expiry_request.client_operation_id = "expiry-context".into();
    expiry_request.client_nonce = "90".repeat(32);
    let expiry_proof =
        sign_read_request(TASK_CONTEXT_PREPARE_PATH, expiry_request, &delegate).unwrap();
    let expiry = consumer(&dir, &runtime, &signer, &store, &world, |client| {
        client.prepare_task_context(&expiry_proof).unwrap()
    });
    assert_eq!(
        expiry.valid_until_tick.0,
        expiry.snapshot.captured_base_binding.base_tick + TASK_CONTEXT_LEASE_TICKS
    );
    assert!(
        expiry.valid_until_tick.0
            < expiry
                .snapshot
                .provider_authority
                .capability_catalog
                .valid_until_tick
    );
    let expiry_prefix = request(
        &world,
        WorldServicePayloadV1::Scheduler(
            sign_read_request("scheduler", public_prefix(&expiry).unwrap(), &delegate).unwrap(),
        ),
    );
    commit(&mut driver, AGE + 13, expiry_prefix.clone());
    assert!(
        game_task_tests::recorded(&driver, &expiry_prefix)
            .rejected
            .is_none()
    );
    let expiry_get = sign_read_request(
        TASK_CONTEXT_READ_PATH,
        TaskContextGetV1 {
            schema_revision: 1,
            context: expiry.identity.clone(),
            prepare_operation_id: expiry_proof.request.client_operation_id.clone(),
        },
        &delegate,
    )
    .unwrap();
    let mut expiry_height = AGE + 13;
    // Administrative receipts preserve simulation time; actual empty simulation
    // blocks advance ticks without fabricating a clock or player action.
    while driver.execution_world.state().time <= expiry.valid_until_tick.0 {
        expiry_height += 1;
        assert!(expiry_height <= AGE + 13 + TASK_CONTEXT_LEASE_TICKS + 1);
        driver
            .on_commit(NodeExecutionCommitContext {
                world_id: "w1".into(),
                node_id: "node-a".into(),
                proposer_id: "node-a".into(),
                height: expiry_height,
                slot: expiry_height,
                epoch: 0,
                node_block_hash: format!("node-h{expiry_height}"),
                action_root: compute_consensus_action_root(&[]).unwrap(),
                committed_actions: vec![],
                committed_at_unix_ms: expiry_height as i64 * 1000,
            })
            .unwrap();
    }
    assert!(driver.execution_world.state().time > expiry.valid_until_tick.0);
    assert_eq!(
        http(
            &dir,
            &runtime,
            &signer,
            Some(&store),
            TASK_CONTEXT_READ_PATH,
            &expiry_get,
            ""
        )
        .0,
        409
    );
    let failure = public_outcome_intent(
        &expiry.identity,
        driver
            .execution_world
            .current_cognition_base_binding()
            .unwrap(),
        "context_expired",
    )
    .unwrap();
    let failure = request(
        &world,
        WorldServicePayloadV1::Scheduler(
            sign_read_request("scheduler", failure, &delegate).unwrap(),
        ),
    );
    commit(&mut driver, expiry_height + 1, failure.clone());
    assert!(
        game_task_tests::recorded(&driver, &failure)
            .rejected
            .is_none()
    );
    let mut fresh_request = g2_prepare.request.clone();
    fresh_request.client_operation_id = "after-expiry-context".into();
    fresh_request.client_nonce = "91".repeat(32);
    let fresh_proof =
        sign_read_request(TASK_CONTEXT_PREPARE_PATH, fresh_request, &delegate).unwrap();
    let fresh = consumer(&dir, &runtime, &signer, &store, &world, |client| {
        client.prepare_task_context(&fresh_proof).unwrap()
    });
    assert_eq!(
        fresh.identity.fence.executor_binding,
        expiry.identity.fence.executor_binding
    );
    let fresh_prefix = request(
        &world,
        WorldServicePayloadV1::Scheduler(
            sign_read_request("scheduler", public_prefix(&fresh).unwrap(), &delegate).unwrap(),
        ),
    );
    commit(&mut driver, expiry_height + 2, fresh_prefix.clone());
    assert!(
        game_task_tests::recorded(&driver, &fresh_prefix)
            .rejected
            .is_none()
    );
    let fresh_get = sign_read_request(
        TASK_CONTEXT_READ_PATH,
        TaskContextGetV1 {
            schema_revision: 1,
            context: fresh.identity.clone(),
            prepare_operation_id: fresh_proof.request.client_operation_id.clone(),
        },
        &delegate,
    )
    .unwrap();
    let fresh_captured = consumer(&dir, &runtime, &signer, &store, &world, |client| {
        client.read_task_context(&fresh_get).unwrap()
    });
    let fresh_action = request(
        &world,
        WorldServicePayloadV1::Cognition(
            sign_read_request(
                "cognition",
                cognition_intent(
                    &fresh_captured,
                    Action::MoveAgent {
                        agent_id: "agent-a".into(),
                        to: oasis7::GeoPos::new(3, 0, 0),
                    },
                )
                .unwrap(),
                &delegate,
            )
            .unwrap(),
        ),
    );
    commit(&mut driver, expiry_height + 3, fresh_action.clone());
    assert!(
        game_task_tests::recorded(&driver, &fresh_action)
            .rejected
            .is_none()
    );
    assert_eq!(
        driver.execution_world.state().agents["agent-a"].state.pos,
        oasis7::GeoPos::new(3, 0, 0)
    );
    // Truly absent historical full evidence is a recovery error, not an unavailable retry.
    let activation = g2_context
        .snapshot
        .control
        .active_goal_reference
        .as_ref()
        .unwrap()
        .activation_committed_height
        .0;
    let historical_path = super::super::super::checkpoint::execution_bridge_record_path(
        &dir.join("records"),
        activation,
    );
    let historical_bytes = std::fs::read(&historical_path).unwrap();
    let mut compacted: serde_json::Value = serde_json::from_slice(&historical_bytes).unwrap();
    compacted["snapshot_ref"] = serde_json::Value::Null;
    compacted["journal_ref"] = serde_json::Value::Null;
    compacted["latest_state_ref"] = serde_json::Value::Null;
    std::fs::write(&historical_path, serde_json::to_vec(&compacted).unwrap()).unwrap();
    let snapshot_request = sign_read_request(
        TASK_SNAPSHOT_PATH,
        TaskGoalSnapshotRequestV1 {
            schema_revision: 1,
            fence: fence.clone(),
        },
        &delegate,
    )
    .unwrap();
    let (status, error) = http(
        &dir,
        &runtime,
        &signer,
        Some(&store),
        TASK_SNAPSHOT_PATH,
        &snapshot_request,
        "",
    );
    assert_eq!(status, 503);
    assert_eq!(error["error"], "private_goal_recovery_required");
    assert!(error.get("payload").is_none());
    let owner_request = sign_read_request(
        oasis7::world_service::private_goal_read::GOAL_CURRENT_PATH,
        oasis7::world_service::private_goal_read::CurrentGoalRequest {
            schema_revision: 1,
            world: world.clone(),
            agent_id: "agent-a".into(),
            task_id: fence.task_id.clone(),
        },
        &owner,
    )
    .unwrap();
    let (status, error) = http(
        &dir,
        &runtime,
        &signer,
        Some(&store),
        oasis7::world_service::private_goal_read::GOAL_CURRENT_PATH,
        &owner_request,
        "",
    );
    assert_eq!(status, 503);
    assert_eq!(error["error"], "private_goal_recovery_required");
    assert!(error.get("payload").is_none());
    std::fs::write(&historical_path, &historical_bytes).unwrap();
    // Unknown cache entries must prevent retention, never cause arbitrary path/height retention.
    let cache_dir = dir.join("store/pin_scopes/execution_bridge_live_tasks_v1");
    let unknown = cache_dir.join("unexpected.json");
    std::fs::write(&unknown, b"{}").unwrap();
    assert!(
        super::super::super::checkpoint::run_execution_bridge_retention_maintenance(
            &dir.join("records"),
            &cas,
            1
        )
        .is_err()
    );
    std::fs::remove_file(unknown).unwrap();
    // A Stop committed after preparation/decryption and before delivery fences the reply.
    let mut next = g2_prepare.request.clone();
    next.client_operation_id = "next-context".into();
    let next = sign_read_request(TASK_CONTEXT_PREPARE_PATH, next, &delegate).unwrap();
    let stop = request(
        &world,
        WorldServicePayloadV1::GameExecutorControl(
            sign_read_request(
                GAME_EXECUTOR_CONTROL_DOMAIN,
                GameExecutorControlV1 {
                    api_rev: 1,
                    client_operation_id: "stop-context".into(),
                    fence,
                    operation: GameExecutorOperationV1::Stop,
                },
                &owner,
            )
            .unwrap(),
        ),
    );
    let (status, body) = http_with_hook(
        &dir,
        &runtime,
        &signer,
        Some(&store),
        TASK_CONTEXT_PREPARE_PATH,
        &next,
        "",
        || {
            commit(&mut driver, expiry_height + 4, stop);
        },
    );
    assert!(status == 403 || status == 409);
    assert!(body.get("payload").is_none());
    assert!(!body.to_string().contains(&goal.request.content.goal_text));
    drop(store);
    let bytes = std::fs::read_dir(&private)
        .unwrap()
        .flat_map(|e| std::fs::read(e.unwrap().path()).unwrap())
        .collect::<Vec<_>>();
    assert!(
        !bytes
            .windows(goal.request.content.goal_text.len())
            .any(|b| b == goal.request.content.goal_text.as_bytes())
    );
    let records = dir.join("records");
    let stale_height =
        super::super::super::checkpoint::list_execution_bridge_record_heights(&records)
            .unwrap()
            .last()
            .copied()
            .unwrap();
    let stale_record = super::super::super::checkpoint::load_execution_bridge_record(
        super::super::super::checkpoint::execution_bridge_record_path(&records, stale_height)
            .as_path(),
    )
    .unwrap();
    let stale_validation = super::super::super::checkpoint::refresh_live_task_read_evidence(
        &records,
        &cas,
        &driver.execution_world,
        2,
    )
    .unwrap();
    commit(&mut driver, stale_height + 1, create.clone());
    assert!(
        super::super::super::checkpoint::run_execution_bridge_incremental_retention_maintenance_after_task_refresh(
            &records,
            &cas,
            &stale_record,
            stale_validation,
            2,
            32,
            3,
        )
        .is_err(),
        "task validation must not be reused after a higher durable record is appended"
    );
    std::fs::remove_dir_all(dir).unwrap();
}
