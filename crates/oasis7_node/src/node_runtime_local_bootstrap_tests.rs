use super::*;

fn restored_continuation(
    engine: &mut PosNodeEngine,
    slot: u64,
    restore_clock: bool,
) -> NodeLocalExecutionContinuation {
    let node_id = "node-clock";
    let world_id = "world-clock";
    let predecessor_height = 19;
    let predecessor_node_hash = "node-parent-19";
    let predecessor_execution_hash = "execution-parent-19";
    let predecessor_state_root = "state-parent-19";
    let bootstrap = NodeExecutionBootstrap {
        height: predecessor_height,
        consensus_block_hash: predecessor_node_hash.into(),
        execution_block_hash: predecessor_execution_hash.into(),
        execution_state_root: predecessor_state_root.into(),
    };
    engine
        .apply_local_execution_bootstrap(&bootstrap)
        .expect("apply qualified predecessor boundary");

    let height = predecessor_height + 1;
    let action_root = compute_consensus_action_root(&[]).expect("empty action root");
    let epoch = engine.slot_epoch(slot);
    let node_block_hash = engine
        .compute_block_hash(
            world_id,
            height,
            slot,
            epoch,
            node_id,
            predecessor_node_hash,
            &action_root,
        )
        .expect("compute qualified proposal hash");
    let saved = NodeLocalExecutionContinuation {
        schema_version: 1,
        context: NodeExecutionCommitContext {
            world_id: world_id.into(),
            node_id: node_id.into(),
            proposer_id: node_id.into(),
            height,
            slot,
            epoch,
            node_block_hash,
            action_root,
            committed_actions: Vec::new(),
            committed_at_unix_ms: 1_000_000,
        },
        predecessor_committed_height: predecessor_height,
        predecessor_node_block_hash: Some(predecessor_node_hash.into()),
        predecessor_execution_height: predecessor_height,
        predecessor_execution_block_hash: Some(predecessor_execution_hash.into()),
        predecessor_execution_state_root: Some(predecessor_state_root.into()),
        next_height: height,
        next_slot: slot + 1,
        reserved_action_bytes: 0,
        approved_stake: engine.required_stake,
        rejected_stake: 0,
        required_stake: engine.required_stake,
        total_stake: engine.total_stake,
    };
    engine.next_slot = saved.next_slot;
    engine
        .restore_guarded_local_decision(&saved, node_id, world_id)
        .expect("restore exact guarded local decision");
    if restore_clock {
        engine
            .restore_guarded_local_schedule_clock(&saved)
            .expect("restore clock from validated continuation");
    }
    saved
}

fn clock_test_config() -> NodeConfig {
    let mut config =
        NodeConfig::new("node-clock", "world-clock", NodeRole::Sequencer).expect("config");
    config.pos_config.slot_duration_ms = 8;
    config.pos_config.ticks_per_slot = 4;
    config.pos_config.proposal_tick_phase = 2;
    // This test exercises the schedule gate without requiring an execution
    // hook after the single-validator proposal commits.
    config.require_execution_on_commit = false;
    config
}

#[test]
fn guarded_continuation_restores_clock_before_first_tick_and_proposes_successor() {
    let config = clock_test_config();
    let committed_slot = 128_084;
    let mut unseeded = PosNodeEngine::new(&config).expect("unseeded engine");
    let unseeded_saved = restored_continuation(&mut unseeded, committed_slot, false);
    assert_eq!(unseeded.next_slot, unseeded_saved.next_slot);

    unseeded
        .tick(
            "node-clock",
            "world-clock",
            1_000_000,
            None,
            None,
            None,
            None,
            Vec::new(),
            None,
        )
        .expect("unseeded first tick");
    unseeded
        .tick(
            "node-clock",
            "world-clock",
            1_000_008,
            None,
            None,
            None,
            None,
            Vec::new(),
            None,
        )
        .expect("unseeded successor remains below restored next slot");
    assert_eq!(unseeded.committed_height, 19);
    assert!(unseeded.pending.is_none());

    let mut engine = PosNodeEngine::new(&config).expect("seeded engine");
    let saved = restored_continuation(&mut engine, committed_slot, true);

    assert_eq!(engine.next_slot, saved.next_slot);
    assert_eq!(engine.last_observed_slot, committed_slot);
    assert_eq!(
        engine.last_observed_tick,
        committed_slot * config.pos_config.ticks_per_slot + config.pos_config.proposal_tick_phase
    );

    let first = engine
        .tick(
            "node-clock",
            "world-clock",
            1_000_000,
            None,
            None,
            None,
            None,
            Vec::new(),
            None,
        )
        .expect("first tick observes the restored committed slot");
    assert_eq!(first.consensus_snapshot.committed_height, 19);
    assert!(engine.pending.is_none());

    let successor = engine
        .tick(
            "node-clock",
            "world-clock",
            1_000_008,
            None,
            None,
            None,
            None,
            Vec::new(),
            None,
        )
        .expect("successor slot becomes eligible");
    assert_eq!(successor.consensus_snapshot.committed_height, 20);
    assert_eq!(engine.committed_height, 20);
}

#[test]
fn guarded_continuation_clock_keeps_configured_absolute_origin() {
    let mut config = clock_test_config();
    config.pos_config.slot_clock_genesis_unix_ms = Some(700_000);
    let mut engine = PosNodeEngine::new(&config).expect("engine");
    restored_continuation(&mut engine, 128_084, true);

    assert_eq!(engine.slot_clock_genesis_unix_ms, Some(700_000));
    assert_eq!(engine.last_observed_slot, 128_084);
}

#[test]
fn guarded_continuation_clock_rejects_unbound_successor_slot() {
    let config = clock_test_config();
    let mut engine = PosNodeEngine::new(&config).expect("engine");
    let mut saved = restored_continuation(&mut engine, 128_084, false);
    saved.next_slot += 1;

    assert!(engine.restore_guarded_local_schedule_clock(&saved).is_err());
    assert_eq!(engine.last_observed_tick, 0);
    assert_eq!(engine.last_observed_slot, 0);
}
