use super::super::checkpoint::{execution_bridge_record_path, load_execution_bridge_record};
use super::super::driver::{
    NodeRuntimeExecutionDriver, load_execution_bridge_state, persist_execution_bridge_state,
};
use super::*;
use oasis7_node::{NodeExecutionCommitContext, NodeExecutionHook, compute_consensus_action_root};

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
                    proposer_id: "node-a".to_string(),
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
            proposer_id: "node-a".to_string(),
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
            proposer_id: "node-a".to_string(),
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
fn node_runtime_execution_driver_restart_recovers_authoritative_cas_when_world_cache_is_removed() {
    let dir = temp_dir("execution-driver-restart-cas-recovery-without-cache");
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
    let action_root = compute_consensus_action_root(&[]).expect("empty action root");
    driver
        .on_commit(NodeExecutionCommitContext {
            world_id: "w1".to_string(),
            node_id: "node-a".to_string(),
            proposer_id: "node-a".to_string(),
            height: 1,
            slot: 0,
            epoch: 0,
            node_block_hash: "node-h1".to_string(),
            action_root,
            committed_actions: Vec::new(),
            committed_at_unix_ms: 1_000,
        })
        .expect("commit");
    let committed_time = driver.execution_world.state().time;
    let committed_journal_len = driver.execution_world.journal().len();
    drop(driver);
    fs::remove_dir_all(world_dir.as_path()).expect("remove non-authoritative world cache");

    let restarted =
        NodeRuntimeExecutionDriver::new(state_path, world_dir, records_dir, storage_root)
            .expect("restart recovers authoritative CAS state");
    assert_eq!(restarted.execution_world.state().time, committed_time);
    assert_eq!(
        restarted.execution_world.journal().len(),
        committed_journal_len
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn node_runtime_execution_driver_restart_fails_closed_when_authoritative_cas_is_missing() {
    let dir = temp_dir("execution-driver-restart-missing-authoritative-cas");
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
    let action_root = compute_consensus_action_root(&[]).expect("empty action root");
    driver
        .on_commit(NodeExecutionCommitContext {
            world_id: "w1".to_string(),
            node_id: "node-a".to_string(),
            proposer_id: "node-a".to_string(),
            height: 1,
            slot: 0,
            epoch: 0,
            node_block_hash: "node-h1".to_string(),
            action_root,
            committed_actions: Vec::new(),
            committed_at_unix_ms: 1_000,
        })
        .expect("commit");
    drop(driver);
    fs::remove_dir_all(world_dir.as_path()).expect("remove non-authoritative world cache");
    fs::remove_dir_all(storage_root.as_path()).expect("remove authoritative CAS blobs");

    let err =
        match NodeRuntimeExecutionDriver::new(state_path, world_dir, records_dir, storage_root) {
            Ok(_) => panic!("restart must fail closed without authoritative CAS data"),
            Err(err) => err,
        };
    assert!(
        err.contains("authoritative") || err.contains("CAS") || err.contains("record"),
        "restart failure must identify authoritative recovery data: {err}"
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn node_runtime_execution_driver_startup_fails_closed_when_state_head_lacks_exact_record() {
    let dir = temp_dir("execution-driver-startup-stale-state-head");
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
    let action_root = compute_consensus_action_root(&[]).expect("empty action root");
    driver
        .on_commit(NodeExecutionCommitContext {
            world_id: "w1".to_string(),
            node_id: "node-a".to_string(),
            proposer_id: "node-a".to_string(),
            height: 1,
            slot: 0,
            epoch: 0,
            node_block_hash: "node-h1".to_string(),
            action_root,
            committed_actions: Vec::new(),
            committed_at_unix_ms: 1_000,
        })
        .expect("seed authoritative record");
    drop(driver);

    persist_execution_bridge_state(
        state_path.as_path(),
        &ExecutionBridgeState {
            last_applied_committed_height: 2,
            last_execution_block_hash: Some("stale-execution-hash".to_string()),
            last_execution_state_root: Some("stale-state-root".to_string()),
            last_node_block_hash: Some("stale-node-hash".to_string()),
        },
    )
    .expect("persist stale state head");

    let err =
        match NodeRuntimeExecutionDriver::new(state_path, world_dir, records_dir, storage_root) {
            Ok(_) => panic!("startup must reject an unverifiable state head"),
            Err(err) => err,
        };
    assert!(
        err.contains("authoritative startup record missing at height 2"),
        "unexpected startup recovery error: {err}"
    );

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
                    proposer_id: "node-a".to_string(),
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
            proposer_id: "node-a".to_string(),
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
                proposer_id: "node-a".to_string(),
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
    fs::write(
        records_dir.join("retention-degraded"),
        b"interrupted incremental retention\n",
    )
    .expect("persist interrupted retention marker");

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
            proposer_id: "node-a".to_string(),
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
    assert!(
        records_dir.join("retention-degraded").exists(),
        "stale-height recovery must precede destructive full reconciliation"
    );

    let continued_height_two = restarted
        .on_commit(NodeExecutionCommitContext {
            world_id: "w1".to_string(),
            node_id: "node-a".to_string(),
            proposer_id: "node-a".to_string(),
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
    assert!(
        repaired_record
            .journal_ref
            .as_deref()
            .is_some_and(|journal_ref| !journal_ref.is_empty())
    );

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
            proposer_id: "node-a".to_string(),
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
            proposer_id: "node-a".to_string(),
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
