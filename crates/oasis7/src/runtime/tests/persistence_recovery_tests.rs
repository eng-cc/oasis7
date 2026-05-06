use super::*;

#[test]
fn load_from_dir_falls_back_to_json_when_distfs_sidecar_is_invalid() {
    let mut world = World::new();
    world.submit_action(Action::RegisterAgent {
        agent_id: "agent-1".to_string(),
        pos: pos(0, 0),
    });
    world.step().unwrap();

    let dir = temp_dir("persist-distfs-fallback");
    world.save_to_dir(&dir).expect("save world");
    fs::write(
        dir.join("snapshot.manifest.json"),
        b"{\"manifest\":\"broken\"}",
    )
    .expect("tamper sidecar");

    let restored = World::load_from_dir(&dir).expect("fallback to legacy json");
    assert_eq!(restored.state(), world.state());
    let audit_value: serde_json::Value = serde_json::from_slice(
        &fs::read(dir.join("distfs.recovery.audit.json")).expect("read distfs fallback audit"),
    )
    .expect("decode distfs fallback audit");
    assert_eq!(
        audit_value.get("status").and_then(|value| value.as_str()),
        Some("fallback_json")
    );
    assert!(audit_value
        .get("reason")
        .and_then(|value| value.as_str())
        .map(|reason| reason.contains("distfs_restore_failed"))
        .unwrap_or(false));
    assert!(audit_value
        .get("timestamp_ms")
        .and_then(|value| value.as_i64())
        .is_some());

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn snapshot_json_without_era_fields_keeps_backward_compatibility() {
    let mut world = World::new();
    world.submit_action(Action::RegisterAgent {
        agent_id: "agent-legacy".to_string(),
        pos: pos(0, 0),
    });
    world.step().expect("step");

    let snapshot = world.snapshot();
    let mut value = serde_json::to_value(&snapshot).expect("encode snapshot");
    let object = value.as_object_mut().expect("snapshot object");
    object.remove("event_id_era");
    object.remove("action_id_era");
    object.remove("intent_id_era");
    object.remove("proposal_id_era");

    let legacy_json = serde_json::to_string(&value).expect("legacy json");
    let restored = Snapshot::from_json(&legacy_json).expect("decode legacy snapshot");
    assert_eq!(restored.event_id_era, 0);
    assert_eq!(restored.action_id_era, 0);
    assert_eq!(restored.intent_id_era, 0);
    assert_eq!(restored.proposal_id_era, 0);
}

#[test]
fn rollback_to_snapshot_resets_state() {
    let mut world = World::new();
    world.submit_action(Action::RegisterAgent {
        agent_id: "agent-1".to_string(),
        pos: pos(0, 0),
    });
    world.step().unwrap();
    let snapshot = world.snapshot();

    world.submit_action(Action::MoveAgent {
        agent_id: "agent-1".to_string(),
        to: pos(9, 9),
    });
    world.step().unwrap();
    assert_eq!(
        world.state().agents.get("agent-1").unwrap().state.pos,
        pos(9, 9)
    );

    let journal = world.journal().clone();
    world
        .rollback_to_snapshot(snapshot.clone(), journal, "test-rollback")
        .unwrap();

    assert_eq!(world.state(), &snapshot.state);
    let last = world.journal().events.last().unwrap();
    assert!(matches!(last.body, WorldEventBody::RollbackApplied(_)));
}

#[test]
fn rollback_with_reconciliation_recovers_from_detected_tick_consensus_drift() {
    let mut world = World::new();
    world
        .bind_node_identity("relay.node.1", "relay-public-key-1")
        .expect("bind relay identity");
    world.submit_action(Action::RegisterAgent {
        agent_id: "agent-1".to_string(),
        pos: pos(0, 0),
    });
    world.step().expect("step");

    let stable_snapshot = world.snapshot();
    let stable_journal = world.journal().clone();

    world
        .record_tick_consensus_propagation_for_tick(0, "relay.node.1")
        .expect("inject propagation record that breaks parent ordering");
    let drift = world
        .first_tick_consensus_drift()
        .expect("drift report should be present");
    assert_eq!(drift.tick, 0);
    assert!(
        drift.reason.contains("parent hash mismatch"),
        "unexpected drift reason: {}",
        drift.reason
    );
    world
        .verify_tick_consensus_chain()
        .expect_err("drifted chain should fail verification");

    world
        .rollback_to_snapshot_with_reconciliation(
            stable_snapshot,
            stable_journal,
            "reconcile-after-drift",
        )
        .expect("rollback with reconciliation");

    assert!(
        world.first_tick_consensus_drift().is_none(),
        "drift should be fully reconciled after rollback"
    );
    world
        .verify_tick_consensus_chain()
        .expect("reconciled chain should verify");
}

#[test]
fn snapshot_retention_policy_prunes_old_entries() {
    let mut world = World::new();
    world.set_snapshot_retention(SnapshotRetentionPolicy { max_snapshots: 1 });

    world.submit_action(Action::RegisterAgent {
        agent_id: "agent-1".to_string(),
        pos: pos(0, 0),
    });
    world.step().unwrap();
    let snap1 = world.create_snapshot().unwrap();

    world.submit_action(Action::MoveAgent {
        agent_id: "agent-1".to_string(),
        to: pos(3, 3),
    });
    world.step().unwrap();
    let snap2 = world.create_snapshot().unwrap();

    assert_eq!(world.snapshot_catalog().records.len(), 1);
    let last_record = &world.snapshot_catalog().records[0];
    assert_eq!(last_record.snapshot_hash, util::hash_json(&snap2).unwrap());
    assert_ne!(last_record.snapshot_hash, util::hash_json(&snap1).unwrap());
}

#[test]
fn snapshot_file_pruning_removes_old_files() {
    let mut world = World::new();
    world.set_snapshot_retention(SnapshotRetentionPolicy { max_snapshots: 1 });

    let dir = std::env::temp_dir().join(format!(
        "oasis7-snapshots-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));

    world.save_snapshot_to_dir(&dir).unwrap();
    world.submit_action(Action::RegisterAgent {
        agent_id: "agent-1".to_string(),
        pos: pos(0, 0),
    });
    world.step().unwrap();
    world.save_snapshot_to_dir(&dir).unwrap();

    let snapshots_dir = dir.join("snapshots");
    let file_count = fs::read_dir(&snapshots_dir)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .count();
    assert_eq!(file_count, 1);

    let _ = fs::remove_dir_all(&dir);
}
