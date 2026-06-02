use super::*;

#[test]
fn remote_apply_bootstraps_midstream_writer_without_overwriting_local_guard() {
    let world_id = "world-remote-apply-midstream-writer";
    let dir_a = temp_dir("remote-apply-midstream-a");
    let dir_b = temp_dir("remote-apply-midstream-b");
    let (_, public_key_a) = deterministic_keypair_hex(163);
    let (_, public_key_b) = deterministic_keypair_hex(164);
    let config_a = signed_replication_config(dir_a.clone(), 163)
        .with_remote_writer_allowlist(vec![public_key_b.clone()])
        .expect("allowlist a");
    let config_b = signed_replication_config(dir_b.clone(), 164)
        .with_remote_writer_allowlist(vec![public_key_a.clone()])
        .expect("allowlist b");
    let mut replication_a = ReplicationRuntime::new(&config_a, "node-a").expect("runtime a");
    let mut replication_b = ReplicationRuntime::new(&config_b, "node-b").expect("runtime b");

    for height in 1..=3 {
        let decision = PosDecision {
            height,
            slot: height,
            epoch: 0,
            status: PosConsensusStatus::Committed,
            block_hash: format!("block-a-{height}"),
            action_root: empty_action_root(),
            committed_actions: Vec::new(),
            approved_stake: 60,
            rejected_stake: 0,
            required_stake: 40,
            total_stake: 100,
        };
        replication_a
            .build_local_commit_message(
                "node-a",
                world_id,
                1_000 + i64::try_from(height).expect("height fits i64"),
                &decision,
                None,
                None,
            )
            .expect("build local a commit")
            .expect("local a message");
    }
    for height in 1..=2 {
        let decision = PosDecision {
            height,
            slot: height,
            epoch: 0,
            status: PosConsensusStatus::Committed,
            block_hash: format!("block-b-{height}"),
            action_root: empty_action_root(),
            committed_actions: Vec::new(),
            approved_stake: 40,
            rejected_stake: 0,
            required_stake: 40,
            total_stake: 100,
        };
        replication_b
            .build_local_commit_message(
                "node-b",
                world_id,
                2_000 + i64::try_from(height).expect("height fits i64"),
                &decision,
                None,
                None,
            )
            .expect("build local b commit")
            .expect("local b message");
    }

    let local_guard_path = dir_b.join("replication_guard.json");
    let local_guard_before: SingleWriterReplicationGuard =
        serde_json::from_slice(&fs::read(&local_guard_path).expect("read local guard before"))
            .expect("parse local guard before");
    assert_eq!(
        local_guard_before.writer_id.as_deref(),
        Some(public_key_b.as_str())
    );
    assert_eq!(local_guard_before.last_sequence, 2);

    let remote_message = replication_a
        .load_commit_message_by_height(world_id, 3)
        .expect("load remote commit")
        .expect("remote commit message");
    assert_eq!(remote_message.record.writer_id, public_key_a);
    assert_eq!(remote_message.record.sequence, 3);
    assert!(replication_b
        .validate_remote_message_for_apply("node-b", world_id, &remote_message)
        .expect("validate remote midstream commit"));
    replication_b
        .apply_remote_message("node-b", world_id, &remote_message)
        .expect("apply remote midstream commit");

    let local_guard_after: SingleWriterReplicationGuard =
        serde_json::from_slice(&fs::read(&local_guard_path).expect("read local guard after"))
            .expect("parse local guard after");
    assert_eq!(local_guard_after, local_guard_before);
    let remote_guards: BTreeMap<String, SingleWriterReplicationGuard> = serde_json::from_slice(
        &fs::read(dir_b.join("replication_remote_guards.json")).expect("read remote guards"),
    )
    .expect("parse remote guards");
    let remote_guard = remote_guards
        .get(public_key_a.as_str())
        .expect("remote writer guard");
    assert_eq!(
        remote_guard.writer_id.as_deref(),
        Some(public_key_a.as_str())
    );
    assert_eq!(remote_guard.last_sequence, 3);

    let _ = fs::remove_dir_all(&dir_a);
    let _ = fs::remove_dir_all(&dir_b);
}

#[test]
fn restart_refresh_seeds_remote_cursor_from_committed_baseline() {
    let dir_a = temp_dir("restart-refresh-remote-cursor-a");
    let dir_b = temp_dir("restart-refresh-remote-cursor-b");
    let world_id = "world-restart-remote-cursor";
    let (_, public_key_a) = deterministic_keypair_hex(165);
    let config_a = signed_replication_config(dir_a.clone(), 165);
    let config_b = signed_replication_config(dir_b.clone(), 166)
        .with_remote_writer_allowlist(vec![public_key_a])
        .expect("allowlist b");
    let mut replication_a = ReplicationRuntime::new(&config_a, "node-a").expect("runtime a");
    let mut replication_b = ReplicationRuntime::new(&config_b, "node-b").expect("runtime b");

    for height in 1..=3 {
        let decision = PosDecision {
            height,
            slot: height,
            epoch: 0,
            status: PosConsensusStatus::Committed,
            block_hash: format!("block-{height}"),
            action_root: empty_action_root(),
            committed_actions: Vec::new(),
            approved_stake: 100,
            rejected_stake: 0,
            required_stake: 67,
            total_stake: 100,
        };
        let message = replication_a
            .build_local_commit_message(
                "node-a",
                world_id,
                1_000 + i64::try_from(height).expect("height fits i64"),
                &decision,
                None,
                None,
            )
            .expect("build remote commit")
            .expect("remote message");
        replication_b
            .apply_remote_message("node-b", world_id, &message)
            .expect("apply remote commit");
    }
    fs::remove_file(
        dir_b
            .join("replication_commit_messages")
            .join("00000000000000000001.json"),
    )
    .expect("remove compacted first commit");

    let restarted_replication = ReplicationRuntime::new(&config_b, "node-b").expect("restart b");
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-b".to_string(),
            stake: 100,
        }],
        &[("node-b", 166)],
    );
    let mut restarted_engine = PosNodeEngine::new(
        &NodeConfig::new("node-b", world_id, NodeRole::Storage)
            .expect("config b")
            .with_pos_config(pos_config)
            .expect("pos config b")
            .with_replication(config_b),
    )
    .expect("engine b");
    restarted_engine.committed_height = 3;
    restarted_engine
        .refresh_replication_persisted_height(&restarted_replication, world_id)
        .expect("refresh remote persisted height");

    assert_eq!(restarted_engine.replication_persisted_height, 3);
    assert_eq!(restarted_replication.writer_last_replicated_height(), 0);

    let _ = fs::remove_dir_all(&dir_a);
    let _ = fs::remove_dir_all(&dir_b);
}

#[test]
fn restart_refresh_falls_back_to_durable_writer_when_committed_message_is_missing() {
    let dir = temp_dir("restart-refresh-falls-back-writer-cursor");
    let world_id = "world-restart-writer-cursor-fallback";
    let validators = vec![PosValidator {
        validator_id: "node-a".to_string(),
        stake: 100,
    }];
    let pos_config = signed_pos_config_with_signer_seeds(validators, &[("node-a", 167)]);
    let config = NodeConfig::new("node-a", world_id, NodeRole::Sequencer)
        .expect("config")
        .with_pos_config(pos_config)
        .expect("pos config")
        .with_replication(signed_replication_config(dir.clone(), 167));
    let mut replication =
        ReplicationRuntime::new(config.replication.as_ref().expect("replication"), "node-a")
            .expect("replication runtime");
    for height in 1..=3 {
        let decision = PosDecision {
            height,
            slot: height,
            epoch: 0,
            status: PosConsensusStatus::Committed,
            block_hash: format!("block-{height}"),
            action_root: empty_action_root(),
            committed_actions: Vec::new(),
            approved_stake: 100,
            rejected_stake: 0,
            required_stake: 67,
            total_stake: 100,
        };
        replication
            .build_local_commit_message(
                "node-a",
                world_id,
                1_000 + i64::try_from(height).expect("height fits i64"),
                &decision,
                None,
                None,
            )
            .expect("build commit")
            .expect("commit message");
    }
    fs::remove_file(
        dir.join("replication_commit_messages")
            .join("00000000000000000001.json"),
    )
    .expect("remove compacted first commit");

    let restarted_replication =
        ReplicationRuntime::new(config.replication.as_ref().expect("replication"), "node-a")
            .expect("restarted replication runtime");
    let mut restarted_engine = PosNodeEngine::new(&config).expect("restarted engine");
    restarted_engine.committed_height = 4;
    restarted_engine
        .refresh_replication_persisted_height(&restarted_replication, world_id)
        .expect("refresh replication persisted height");

    assert_eq!(restarted_engine.replication_persisted_height, 3);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn restart_reconcile_rolls_back_unreplicated_committed_head() {
    let dir = temp_dir("restart-reconcile-unreplicated-head");
    let world_id = "world-reconcile-unreplicated-head";
    let validators = vec![PosValidator {
        validator_id: "node-a".to_string(),
        stake: 100,
    }];
    let pos_config = signed_pos_config_with_signer_seeds(validators, &[("node-a", 168)]);
    let config = NodeConfig::new("node-a", world_id, NodeRole::Sequencer)
        .expect("config")
        .with_pos_config(pos_config)
        .expect("pos config")
        .with_replication(signed_replication_config(dir.clone(), 168));
    let mut replication =
        ReplicationRuntime::new(config.replication.as_ref().expect("replication"), "node-a")
            .expect("replication runtime");
    let mut last_payload = None;
    for height in 1..=3 {
        let decision = PosDecision {
            height,
            slot: height,
            epoch: 0,
            status: PosConsensusStatus::Committed,
            block_hash: format!("block-{height}"),
            action_root: empty_action_root(),
            committed_actions: Vec::new(),
            approved_stake: 100,
            rejected_stake: 0,
            required_stake: 67,
            total_stake: 100,
        };
        let message = replication
            .build_local_commit_message(
                "node-a",
                world_id,
                1_000 + i64::try_from(height).expect("height fits i64"),
                &decision,
                Some(format!("exec-block-{height}").as_str()),
                Some(format!("exec-state-{height}").as_str()),
            )
            .expect("build commit")
            .expect("commit message");
        last_payload = Some(
            super::replication_state_reconcile::parse_replication_commit_payload(
                message.payload.as_slice(),
            )
            .expect("parse payload"),
        );
    }
    fs::remove_file(
        dir.join("replication_commit_messages")
            .join("00000000000000000001.json"),
    )
    .expect("remove compacted first commit");

    let mut engine = PosNodeEngine::new(&config).expect("engine");
    engine.committed_height = 4;
    engine.network_committed_height = 4;
    engine.next_height = 5;
    engine.last_committed_block_hash = Some("unreplicated-block-4".to_string());
    engine.last_execution_height = 4;
    engine.last_execution_block_hash = Some("unreplicated-exec-block-4".to_string());
    engine.last_execution_state_root = Some("unreplicated-exec-state-4".to_string());
    engine
        .refresh_replication_persisted_height(&replication, world_id)
        .expect("refresh persisted height");
    assert_eq!(engine.replication_persisted_height, 3);

    super::replication_state_reconcile::reconcile_engine_with_persisted_replication(
        &mut engine,
        &replication,
        world_id,
        None,
    )
    .expect("reconcile");
    let payload = last_payload.expect("last payload");
    assert_eq!(engine.committed_height, 3);
    assert_eq!(engine.network_committed_height, 3);
    assert_eq!(engine.next_height, 4);
    assert_eq!(
        engine.last_committed_block_hash.as_deref(),
        Some(payload.block_hash.as_str())
    );
    assert_eq!(engine.last_execution_height, 3);
    assert_eq!(
        engine.last_execution_block_hash.as_deref(),
        payload.execution_block_hash.as_deref()
    );
    assert_eq!(
        engine.last_execution_state_root.as_deref(),
        payload.execution_state_root.as_deref()
    );
    assert_eq!(engine.last_replication_gap_sync_blocked_height, None);

    let _ = fs::remove_dir_all(&dir);
}
