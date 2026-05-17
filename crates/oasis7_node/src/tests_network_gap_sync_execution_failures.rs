use std::fs;

use super::*;

struct FailExecutionHook;

impl NodeExecutionHook for FailExecutionHook {
    fn on_commit(
        &mut self,
        context: NodeExecutionCommitContext,
    ) -> Result<NodeExecutionCommitResult, String> {
        Err(format!(
            "forced execution failure at height {}",
            context.height
        ))
    }
}

#[test]
fn successor_probe_does_not_advance_replication_cursor_when_execution_fails() {
    let dir_remote = temp_dir("successor-probe-execution-fail-remote");
    let dir_local = temp_dir("successor-probe-execution-fail-local");
    let world_id = "world-successor-probe-execution-fail";
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(TestInMemoryNetwork::default());
    let (_, remote_public_key_hex) = deterministic_keypair_hex(150);
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", 150)],
    );
    let local_replication_config = signed_replication_config(dir_local.clone(), 151)
        .with_remote_writer_allowlist(vec![remote_public_key_hex])
        .expect("local remote writer allowlist");
    let config = NodeConfig::new("node-b", world_id, NodeRole::Sequencer)
        .expect("config")
        .with_tick_interval(Duration::from_millis(10))
        .expect("tick")
        .with_pos_config(pos_config)
        .expect("pos config")
        .with_replication(local_replication_config.clone());
    let handle = NodeReplicationNetworkHandle::new(Arc::clone(&network));
    let endpoint =
        ReplicationNetworkEndpoint::new(&handle, world_id, false, &config.network_policy)
            .expect("endpoint");
    let mut remote_replication = super::replication::ReplicationRuntime::new(
        &signed_replication_config(dir_remote.clone(), 150),
        "node-a",
    )
    .expect("remote replication runtime");
    let decision = PosDecision {
        height: 1,
        slot: 0,
        epoch: 0,
        status: PosConsensusStatus::Committed,
        block_hash: "block-1".to_string(),
        action_root: empty_action_root(),
        committed_actions: Vec::new(),
        approved_stake: 100,
        rejected_stake: 0,
        required_stake: 67,
        total_stake: 100,
    };
    let message = remote_replication
        .build_local_commit_message("node-a", world_id, 1_000, &decision, None, None)
        .expect("build local commit message")
        .expect("commit payload");
    let expected_hash = message.record.content_hash.clone();
    let expected_blob = message.payload.clone();
    network
        .register_handler(
            super::replication::REPLICATION_FETCH_COMMIT_PROTOCOL,
            Box::new({
                let message = message.clone();
                move |_payload| {
                    serde_json::to_vec(&super::replication::FetchCommitResponse {
                        found: true,
                        message: Some(message.clone()),
                    })
                    .map_err(|err| WorldError::DistributedValidationFailed {
                        reason: format!("encode fetch commit response failed: {err}"),
                    })
                }
            }),
        )
        .expect("register fetch commit handler");
    network
        .register_handler(
            super::replication::REPLICATION_FETCH_BLOB_PROTOCOL,
            Box::new(move |payload| {
                let request =
                    serde_json::from_slice::<super::replication::FetchBlobRequest>(payload)
                        .map_err(|err| WorldError::DistributedValidationFailed {
                            reason: format!("decode fetch blob request failed: {err}"),
                        })?;
                serde_json::to_vec(&super::replication::FetchBlobResponse {
                    found: request.content_hash == expected_hash,
                    blob: (request.content_hash == expected_hash).then(|| expected_blob.clone()),
                })
                .map_err(|err| WorldError::DistributedValidationFailed {
                    reason: format!("encode fetch blob response failed: {err}"),
                })
            }),
        )
        .expect("register fetch blob handler");

    let mut engine = PosNodeEngine::new(&config).expect("engine");
    let mut replication =
        super::replication::ReplicationRuntime::new(&local_replication_config, "node-b")
            .expect("local replication runtime");
    let mut hook = FailExecutionHook;
    let err = engine
        .maybe_hold_proposal_for_replication_successor_probe(
            &endpoint,
            "node-b",
            world_id,
            1_000,
            Some(&mut replication),
            Some(&mut hook),
        )
        .expect_err("probe should surface execution failure");
    assert!(
        matches!(err, NodeError::Execution { ref reason } if reason.contains("forced execution failure at height 1")),
        "unexpected probe error: {err:?}"
    );
    assert_eq!(engine.replication_persisted_height, 0);
    assert_eq!(engine.committed_height, 0);
    assert_eq!(
        replication
            .latest_persisted_commit_height(world_id)
            .expect("latest persisted height after failed probe"),
        0
    );
    assert!(replication
        .load_commit_message_by_height(world_id, 1)
        .expect("load persisted commit after failed probe")
        .is_none());
    let reopened_replication =
        super::replication::ReplicationRuntime::new(&local_replication_config, "node-b")
            .expect("reopen local replication runtime");
    assert_eq!(
        reopened_replication
            .latest_persisted_commit_height(world_id)
            .expect("reopened latest persisted height after failed probe"),
        0
    );
    let retry_err = engine
        .maybe_hold_proposal_for_replication_successor_probe(
            &endpoint,
            "node-b",
            world_id,
            1_001,
            Some(&mut replication),
            Some(&mut hook),
        )
        .expect_err("probe retry should still surface execution failure");
    assert!(
        matches!(retry_err, NodeError::Execution { ref reason } if reason.contains("forced execution failure at height 1")),
        "unexpected probe retry error: {retry_err:?}"
    );

    let _ = fs::remove_dir_all(&dir_remote);
    let _ = fs::remove_dir_all(&dir_local);
}

#[test]
fn gap_sync_does_not_advance_replication_cursor_when_execution_fails() {
    let dir_remote = temp_dir("gap-sync-execution-fail-remote");
    let dir_local = temp_dir("gap-sync-execution-fail-local");
    let world_id = "world-gap-sync-execution-fail";
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(TestInMemoryNetwork::default());
    let (_, remote_public_key_hex) = deterministic_keypair_hex(152);
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", 152)],
    );
    let local_replication_config = signed_replication_config(dir_local.clone(), 153)
        .with_remote_writer_allowlist(vec![remote_public_key_hex])
        .expect("local remote writer allowlist");
    let config = NodeConfig::new("node-b", world_id, NodeRole::Sequencer)
        .expect("config")
        .with_tick_interval(Duration::from_millis(10))
        .expect("tick")
        .with_pos_config(pos_config)
        .expect("pos config")
        .with_replication(local_replication_config.clone());
    let handle = NodeReplicationNetworkHandle::new(Arc::clone(&network));
    let endpoint =
        ReplicationNetworkEndpoint::new(&handle, world_id, false, &config.network_policy)
            .expect("endpoint");
    let mut remote_replication = super::replication::ReplicationRuntime::new(
        &signed_replication_config(dir_remote.clone(), 152),
        "node-a",
    )
    .expect("remote replication runtime");
    let decision = PosDecision {
        height: 1,
        slot: 0,
        epoch: 0,
        status: PosConsensusStatus::Committed,
        block_hash: "block-1".to_string(),
        action_root: empty_action_root(),
        committed_actions: Vec::new(),
        approved_stake: 100,
        rejected_stake: 0,
        required_stake: 67,
        total_stake: 100,
    };
    let message = remote_replication
        .build_local_commit_message("node-a", world_id, 1_000, &decision, None, None)
        .expect("build local commit message")
        .expect("commit payload");
    let expected_hash = message.record.content_hash.clone();
    let expected_blob = message.payload.clone();
    network
        .register_handler(
            super::replication::REPLICATION_FETCH_COMMIT_PROTOCOL,
            Box::new({
                let message = message.clone();
                move |_payload| {
                    serde_json::to_vec(&super::replication::FetchCommitResponse {
                        found: true,
                        message: Some(message.clone()),
                    })
                    .map_err(|err| WorldError::DistributedValidationFailed {
                        reason: format!("encode fetch commit response failed: {err}"),
                    })
                }
            }),
        )
        .expect("register fetch commit handler");
    network
        .register_handler(
            super::replication::REPLICATION_FETCH_BLOB_PROTOCOL,
            Box::new(move |payload| {
                let request =
                    serde_json::from_slice::<super::replication::FetchBlobRequest>(payload)
                        .map_err(|err| WorldError::DistributedValidationFailed {
                            reason: format!("decode fetch blob request failed: {err}"),
                        })?;
                serde_json::to_vec(&super::replication::FetchBlobResponse {
                    found: request.content_hash == expected_hash,
                    blob: (request.content_hash == expected_hash).then(|| expected_blob.clone()),
                })
                .map_err(|err| WorldError::DistributedValidationFailed {
                    reason: format!("encode fetch blob response failed: {err}"),
                })
            }),
        )
        .expect("register fetch blob handler");

    let mut engine = PosNodeEngine::new(&config).expect("engine");
    engine.network_committed_height = 1;
    let mut replication =
        super::replication::ReplicationRuntime::new(&local_replication_config, "node-b")
            .expect("local replication runtime");
    let mut hook = FailExecutionHook;
    let err = engine
        .sync_missing_replication_commits(
            &endpoint,
            "node-b",
            world_id,
            Some(&mut replication),
            Some(&mut hook),
        )
        .expect_err("gap sync should surface execution failure");
    assert!(
        matches!(err, NodeError::Execution { ref reason } if reason.contains("forced execution failure at height 1")),
        "unexpected gap sync error: {err:?}"
    );
    assert_eq!(engine.replication_persisted_height, 0);
    assert_eq!(engine.committed_height, 0);
    assert_eq!(
        replication
            .latest_persisted_commit_height(world_id)
            .expect("latest persisted height after failed gap sync"),
        0
    );
    assert!(replication
        .load_commit_message_by_height(world_id, 1)
        .expect("load persisted commit after failed gap sync")
        .is_none());
    let reopened_replication =
        super::replication::ReplicationRuntime::new(&local_replication_config, "node-b")
            .expect("reopen local replication runtime");
    assert_eq!(
        reopened_replication
            .latest_persisted_commit_height(world_id)
            .expect("reopened latest persisted height after failed gap sync"),
        0
    );
    let retry_err = engine
        .sync_missing_replication_commits(
            &endpoint,
            "node-b",
            world_id,
            Some(&mut replication),
            Some(&mut hook),
        )
        .expect_err("gap sync retry should still surface execution failure");
    assert!(
        matches!(retry_err, NodeError::Execution { ref reason } if reason.contains("forced execution failure at height 1")),
        "unexpected gap sync retry error: {retry_err:?}"
    );

    let _ = fs::remove_dir_all(&dir_remote);
    let _ = fs::remove_dir_all(&dir_local);
}
