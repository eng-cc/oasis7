use std::fs;
use std::sync::{Arc, Mutex};

use oasis7_proto::distributed_checkpoint_lineage::CheckpointLineageHeadV1;

use super::*;
use super::storage_replication_first_ready_checkpoint_tests::{
    checkpoint_bundle, committed_decision, BootstrapBeforeIncrementalHook,
    FirstReadyHeadCheckpointNetwork, CheckpointProbeNonceGuard, lock_checkpoint_probe_nonce,
};

#[test]
fn fresh_observer_execution_mismatch_falls_back_to_authenticated_retained_checkpoint() {
    let _nonce_lock = lock_checkpoint_probe_nonce();
    let _probe_nonce = CheckpointProbeNonceGuard::install();
    let world_id = "world-fresh-observer-execution-mismatch-checkpoint-fallback";
    let dir_a = temp_dir("fresh-observer-execution-mismatch-fallback-a");
    let dir_b = temp_dir("fresh-observer-execution-mismatch-fallback-b");
    let (_, public_key_a) = deterministic_keypair_hex(192);
    let (_, public_key_b) = deterministic_keypair_hex(193);
    let checkpoint_height = REPLICATION_GAP_SYNC_MAX_HEIGHTS_PER_POLL;
    let peer_head_height = checkpoint_height + 36;
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", 192)],
    );
    let replication_config_a = signed_replication_config(dir_a.clone(), 192)
        .with_remote_writer_allowlist(vec![public_key_b.clone()])
        .expect("allowlist a")
        .with_fetch_requester_allowlist(vec![public_key_b.clone()])
        .expect("fetch allowlist a");
    let replication_config_b = signed_replication_config(dir_b.clone(), 193)
        .with_remote_writer_allowlist(vec![public_key_a.clone()])
        .expect("allowlist b")
        .with_fetch_requester_allowlist(vec![public_key_a.clone()])
        .expect("fetch allowlist b");
    let config_a = NodeConfig::new("node-a", world_id, NodeRole::Sequencer)
        .expect("config a")
        .with_pos_config(pos_config.clone())
        .expect("pos config a")
        .with_replication(replication_config_a.clone());
    let config_b = NodeConfig::new("node-b", world_id, NodeRole::Observer)
        .expect("config b")
        .with_pos_config(pos_config)
        .expect("pos config b")
        .with_require_execution_on_commit(true)
        .with_replication(replication_config_b.clone());
    let mut replication_a =
        ReplicationRuntime::new(config_a.replication.as_ref().expect("repl a"), "node-a")
            .expect("remote replication runtime");
    replication_a
        .build_local_commit_message(
            "node-a",
            world_id,
            7_100,
            &committed_decision(1),
            Some("peer-execution-block-1"),
            Some("peer-execution-state-1"),
        )
        .expect("build valid height-one commit")
        .expect("height-one commit");
    let checkpoint_execution_block_hash = format!("execution-block-{checkpoint_height}");
    let checkpoint_execution_state_root = format!("execution-state-{checkpoint_height}");
    replication_a
        .build_local_commit_message_with_checkpoint(
            "node-a",
            world_id,
            7_164,
            &committed_decision(checkpoint_height),
            Some(checkpoint_execution_block_hash.as_str()),
            Some(checkpoint_execution_state_root.as_str()),
            Some(checkpoint_bundle(
                checkpoint_height,
                checkpoint_execution_block_hash.as_str(),
                checkpoint_execution_state_root.as_str(),
            )),
        )
        .expect("build retained checkpoint commit")
        .expect("retained checkpoint commit");
    let engine_a = PosNodeEngine::new(&config_a).expect("lineage authority engine");
    let peer_head_block_hash = format!("block-{peer_head_height}");
    let peer_head_execution_block_hash = format!("execution-block-{peer_head_height}");
    let peer_head_execution_state_root = format!("execution-state-{peer_head_height}");
    let checkpoint_head = CheckpointLineageHeadV1 {
        height: peer_head_height,
        block_hash: peer_head_block_hash.clone(),
        state_root: peer_head_execution_state_root.clone(),
        execution_block_hash: peer_head_execution_block_hash.clone(),
        execution_state_root: peer_head_execution_state_root.clone(),
    };
    let lineage_envelope = super::storage_replication_live_retained_boundary_tests::
        attach_production_lineage_envelope(
            &mut replication_a,
            world_id,
            checkpoint_height,
            checkpoint_head.clone(),
            &[&engine_a],
        );
    assert_eq!(lineage_envelope.head, checkpoint_head);
    let persisted_checkpoint = replication_a
        .load_commit_message_by_height(world_id, checkpoint_height)
        .expect("load retained checkpoint source")
        .expect("retained checkpoint source");
    assert!(
        parse_replication_commit_payload(persisted_checkpoint.payload.as_slice())
            .expect("decode retained checkpoint source")
            .lineage_envelope
            .is_some(),
        "the fallback fixture must retain the signed lineage sidecar"
    );

    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(FirstReadyHeadCheckpointNetwork {
        inner: Arc::new(TestInMemoryNetwork::default()),
        fetch_protocols: Arc::new(Mutex::new(Vec::new())),
    });
    register_replication_fetch_handlers(
        &NodeReplicationNetworkHandle::new(Arc::clone(&network)),
        &replication_config_a,
        world_id,
        &config_a.network_policy,
    )
    .expect("register authenticated checkpoint provider");
    let handle_b = NodeReplicationNetworkHandle::new(Arc::clone(&network));
    let endpoint_b =
        ReplicationNetworkEndpoint::new(&handle_b, world_id, true, &config_b.network_policy)
            .expect("fresh observer endpoint");
    let mut replication_b =
        ReplicationRuntime::new(config_b.replication.as_ref().expect("repl b"), "node-b")
            .expect("fresh observer replication runtime");
    let mut engine_b = PosNodeEngine::new(&config_b).expect("fresh observer engine");
    // Model the probe's stale low network cursor plus an authenticated high
    // peer head.  The current implementation enters height-one replay from
    // this state and returns the mismatch before trying the retained boundary.
    engine_b.network_committed_height = 1;
    engine_b.peer_heads.insert(
        "node-a".to_string(),
        PeerCommittedHead {
            height: peer_head_height,
            block_hash: peer_head_block_hash,
            committed_at_ms: 7_164,
            observed_at_ms: 7_200,
            execution_block_hash: Some(peer_head_execution_block_hash),
            execution_state_root: Some(peer_head_execution_state_root),
            action_root: empty_action_root(),
            public_key_hex: Some(public_key_a),
            signature_hex: Some(format!("signed-node-a-{peer_head_height}")),
        },
    );
    let mut execution_hook = BootstrapBeforeIncrementalHook {
        installed: Vec::new(),
        incremental_commits: Vec::new(),
        rollback_heights: Vec::new(),
    };

    // Keep the valid height-one commit available and make its clean-local
    // execution result disagree with the signed peer binding.  Height zero
    // rollback is deliberately unavailable; the expected recovery is the
    // authenticated retained checkpoint closure above.
    let result = engine_b.sync_missing_replication_commits(
        &endpoint_b,
        "node-b",
        world_id,
        Some(&mut replication_b),
        Some(&mut execution_hook),
    );
    assert!(
        result.is_ok(),
        "fresh observer must recover through the retained checkpoint after height-one execution mismatch: result={result:?} installed={:?} incremental={:?} rollback={:?}",
        execution_hook.installed,
        execution_hook.incremental_commits,
        execution_hook.rollback_heights
    );
    assert_eq!(execution_hook.incremental_commits, vec![1]);
    assert_eq!(execution_hook.rollback_heights, vec![0]);
    assert_eq!(execution_hook.installed, vec![checkpoint_height]);
    assert_eq!(engine_b.committed_height, checkpoint_height);
    assert_eq!(engine_b.replication_persisted_height, checkpoint_height);
    assert_eq!(engine_b.last_execution_height, checkpoint_height);
    assert!(
        replication_b
            .load_commit_message_by_height(world_id, 1)
            .expect("inspect height-one persistence")
            .is_none(),
        "mismatched height-one commit must not be persisted before checkpoint recovery"
    );
    assert!(
        replication_b
            .load_commit_message_by_height(world_id, checkpoint_height)
            .expect("inspect checkpoint persistence")
            .is_some(),
        "verified retained checkpoint must be persisted after fallback"
    );
    assert!(
        dir_b
            .join("checkpoint-verification")
            .join(format!("{checkpoint_height}.json"))
            .exists(),
        "checkpoint probe receipt must be retained"
    );
    let _ = fs::remove_dir_all(&dir_a);
    let _ = fs::remove_dir_all(&dir_b);
}
