//! Deterministic real-handler classification of read-only quote races.
//! These status codes do not grant admission or extend the captured lease.
use super::super::private_goal_tests::{http, http_with_hook};
use super::*;
use oasis7::world_service::{private_task_context::*, task_factory_quote::*, task_recipe_quote::*};
use std::path::PathBuf;
struct Fixture {
    dir: PathBuf,
    driver: NodeRuntimeExecutionDriver,
    world: WorldIdentity,
    owner: String,
    delegate: String,
    store: PrivateGoalStore,
    signer: crate::feedback_submit_api::FeedbackSubmitSigner,
    runtime: Arc<Mutex<oasis7_node::NodeRuntime>>,
    get: SignedReadRequest<TaskContextGetV1>,
    create: SubmitIntentRequest<WorldServicePayloadV1>,
    height: u64,
}
fn fixture() -> Fixture {
    let (dir, mut driver, world, owner, owner_public) = production_playtest::fixture();
    let create = game_task_tests::create(&world, &owner, "context-task");
    commit(&mut driver, 3, create.clone());
    let c = game_task::current_control(&driver.execution_world, "agent-a")
        .unwrap()
        .unwrap();
    let private = dir.canonicalize().unwrap().join("private-goals");
    let store = PrivateGoalStore::open(&private, [7; 32]).unwrap();
    let content = production_playtest::goal();
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

    let signer = crate::feedback_submit_api::FeedbackSubmitSigner {
        private_key_hex: owner.clone(),
        public_key_hex: owner_public,
    };
    let runtime = Arc::new(Mutex::new(oasis7_node::NodeRuntime::new(
        oasis7_node::NodeConfig::new("node-a", "w1", NodeRole::Sequencer).unwrap(),
    )));
    let control = game_task::current_control(&driver.execution_world, "agent-a")
        .unwrap()
        .unwrap();
    let prepare = sign_read_request(
        TASK_CONTEXT_PREPARE_PATH,
        TaskContextPrepareV1 {
            schema_revision: 1,
            fence: GameTaskFenceV1::from_control(&control),
            client_operation_id: "quote-classification".into(),
            client_nonce: "cd".repeat(32),
        },
        &delegate,
    )
    .unwrap();
    let (status, body) = http(
        &dir,
        &runtime,
        &signer,
        Some(&store),
        TASK_CONTEXT_PREPARE_PATH,
        &prepare,
        "",
    );
    assert_eq!(status, 200, "{body}");
    let prepared: PreparedTaskContextV1 = serde_json::from_value(body["payload"].clone()).unwrap();
    let prefix = request(
        &world,
        WorldServicePayloadV1::Scheduler(
            sign_read_request("scheduler", public_prefix(&prepared).unwrap(), &delegate).unwrap(),
        ),
    );
    commit(&mut driver, 7, prefix.clone());
    assert!(
        game_task_tests::recorded(&driver, &prefix)
            .rejected
            .is_none()
    );
    let get = sign_read_request(
        TASK_CONTEXT_READ_PATH,
        TaskContextGetV1 {
            schema_revision: 1,
            context: prepared.identity,
            prepare_operation_id: prepare.request.client_operation_id,
        },
        &delegate,
    )
    .unwrap();
    let (status, body) = http(
        &dir,
        &runtime,
        &signer,
        Some(&store),
        TASK_CONTEXT_READ_PATH,
        &get,
        "",
    );
    assert_eq!(status, 200, "{body}");
    Fixture {
        dir,
        driver,
        world,
        owner,
        delegate,
        store,
        signer,
        runtime,
        get,
        create,
        height: 8,
    }
}
impl Fixture {
    fn quote(&self, recipe: bool, stale: bool) -> (&'static str, serde_json::Value) {
        let mut base = self
            .driver
            .execution_world
            .current_cognition_base_binding()
            .unwrap();
        if stale {
            base.base_tick = base.base_tick.saturating_add(1);
        }
        if recipe {
            (
                TASK_RECIPE_QUOTE_PATH,
                serde_json::to_value(
                    sign_read_request(
                        TASK_RECIPE_QUOTE_PATH,
                        TaskRecipeQuoteRequestV1 {
                            schema_revision: 1,
                            context_proof: self.get.clone(),
                            expected_base_binding: base,
                            factory_id: "production-factory".into(),
                            recipe_id: oasis7::runtime::STARTER_SMELTER_RECIPE_ID.into(),
                            batches: 1,
                        },
                        &self.delegate,
                    )
                    .unwrap(),
                )
                .unwrap(),
            )
        } else {
            (
                TASK_FACTORY_QUOTE_PATH,
                serde_json::to_value(
                    sign_read_request(
                        TASK_FACTORY_QUOTE_PATH,
                        TaskFactoryQuoteRequestV1 {
                            schema_revision: 1,
                            context_proof: self.get.clone(),
                            expected_base_binding: base,
                            factory_id: "quote-only-new-factory".into(),
                            factory_kind: "factory.smelter.mk1".into(),
                            site_id: "production-site".into(),
                        },
                        &self.delegate,
                    )
                    .unwrap(),
                )
                .unwrap(),
            )
        }
    }
    fn advance(&mut self) {
        commit(&mut self.driver, self.height, self.create.clone());
        self.height += 1;
    }
}
#[test]
fn task_factory_and_recipe_quote_classify_only_base_and_delivery_drift() {
    for terminal in ["expiry", "fence"] {
        let mut f = fixture();
        for recipe in [false, true] {
            let (auth_path, mut invalid_auth) = f.quote(recipe, false);
            invalid_auth["signature_hex"] = serde_json::json!("00".repeat(64));
            let (status, body) = http(
                &f.dir,
                &f.runtime,
                &f.signer,
                Some(&f.store),
                auth_path,
                &invalid_auth,
                "",
            );
            assert!(
                matches!(status, 400 | 403 | 409),
                "invalid auth: {status} {body}"
            );
            assert_ne!(body["error"], TASK_QUOTE_BASE_DRIFT);
            assert_ne!(body["error"], TASK_QUOTE_DELIVERY_DRIFT);
            let (path, stale) = f.quote(recipe, true);
            let (status, body) = http(
                &f.dir,
                &f.runtime,
                &f.signer,
                Some(&f.store),
                path,
                &stale,
                "",
            );
            assert_eq!(status, 409, "{path}: {body}");
            assert_eq!(body["error"], TASK_QUOTE_BASE_DRIFT);
            let (path, current) = f.quote(recipe, false);
            let (status, body) = http_with_hook(
                &f.dir,
                &f.runtime,
                &f.signer,
                Some(&f.store),
                path,
                &current,
                "",
                || {
                    commit(&mut f.driver, f.height, f.create.clone());
                },
            );
            f.height += 1;
            assert_eq!(status, 409, "{path}: {body}");
            assert_eq!(body["error"], TASK_QUOTE_DELIVERY_DRIFT);
        }
        if terminal == "expiry" {
            while f.driver.execution_world.state().time <= f.get.request.context.valid_until_tick.0
            {
                f.advance();
            }
        } else {
            let control = game_task::current_control(&f.driver.execution_world, "agent-a")
                .unwrap()
                .unwrap();
            let stop = request(
                &f.world,
                WorldServicePayloadV1::GameExecutorControl(
                    sign_read_request(
                        GAME_EXECUTOR_CONTROL_DOMAIN,
                        GameExecutorControlV1 {
                            api_rev: 1,
                            client_operation_id: "quote-stop".into(),
                            fence: GameTaskFenceV1::from_control(&control),
                            operation: GameExecutorOperationV1::Stop,
                        },
                        &f.owner,
                    )
                    .unwrap(),
                ),
            );
            commit(&mut f.driver, f.height, stop);
        }
        for recipe in [false, true] {
            let (path, proof) = f.quote(recipe, false);
            let (status, body) = http(
                &f.dir,
                &f.runtime,
                &f.signer,
                Some(&f.store),
                path,
                &proof,
                "",
            );
            assert!(
                matches!(status, 403 | 409),
                "{terminal} {path}: {status} {body}"
            );
            assert_ne!(body["error"], TASK_QUOTE_BASE_DRIFT);
            assert_ne!(body["error"], TASK_QUOTE_DELIVERY_DRIFT);
        }
        drop(f.store);
        std::fs::remove_dir_all(f.dir).unwrap();
    }
}
