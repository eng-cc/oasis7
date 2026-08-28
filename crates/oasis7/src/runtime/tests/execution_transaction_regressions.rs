use super::super::*;
use super::pos;
use crate::simulator::ResourceKind;
use oasis7_wasm_abi::{
    ModuleCallErrorCode, ModuleCallFailure, ModuleCallRequest, ModuleOutput, ModuleSandbox,
};

#[derive(Default)]
struct NoopSandbox;

impl ModuleSandbox for NoopSandbox {
    fn call(&mut self, _request: &ModuleCallRequest) -> Result<ModuleOutput, ModuleCallFailure> {
        Ok(ModuleOutput {
            new_state: None,
            effects: Vec::new(),
            emits: Vec::new(),
            tick_lifecycle: None,
            output_bytes: 0,
        })
    }
}

#[derive(Default)]
struct StateWritingThenFailingSandbox {
    calls: usize,
}

impl ModuleSandbox for StateWritingThenFailingSandbox {
    fn call(&mut self, request: &ModuleCallRequest) -> Result<ModuleOutput, ModuleCallFailure> {
        self.calls = self.calls.saturating_add(1);
        if self.calls > 1 {
            return Err(ModuleCallFailure {
                module_id: request.module_id.clone(),
                trace_id: request.trace_id.clone(),
                code: ModuleCallErrorCode::Trap,
                detail: "failure injected after staged module state write".to_string(),
            });
        }
        Ok(ModuleOutput {
            new_state: Some(b"state-written-before-later-failure".to_vec()),
            effects: Vec::new(),
            emits: Vec::new(),
            tick_lifecycle: None,
            output_bytes: 34,
        })
    }
}

fn world_ready_for_overflowing_transfer() -> World {
    let mut world = World::new();
    world.submit_action(Action::RegisterAgent {
        agent_id: "from".to_string(),
        pos: pos(0, 0),
    });
    world.submit_action(Action::RegisterAgent {
        agent_id: "to".to_string(),
        pos: pos(0, 0),
    });
    world.step().expect("register agents");
    world
        .set_agent_resource_balance("from", ResourceKind::Data, 10)
        .expect("seed source balance");
    world
        .set_agent_resource_balance("to", ResourceKind::Data, i64::MAX)
        .expect("seed target overflow boundary");
    world.submit_action(Action::GrantDataAccess {
        owner_agent_id: "from".to_string(),
        grantee_agent_id: "to".to_string(),
    });
    world.step().expect("grant transfer access");
    world
}

fn submit_overflowing_transfer(world: &mut World) {
    world.submit_action(Action::EmitResourceTransfer {
        from_agent_id: "from".to_string(),
        to_agent_id: "to".to_string(),
        kind: ResourceKind::Data,
        amount: 1,
    });
}

fn world_with_overflowing_transfer_pending() -> World {
    let mut world = world_ready_for_overflowing_transfer();
    submit_overflowing_transfer(&mut world);
    world
}

fn assert_failed_transition_is_unpublished(
    world: &World,
    snapshot_before: &Snapshot,
    journal_before: &Journal,
) {
    assert_eq!(
        world.snapshot(),
        *snapshot_before,
        "failed transition changed snapshot state"
    );
    assert_eq!(
        world.journal(),
        journal_before,
        "failed transition changed journal"
    );
    assert_eq!(
        world.pending_actions_len(),
        1,
        "failed transition consumed its pending action"
    );
}

fn activate_post_move_state_writer(world: &mut World) {
    world.set_policy(PolicySet::allow_all());
    let wasm_bytes = b"transaction-regression-state-writer";
    let wasm_hash = util::sha256_hex(wasm_bytes);
    world
        .register_module_artifact(wasm_hash.clone(), wasm_bytes)
        .expect("register state-writer artifact");
    super::modules::activate_module_manifest(
        world,
        ModuleManifest {
            module_id: "m.transaction-regression.state-writer".to_string(),
            name: "TransactionRegressionStateWriter".to_string(),
            version: "0.1.0".to_string(),
            kind: ModuleKind::Reducer,
            role: ModuleRole::Domain,
            wasm_hash: wasm_hash.clone(),
            interface_version: "wasm-1".to_string(),
            abi_contract: ModuleAbiContract::default(),
            exports: vec!["reduce".to_string()],
            subscriptions: vec![ModuleSubscription {
                event_kinds: Vec::new(),
                action_kinds: vec!["action.move_agent".to_string()],
                stage: Some(ModuleSubscriptionStage::PostAction),
                filters: None,
            }],
            required_caps: Vec::new(),
            artifact_identity: Some(super::signed_test_artifact_identity(wasm_hash.as_str())),
            limits: ModuleLimits {
                max_mem_bytes: 1024,
                max_gas: 10_000,
                max_call_rate: 1,
                max_output_bytes: 1024,
                max_effects: 0,
                max_emits: 0,
            },
        },
    );
}

#[test]
fn failed_step_does_not_publish_partial_world_mutations() {
    let mut world = world_with_overflowing_transfer_pending();
    let snapshot_before = world.snapshot();
    let journal_before = world.journal().clone();

    let error = world.step().expect_err("overflowing transfer must fail");
    assert!(matches!(error, WorldError::ResourceBalanceInvalid { .. }));

    assert_failed_transition_is_unpublished(&world, &snapshot_before, &journal_before);
}

#[test]
fn failed_step_with_modules_does_not_publish_partial_world_mutations() {
    let mut world = world_with_overflowing_transfer_pending();
    let snapshot_before = world.snapshot();
    let journal_before = world.journal().clone();
    let mut sandbox = NoopSandbox;

    let error = world
        .step_with_modules(&mut sandbox)
        .expect_err("overflowing transfer must fail");
    assert!(matches!(error, WorldError::ResourceBalanceInvalid { .. }));

    assert_failed_transition_is_unpublished(&world, &snapshot_before, &journal_before);
}

#[test]
fn failed_step_with_modules_rolls_back_prior_module_state_write() {
    let mut world = world_ready_for_overflowing_transfer();
    activate_post_move_state_writer(&mut world);
    world.submit_action(Action::MoveAgent {
        agent_id: "from".to_string(),
        to: pos(1, 0),
    });
    world.submit_action(Action::MoveAgent {
        agent_id: "from".to_string(),
        to: pos(2, 0),
    });
    let snapshot_before = world.snapshot();
    let journal_before = world.journal().clone();
    let pending_before = world.pending_actions_len();
    let mut sandbox = StateWritingThenFailingSandbox::default();

    world
        .step_with_modules(&mut sandbox)
        .expect_err("second module call must fail after first state write");
    assert_eq!(
        sandbox.calls, 2,
        "one state-writing call must precede the injected failure"
    );
    assert_eq!(world.snapshot(), snapshot_before);
    assert_eq!(world.journal(), &journal_before);
    assert_eq!(world.pending_actions_len(), pending_before);
}

#[test]
fn failed_committed_context_step_does_not_publish_partial_world_mutations() {
    let mut world = world_with_overflowing_transfer_pending();
    let snapshot_before = world.snapshot();
    let journal_before = world.journal().clone();
    let mut sandbox = NoopSandbox;
    let committed_height = world.state().time.saturating_add(1);
    let context = RuntimeCommittedTickContext {
        height: committed_height,
        slot: committed_height.saturating_sub(1),
        epoch: 0,
        node_block_hash: "block.transaction-regression".to_string(),
        action_root: "action-root.transaction-regression".to_string(),
        authority_node_id: "builtin.module.release.signer".to_string(),
        committed_at_unix_ms: 0,
    };

    let error = world
        .step_with_modules_for_committed_context(&mut sandbox, &context)
        .expect_err("overflowing transfer must fail");
    assert!(matches!(error, WorldError::ResourceBalanceInvalid { .. }));

    assert_failed_transition_is_unpublished(&world, &snapshot_before, &journal_before);
}

#[test]
fn successful_staged_step_preserves_snapshot_journal_replay_equivalence() {
    let mut world = World::new();
    world.submit_action(Action::RegisterAgent {
        agent_id: "replay-agent".to_string(),
        pos: pos(0, 0),
    });
    world.step().expect("register replay agent");
    let stable_snapshot = world.snapshot();

    world.submit_action(Action::MoveAgent {
        agent_id: "replay-agent".to_string(),
        to: pos(4, 7),
    });
    world.step().expect("commit staged move");

    let restored = World::from_snapshot(stable_snapshot, world.journal().clone())
        .expect("replay staged transition");
    assert_eq!(restored.state(), world.state());
    assert_eq!(restored.journal(), world.journal());
    assert_eq!(
        restored.current_state_root_hash().expect("restored root"),
        world.current_state_root_hash().expect("live root")
    );
}
