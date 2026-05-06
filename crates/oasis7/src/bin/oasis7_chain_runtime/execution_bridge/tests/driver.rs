use super::super::checkpoint::{
    execution_bridge_record_path, execution_checkpoint_manifest_rel_path,
    load_execution_bridge_record,
};
use super::super::driver::{
    load_execution_bridge_state, load_execution_world, load_execution_world_with_policy,
    persist_execution_bridge_state, persist_execution_world,
    simulator_world_dir_from_execution_world_dir, NodeRuntimeExecutionDriver,
};
use super::super::external_effect::load_execution_external_effect_materialization;
use super::*;
use oasis7::consensus_action_payload::encode_consensus_action_payload;
use oasis7::consensus_action_payload::ConsensusActionPayloadEnvelope;
use oasis7::runtime::{
    production_hardened_main_token_config, Action as RuntimeAction, DomainEvent, LocalCasStore,
    ModuleKind, ModuleLimits, ModuleManifest, ModuleRole, ModuleSubscription,
    ModuleSubscriptionStage, ReleaseSecurityPolicy, WorldEventBody,
    FROZEN_MAIN_TOKEN_INITIAL_SUPPLY,
};
use oasis7::simulator::{Action as SimulatorAction, ActionSubmitter};
use oasis7_node::{compute_consensus_action_root, NodeExecutionCommitContext, NodeExecutionHook};
use oasis7_proto::storage_profile::StorageProfile;
use oasis7_proto::storage_profile::StorageProfileConfig;
use oasis7_wasm_abi::ModuleCallFailure;
use oasis7_wasm_executor::FixedSandbox;

#[test]
fn execution_world_persistence_roundtrip() {
    let dir = temp_dir("execution-world");
    let world_dir = dir.join("world");
    let world = RuntimeWorld::new();

    persist_execution_world(world_dir.as_path(), &world).expect("persist world");
    let loaded = load_execution_world(world_dir.as_path()).expect("load world");
    assert_eq!(loaded.journal().len(), world.journal().len());

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn load_execution_world_defaults_to_hardened_release_policy() {
    let dir = temp_dir("execution-world-release-policy");
    let missing_world = load_execution_world(dir.as_path()).expect("load missing world");
    assert!(missing_world
        .release_security_policy()
        .is_production_hardened());
    assert_eq!(
        missing_world.main_token_config().initial_supply,
        FROZEN_MAIN_TOKEN_INITIAL_SUPPLY
    );

    let legacy_world_dir = dir.join("legacy");
    let legacy_world = RuntimeWorld::new();
    persist_execution_world(legacy_world_dir.as_path(), &legacy_world).expect("persist world");
    let loaded_world = load_execution_world(legacy_world_dir.as_path()).expect("load world");
    assert!(loaded_world
        .release_security_policy()
        .is_production_hardened());
    assert_eq!(
        loaded_world.main_token_config().initial_supply,
        FROZEN_MAIN_TOKEN_INITIAL_SUPPLY
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn load_execution_world_with_dev_local_policy_keeps_generic_supply_for_missing_world() {
    let dir = temp_dir("execution-world-dev-local-policy");
    let missing_world =
        load_execution_world_with_policy(dir.as_path(), ReleaseSecurityPolicy::default())
            .expect("load missing world with dev_local policy");

    assert_eq!(
        missing_world.release_security_policy(),
        &ReleaseSecurityPolicy::default()
    );
    assert_eq!(missing_world.main_token_config().initial_supply, 0);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn load_execution_world_with_dev_local_policy_clears_pristine_frozen_supply_from_existing_world() {
    let dir = temp_dir("execution-world-dev-local-clears-pristine-frozen");
    let world_dir = dir.join("world");
    let mut world = RuntimeWorld::new();
    world.set_main_token_config(production_hardened_main_token_config());
    persist_execution_world(world_dir.as_path(), &world).expect("persist release-like world");

    let loaded_world =
        load_execution_world_with_policy(world_dir.as_path(), ReleaseSecurityPolicy::default())
            .expect("load existing world with dev_local policy");

    assert_eq!(
        loaded_world.release_security_policy(),
        &ReleaseSecurityPolicy::default()
    );
    assert_eq!(loaded_world.main_token_config().initial_supply, 0);

    let _ = fs::remove_dir_all(dir);
}

fn tick_manifest(wasm_hash: &str) -> ModuleManifest {
    ModuleManifest {
        module_id: "m.test.tick".to_string(),
        name: "Tick Test".to_string(),
        version: "0.1.0".to_string(),
        kind: ModuleKind::Reducer,
        role: ModuleRole::Rule,
        wasm_hash: wasm_hash.to_string(),
        interface_version: "wasm-1".to_string(),
        abi_contract: oasis7_wasm_abi::ModuleAbiContract::default(),
        exports: vec!["reduce".to_string()],
        subscriptions: vec![ModuleSubscription {
            event_kinds: Vec::new(),
            action_kinds: Vec::new(),
            stage: Some(ModuleSubscriptionStage::Tick),
            filters: None,
        }],
        required_caps: Vec::new(),
        artifact_identity: Some(signed_test_artifact_identity(wasm_hash)),
        limits: ModuleLimits::default(),
    }
}

#[test]
fn node_runtime_execution_driver_commit_routes_modules_via_step_with_modules() {
    let dir = temp_dir("execution-driver-modules");
    let state_path = dir.join("state.json");
    let world_dir = dir.join("world");
    let records_dir = dir.join("records");
    let storage_root = dir.join("store");

    let wasm_bytes = b"bridge-modules-wasm".to_vec();
    let wasm_hash = {
        let mut hasher = Sha256::new();
        hasher.update(wasm_bytes.as_slice());
        hex::encode(hasher.finalize())
    };
    let manifest = tick_manifest(&wasm_hash);
    let mut world = RuntimeWorld::new();
    let signer_public_key_hex = hex::encode(
        test_module_artifact_signing_key()
            .verifying_key()
            .to_bytes(),
    );
    world
        .bind_node_identity(
            TEST_MODULE_ARTIFACT_SIGNER_NODE_ID,
            signer_public_key_hex.as_str(),
        )
        .expect("bind test module signer");
    world.submit_action(RuntimeAction::RegisterAgent {
        agent_id: "agent-0".to_string(),
        pos: oasis7::geometry::GeoPos::new(0, 0, 0),
    });
    world.step().expect("register");
    world
        .set_agent_resource_balance("agent-0", oasis7::simulator::ResourceKind::Electricity, 128)
        .expect("seed electricity");
    world
        .set_agent_resource_balance("agent-0", oasis7::simulator::ResourceKind::Data, 64)
        .expect("seed data");
    world.submit_action(RuntimeAction::DeployModuleArtifact {
        publisher_agent_id: "agent-0".to_string(),
        wasm_hash: wasm_hash.clone(),
        wasm_bytes: wasm_bytes.clone(),
    });
    world.step().expect("deploy");
    world.submit_action(RuntimeAction::InstallModuleFromArtifact {
        installer_agent_id: "agent-0".to_string(),
        manifest: manifest.clone(),
        activate: true,
    });
    world.step().expect("install");

    let instance_id = {
        let event = world.journal().events.last().expect("install event");
        let WorldEventBody::Domain(DomainEvent::ModuleInstalled { instance_id, .. }) = &event.body
        else {
            panic!("expected module installed event: {:?}", event.body);
        };
        instance_id.clone()
    };
    let instance = world
        .state()
        .module_instances
        .get(&instance_id)
        .expect("installed module instance");
    assert!(
        instance.active,
        "installed module instance should be active"
    );
    assert!(
        world
            .snapshot()
            .module_tick_schedule
            .contains_key(&instance_id),
        "installed tick module should be scheduled"
    );

    let expected_trace = format!(
        "tick-{}-{}",
        world.state().time.saturating_add(1),
        instance_id
    );
    let sandbox = FixedSandbox::fail(ModuleCallFailure {
        module_id: manifest.module_id.clone(),
        trace_id: expected_trace.clone(),
        code: oasis7_wasm_abi::ModuleCallErrorCode::PolicyDenied,
        detail: "forced failure for routing assertion".to_string(),
    });
    let mut driver = NodeRuntimeExecutionDriver::new_with_sandbox(
        state_path,
        world_dir,
        records_dir,
        storage_root,
        ExecutionBridgeState::default(),
        world,
        Box::new(sandbox.clone()),
        EXECUTION_BRIDGE_DEFAULT_HOT_WINDOW_HEIGHTS,
        EXECUTION_BRIDGE_DEFAULT_CHECKPOINT_INTERVAL_HEIGHTS,
        EXECUTION_BRIDGE_DEFAULT_CHECKPOINT_KEEP_LATEST,
    );

    let empty_action_root = compute_consensus_action_root(&[]).expect("empty action root");
    let err = driver
        .on_commit(NodeExecutionCommitContext {
            world_id: "w1".to_string(),
            node_id: "node-a".to_string(),
            height: 1,
            slot: 0,
            epoch: 0,
            node_block_hash: "node-h1".to_string(),
            action_root: empty_action_root,
            committed_actions: Vec::new(),
            committed_at_unix_ms: 1_000,
        })
        .expect_err("forced module failure should bubble");
    assert!(
        err.contains("world.step failed"),
        "unexpected error from commit path: {err}"
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn node_runtime_execution_driver_persists_chain_records() {
    let dir = temp_dir("execution-driver");
    let state_path = dir.join("state.json");
    let world_dir = dir.join("world");
    let records_dir = dir.join("records");
    let storage_root = dir.join("store");
    let mut driver = NodeRuntimeExecutionDriver::new(
        state_path.clone(),
        world_dir.clone(),
        records_dir.clone(),
        storage_root,
    )
    .expect("driver");
    let empty_action_root = compute_consensus_action_root(&[]).expect("empty action root");

    let first = driver
        .on_commit(NodeExecutionCommitContext {
            world_id: "w1".to_string(),
            node_id: "node-a".to_string(),
            height: 1,
            slot: 0,
            epoch: 0,
            node_block_hash: "node-h1".to_string(),
            action_root: empty_action_root.clone(),
            committed_actions: Vec::new(),
            committed_at_unix_ms: 1_000,
        })
        .expect("first commit");
    let second = driver
        .on_commit(NodeExecutionCommitContext {
            world_id: "w1".to_string(),
            node_id: "node-a".to_string(),
            height: 2,
            slot: 1,
            epoch: 0,
            node_block_hash: "node-h2".to_string(),
            action_root: empty_action_root,
            committed_actions: Vec::new(),
            committed_at_unix_ms: 2_000,
        })
        .expect("second commit");

    assert_eq!(first.execution_height, 1);
    assert_eq!(second.execution_height, 2);
    assert_ne!(first.execution_block_hash, second.execution_block_hash);
    assert!(records_dir.join("00000000000000000001.json").exists());
    assert!(records_dir.join("00000000000000000002.json").exists());

    let state = load_execution_bridge_state(state_path.as_path()).expect("load state");
    assert_eq!(state.last_applied_committed_height, 2);
    assert_eq!(state.last_node_block_hash.as_deref(), Some("node-h2"));

    let store = LocalCasStore::new(dir.join("store"));
    let record_bytes = fs::read(records_dir.join("00000000000000000002.json"))
        .expect("read second execution bridge record");
    let record: ExecutionBridgeRecord =
        serde_json::from_slice(record_bytes.as_slice()).expect("parse second record");
    let external_effect_ref = record
        .external_effect_ref
        .as_deref()
        .expect("external effect ref should exist");
    let external_effect =
        load_execution_external_effect_materialization(&store, external_effect_ref)
            .expect("load external effect materialization");
    assert_eq!(external_effect.height, 2);
    assert_eq!(external_effect.slot, 1);
    assert_eq!(external_effect.epoch, 0);
    assert_eq!(
        external_effect.action_root,
        compute_consensus_action_root(&[]).expect("empty root")
    );
    assert!(external_effect.committed_actions.is_empty());
    assert!(external_effect.unresolved_inputs.is_empty());

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn node_runtime_execution_driver_rejects_non_contiguous_commit_without_predecessor_record() {
    let dir = temp_dir("execution-driver-gap");
    let state_path = dir.join("state.json");
    let world_dir = dir.join("world");
    let records_dir = dir.join("records");
    let storage_root = dir.join("store");
    let mut driver = NodeRuntimeExecutionDriver::new(
        state_path.clone(),
        world_dir,
        records_dir.clone(),
        storage_root,
    )
    .expect("driver");
    let empty_action_root = compute_consensus_action_root(&[]).expect("empty action root");

    driver
        .on_commit(NodeExecutionCommitContext {
            world_id: "w1".to_string(),
            node_id: "node-a".to_string(),
            height: 1,
            slot: 0,
            epoch: 0,
            node_block_hash: "node-h1".to_string(),
            action_root: empty_action_root.clone(),
            committed_actions: Vec::new(),
            committed_at_unix_ms: 1_000,
        })
        .expect("first commit");
    let err = driver
        .on_commit(NodeExecutionCommitContext {
            world_id: "w1".to_string(),
            node_id: "node-a".to_string(),
            height: 3,
            slot: 2,
            epoch: 0,
            node_block_hash: "node-h3".to_string(),
            action_root: empty_action_root,
            committed_actions: Vec::new(),
            committed_at_unix_ms: 3_000,
        })
        .expect_err("gap commit without predecessor record should fail");
    assert!(
        err.contains("missing predecessor record"),
        "unexpected gap error: {err}"
    );
    assert!(records_dir.join("00000000000000000001.json").exists());
    assert!(!records_dir.join("00000000000000000003.json").exists());
    assert!(!records_dir.join("00000000000000000002.json").exists());

    let state = load_execution_bridge_state(state_path.as_path()).expect("load state");
    assert_eq!(state.last_applied_committed_height, 1);
    assert_eq!(state.last_node_block_hash.as_deref(), Some("node-h1"));

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn node_runtime_execution_driver_restores_predecessor_before_gap_commit() {
    let dir = temp_dir("execution-driver-gap-restore");
    let state_path = dir.join("state.json");
    let world_dir = dir.join("world");
    let records_dir = dir.join("records");
    let storage_root = dir.join("store");
    let mut driver =
        NodeRuntimeExecutionDriver::new(state_path.clone(), world_dir, records_dir, storage_root)
            .expect("driver");
    let empty_action_root = compute_consensus_action_root(&[]).expect("empty action root");

    let first = driver
        .on_commit(NodeExecutionCommitContext {
            world_id: "w1".to_string(),
            node_id: "node-a".to_string(),
            height: 1,
            slot: 0,
            epoch: 0,
            node_block_hash: "node-h1".to_string(),
            action_root: empty_action_root.clone(),
            committed_actions: Vec::new(),
            committed_at_unix_ms: 1_000,
        })
        .expect("first commit");
    driver
        .on_commit(NodeExecutionCommitContext {
            world_id: "w1".to_string(),
            node_id: "node-a".to_string(),
            height: 2,
            slot: 1,
            epoch: 0,
            node_block_hash: "node-h2".to_string(),
            action_root: empty_action_root.clone(),
            committed_actions: Vec::new(),
            committed_at_unix_ms: 2_000,
        })
        .expect("second commit");
    let third = driver
        .on_commit(NodeExecutionCommitContext {
            world_id: "w1".to_string(),
            node_id: "node-a".to_string(),
            height: 3,
            slot: 2,
            epoch: 0,
            node_block_hash: "node-h3".to_string(),
            action_root: empty_action_root.clone(),
            committed_actions: Vec::new(),
            committed_at_unix_ms: 3_000,
        })
        .expect("third commit");

    let replayed_first = driver
        .on_commit(NodeExecutionCommitContext {
            world_id: "w1".to_string(),
            node_id: "node-a".to_string(),
            height: 1,
            slot: 0,
            epoch: 0,
            node_block_hash: "node-h1".to_string(),
            action_root: empty_action_root.clone(),
            committed_actions: Vec::new(),
            committed_at_unix_ms: 1_000,
        })
        .expect("replay first commit");
    assert_eq!(replayed_first.execution_height, first.execution_height);
    assert_eq!(
        replayed_first.execution_block_hash,
        first.execution_block_hash
    );
    assert_eq!(
        replayed_first.execution_state_root,
        first.execution_state_root
    );

    let replayed_third = driver
        .on_commit(NodeExecutionCommitContext {
            world_id: "w1".to_string(),
            node_id: "node-a".to_string(),
            height: 3,
            slot: 2,
            epoch: 0,
            node_block_hash: "node-h3".to_string(),
            action_root: empty_action_root,
            committed_actions: Vec::new(),
            committed_at_unix_ms: 3_000,
        })
        .expect("replay third commit after restoring predecessor");
    assert_eq!(replayed_third.execution_height, third.execution_height);
    assert_eq!(
        replayed_third.execution_block_hash,
        third.execution_block_hash
    );
    assert_eq!(
        replayed_third.execution_state_root,
        third.execution_state_root
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn node_runtime_execution_driver_restart_recovers_latest_head_after_retention() {
    let dir = temp_dir("execution-driver-restart-recovery");
    let state_path = dir.join("state.json");
    let world_dir = dir.join("world");
    let records_dir = dir.join("records");
    let storage_root = dir.join("store");
    let mut driver = NodeRuntimeExecutionDriver::new(
        state_path.clone(),
        world_dir.clone(),
        records_dir.clone(),
        storage_root.clone(),
    )
    .expect("driver");
    let empty_action_root = compute_consensus_action_root(&[]).expect("empty action root");
    let mut latest_result = None;
    for height in 1..=33 {
        latest_result = Some(
            driver
                .on_commit(NodeExecutionCommitContext {
                    world_id: "w1".to_string(),
                    node_id: "node-a".to_string(),
                    height,
                    slot: height.saturating_sub(1),
                    epoch: 0,
                    node_block_hash: format!("node-h{height}"),
                    action_root: empty_action_root.clone(),
                    committed_actions: Vec::new(),
                    committed_at_unix_ms: height as i64 * 1_000,
                })
                .expect("commit before restart"),
        );
    }
    let latest_result = latest_result.expect("latest result before restart");
    drop(driver);

    let mut restarted = NodeRuntimeExecutionDriver::new(
        state_path.clone(),
        world_dir.clone(),
        records_dir.clone(),
        storage_root,
    )
    .expect("restarted driver");
    let replayed_latest = restarted
        .on_commit(NodeExecutionCommitContext {
            world_id: "w1".to_string(),
            node_id: "node-a".to_string(),
            height: 33,
            slot: 32,
            epoch: 0,
            node_block_hash: "node-h33".to_string(),
            action_root: empty_action_root.clone(),
            committed_actions: Vec::new(),
            committed_at_unix_ms: 33_000,
        })
        .expect("replay latest commit after restart");
    assert_eq!(
        replayed_latest.execution_height,
        latest_result.execution_height
    );
    assert_eq!(
        replayed_latest.execution_block_hash,
        latest_result.execution_block_hash
    );
    assert_eq!(
        replayed_latest.execution_state_root,
        latest_result.execution_state_root
    );

    let next = restarted
        .on_commit(NodeExecutionCommitContext {
            world_id: "w1".to_string(),
            node_id: "node-a".to_string(),
            height: 34,
            slot: 33,
            epoch: 0,
            node_block_hash: "node-h34".to_string(),
            action_root: empty_action_root,
            committed_actions: Vec::new(),
            committed_at_unix_ms: 34_000,
        })
        .expect("next commit after restart");
    assert_eq!(next.execution_height, 34);

    let state = load_execution_bridge_state(state_path.as_path()).expect("load state");
    assert_eq!(state.last_applied_committed_height, 34);
    assert_eq!(state.last_node_block_hash.as_deref(), Some("node-h34"));

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn node_runtime_execution_driver_reconciles_stale_state_from_exact_record() {
    let dir = temp_dir("execution-driver-stale-state-reconcile");
    let state_path = dir.join("state.json");
    let world_dir = dir.join("world");
    let records_dir = dir.join("records");
    let storage_root = dir.join("store");
    let mut driver = NodeRuntimeExecutionDriver::new(
        state_path.clone(),
        world_dir.clone(),
        records_dir.clone(),
        storage_root.clone(),
    )
    .expect("driver");
    let empty_action_root = compute_consensus_action_root(&[]).expect("empty action root");
    let mut last_result = None;
    for height in 1..=3 {
        last_result = Some(
            driver
                .on_commit(NodeExecutionCommitContext {
                    world_id: "w1".to_string(),
                    node_id: "node-a".to_string(),
                    height,
                    slot: height.saturating_sub(1),
                    epoch: 0,
                    node_block_hash: format!("node-h{height}"),
                    action_root: empty_action_root.clone(),
                    committed_actions: Vec::new(),
                    committed_at_unix_ms: height as i64 * 1_000,
                })
                .expect("seed commit"),
        );
    }
    let height_three = last_result.expect("height three result");
    drop(driver);

    let stale_state = ExecutionBridgeState {
        last_applied_committed_height: 4,
        last_execution_block_hash: Some("stale-execution-hash".to_string()),
        last_execution_state_root: Some("stale-state-root".to_string()),
        last_node_block_hash: Some("stale-node-hash".to_string()),
    };
    persist_execution_bridge_state(state_path.as_path(), &stale_state)
        .expect("persist stale state");

    let mut restarted = NodeRuntimeExecutionDriver::new(
        state_path.clone(),
        world_dir.clone(),
        records_dir.clone(),
        storage_root,
    )
    .expect("restarted driver");
    let reconciled = restarted
        .on_commit(NodeExecutionCommitContext {
            world_id: "w1".to_string(),
            node_id: "node-a".to_string(),
            height: 3,
            slot: 2,
            epoch: 0,
            node_block_hash: "node-h3".to_string(),
            action_root: empty_action_root,
            committed_actions: Vec::new(),
            committed_at_unix_ms: 3_000,
        })
        .expect("reconcile stale state from record");

    assert_eq!(reconciled.execution_height, 3);
    assert_eq!(
        reconciled.execution_block_hash,
        height_three.execution_block_hash
    );
    assert_eq!(
        reconciled.execution_state_root,
        height_three.execution_state_root
    );

    let state = load_execution_bridge_state(state_path.as_path()).expect("load reconciled state");
    assert_eq!(state.last_applied_committed_height, 3);
    assert_eq!(
        state.last_execution_block_hash.as_deref(),
        Some(height_three.execution_block_hash.as_str())
    );
    assert_eq!(
        state.last_execution_state_root.as_deref(),
        Some(height_three.execution_state_root.as_str())
    );
    assert_eq!(state.last_node_block_hash.as_deref(), Some("node-h3"));

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn node_runtime_execution_driver_recovers_malformed_v2_record_from_state_root_and_local_journal() {
    let dir = temp_dir("execution-driver-malformed-v2-recovery");
    let state_path = dir.join("state.json");
    let world_dir = dir.join("world");
    let records_dir = dir.join("records");
    let storage_root = dir.join("store");
    let mut driver = NodeRuntimeExecutionDriver::new(
        state_path.clone(),
        world_dir.clone(),
        records_dir.clone(),
        storage_root.clone(),
    )
    .expect("driver");
    let empty_action_root = compute_consensus_action_root(&[]).expect("empty action root");
    let mut commit_results = Vec::new();
    for height in 1..=3 {
        let result = driver
            .on_commit(NodeExecutionCommitContext {
                world_id: "w1".to_string(),
                node_id: "node-a".to_string(),
                height,
                slot: height.saturating_sub(1),
                epoch: 0,
                node_block_hash: format!("node-h{height}"),
                action_root: empty_action_root.clone(),
                committed_actions: Vec::new(),
                committed_at_unix_ms: height as i64 * 1_000,
            })
            .expect("seed commit");
        commit_results.push(result);
    }
    drop(driver);

    let record_path = execution_bridge_record_path(records_dir.as_path(), 1);
    let record_bytes = fs::read(record_path.as_path()).expect("read original record");
    let mut record_json: serde_json::Value =
        serde_json::from_slice(record_bytes.as_slice()).expect("parse original record");
    record_json
        .as_object_mut()
        .expect("record json object")
        .remove("latest_state_ref");
    record_json
        .as_object_mut()
        .expect("record json object")
        .remove("snapshot_ref");
    record_json
        .as_object_mut()
        .expect("record json object")
        .remove("journal_ref");
    let malformed_bytes =
        serde_json::to_vec_pretty(&record_json).expect("serialize malformed record");
    crate::write_bytes_atomic(record_path.as_path(), malformed_bytes.as_slice())
        .expect("persist malformed record");

    let stale_state = ExecutionBridgeState {
        last_applied_committed_height: 4,
        last_execution_block_hash: Some("stale-execution-hash".to_string()),
        last_execution_state_root: Some("stale-state-root".to_string()),
        last_node_block_hash: Some("stale-node-hash".to_string()),
    };
    persist_execution_bridge_state(state_path.as_path(), &stale_state)
        .expect("persist stale state");

    let mut restarted = NodeRuntimeExecutionDriver::new(
        state_path.clone(),
        world_dir,
        records_dir.clone(),
        storage_root,
    )
    .expect("restarted driver");
    let recovered_height_one = restarted
        .on_commit(NodeExecutionCommitContext {
            world_id: "w1".to_string(),
            node_id: "node-a".to_string(),
            height: 1,
            slot: 0,
            epoch: 0,
            node_block_hash: "node-h1".to_string(),
            action_root: empty_action_root.clone(),
            committed_actions: Vec::new(),
            committed_at_unix_ms: 1_000,
        })
        .expect("recover malformed height-1 record");
    assert_eq!(recovered_height_one.execution_height, 1);
    assert_eq!(
        recovered_height_one.execution_block_hash,
        commit_results[0].execution_block_hash
    );
    assert_eq!(
        recovered_height_one.execution_state_root,
        commit_results[0].execution_state_root
    );

    let continued_height_two = restarted
        .on_commit(NodeExecutionCommitContext {
            world_id: "w1".to_string(),
            node_id: "node-a".to_string(),
            height: 2,
            slot: 1,
            epoch: 0,
            node_block_hash: "node-h2".to_string(),
            action_root: empty_action_root,
            committed_actions: Vec::new(),
            committed_at_unix_ms: 2_000,
        })
        .expect("continue after malformed-record recovery");
    assert_eq!(continued_height_two.execution_height, 2);
    assert_eq!(
        continued_height_two.execution_block_hash,
        commit_results[1].execution_block_hash
    );
    assert_eq!(
        continued_height_two.execution_state_root,
        commit_results[1].execution_state_root
    );

    let repaired_record =
        load_execution_bridge_record(record_path.as_path()).expect("load repaired height-1 record");
    assert_eq!(
        repaired_record.latest_state_ref.as_deref(),
        Some(repaired_record.execution_state_root.as_str())
    );
    assert_eq!(
        repaired_record.snapshot_ref.as_deref(),
        Some(repaired_record.execution_state_root.as_str())
    );
    assert!(repaired_record
        .journal_ref
        .as_deref()
        .is_some_and(|journal_ref| !journal_ref.is_empty()));

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn node_runtime_execution_driver_rejects_stale_restore_from_other_world() {
    let dir = temp_dir("execution-driver-stale-state-world-mismatch");
    let state_path = dir.join("state.json");
    let world_dir = dir.join("world");
    let records_dir = dir.join("records");
    let storage_root = dir.join("store");
    let mut driver = NodeRuntimeExecutionDriver::new(
        state_path.clone(),
        world_dir.clone(),
        records_dir.clone(),
        storage_root.clone(),
    )
    .expect("driver");
    let empty_action_root = compute_consensus_action_root(&[]).expect("empty action root");

    driver
        .on_commit(NodeExecutionCommitContext {
            world_id: "w1".to_string(),
            node_id: "node-a".to_string(),
            height: 1,
            slot: 0,
            epoch: 0,
            node_block_hash: "node-h1".to_string(),
            action_root: empty_action_root.clone(),
            committed_actions: Vec::new(),
            committed_at_unix_ms: 1_000,
        })
        .expect("seed commit");
    drop(driver);

    let stale_state = ExecutionBridgeState {
        last_applied_committed_height: 2,
        last_execution_block_hash: Some("stale-execution-hash".to_string()),
        last_execution_state_root: Some("stale-state-root".to_string()),
        last_node_block_hash: Some("stale-node-hash".to_string()),
    };
    persist_execution_bridge_state(state_path.as_path(), &stale_state)
        .expect("persist stale state");

    let mut restarted =
        NodeRuntimeExecutionDriver::new(state_path, world_dir, records_dir, storage_root)
            .expect("restarted driver");
    let err = restarted
        .on_commit(NodeExecutionCommitContext {
            world_id: "w2".to_string(),
            node_id: "node-a".to_string(),
            height: 1,
            slot: 0,
            epoch: 0,
            node_block_hash: "node-h1".to_string(),
            action_root: empty_action_root,
            committed_actions: Vec::new(),
            committed_at_unix_ms: 1_000,
        })
        .expect_err("world mismatch should fail closed");
    assert!(
        err.contains("stale-height restore world_id mismatch"),
        "unexpected mismatch error: {err}"
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn node_runtime_execution_driver_uses_storage_profile_checkpoint_interval() {
    let dir = temp_dir("execution-driver-storage-profile-checkpoint");
    let state_path = dir.join("state.json");
    let world_dir = dir.join("world");
    let records_dir = dir.join("records");
    let storage_root = dir.join("store");
    let storage_profile = StorageProfileConfig::for_profile(StorageProfile::ReleaseDefault);
    let mut driver = NodeRuntimeExecutionDriver::new_with_storage_profile(
        state_path.clone(),
        world_dir.clone(),
        records_dir.clone(),
        storage_root.clone(),
        &storage_profile,
    )
    .expect("driver");
    let empty_action_root = compute_consensus_action_root(&[]).expect("empty action root");

    for height in 1..=64 {
        driver
            .on_commit(NodeExecutionCommitContext {
                world_id: "w1".to_string(),
                node_id: "node-a".to_string(),
                height,
                slot: height.saturating_sub(1),
                epoch: 0,
                node_block_hash: format!("node-h{height}"),
                action_root: empty_action_root.clone(),
                committed_actions: Vec::new(),
                committed_at_unix_ms: height as i64 * 1_000,
            })
            .expect("commit with release_default profile");
    }

    let record_32 = load_execution_bridge_record(
        execution_bridge_record_path(records_dir.as_path(), 32).as_path(),
    )
    .expect("load record 32");
    let record_64 = load_execution_bridge_record(
        execution_bridge_record_path(records_dir.as_path(), 64).as_path(),
    )
    .expect("load record 64");
    assert!(record_32.checkpoint_ref.is_none());
    assert_eq!(
        record_64.checkpoint_ref.as_deref(),
        Some(execution_checkpoint_manifest_rel_path(64).as_str())
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn production_release_policy_release_default_applies_hardened_policy() {
    let dir = temp_dir("execution-driver-release-policy-release-default");
    let state_path = dir.join("state.json");
    let world_dir = dir.join("world");
    let records_dir = dir.join("records");
    let storage_root = dir.join("store");
    let storage_profile = StorageProfileConfig::for_profile(StorageProfile::ReleaseDefault);
    let driver = NodeRuntimeExecutionDriver::new_with_storage_profile(
        state_path,
        world_dir,
        records_dir,
        storage_root,
        &storage_profile,
    )
    .expect("driver");

    assert_eq!(
        driver.execution_world.release_security_policy(),
        &ReleaseSecurityPolicy::production_hardened()
    );
    assert!(driver
        .execution_world
        .release_security_policy()
        .is_production_hardened());

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn dev_local_storage_profile_keeps_generic_supply_for_missing_execution_world() {
    let dir = temp_dir("execution-driver-release-policy-dev-local");
    let state_path = dir.join("state.json");
    let world_dir = dir.join("world");
    let records_dir = dir.join("records");
    let storage_root = dir.join("store");
    let storage_profile = StorageProfileConfig::for_profile(StorageProfile::DevLocal);
    let driver = NodeRuntimeExecutionDriver::new_with_storage_profile(
        state_path,
        world_dir,
        records_dir,
        storage_root,
        &storage_profile,
    )
    .expect("driver");

    assert_eq!(
        driver.execution_world.release_security_policy(),
        &ReleaseSecurityPolicy::default()
    );
    assert_eq!(driver.execution_world.main_token_config().initial_supply, 0);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn dev_local_storage_profile_clears_pristine_frozen_supply_from_existing_execution_world() {
    let dir = temp_dir("execution-driver-release-policy-dev-local-existing-world");
    let state_path = dir.join("state.json");
    let world_dir = dir.join("world");
    let records_dir = dir.join("records");
    let storage_root = dir.join("store");
    let mut world = RuntimeWorld::new();
    world.set_main_token_config(production_hardened_main_token_config());
    persist_execution_world(world_dir.as_path(), &world).expect("persist release-like world");

    let storage_profile = StorageProfileConfig::for_profile(StorageProfile::DevLocal);
    let driver = NodeRuntimeExecutionDriver::new_with_storage_profile(
        state_path,
        world_dir,
        records_dir,
        storage_root,
        &storage_profile,
    )
    .expect("driver");

    assert_eq!(
        driver.execution_world.release_security_policy(),
        &ReleaseSecurityPolicy::default()
    );
    assert_eq!(driver.execution_world.main_token_config().initial_supply, 0);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn node_runtime_execution_driver_processes_simulator_payload_envelope() {
    let dir = temp_dir("execution-driver-simulator-payload");
    let state_path = dir.join("state.json");
    let world_dir = dir.join("world");
    let simulator_world_dir = simulator_world_dir_from_execution_world_dir(world_dir.as_path());
    let records_dir = dir.join("records");
    let storage_root = dir.join("store");
    let mut driver = NodeRuntimeExecutionDriver::new(
        state_path.clone(),
        world_dir,
        records_dir.clone(),
        storage_root,
    )
    .expect("driver");

    let payload =
        encode_consensus_action_payload(&ConsensusActionPayloadEnvelope::from_simulator_action(
            SimulatorAction::HarvestRadiation {
                agent_id: "agent-0".to_string(),
                max_amount: 1,
            },
            ActionSubmitter::System,
        ))
        .expect("encode simulator payload");
    let committed_action = oasis7_node::NodeConsensusAction::from_payload(1, "node-a", payload)
        .expect("consensus action");
    let action_root =
        compute_consensus_action_root(std::slice::from_ref(&committed_action)).expect("root");
    let expected_action_root = action_root.clone();
    let expected_payload_hash = committed_action.payload_hash.clone();

    let result = driver
        .on_commit(NodeExecutionCommitContext {
            world_id: "w1".to_string(),
            node_id: "node-a".to_string(),
            height: 1,
            slot: 0,
            epoch: 0,
            node_block_hash: "node-h1".to_string(),
            action_root,
            committed_actions: vec![committed_action],
            committed_at_unix_ms: 1_000,
        })
        .expect("commit");

    assert_eq!(result.execution_height, 1);
    assert!(records_dir.join("00000000000000000001.json").exists());
    let record_bytes = fs::read(records_dir.join("00000000000000000001.json"))
        .expect("read execution bridge record");
    let record: ExecutionBridgeRecord =
        serde_json::from_slice(record_bytes.as_slice()).expect("parse execution bridge record");
    assert_eq!(record.schema_version, EXECUTION_BRIDGE_RECORD_SCHEMA_V2);
    assert_eq!(
        record.latest_state_ref.as_deref(),
        record.snapshot_ref.as_deref()
    );
    assert!(record
        .snapshot_ref
        .as_deref()
        .is_some_and(|snapshot_ref| !snapshot_ref.is_empty()));
    assert!(record
        .journal_ref
        .as_deref()
        .is_some_and(|journal_ref| !journal_ref.is_empty()));
    let external_effect_ref = record
        .external_effect_ref
        .as_deref()
        .expect("external effect ref should exist");
    let store = LocalCasStore::new(dir.join("store"));
    let external_effect =
        load_execution_external_effect_materialization(&store, external_effect_ref)
            .expect("load external effect materialization");
    assert_eq!(external_effect.height, 1);
    assert_eq!(external_effect.slot, 0);
    assert_eq!(external_effect.epoch, 0);
    assert_eq!(external_effect.action_root, expected_action_root);
    assert_eq!(external_effect.committed_actions.len(), 1);
    assert_eq!(external_effect.committed_actions[0].action_id, 1);
    assert_eq!(
        external_effect.committed_actions[0].payload_hash,
        expected_payload_hash
    );
    assert!(external_effect.unresolved_inputs.is_empty());
    let simulator = record
        .simulator_mirror
        .expect("simulator mirror record should exist");
    assert_eq!(simulator.action_count, 1);
    assert_eq!(simulator.rejected_action_count, 1);
    assert!(!simulator.snapshot_ref.is_empty());
    assert!(!simulator.journal_ref.is_empty());
    assert!(!simulator.state_root.is_empty());
    assert!(simulator_world_dir.join("snapshot.json").exists());
    assert!(simulator_world_dir.join("journal.json").exists());
    let _ = fs::remove_dir_all(dir);
}
