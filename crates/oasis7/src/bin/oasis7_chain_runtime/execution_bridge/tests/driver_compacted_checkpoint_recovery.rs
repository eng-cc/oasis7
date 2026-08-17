use super::super::checkpoint::{
    execution_bridge_record_path, execution_checkpoint_manifest_path, load_execution_bridge_record,
    maybe_persist_execution_checkpoint_for_record, persist_execution_bridge_record,
};
use super::super::driver::NodeRuntimeExecutionDriver;
use super::*;
use oasis7::runtime::LocalCasStore;
use oasis7_node::{NodeExecutionCommitContext, NodeExecutionHook, compute_consensus_action_root};

struct CompactedNormalV3Fixture {
    state_path: std::path::PathBuf,
    world_dir: std::path::PathBuf,
    records_dir: std::path::PathBuf,
    storage_root: std::path::PathBuf,
    record: ExecutionBridgeRecord,
}

fn seed_compacted_normal_v3_fixture(
    dir: &std::path::Path,
    checkpoint_height: u64,
    latest_height: u64,
) -> CompactedNormalV3Fixture {
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
    .expect("seed driver");
    let action_root = compute_consensus_action_root(&[]).expect("empty action root");
    for height in 1..=latest_height {
        driver
            .on_commit(NodeExecutionCommitContext {
                world_id: "w1".to_string(),
                node_id: "node-a".to_string(),
                proposer_id: "node-a".to_string(),
                height,
                slot: height.saturating_sub(1),
                epoch: 0,
                node_block_hash: format!("node-h{height}"),
                action_root: action_root.clone(),
                committed_actions: Vec::new(),
                committed_at_unix_ms: height as i64 * 1_000,
            })
            .expect("seed normal V3 commit");
    }
    drop(driver);

    let record_path = execution_bridge_record_path(records_dir.as_path(), checkpoint_height);
    let mut record = load_execution_bridge_record(record_path.as_path()).expect("load V3 record");
    let checkpoint_ref =
        maybe_persist_execution_checkpoint_for_record(records_dir.as_path(), &record, 1, 1)
            .expect("persist retained checkpoint manifest")
            .expect("checkpoint at requested height");
    record.checkpoint_ref = Some(checkpoint_ref);
    assert!(
        record.proposer_id.is_some(),
        "fixture must remain a normal V3 record"
    );
    assert!(
        record.action_root.is_some(),
        "fixture must retain action metadata"
    );
    record.latest_state_ref = None;
    record.snapshot_ref = None;
    record.journal_ref = None;
    persist_execution_bridge_record(records_dir.as_path(), &record)
        .expect("persist compacted normal V3 record");

    CompactedNormalV3Fixture {
        state_path,
        world_dir,
        records_dir,
        storage_root,
        record,
    }
}

#[test]
fn node_runtime_execution_driver_startup_restores_compacted_normal_v3_record_from_checkpoint() {
    let dir = temp_dir("execution-driver-startup-compacted-normal-v3-checkpoint");
    let fixture = seed_compacted_normal_v3_fixture(dir.as_path(), 1, 1);

    let restarted = NodeRuntimeExecutionDriver::new(
        fixture.state_path.clone(),
        fixture.world_dir,
        fixture.records_dir.clone(),
        fixture.storage_root,
    )
    .expect("startup should restore compacted normal V3 record from retained checkpoint");
    assert_eq!(restarted.state.last_applied_committed_height, 1);
    assert_eq!(
        restarted.state.last_execution_block_hash.as_deref(),
        Some(fixture.record.execution_block_hash.as_str())
    );
    assert_eq!(
        restarted.state.last_execution_state_root.as_deref(),
        Some(fixture.record.execution_state_root.as_str())
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn node_runtime_execution_driver_stale_restore_reuses_compacted_normal_v3_checkpoint() {
    let dir = temp_dir("execution-driver-stale-compacted-normal-v3-checkpoint");
    let fixture = seed_compacted_normal_v3_fixture(dir.as_path(), 1, 2);
    let mut restarted = NodeRuntimeExecutionDriver::new(
        fixture.state_path,
        fixture.world_dir,
        fixture.records_dir,
        fixture.storage_root,
    )
    .expect("latest hot record should start before stale restore");

    let result = restarted
        .on_commit(NodeExecutionCommitContext {
            world_id: "w1".to_string(),
            node_id: "node-a".to_string(),
            proposer_id: "node-a".to_string(),
            height: 1,
            slot: 0,
            epoch: 0,
            node_block_hash: "node-h1".to_string(),
            action_root: compute_consensus_action_root(&[]).expect("empty action root"),
            committed_actions: Vec::new(),
            committed_at_unix_ms: 1_000,
        })
        .expect("stale rollback should restore compacted checkpoint-backed record");
    assert_eq!(result.execution_height, 1);
    assert_eq!(
        result.execution_block_hash,
        fixture.record.execution_block_hash
    );
    assert_eq!(
        result.execution_state_root,
        fixture.record.execution_state_root
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn node_runtime_execution_driver_rejects_tampered_compacted_checkpoint_manifest() {
    let dir = temp_dir("execution-driver-tampered-compacted-checkpoint-manifest");
    let fixture = seed_compacted_normal_v3_fixture(dir.as_path(), 1, 1);
    let manifest_path = execution_checkpoint_manifest_path(fixture.records_dir.as_path(), 1);
    let mut manifest_json: serde_json::Value = serde_json::from_slice(
        fs::read(manifest_path.as_path())
            .expect("read manifest")
            .as_slice(),
    )
    .expect("parse manifest");
    manifest_json["execution_state_root"] =
        serde_json::Value::String("tampered-state-root".to_string());
    crate::write_bytes_atomic(
        manifest_path.as_path(),
        serde_json::to_vec_pretty(&manifest_json)
            .expect("serialize tampered manifest")
            .as_slice(),
    )
    .expect("persist tampered manifest");

    let err = match NodeRuntimeExecutionDriver::new(
        fixture.state_path,
        fixture.world_dir,
        fixture.records_dir,
        fixture.storage_root,
    ) {
        Ok(_) => panic!("tampered checkpoint manifest must fail closed"),
        Err(err) => err,
    };
    assert!(
        err.contains("hash mismatch") || err.contains("checkpoint"),
        "unexpected tampered manifest error: {err}"
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn node_runtime_execution_driver_rejects_missing_compacted_checkpoint_blob() {
    let dir = temp_dir("execution-driver-missing-compacted-checkpoint-blob");
    let fixture = seed_compacted_normal_v3_fixture(dir.as_path(), 1, 1);
    let manifest_path = execution_checkpoint_manifest_path(fixture.records_dir.as_path(), 1);
    let manifest: ExecutionCheckpointManifest = serde_json::from_slice(
        fs::read(manifest_path.as_path())
            .expect("read manifest")
            .as_slice(),
    )
    .expect("parse manifest");
    let store = LocalCasStore::new(fixture.storage_root.clone());
    fs::remove_file(
        store
            .blobs_dir()
            .join(format!("{}.blob", manifest.latest_state_ref)),
    )
    .expect("remove checkpoint snapshot blob");

    let err = match NodeRuntimeExecutionDriver::new(
        fixture.state_path,
        fixture.world_dir,
        fixture.records_dir,
        fixture.storage_root,
    ) {
        Ok(_) => panic!("missing checkpoint blob must fail closed"),
        Err(err) => err,
    };
    assert!(
        err.contains("CAS snapshot") || err.contains("checkpoint"),
        "unexpected missing checkpoint blob error: {err}"
    );

    let _ = fs::remove_dir_all(dir);
}
