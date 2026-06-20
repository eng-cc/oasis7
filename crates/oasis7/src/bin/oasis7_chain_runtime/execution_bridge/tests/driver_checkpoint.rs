use super::super::checkpoint::{
    execution_bridge_record_path, execution_checkpoint_manifest_rel_path,
    load_execution_bridge_record,
};
use super::super::driver::{NodeRuntimeExecutionDriver, load_execution_bridge_state};
use super::*;
use oasis7_node::{
    NodeExecutionCheckpointInstallContext, NodeExecutionCommitContext, NodeExecutionHook,
    compute_consensus_action_root,
};
use oasis7_proto::storage_profile::{StorageProfile, StorageProfileConfig};

#[test]
fn node_runtime_execution_driver_exports_and_installs_checkpoint_bundle() {
    let dir = temp_dir("execution-driver-checkpoint-bundle");
    let source_root = dir.join("source");
    let target_root = dir.join("target");
    let storage_profile = StorageProfileConfig {
        execution_checkpoint_interval: 2,
        execution_checkpoint_keep: 2,
        ..StorageProfileConfig::for_profile(StorageProfile::DevLocal)
    };
    let mut source = NodeRuntimeExecutionDriver::new_with_storage_profile(
        source_root.join("bridge-state.json"),
        source_root.join("world"),
        source_root.join("records"),
        source_root.join("storage"),
        &storage_profile,
    )
    .expect("source driver");
    let action_root = compute_consensus_action_root(&[]).expect("empty action root");

    let first = source
        .on_commit(NodeExecutionCommitContext {
            world_id: "world-checkpoint-bundle".to_string(),
            node_id: "node-a".to_string(),
            height: 1,
            slot: 1,
            epoch: 0,
            node_block_hash: "block-1".to_string(),
            action_root: action_root.clone(),
            committed_actions: Vec::new(),
            committed_at_unix_ms: 10_001,
        })
        .expect("commit 1");
    let second = source
        .on_commit(NodeExecutionCommitContext {
            world_id: "world-checkpoint-bundle".to_string(),
            node_id: "node-a".to_string(),
            height: 2,
            slot: 2,
            epoch: 0,
            node_block_hash: "block-2".to_string(),
            action_root,
            committed_actions: Vec::new(),
            committed_at_unix_ms: 10_002,
        })
        .expect("commit 2");
    assert_ne!(first.execution_state_root, second.execution_state_root);
    let bundle = source
        .export_checkpoint_bundle(2)
        .expect("export checkpoint")
        .expect("checkpoint bundle");

    let mut target = NodeRuntimeExecutionDriver::new_with_storage_profile(
        target_root.join("bridge-state.json"),
        target_root.join("world"),
        target_root.join("records"),
        target_root.join("storage"),
        &storage_profile,
    )
    .expect("target driver");
    let installed = target
        .install_checkpoint_bundle(
            NodeExecutionCheckpointInstallContext {
                world_id: "world-checkpoint-bundle".to_string(),
                node_id: "node-b".to_string(),
                height: 2,
                node_block_hash: "block-2".to_string(),
                execution_block_hash: second.execution_block_hash.clone(),
                execution_state_root: second.execution_state_root.clone(),
                committed_at_unix_ms: 10_002,
            },
            bundle,
        )
        .expect("install checkpoint");

    assert_eq!(installed, second);
    let target_state = load_execution_bridge_state(target_root.join("bridge-state.json").as_path())
        .expect("target state");
    assert_eq!(target_state.last_applied_committed_height, 2);
    assert_eq!(
        target_state.last_execution_state_root.as_deref(),
        Some(second.execution_state_root.as_str())
    );
    let target_record = load_execution_bridge_record(
        execution_bridge_record_path(target_root.join("records").as_path(), 2).as_path(),
    )
    .expect("target record");
    assert_eq!(
        target_record.checkpoint_ref.as_deref(),
        Some(execution_checkpoint_manifest_rel_path(2).as_str())
    );
    assert_eq!(
        target_record.execution_state_root,
        second.execution_state_root
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn node_runtime_execution_driver_rejects_checkpoint_bundle_snapshot_root_mismatch() {
    let dir = temp_dir("execution-driver-checkpoint-root-mismatch");
    let source_root = dir.join("source");
    let target_root = dir.join("target");
    let storage_profile = StorageProfileConfig {
        execution_checkpoint_interval: 2,
        execution_checkpoint_keep: 2,
        ..StorageProfileConfig::for_profile(StorageProfile::DevLocal)
    };
    let mut source = NodeRuntimeExecutionDriver::new_with_storage_profile(
        source_root.join("bridge-state.json"),
        source_root.join("world"),
        source_root.join("records"),
        source_root.join("storage"),
        &storage_profile,
    )
    .expect("source driver");
    let action_root = compute_consensus_action_root(&[]).expect("empty action root");
    source
        .on_commit(NodeExecutionCommitContext {
            world_id: "world-checkpoint-root-mismatch".to_string(),
            node_id: "node-a".to_string(),
            height: 1,
            slot: 1,
            epoch: 0,
            node_block_hash: "block-1".to_string(),
            action_root: action_root.clone(),
            committed_actions: Vec::new(),
            committed_at_unix_ms: 11_001,
        })
        .expect("commit 1");
    let second = source
        .on_commit(NodeExecutionCommitContext {
            world_id: "world-checkpoint-root-mismatch".to_string(),
            node_id: "node-a".to_string(),
            height: 2,
            slot: 2,
            epoch: 0,
            node_block_hash: "block-2".to_string(),
            action_root,
            committed_actions: Vec::new(),
            committed_at_unix_ms: 11_002,
        })
        .expect("commit 2");
    let record_1 = load_execution_bridge_record(
        execution_bridge_record_path(source_root.join("records").as_path(), 1).as_path(),
    )
    .expect("record 1");
    let mut bundle = source
        .export_checkpoint_bundle(2)
        .expect("export checkpoint")
        .expect("checkpoint bundle");
    let malformed_manifest = super::super::ExecutionCheckpointManifest::new(
        "world-checkpoint-root-mismatch".to_string(),
        2,
        second.execution_block_hash.clone(),
        second.execution_state_root.clone(),
        record_1
            .latest_state_ref
            .clone()
            .expect("record 1 latest state ref"),
        record_1.snapshot_ref.clone(),
        record_1.journal_ref.clone(),
        11_002,
    )
    .expect("malformed manifest with valid hash");
    for content_hash in &malformed_manifest.pinned_refs {
        if bundle
            .blobs
            .iter()
            .any(|blob| blob.content_hash == *content_hash)
        {
            continue;
        }
        let bytes = source
            .execution_store
            .get_verified(content_hash.as_str())
            .expect("source blob");
        bundle.blobs.push(oasis7_node::NodeExecutionCheckpointBlob {
            content_hash: content_hash.clone(),
            bytes,
        });
    }
    bundle.manifest_json = serde_json::to_vec_pretty(&malformed_manifest).expect("manifest json");

    let mut target = NodeRuntimeExecutionDriver::new_with_storage_profile(
        target_root.join("bridge-state.json"),
        target_root.join("world"),
        target_root.join("records"),
        target_root.join("storage"),
        &storage_profile,
    )
    .expect("target driver");
    let err = target
        .install_checkpoint_bundle(
            NodeExecutionCheckpointInstallContext {
                world_id: "world-checkpoint-root-mismatch".to_string(),
                node_id: "node-b".to_string(),
                height: 2,
                node_block_hash: "block-2".to_string(),
                execution_block_hash: second.execution_block_hash,
                execution_state_root: second.execution_state_root,
                committed_at_unix_ms: 11_002,
            },
            bundle,
        )
        .expect_err("snapshot root mismatch should fail closed");

    assert!(err.contains("snapshot root mismatch"), "{err}");
    let target_state = load_execution_bridge_state(target_root.join("bridge-state.json").as_path())
        .expect("target state");
    assert_eq!(target_state.last_applied_committed_height, 0);
    assert!(
        !execution_bridge_record_path(target_root.join("records").as_path(), 2).exists(),
        "failed checkpoint install must not persist a height-2 execution record"
    );

    let _ = fs::remove_dir_all(dir);
}
