use super::super::checkpoint::{execution_bridge_record_path, load_execution_bridge_record};
use super::super::driver::{NodeRuntimeExecutionDriver, load_execution_world};
use super::*;
use oasis7::geometry::GeoPos;
use oasis7::runtime::{Action, ChainResourceDerivationContext, World as RuntimeWorld};
use oasis7_node::{
    NodeConsensusAction, NodeExecutionCommitContext, NodeExecutionHook,
    NodeReplicatedExecutionInputV1, PROVIDER_BACKED_BOOTSTRAP_EXECUTION_INPUT_KIND,
    REPLICATED_EXECUTION_INPUT_ACTION_ID, REPLICATED_EXECUTION_INPUT_SUBMITTER,
    compute_consensus_action_root,
};

fn provider_bootstrap_fixture(
    world_dir: &std::path::Path,
) -> oasis7::runtime::ProviderBackedBootstrapAuthorityV1 {
    let mut world = RuntimeWorld::new();
    world.submit_action(Action::RegisterAgent {
        agent_id: "agent-a".to_string(),
        pos: GeoPos::new(0, 0, 0),
    });
    world.step().expect("register fixture agent");
    world
        .bind_cognition_runtime("bootstrap-world", "main", 0, None, "pending", 0)
        .expect("bind fixture cognition");
    world
        .install_test_provider_capability_fixture_without_cognition_balance("agent-a")
        .expect("install fixture authority");
    let input = world
        .test_provider_backed_bootstrap_authority(
            "agent-a",
            "provider-bootstrap-chain-commit",
            "provider-bootstrap-chain-authority",
            7,
        )
        .expect("build provider bootstrap input");

    // Keep the setup tick in the persisted baseline so the fixture's grant is
    // live. The bridge tests seed the matching committed predecessor head
    // below, then exercise canonical records at heights two and three.
    world
        .save_to_dir_with_chain_resource_context(
            world_dir,
            ChainResourceDerivationContext {
                world_id: "bootstrap-world",
                chain_id: "bootstrap-world",
                genesis_ref: None,
                created_at_height: 0,
                manifest_height: 0,
                commit_block_hash: None,
                tick: world.state().time,
            },
            "provider-bootstrap-config",
            "provider-bootstrap-generation",
        )
        .expect("persist fixture baseline");
    input
}

fn commit_context_with_actions(
    height: u64,
    committed_actions: Vec<NodeConsensusAction>,
) -> NodeExecutionCommitContext {
    NodeExecutionCommitContext {
        world_id: "bootstrap-world".to_string(),
        node_id: "node-a".to_string(),
        proposer_id: "node-a".to_string(),
        height,
        slot: height.saturating_sub(1),
        epoch: 0,
        node_block_hash: format!("node-h{height}"),
        action_root: compute_consensus_action_root(committed_actions.as_slice())
            .expect("action root"),
        committed_actions,
        committed_at_unix_ms: height as i64 * 1_000,
    }
}

fn commit_context(height: u64) -> NodeExecutionCommitContext {
    commit_context_with_actions(height, Vec::new())
}

fn provider_bootstrap_action(
    target_height: u64,
    input: oasis7::runtime::ProviderBackedBootstrapAuthorityV1,
) -> NodeConsensusAction {
    let authorities = serde_cbor::to_vec(&vec![input]).expect("encode provider authorities");
    let input = NodeReplicatedExecutionInputV1::new(
        PROVIDER_BACKED_BOOTSTRAP_EXECUTION_INPUT_KIND,
        target_height,
        authorities,
    );
    NodeConsensusAction::from_payload(
        REPLICATED_EXECUTION_INPUT_ACTION_ID,
        REPLICATED_EXECUTION_INPUT_SUBMITTER,
        input.encode().expect("encode provider execution input"),
    )
    .expect("build provider bootstrap action")
}

fn seed_predecessor_commit(
    driver: &mut NodeRuntimeExecutionDriver,
    state_path: &std::path::Path,
    world_dir: &std::path::Path,
    records_dir: &std::path::Path,
    storage_root: &std::path::Path,
) {
    // The fixture needs one setup tick so its capability grant is live. Seed
    // a real height-one bridge record with an isolated empty runtime, leaving
    // the fixture's persisted predecessor untouched for the bootstrap commit
    // at height two.
    let predecessor_state_path = state_path.with_extension("predecessor.json");
    let predecessor_world_dir = world_dir.with_extension("predecessor");
    let predecessor_simulator_dir = predecessor_world_dir.with_extension("simulator");
    let mut predecessor = NodeRuntimeExecutionDriver::new(
        predecessor_state_path.clone(),
        predecessor_world_dir.clone(),
        records_dir.to_path_buf(),
        storage_root.to_path_buf(),
    )
    .expect("predecessor driver");
    let result = predecessor
        .on_commit(commit_context(1))
        .expect("seed canonical predecessor record");
    driver.state.last_applied_committed_height = result.execution_height;
    driver.state.last_execution_block_hash = Some(result.execution_block_hash);
    driver.state.last_execution_state_root = Some(result.execution_state_root);
    let _ = fs::remove_dir_all(predecessor_world_dir);
    let _ = fs::remove_dir_all(predecessor_simulator_dir);
    let _ = fs::remove_file(predecessor_state_path);
}

#[test]
fn provider_bootstrap_waits_for_canonical_commit_and_replays_after_restart() {
    let dir = temp_dir("provider-bootstrap-canonical-commit");
    let state_path = dir.join("state.json");
    let world_dir = dir.join("world");
    let records_dir = dir.join("records");
    let storage_root = dir.join("store");
    let input = provider_bootstrap_fixture(world_dir.as_path());

    let mut driver = NodeRuntimeExecutionDriver::new(
        state_path.clone(),
        world_dir.clone(),
        records_dir.clone(),
        storage_root.clone(),
    )
    .expect("driver");
    seed_predecessor_commit(
        &mut driver,
        state_path.as_path(),
        world_dir.as_path(),
        records_dir.as_path(),
        storage_root.as_path(),
    );
    let before = load_execution_world(world_dir.as_path()).expect("load baseline world");
    assert_eq!(
        before
            .cognition_economy()
            .expect("baseline cognition economy")
            .available_balance(input.owner_binding.as_str(), "cognition_units"),
        0,
        "startup staging must not persist a cognition allowance"
    );

    assert!(
        !execution_bridge_record_path(records_dir.as_path(), 2).exists(),
        "producer input must not fabricate the bootstrap execution record"
    );

    let first_context =
        commit_context_with_actions(2, vec![provider_bootstrap_action(2, input.clone())]);
    let first_result = driver
        .on_commit(first_context.clone())
        .expect("canonical commit");
    let first_record = load_execution_bridge_record(
        execution_bridge_record_path(records_dir.as_path(), 2).as_path(),
    )
    .expect("load canonical bootstrap record");
    assert_eq!(first_result.execution_height, 2);
    assert_eq!(first_record.height, 2);
    assert_eq!(
        first_record.execution_state_root, first_result.execution_state_root,
        "commit result must expose the recorded execution root"
    );
    // Ordinary bridge commits persist the authoritative world in the CAS
    // record; the world directory remains the startup cache until a restart
    // reconciles it to that record.
    assert_eq!(driver.execution_world.state().time, 2);
    assert_eq!(
        driver
            .execution_world
            .cognition_economy()
            .expect("committed cognition economy")
            .available_balance(input.owner_binding.as_str(), "cognition_units"),
        input.allowance,
        "authority allowance is published with the canonical tick"
    );

    let mut restarted = NodeRuntimeExecutionDriver::new(
        state_path.clone(),
        world_dir.clone(),
        records_dir.clone(),
        storage_root.clone(),
    )
    .expect("restart from canonical bootstrap record");
    assert_eq!(
        restarted.state.last_execution_state_root.as_deref(),
        Some(first_record.execution_state_root.as_str())
    );
    assert_eq!(restarted.execution_world.state().time, 2);
    assert_eq!(
        restarted
            .execution_world
            .cognition_economy()
            .expect("restarted cognition economy")
            .available_balance(input.owner_binding.as_str(), "cognition_units"),
        input.allowance,
        "restart must retain the exact provisioned allowance"
    );

    let replay_result = restarted
        .on_commit(first_context)
        .expect("exact replay of the committed bootstrap input");
    assert_eq!(replay_result, first_result);

    // The bootstrap transaction is height-bound and is not re-injected by the
    // process on later commits. A replayed chain state must still tick without
    // refilling the account.
    let owner_binding = input.owner_binding.clone();
    let second_result = restarted
        .on_commit(commit_context(3))
        .expect("commit later height after restart");
    let second_record = load_execution_bridge_record(
        execution_bridge_record_path(records_dir.as_path(), 3).as_path(),
    )
    .expect("load later canonical record");
    assert_eq!(second_result.execution_height, 3);
    assert_ne!(
        second_record.execution_state_root, first_record.execution_state_root,
        "later tick must advance the canonical execution root"
    );
    assert_eq!(restarted.execution_world.state().time, 3);
    assert_eq!(
        restarted
            .execution_world
            .cognition_economy()
            .expect("later cognition economy")
            .available_balance(owner_binding.as_str(), "cognition_units"),
        7,
        "exact replay must not refill the account"
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn provider_bootstrap_replicated_input_converges_across_materializers() {
    let dir_a = temp_dir("provider-bootstrap-peer-a");
    let dir_b = temp_dir("provider-bootstrap-peer-b");
    let world_a = dir_a.join("world");
    let world_b = dir_b.join("world");
    let input_a = provider_bootstrap_fixture(world_a.as_path());
    let input_b = provider_bootstrap_fixture(world_b.as_path());
    assert_eq!(
        input_a, input_b,
        "same ordered authority input must be reproducible"
    );

    let mut driver_a = NodeRuntimeExecutionDriver::new(
        dir_a.join("state.json"),
        world_a.clone(),
        dir_a.join("records"),
        dir_a.join("store"),
    )
    .expect("peer A driver");
    let mut driver_b = NodeRuntimeExecutionDriver::new(
        dir_b.join("state.json"),
        world_b.clone(),
        dir_b.join("records"),
        dir_b.join("store"),
    )
    .expect("peer B driver");
    seed_predecessor_commit(
        &mut driver_a,
        dir_a.join("state.json").as_path(),
        world_a.as_path(),
        dir_a.join("records").as_path(),
        dir_a.join("store").as_path(),
    );
    seed_predecessor_commit(
        &mut driver_b,
        dir_b.join("state.json").as_path(),
        world_b.as_path(),
        dir_b.join("records").as_path(),
        dir_b.join("store").as_path(),
    );

    let context_a = commit_context_with_actions(2, vec![provider_bootstrap_action(2, input_a)]);
    let context_b = commit_context_with_actions(2, vec![provider_bootstrap_action(2, input_b)]);
    assert_eq!(
        context_a.action_root, context_b.action_root,
        "peer commit action roots must include identical replicated input bytes"
    );
    let result_a = driver_a.on_commit(context_a).expect("peer A commit");
    let result_b = driver_b.on_commit(context_b).expect("peer B commit");
    assert_eq!(
        result_a, result_b,
        "peer materializers must converge on execution roots"
    );

    let _ = fs::remove_dir_all(dir_a);
    let _ = fs::remove_dir_all(dir_b);
}

#[test]
fn provider_bootstrap_commit_root_survives_world_restart() {
    let dir = temp_dir("provider-bootstrap-root-restart");
    let state_path = dir.join("state.json");
    let world_dir = dir.join("world");
    let records_dir = dir.join("records");
    let storage_root = dir.join("store");
    let input = provider_bootstrap_fixture(world_dir.as_path());
    let mut driver = NodeRuntimeExecutionDriver::new(
        state_path.clone(),
        world_dir.clone(),
        records_dir.clone(),
        storage_root.clone(),
    )
    .expect("driver");
    seed_predecessor_commit(
        &mut driver,
        state_path.as_path(),
        world_dir.as_path(),
        records_dir.as_path(),
        storage_root.as_path(),
    );
    driver
        .on_commit(commit_context_with_actions(
            2,
            vec![provider_bootstrap_action(2, input)],
        ))
        .expect("canonical commit");

    let record = load_execution_bridge_record(
        execution_bridge_record_path(records_dir.as_path(), 2).as_path(),
    )
    .expect("record");
    let snapshot_ref = record.snapshot_ref.as_deref().expect("record snapshot ref");
    let snapshot_bytes = driver
        .execution_store
        .get_verified(snapshot_ref)
        .expect("record snapshot blob");
    assert_eq!(
        blake3_hex(snapshot_bytes.as_slice()),
        record.execution_state_root,
        "canonical record root must match its committed snapshot blob"
    );

    let restarted =
        NodeRuntimeExecutionDriver::new(state_path, world_dir, records_dir, storage_root)
            .expect("restart");
    assert_eq!(
        restarted.state.last_execution_state_root.as_deref(),
        Some(record.execution_state_root.as_str()),
        "restart head must retain the canonical committed root"
    );
    assert_eq!(
        restarted.execution_world.state().time,
        2,
        "restart must restore the committed tick"
    );

    let _ = fs::remove_dir_all(dir);
}
