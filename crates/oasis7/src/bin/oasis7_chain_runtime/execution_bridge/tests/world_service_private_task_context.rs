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
    let prepared = consumer(&dir, &runtime, &signer, &store, &world, |client| {
        client.prepare_task_context(&proof).unwrap()
    });
    assert_eq!(
        prepared.snapshot.active_goal.owner_proof.request.content,
        goal.request.content
    );
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
    commit(&mut driver, 7, submit.clone());
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
    commit(&mut driver, 8, competing_submit.clone());
    assert!(
        game_task_tests::recorded(&driver, &competing_submit)
            .rejected
            .is_some()
    );
    let captured = consumer(&dir, &runtime, &signer, &store, &world, |client| {
        client.read_task_context(&get).unwrap()
    });
    assert_eq!(captured.capture_commit.position, 7);
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
    // Authenticated capture cannot conceal a missing or corrupted encrypted record.
    let context_file = std::fs::read_dir(&private)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("context-")
        })
        .unwrap();
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
    commit(&mut driver, 9, submit.clone());
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
        10,
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
        11,
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
    commit(&mut driver, 12, g2_action.clone());
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
    commit(&mut driver, 13, expiry_prefix.clone());
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
    let mut expiry_height = 13;
    // Administrative receipts preserve simulation time; actual empty simulation
    // blocks advance ticks without fabricating a clock or player action.
    while driver.execution_world.state().time <= expiry.valid_until_tick.0 {
        expiry_height += 1;
        assert!(expiry_height <= 13 + TASK_CONTEXT_LEASE_TICKS + 1);
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
    std::fs::remove_dir_all(dir).unwrap();
}
