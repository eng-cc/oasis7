use super::super::checkpoint::{
    execution_bridge_record_path, execution_checkpoint_latest_path,
    execution_checkpoint_manifest_path, load_execution_bridge_record,
};
use super::super::driver::{NodeRuntimeExecutionDriver, load_execution_bridge_state};
use super::super::driver_checkpoint_install::{
    CheckpointInstallFault, set_checkpoint_install_fault_for_test,
};
use super::*;
use oasis7_node::{
    NodeExecutionCheckpointInstallContext, NodeExecutionCommitContext, NodeExecutionHook,
    compute_consensus_action_root,
};
use oasis7_proto::storage_profile::{StorageProfile, StorageProfileConfig};

#[test]
fn node_runtime_execution_driver_checkpoint_install_startup_rolls_back_after_final_state_persist_fault()
 {
    let dir = temp_dir("execution-driver-checkpoint-final-state-persist-rollback");
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
            world_id: "world-checkpoint-final-state-persist-rollback".to_string(),
            node_id: "node-a".to_string(),
            proposer_id: "node-a".to_string(),
            height: 1,
            slot: 1,
            epoch: 0,
            node_block_hash: "block-1".to_string(),
            action_root: action_root.clone(),
            committed_actions: Vec::new(),
            committed_at_unix_ms: 10_001,
        })
        .expect("source commit 1");
    let second = source
        .on_commit(NodeExecutionCommitContext {
            world_id: "world-checkpoint-final-state-persist-rollback".to_string(),
            node_id: "node-a".to_string(),
            proposer_id: "node-a".to_string(),
            height: 2,
            slot: 2,
            epoch: 0,
            node_block_hash: "block-2".to_string(),
            action_root: action_root.clone(),
            committed_actions: Vec::new(),
            committed_at_unix_ms: 10_002,
        })
        .expect("source commit 2");
    let bundle = source
        .export_checkpoint_bundle(2)
        .expect("export checkpoint")
        .expect("checkpoint bundle");

    let state_path = target_root.join("bridge-state.json");
    let world_dir = target_root.join("world");
    let records_dir = target_root.join("records");
    let mut target = NodeRuntimeExecutionDriver::new_with_storage_profile(
        state_path.clone(),
        world_dir.clone(),
        records_dir.clone(),
        target_root.join("storage"),
        &storage_profile,
    )
    .expect("target driver");
    target
        .on_commit(NodeExecutionCommitContext {
            world_id: "world-checkpoint-final-state-persist-rollback".to_string(),
            node_id: "node-b".to_string(),
            proposer_id: "node-a".to_string(),
            height: 1,
            slot: 1,
            epoch: 0,
            node_block_hash: "block-1".to_string(),
            action_root: action_root.clone(),
            committed_actions: Vec::new(),
            committed_at_unix_ms: 10_001,
        })
        .expect("target commit 1");
    let original_state = target.state.clone();
    let original_world_state = target.execution_world.state().clone();

    set_checkpoint_install_fault_for_test(Some(CheckpointInstallFault::AfterFinalStatePersist));
    let err = target
        .install_checkpoint_bundle(
            NodeExecutionCheckpointInstallContext {
                world_id: "world-checkpoint-final-state-persist-rollback".to_string(),
                node_id: "node-b".to_string(),
                height: 2,
                node_block_hash: "block-2".to_string(),
                execution_block_hash: second.execution_block_hash.clone(),
                execution_state_root: second.execution_state_root.clone(),
                committed_at_unix_ms: 10_002,
            },
            bundle.clone(),
        )
        .expect_err("final execution bridge state persistence fault must simulate a crash");
    assert!(
        err.contains("after final state persistence"),
        "unexpected injected fault error: {err}"
    );
    assert_eq!(target.state.last_applied_committed_height, 2);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(
            fs::read(records_dir.join("checkpoint-install-transaction.json"))
                .expect("prepared marker bytes")
                .as_slice(),
        )
        .expect("prepared marker JSON")["phase"],
        "Prepared",
        "the marker remains Prepared until the final durable state publication"
    );

    drop(target);
    let restart_state_path = state_path.clone();
    let restart_world_dir = world_dir.clone();
    let restart_records_dir = records_dir.clone();
    let mut restarted = NodeRuntimeExecutionDriver::new_with_storage_profile(
        state_path,
        world_dir,
        records_dir.clone(),
        target_root.join("storage"),
        &storage_profile,
    )
    .expect("restart must roll back final-state-persist checkpoint publication");
    assert_eq!(restarted.state.last_applied_committed_height, 1);
    assert_eq!(restarted.state, original_state);
    assert_eq!(restarted.execution_world.state(), &original_world_state);
    assert!(
        !execution_bridge_record_path(records_dir.as_path(), 2).exists(),
        "restart rollback must remove the height-2 record"
    );
    assert!(
        !execution_checkpoint_manifest_path(records_dir.as_path(), 2).exists(),
        "restart rollback must remove the height-2 checkpoint manifest"
    );
    assert!(
        !records_dir
            .join("checkpoint-install-transaction.json")
            .exists(),
        "restart rollback must durably clean up the Prepared marker"
    );

    set_checkpoint_install_fault_for_test(Some(
        CheckpointInstallFault::AfterCommittedMarkerPersist,
    ));
    let committed_err = restarted
        .install_checkpoint_bundle(
            NodeExecutionCheckpointInstallContext {
                world_id: "world-checkpoint-final-state-persist-rollback".to_string(),
                node_id: "node-b".to_string(),
                height: 2,
                node_block_hash: "block-2".to_string(),
                execution_block_hash: second.execution_block_hash.clone(),
                execution_state_root: second.execution_state_root.clone(),
                committed_at_unix_ms: 10_002,
            },
            bundle,
        )
        .expect_err("Committed marker fault must simulate a crash after authoritative publication");
    assert!(committed_err.contains("after committed marker persistence"));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(
            fs::read(records_dir.join("checkpoint-install-transaction.json"))
                .expect("Committed marker bytes")
                .as_slice(),
        )
        .expect("Committed marker JSON")["phase"],
        "Committed"
    );
    drop(restarted);
    let mut restarted = NodeRuntimeExecutionDriver::new_with_storage_profile(
        restart_state_path,
        restart_world_dir,
        restart_records_dir,
        target_root.join("storage"),
        &storage_profile,
    )
    .expect("restart must finalize a durable Committed marker");
    assert_eq!(restarted.state.last_applied_committed_height, 2);
    assert!(
        !records_dir
            .join("checkpoint-install-transaction.json")
            .exists(),
        "Committed marker restart finalization must clean up the marker"
    );
    let replay_err = restarted
        .on_commit(NodeExecutionCommitContext {
            world_id: "world-checkpoint-final-state-persist-rollback".to_string(),
            node_id: "node-b".to_string(),
            proposer_id: "node-a".to_string(),
            height: 2,
            slot: 2,
            epoch: 0,
            node_block_hash: "block-2".to_string(),
            action_root,
            committed_actions: Vec::new(),
            committed_at_unix_ms: 10_002,
        })
        .expect_err("retried checkpoint-installed head rejects equal-height replay");
    assert!(replay_err.contains("checkpoint-install"), "{replay_err}");

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn node_runtime_execution_driver_rolls_back_final_state_persist_io_failure_in_process() {
    let dir = temp_dir("execution-driver-checkpoint-final-state-io-rollback");
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
            world_id: "world-checkpoint-final-state-io-rollback".to_string(),
            node_id: "node-a".to_string(),
            proposer_id: "node-a".to_string(),
            height: 1,
            slot: 1,
            epoch: 0,
            node_block_hash: "block-1".to_string(),
            action_root: action_root.clone(),
            committed_actions: Vec::new(),
            committed_at_unix_ms: 12_001,
        })
        .expect("source commit 1");
    let second = source
        .on_commit(NodeExecutionCommitContext {
            world_id: "world-checkpoint-final-state-io-rollback".to_string(),
            node_id: "node-a".to_string(),
            proposer_id: "node-a".to_string(),
            height: 2,
            slot: 2,
            epoch: 0,
            node_block_hash: "block-2".to_string(),
            action_root,
            committed_actions: Vec::new(),
            committed_at_unix_ms: 12_002,
        })
        .expect("source commit 2");
    let bundle = source
        .export_checkpoint_bundle(2)
        .expect("export checkpoint")
        .expect("checkpoint bundle");

    let state_path = target_root.join("bridge-state.json");
    let world_dir = target_root.join("world");
    let records_dir = target_root.join("records");
    let storage_root = target_root.join("storage");
    let mut target = NodeRuntimeExecutionDriver::new_with_storage_profile(
        state_path.clone(),
        world_dir.clone(),
        records_dir.clone(),
        storage_root,
        &storage_profile,
    )
    .expect("target driver");
    target
        .on_commit(NodeExecutionCommitContext {
            world_id: "world-checkpoint-final-state-io-rollback".to_string(),
            node_id: "node-b".to_string(),
            proposer_id: "node-a".to_string(),
            height: 1,
            slot: 1,
            epoch: 0,
            node_block_hash: "block-1".to_string(),
            action_root: compute_consensus_action_root(&[]).expect("empty action root"),
            committed_actions: Vec::new(),
            committed_at_unix_ms: 12_001,
        })
        .expect("target commit 1");

    let previous_state = target.state.clone();
    let previous_world_state = target.execution_world.state().clone();
    let previous_world_journal = target.execution_world.journal().clone();
    let previous_state_bytes = fs::read(state_path.as_path()).expect("previous state bytes");
    let previous_snapshot_bytes =
        fs::read(world_dir.join("snapshot.json")).expect("previous snapshot bytes");
    let previous_journal_bytes =
        fs::read(world_dir.join("journal.json")).expect("previous journal bytes");
    let previous_latest_bytes =
        fs::read(records_dir.join("latest.json")).expect("previous latest bytes");
    let sidecar = world_dir
        .join("distfs")
        .join("module-store")
        .join("index.bin");
    fs::create_dir_all(sidecar.parent().expect("sidecar parent")).expect("create sidecar parent");
    fs::write(sidecar.as_path(), b"previous sidecar").expect("write previous sidecar");

    set_checkpoint_install_fault_for_test(Some(CheckpointInstallFault::FinalStatePersistFailure));
    let err = target
        .install_checkpoint_bundle(
            NodeExecutionCheckpointInstallContext {
                world_id: "world-checkpoint-final-state-io-rollback".to_string(),
                node_id: "node-b".to_string(),
                height: 2,
                node_block_hash: "block-2".to_string(),
                execution_block_hash: second.execution_block_hash.clone(),
                execution_state_root: second.execution_state_root.clone(),
                committed_at_unix_ms: 12_002,
            },
            bundle.clone(),
        )
        .expect_err("final state persistence I/O failure must roll back in process");
    assert!(err.contains("final state persistence failure"), "{err}");

    assert_eq!(target.state, previous_state);
    assert_eq!(target.execution_world.state(), &previous_world_state);
    assert_eq!(target.execution_world.journal(), &previous_world_journal);
    assert_eq!(
        fs::read(state_path.as_path()).expect("restored state bytes"),
        previous_state_bytes
    );
    assert_eq!(
        fs::read(world_dir.join("snapshot.json")).expect("restored snapshot bytes"),
        previous_snapshot_bytes
    );
    assert_eq!(
        fs::read(world_dir.join("journal.json")).expect("restored journal bytes"),
        previous_journal_bytes
    );
    assert_eq!(
        fs::read(records_dir.join("latest.json")).expect("restored latest bytes"),
        previous_latest_bytes
    );
    assert_eq!(
        load_execution_bridge_state(state_path.as_path()).expect("restored state"),
        previous_state
    );
    assert_eq!(
        load_execution_bridge_record(records_dir.join("latest.json").as_path())
            .expect("restored latest record")
            .height,
        1
    );
    assert!(!execution_bridge_record_path(records_dir.as_path(), 2).exists());
    assert!(!execution_checkpoint_manifest_path(records_dir.as_path(), 2).exists());
    assert!(!execution_checkpoint_latest_path(records_dir.as_path()).exists());
    assert_eq!(
        fs::read(sidecar.as_path()).expect("restored sidecar"),
        b"previous sidecar"
    );
    assert!(
        !records_dir
            .join("checkpoint-install-transaction.json")
            .exists()
    );
    let leaked_world_transaction_paths: Vec<_> = fs::read_dir(target_root.as_path())
        .expect("read target root")
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with(".world.checkpoint-install-")
        })
        .collect();
    assert!(
        leaked_world_transaction_paths.is_empty(),
        "staging/backup paths leaked: {leaked_world_transaction_paths:?}"
    );

    target
        .install_checkpoint_bundle(
            NodeExecutionCheckpointInstallContext {
                world_id: "world-checkpoint-final-state-io-rollback".to_string(),
                node_id: "node-b".to_string(),
                height: 2,
                node_block_hash: "block-2".to_string(),
                execution_block_hash: second.execution_block_hash,
                execution_state_root: second.execution_state_root,
                committed_at_unix_ms: 12_002,
            },
            bundle,
        )
        .expect("retry after in-process rollback");
    assert_eq!(target.state.last_applied_committed_height, 2);

    let _ = fs::remove_dir_all(dir);
}
