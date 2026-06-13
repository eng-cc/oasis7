use super::*;

struct CheckpointInstallingExecutionHook {
    installed: Vec<u64>,
}

impl NodeExecutionHook for CheckpointInstallingExecutionHook {
    fn on_commit(
        &mut self,
        context: NodeExecutionCommitContext,
    ) -> Result<NodeExecutionCommitResult, String> {
        Err(format!(
            "{} at height {}",
            EXECUTION_MISSING_PREDECESSOR_RECORD_SIGNATURE, context.height
        ))
    }

    fn install_checkpoint_bundle(
        &mut self,
        context: NodeExecutionCheckpointInstallContext,
        bundle: NodeExecutionCheckpointBundle,
    ) -> Result<NodeExecutionCommitResult, String> {
        if bundle.height != context.height
            || bundle.execution_block_hash != context.execution_block_hash
            || bundle.execution_state_root != context.execution_state_root
        {
            return Err("checkpoint bundle mismatch".to_string());
        }
        self.installed.push(context.height);
        Ok(NodeExecutionCommitResult {
            execution_height: context.height,
            execution_block_hash: context.execution_block_hash,
            execution_state_root: context.execution_state_root,
        })
    }
}

fn test_execution_checkpoint_bundle(
    height: u64,
    execution_block_hash: &str,
    execution_state_root: &str,
) -> NodeExecutionCheckpointBundle {
    let snapshot_bytes =
        format!("checkpoint-snapshot-{height}-{execution_state_root}").into_bytes();
    NodeExecutionCheckpointBundle {
        height,
        execution_block_hash: execution_block_hash.to_string(),
        execution_state_root: execution_state_root.to_string(),
        manifest_json: br#"{"test":"manifest"}"#.to_vec(),
        blobs: vec![NodeExecutionCheckpointBlob {
            content_hash: oasis7_distfs::blake3_hex(snapshot_bytes.as_slice()),
            bytes: snapshot_bytes,
        }],
    }
}

#[test]
fn high_replication_checkpoint_candidates_cover_release_default_retained_window() {
    let candidates = PosNodeEngine::high_replication_checkpoint_candidates(16_715, 0);

    assert_eq!(candidates.first().copied(), Some(16_715));
    for height in [16_640, 16_576, 16_512, 16_448, 16_384, 16_320, 16_256, 16_192] {
        assert!(
            candidates.contains(&height),
            "missing release_default retained checkpoint candidate {height}; candidates={candidates:?}"
        );
    }
}

#[test]
fn high_replication_checkpoint_probe_continues_after_fetch_commit_timeout() {
    let err = NodeError::Replication {
        reason:
            "replication network error: NetworkProtocolUnavailable { protocol: \"libp2p-replication outbound request failed: request failed: Timeout /aw/node/replication/fetch-commit/1.0.0\" }"
                .to_string(),
    };

    assert!(PosNodeEngine::high_replication_checkpoint_probe_can_continue(&err));
}

#[test]
fn high_replication_checkpoint_probe_does_not_continue_after_blob_or_execution_errors() {
    let blob_err = NodeError::Replication {
        reason: "execution checkpoint blob not found hash=missing-blob".to_string(),
    };
    let execution_err = NodeError::Execution {
        reason: "execution checkpoint install returned mismatched binding at height 16640"
            .to_string(),
    };
    let protocol_omitted_blob_timeout = NodeError::Replication {
        reason:
            "replication network availability gap: libp2p-replication outbound request failed: NetworkProtocolUnavailable { protocol: \"request failed: Timeout\" }"
                .to_string(),
    };

    assert!(!PosNodeEngine::high_replication_checkpoint_probe_can_continue(
        &blob_err
    ));
    assert!(!PosNodeEngine::high_replication_checkpoint_probe_can_continue(
        &execution_err
    ));
    assert!(!PosNodeEngine::high_replication_checkpoint_probe_can_continue(
        &protocol_omitted_blob_timeout
    ));
}

#[test]
fn observer_gap_sync_probes_retained_checkpoint_window_below_non_checkpoint_head() {
    let world_id = "world-gap-sync-retained-window-below-head";
    let dir_a = temp_dir("gap-sync-retained-window-below-head-a");
    let dir_b = temp_dir("gap-sync-retained-window-below-head-b");
    let (_, public_key_a) = deterministic_keypair_hex(250);
    let (_, public_key_b) = deterministic_keypair_hex(251);
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", 250)],
    );
    let replication_config_a = signed_replication_config(dir_a.clone(), 250)
        .with_remote_writer_allowlist(vec![public_key_b.clone()])
        .expect("allowlist a")
        .with_fetch_requester_allowlist(vec![public_key_b])
        .expect("fetch allowlist a");
    let replication_config_b = signed_replication_config(dir_b.clone(), 251)
        .with_remote_writer_allowlist(vec![public_key_a.clone()])
        .expect("allowlist b")
        .with_fetch_requester_allowlist(vec![public_key_a])
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
        .with_replication(replication_config_b.clone());

    let mut replication_a =
        ReplicationRuntime::new(config_a.replication.as_ref().expect("repl a"), "node-a")
            .expect("runtime a");
    for height in [64_u64, 300_u64] {
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
        let execution_block_hash = format!("exec-block-{height}");
        let execution_state_root = format!("exec-state-{height}");
        let checkpoint = (height == 64).then(|| {
            test_execution_checkpoint_bundle(height, &execution_block_hash, &execution_state_root)
        });
        replication_a
            .build_local_commit_message_with_checkpoint(
                "node-a",
                world_id,
                7_000 + i64::try_from(height).expect("height fits i64"),
                &decision,
                Some(execution_block_hash.as_str()),
                Some(execution_state_root.as_str()),
                checkpoint,
            )
            .expect("build local message")
            .expect("message");
    }

    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(TestInMemoryNetwork::default());
    register_replication_fetch_handlers(
        &NodeReplicationNetworkHandle::new(Arc::clone(&network)),
        &replication_config_a,
        world_id,
        &config_a.network_policy,
    )
    .expect("register fetch handlers");
    let handle_b = NodeReplicationNetworkHandle::new(Arc::clone(&network));
    let endpoint_b =
        ReplicationNetworkEndpoint::new(&handle_b, world_id, false, &config_b.network_policy)
            .expect("endpoint b");
    let mut replication_b =
        ReplicationRuntime::new(config_b.replication.as_ref().expect("repl b"), "node-b")
            .expect("runtime b");
    let mut engine_b = PosNodeEngine::new(&config_b).expect("engine b");
    engine_b.network_committed_height = 300;
    let mut execution_hook = CheckpointInstallingExecutionHook {
        installed: Vec::new(),
    };

    engine_b
        .sync_missing_replication_commits(
            &endpoint_b,
            "node-b",
            world_id,
            Some(&mut replication_b),
            Some(&mut execution_hook),
        )
        .expect("retained-window checkpoint gap sync");

    assert_eq!(execution_hook.installed, vec![64]);
    assert_eq!(engine_b.committed_height, 64);
    assert_eq!(engine_b.replication_persisted_height, 64);
    assert_eq!(engine_b.last_execution_height, 64);
    assert_eq!(
        engine_b.execution_binding_for_height(64),
        Some(("exec-block-64", "exec-state-64"))
    );
    assert_eq!(engine_b.last_replication_gap_sync_blocked_height, None);

    let _ = fs::remove_dir_all(&dir_a);
    let _ = fs::remove_dir_all(&dir_b);
}
