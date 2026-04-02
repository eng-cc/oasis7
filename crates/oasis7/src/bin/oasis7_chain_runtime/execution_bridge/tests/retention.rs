use super::super::checkpoint::{
    execution_bridge_record_path, execution_checkpoint_manifest_rel_path,
    list_execution_checkpoint_heights, load_execution_bridge_record,
    load_latest_execution_checkpoint_manifest, maybe_persist_execution_checkpoint_for_record,
    persist_execution_bridge_record, persist_execution_checkpoint_manifest,
    run_execution_bridge_retention_maintenance, sync_execution_bridge_pin_set,
};
use super::super::driver::bridge_committed_heights;
use super::super::external_effect::build_execution_replay_plan;
use super::*;
use std::collections::BTreeSet;

use oasis7::runtime::BlobStore;
use oasis7::runtime::{LocalCasStore, World as RuntimeWorld};
use oasis7_wasm_abi::ModuleOutput;
use oasis7_wasm_executor::FixedSandbox;

#[test]
fn execution_checkpoint_cadence_trims_old_manifests_and_clears_record_refs() {
    let dir = temp_dir("execution-checkpoint-cadence-trim");
    let records_dir = dir.join("records");
    let store = LocalCasStore::new(dir.join("store"));
    fs::create_dir_all(records_dir.as_path()).expect("create records dir");

    for height in 1..=6 {
        let mut record =
            persist_test_execution_record_with_store_refs(records_dir.as_path(), &store, height);
        record.checkpoint_ref =
            maybe_persist_execution_checkpoint_for_record(records_dir.as_path(), &record, 2, 2)
                .expect("maybe persist checkpoint");
        persist_execution_bridge_record(records_dir.as_path(), &record)
            .expect("persist checkpointed record");
    }

    assert_eq!(
        list_execution_checkpoint_heights(records_dir.as_path()).expect("list checkpoint heights"),
        vec![4, 6]
    );
    let record_2 = load_execution_bridge_record(
        execution_bridge_record_path(records_dir.as_path(), 2).as_path(),
    )
    .expect("load record 2");
    let record_4 = load_execution_bridge_record(
        execution_bridge_record_path(records_dir.as_path(), 4).as_path(),
    )
    .expect("load record 4");
    let record_6 = load_execution_bridge_record(
        execution_bridge_record_path(records_dir.as_path(), 6).as_path(),
    )
    .expect("load record 6");
    assert!(record_2.checkpoint_ref.is_none());
    assert_eq!(
        record_4.checkpoint_ref.as_deref(),
        Some(execution_checkpoint_manifest_rel_path(4).as_str())
    );
    assert_eq!(
        record_6.checkpoint_ref.as_deref(),
        Some(execution_checkpoint_manifest_rel_path(6).as_str())
    );
    let latest = load_latest_execution_checkpoint_manifest(records_dir.as_path())
        .expect("load latest checkpoint")
        .expect("latest checkpoint exists");
    assert_eq!(latest.height, 6);

    let plan = build_execution_replay_plan(records_dir.as_path(), &store, 5)
        .expect("build replay plan from sparse checkpoint");
    assert_eq!(
        plan.checkpoint.as_ref().map(|manifest| manifest.height),
        Some(4)
    );
    assert_eq!(plan.start_height, 5);
    assert_eq!(plan.records.len(), 1);
    assert_eq!(plan.records[0].record.height, 5);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn bridge_committed_heights_persists_sparse_checkpoint_at_default_interval() {
    let dir = temp_dir("execution-bridge-default-checkpoint");
    let store = LocalCasStore::new(dir.join("store"));
    let mut world = RuntimeWorld::new();
    let mut sandbox = FixedSandbox::succeed(ModuleOutput {
        new_state: None,
        effects: Vec::new(),
        emits: Vec::new(),
        tick_lifecycle: None,
        output_bytes: 0,
    });
    let mut state = ExecutionBridgeState::default();
    let records_dir = dir.join("records");
    let snapshot = sample_snapshot(
        EXECUTION_BRIDGE_DEFAULT_CHECKPOINT_INTERVAL_HEIGHTS,
        Some("node-h32"),
    );

    let records = bridge_committed_heights(
        &snapshot,
        1_000,
        &mut world,
        &mut sandbox,
        &store,
        records_dir.as_path(),
        &mut state,
    )
    .expect("bridge committed heights");

    assert_eq!(
        records.len() as u64,
        EXECUTION_BRIDGE_DEFAULT_CHECKPOINT_INTERVAL_HEIGHTS
    );
    let latest_record = records.last().expect("latest record");
    assert_eq!(
        latest_record.checkpoint_ref.as_deref(),
        Some(
            execution_checkpoint_manifest_rel_path(
                EXECUTION_BRIDGE_DEFAULT_CHECKPOINT_INTERVAL_HEIGHTS,
            )
            .as_str()
        )
    );
    let latest_checkpoint = load_latest_execution_checkpoint_manifest(records_dir.as_path())
        .expect("load latest checkpoint")
        .expect("latest checkpoint exists");
    assert_eq!(
        latest_checkpoint.height,
        EXECUTION_BRIDGE_DEFAULT_CHECKPOINT_INTERVAL_HEIGHTS
    );

    let plan = build_execution_replay_plan(
        records_dir.as_path(),
        &store,
        EXECUTION_BRIDGE_DEFAULT_CHECKPOINT_INTERVAL_HEIGHTS,
    )
    .expect("build replay plan");
    assert_eq!(
        plan.checkpoint.as_ref().map(|manifest| manifest.height),
        Some(EXECUTION_BRIDGE_DEFAULT_CHECKPOINT_INTERVAL_HEIGHTS)
    );
    assert!(plan.records.is_empty());

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execution_bridge_retention_maintenance_clears_archive_refs_and_prunes_orphans() {
    let dir = temp_dir("execution-bridge-retention-maintenance");
    let records_dir = dir.join("records");
    let store = LocalCasStore::new(dir.join("store"));
    fs::create_dir_all(records_dir.as_path()).expect("create records dir");

    let mut records = Vec::new();
    for height in 1..=6 {
        let mut record =
            persist_test_execution_record_with_store_refs(records_dir.as_path(), &store, height);
        record.checkpoint_ref =
            maybe_persist_execution_checkpoint_for_record(records_dir.as_path(), &record, 2, 2)
                .expect("maybe persist checkpoint");
        persist_execution_bridge_record(records_dir.as_path(), &record)
            .expect("persist checkpointed record");
        records.push(record);
    }

    let freed_bytes = run_execution_bridge_retention_maintenance(records_dir.as_path(), &store, 2)
        .expect("run retention maintenance");
    assert!(freed_bytes > 0, "expected orphan sweep to free bytes");

    let record_1 = load_execution_bridge_record(
        execution_bridge_record_path(records_dir.as_path(), 1).as_path(),
    )
    .expect("load record 1");
    let record_4 = load_execution_bridge_record(
        execution_bridge_record_path(records_dir.as_path(), 4).as_path(),
    )
    .expect("load record 4");
    let record_5 = load_execution_bridge_record(
        execution_bridge_record_path(records_dir.as_path(), 5).as_path(),
    )
    .expect("load record 5");
    let record_6 = load_execution_bridge_record(
        execution_bridge_record_path(records_dir.as_path(), 6).as_path(),
    )
    .expect("load record 6");

    assert!(record_1.latest_state_ref.is_none());
    assert!(record_1.snapshot_ref.is_none());
    assert!(record_1.journal_ref.is_none());
    assert!(record_1.simulator_mirror.is_none());
    assert_eq!(
        record_4.checkpoint_ref.as_deref(),
        Some(execution_checkpoint_manifest_rel_path(4).as_str())
    );
    assert!(record_4.snapshot_ref.is_none());
    assert!(record_4.journal_ref.is_none());
    assert!(record_4.simulator_mirror.is_none());
    assert!(record_5.snapshot_ref.is_some());
    assert!(record_5.journal_ref.is_some());
    assert!(record_6.snapshot_ref.is_some());
    assert!(record_6.journal_ref.is_some());

    assert!(!store
        .has(
            records[0]
                .snapshot_ref
                .as_deref()
                .expect("record1 snapshot ref")
        )
        .expect("check archive snapshot"));
    assert!(store
        .has(
            records[3]
                .snapshot_ref
                .as_deref()
                .expect("record4 snapshot ref")
        )
        .expect("check checkpoint snapshot"));
    assert!(store
        .has(
            records[4]
                .journal_ref
                .as_deref()
                .expect("record5 journal ref")
        )
        .expect("check hot journal"));

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn bridge_committed_heights_sweeps_archive_refs_outside_default_hot_window() {
    let dir = temp_dir("execution-bridge-default-retention-sweep");
    let store = LocalCasStore::new(dir.join("store"));
    let mut world = RuntimeWorld::new();
    let mut sandbox = FixedSandbox::succeed(ModuleOutput {
        new_state: None,
        effects: Vec::new(),
        emits: Vec::new(),
        tick_lifecycle: None,
        output_bytes: 0,
    });
    let mut state = ExecutionBridgeState::default();
    let records_dir = dir.join("records");
    let target_height = EXECUTION_BRIDGE_DEFAULT_HOT_WINDOW_HEIGHTS
        + EXECUTION_BRIDGE_DEFAULT_CHECKPOINT_INTERVAL_HEIGHTS;
    let snapshot = sample_snapshot(target_height, Some("node-h64"));

    let records = bridge_committed_heights(
        &snapshot,
        1_000,
        &mut world,
        &mut sandbox,
        &store,
        records_dir.as_path(),
        &mut state,
    )
    .expect("bridge committed heights");

    let record_1 = load_execution_bridge_record(
        execution_bridge_record_path(records_dir.as_path(), 1).as_path(),
    )
    .expect("load record 1");
    let checkpoint_height = EXECUTION_BRIDGE_DEFAULT_CHECKPOINT_INTERVAL_HEIGHTS;
    let record_checkpoint = load_execution_bridge_record(
        execution_bridge_record_path(records_dir.as_path(), checkpoint_height).as_path(),
    )
    .expect("load checkpoint record");
    let record_hot = load_execution_bridge_record(
        execution_bridge_record_path(records_dir.as_path(), checkpoint_height + 1).as_path(),
    )
    .expect("load hot record");

    assert!(record_1.snapshot_ref.is_none());
    assert!(record_1.journal_ref.is_none());
    assert_eq!(
        record_checkpoint.checkpoint_ref.as_deref(),
        Some(execution_checkpoint_manifest_rel_path(checkpoint_height).as_str())
    );
    assert!(record_checkpoint.snapshot_ref.is_none());
    assert!(record_checkpoint.journal_ref.is_none());
    assert!(record_hot.snapshot_ref.is_some());
    assert!(record_hot.journal_ref.is_some());

    assert!(!store
        .has(
            records[0]
                .snapshot_ref
                .as_deref()
                .expect("record1 snapshot ref")
        )
        .expect("check archive snapshot"));
    let checkpoint_index = checkpoint_height.saturating_sub(1) as usize;
    assert!(store
        .has(
            records[checkpoint_index]
                .snapshot_ref
                .as_deref()
                .expect("checkpoint snapshot ref"),
        )
        .expect("check checkpoint snapshot"));
    assert!(store
        .has(
            records[checkpoint_index + 1]
                .journal_ref
                .as_deref()
                .expect("hot journal ref"),
        )
        .expect("check hot journal"));

    let plan = build_execution_replay_plan(records_dir.as_path(), &store, checkpoint_height + 8)
        .expect("build replay plan from sparse checkpoint");
    assert_eq!(
        plan.checkpoint.as_ref().map(|manifest| manifest.height),
        Some(checkpoint_height)
    );
    assert_eq!(plan.start_height, checkpoint_height + 1);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execution_bridge_pin_set_keeps_latest_head_and_hot_window_refs() {
    let dir = temp_dir("execution-bridge-pin-set-hot-window");
    let records_dir = dir.join("records");
    let store = LocalCasStore::new(dir.join("store"));
    fs::create_dir_all(records_dir.as_path()).expect("create records dir");

    let mut all_refs = Vec::new();
    let mut records = Vec::new();
    for height in 1..=4 {
        let record =
            persist_test_execution_record_with_store_refs(records_dir.as_path(), &store, height);
        all_refs.extend(record.snapshot_ref.iter().cloned());
        all_refs.extend(record.journal_ref.iter().cloned());
        all_refs.extend(record.latest_state_ref.iter().cloned());
        all_refs.extend(record.external_effect_ref.iter().cloned());
        if let Some(simulator_mirror) = record.simulator_mirror.as_ref() {
            all_refs.push(simulator_mirror.snapshot_ref.clone());
            all_refs.push(simulator_mirror.journal_ref.clone());
        }
        records.push(record);
    }
    all_refs.sort();
    all_refs.dedup();

    for content_ref in &all_refs {
        store.pin(content_ref.as_str()).expect("pre-pin record ref");
    }

    let pin_set =
        sync_execution_bridge_pin_set(records_dir.as_path(), &store, 2).expect("sync pin set");
    assert_eq!(pin_set.latest_height, Some(4));
    assert_eq!(pin_set.hot_window_start_height, Some(3));

    let actual_pins = store
        .list_pins()
        .expect("list pins")
        .into_iter()
        .collect::<BTreeSet<_>>();
    let mut expected_pins = BTreeSet::new();
    for record in &records {
        expected_pins.extend(record.external_effect_ref.iter().cloned());
        if record.height >= 3 {
            expected_pins.extend(record.snapshot_ref.iter().cloned());
            expected_pins.extend(record.journal_ref.iter().cloned());
            if let Some(simulator_mirror) = record.simulator_mirror.as_ref() {
                expected_pins.insert(simulator_mirror.snapshot_ref.clone());
                expected_pins.insert(simulator_mirror.journal_ref.clone());
            }
        }
        if record.height == 4 {
            expected_pins.extend(record.latest_state_ref.iter().cloned());
        }
    }
    assert_eq!(actual_pins, expected_pins);
    assert!(!records[0]
        .snapshot_ref
        .as_ref()
        .is_some_and(|snapshot_ref| actual_pins.contains(snapshot_ref)));
    assert!(!records[1]
        .journal_ref
        .as_ref()
        .is_some_and(|journal_ref| actual_pins.contains(journal_ref)));

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn execution_bridge_pin_set_keeps_checkpoint_refs_outside_hot_window() {
    let dir = temp_dir("execution-bridge-pin-set-checkpoint");
    let records_dir = dir.join("records");
    let store = LocalCasStore::new(dir.join("store"));
    fs::create_dir_all(records_dir.as_path()).expect("create records dir");

    for height in 1..=3 {
        let _ =
            persist_test_execution_record_with_store_refs(records_dir.as_path(), &store, height);
    }

    let checkpoint_latest_state_ref = store
        .put_bytes(b"checkpoint-latest-state")
        .expect("store checkpoint latest state");
    let checkpoint_snapshot_ref = store
        .put_bytes(b"checkpoint-snapshot")
        .expect("store checkpoint snapshot");
    let checkpoint_journal_ref = store
        .put_bytes(b"checkpoint-journal")
        .expect("store checkpoint journal");
    let checkpoint = ExecutionCheckpointManifest::new(
        "w1".to_string(),
        1,
        "exec-h1".to_string(),
        "state-root-1".to_string(),
        checkpoint_latest_state_ref.clone(),
        Some(checkpoint_snapshot_ref.clone()),
        Some(checkpoint_journal_ref.clone()),
        1_000,
    )
    .expect("checkpoint");
    persist_execution_checkpoint_manifest(records_dir.as_path(), &checkpoint)
        .expect("persist checkpoint");

    let pin_set =
        sync_execution_bridge_pin_set(records_dir.as_path(), &store, 1).expect("sync pin set");
    let actual_pins = pin_set.pinned_refs;
    assert!(actual_pins.contains(&checkpoint_latest_state_ref));
    assert!(actual_pins.contains(&checkpoint_snapshot_ref));
    assert!(actual_pins.contains(&checkpoint_journal_ref));

    let _ = fs::remove_dir_all(dir);
}
