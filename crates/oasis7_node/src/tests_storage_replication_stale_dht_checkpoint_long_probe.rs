#[test]
fn fresh_observer_package_probe_does_not_reenter_height_one_after_300s_without_checkpoint_receipt()
{
    let _nonce_lock = lock_checkpoint_probe_nonce();
    let _probe_nonce = CheckpointProbeNonceGuard::install();
    let world_id = "world-live-package-probe-300s-height-one-reentry";
    let dir_a = temp_dir("live-package-probe-300s-height-one-reentry-a");
    let dir_b = temp_dir("live-package-probe-300s-height-one-reentry-b");
    let (_, public_key_a) = deterministic_keypair_hex(207);
    let (_, public_key_c) = deterministic_keypair_hex(208);
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![
            PosValidator {
                validator_id: "node-a".to_string(),
                stake: 50,
            },
            PosValidator {
                validator_id: "node-c".to_string(),
                stake: 50,
            },
        ],
        &[("node-a", 207), ("node-c", 208)],
    );
    let replication_config_a = signed_replication_config(dir_a.clone(), 207)
        .with_remote_writer_allowlist(vec![deterministic_keypair_hex(209).1])
        .expect("allowlist a")
        .with_fetch_requester_allowlist(vec![deterministic_keypair_hex(209).1])
        .expect("fetch allowlist a");
    let replication_config_b = signed_replication_config(dir_b.clone(), 209)
        .with_remote_writer_allowlist(vec![public_key_a.clone(), public_key_c.clone()])
        .expect("allowlist b")
        .with_fetch_requester_allowlist(vec![public_key_a.clone(), public_key_c.clone()])
        .expect("fetch allowlist b");
    let config_a = NodeConfig::new("node-a", world_id, NodeRole::Sequencer)
        .expect("config a")
        .with_pos_config(pos_config.clone())
        .expect("pos config a")
        .with_replication(replication_config_a);
    let config_b = NodeConfig::new("node-b", world_id, NodeRole::Observer)
        .expect("config b")
        .with_pos_config(pos_config)
        .expect("pos config b")
        .with_require_execution_on_commit(true)
        .with_replication(replication_config_b.clone());
    let mut replication_a =
        ReplicationRuntime::new(config_a.replication.as_ref().expect("repl a"), "node-a")
            .expect("runtime a");
    let height_one = replication_a
        .build_local_commit_message(
            "node-a",
            world_id,
            20_100,
            &committed_decision(1),
            Some("peer-exec-block-1"),
            Some("peer-exec-state-1"),
        )
        .expect("build delayed height-one candidate")
        .expect("candidate");
    let checkpoint_height = 43_340;
    let head = Arc::new(Mutex::new(super::replication::FetchHeadResponse {
        found: true,
        head: Some(super::replication::ReplicationHeadSummary {
            world_id: world_id.to_string(),
            height: 1,
            block_hash: "block-1".to_string(),
            state_root: "peer-exec-state-1".to_string(),
            timestamp_ms: 20_100,
        }),
    }));
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(PeerHeadCheckpointNetwork {
        inner: Arc::new(TestInMemoryNetwork::default()),
        fetch_protocols: Arc::new(Mutex::new(Vec::new())),
        head: Arc::clone(&head),
        checkpoint_fetch_available: Arc::new(AtomicBool::new(false)),
        checkpoint_fetch_not_found: Arc::new(AtomicBool::new(true)),
        connected_peer_ids: vec!["node-a".to_string(), "node-c".to_string()],
    });
    network
        .register_handler(
            REPLICATION_FETCH_COMMIT_PROTOCOL,
            Box::new(|_| {
                serde_json::to_vec(&super::replication::FetchCommitResponse {
                    found: false,
                    message: None,
                })
                .map_err(|err| WorldError::DistributedValidationFailed {
                    reason: format!("encode absent checkpoint response failed: {err}"),
                })
            }),
        )
        .expect("register absent checkpoint response");
    let handle_b = NodeReplicationNetworkHandle::new(Arc::clone(&network));
    let mut endpoint_b =
        ReplicationNetworkEndpoint::new(&handle_b, world_id, true, &config_b.network_policy)
            .expect("fresh observer endpoint");
    let mut replication_b =
        ReplicationRuntime::new(config_b.replication.as_ref().expect("repl b"), "node-b")
            .expect("fresh observer runtime");
    let mut engine_b = PosNodeEngine::new(&config_b).expect("fresh observer engine");
    let high_head = |node_id: &str, public_key_hex: String| PeerCommittedHead {
        height: checkpoint_height,
        block_hash: format!("block-{checkpoint_height}"),
        committed_at_ms: 20_164,
        observed_at_ms: 20_200,
        execution_block_hash: Some(format!("exec-block-{checkpoint_height}")),
        execution_state_root: Some(format!("exec-state-{checkpoint_height}")),
        action_root: empty_action_root(),
        public_key_hex: Some(public_key_hex),
        signature_hex: Some(format!("signed-{node_id}-{checkpoint_height}")),
    };
    engine_b.peer_heads.insert(
        "node-a".to_string(),
        high_head("node-a", deterministic_keypair_hex(207).1),
    );
    engine_b.peer_heads.insert(
        "node-c".to_string(),
        high_head("node-c", deterministic_keypair_hex(208).1),
    );
    assert_eq!(engine_b.peer_heads.len(), 2);
    let mut execution_hook = PackageProbeHeightOneFailureHook {
        incremental_commits: Vec::new(),
        rollback_heights: Vec::new(),
    };
    macro_rules! package_tick {
        ($now_ms:expr) => {
            engine_b.tick(
                "node-b",
                world_id,
                $now_ms,
                None,
                Some(&mut replication_b),
                Some(&mut endpoint_b),
                None,
                Vec::new(),
                Some(&mut execution_hook),
            )
        };
    }
    package_tick!(20_300).expect("initial probe must hold height zero");
    package_tick!(320_300).expect("300s probe must hold height zero");
    assert_eq!(engine_b.committed_height, 0);
    assert!(!dir_b
        .join("checkpoint-verification")
        .join(format!("{checkpoint_height}.json"))
        .exists());
    endpoint_b
        .publish_replication(&height_one)
        .expect("publish delayed candidate");
    let result = package_tick!(320_301);
    assert!(result.is_ok(), "height-one candidate must remain deferred: result={result:?} incremental={:?} rollback={:?}", execution_hook.incremental_commits, execution_hook.rollback_heights);
    assert_eq!(
        result
            .expect("height-zero snapshot")
            .consensus_snapshot
            .committed_height,
        0
    );
    assert!(execution_hook.incremental_commits.is_empty());
    assert!(execution_hook.rollback_heights.is_empty());
    endpoint_b
        .publish_replication(&height_one)
        .expect("republish delayed candidate");
    let result = package_tick!(320_302);
    assert!(
        result.is_ok(),
        "reentry candidate must remain deferred: result={result:?} incremental={:?} rollback={:?}",
        execution_hook.incremental_commits,
        execution_hook.rollback_heights
    );
    assert_eq!(
        result
            .expect("height-zero reentry snapshot")
            .consensus_snapshot
            .committed_height,
        0
    );
    assert!(execution_hook.incremental_commits.is_empty());
    assert!(execution_hook.rollback_heights.is_empty());
    let _ = fs::remove_dir_all(&dir_a);
    let _ = fs::remove_dir_all(&dir_b);
}

#[test]
fn testnet_249_connected_high_heads_without_cache_reenter_height_one_after_probe_window() {
    let _nonce_lock = lock_checkpoint_probe_nonce();
    let _probe_nonce = CheckpointProbeNonceGuard::install();
    let world_id = "world-testnet-249-connected-high-heads-without-cache";
    let dir_a = temp_dir("testnet-249-connected-high-heads-a");
    let dir_b = temp_dir("testnet-249-connected-high-heads-b");
    let dir_c = temp_dir("testnet-249-connected-high-heads-c");
    let (_, public_key_a) = deterministic_keypair_hex(249);
    let (_, public_key_b) = deterministic_keypair_hex(250);
    let (_, public_key_c) = deterministic_keypair_hex(251);
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![
            PosValidator { validator_id: "node-a".to_string(), stake: 50 },
            PosValidator { validator_id: "node-c".to_string(), stake: 50 },
        ],
        &[("node-a", 249), ("node-c", 251)],
    );
    let replication_config_a = signed_replication_config(dir_a.clone(), 249)
        .with_remote_writer_allowlist(vec![public_key_b.clone()])
        .expect("allowlist a")
        .with_fetch_requester_allowlist(vec![public_key_b.clone()])
        .expect("fetch allowlist a");
    let replication_config_b = signed_replication_config(dir_b.clone(), 250)
        .with_remote_writer_allowlist(vec![public_key_a.clone(), public_key_c.clone()])
        .expect("allowlist b")
        .with_fetch_requester_allowlist(vec![public_key_a.clone(), public_key_c.clone()])
        .expect("fetch allowlist b");
    let replication_config_c = signed_replication_config(dir_c.clone(), 251)
        .with_remote_writer_allowlist(vec![public_key_b.clone()])
        .expect("allowlist c")
        .with_fetch_requester_allowlist(vec![public_key_b.clone()])
        .expect("fetch allowlist c");
    let config_a = NodeConfig::new("node-a", world_id, NodeRole::Sequencer)
        .expect("config a")
        .with_pos_config(pos_config.clone())
        .expect("pos config a")
        .with_replication(replication_config_a);
    let config_b = NodeConfig::new("node-b", world_id, NodeRole::Observer)
        .expect("config b")
        .with_pos_config(pos_config.clone())
        .expect("pos config b")
        .with_require_execution_on_commit(true)
        .with_replication(replication_config_b.clone());
    let config_c = NodeConfig::new("node-c", world_id, NodeRole::Sequencer)
        .expect("config c")
        .with_pos_config(pos_config)
        .expect("pos config c")
        .with_replication(replication_config_c);
    let mut replication_a = ReplicationRuntime::new(
        config_a.replication.as_ref().expect("repl a"), "node-a",
    )
    .expect("runtime a");
    let mut replication_c = ReplicationRuntime::new(
        config_c.replication.as_ref().expect("repl c"), "node-c",
    )
    .expect("runtime c");
    let height_one_a = replication_a
        .build_local_commit_message(
            "node-a", world_id, 249_100, &committed_decision(1),
            Some("peer-exec-block-1"), Some("peer-exec-state-1"),
        )
        .expect("build node-a height-one candidate")
        .expect("node-a height-one candidate");
    let height_one_c = replication_c
        .build_local_commit_message(
            "node-c", world_id, 249_101,
            &PosDecision { proposer_id: "node-c".to_string(), ..committed_decision(1) },
            Some("peer-exec-block-1"), Some("peer-exec-state-1"),
        )
        .expect("build node-c height-one candidate")
        .expect("node-c height-one candidate");
    let head = Arc::new(Mutex::new(super::replication::FetchHeadResponse {
        found: true,
        head: Some(super::replication::ReplicationHeadSummary {
            world_id: world_id.to_string(),
            height: 1,
            block_hash: "block-1".to_string(),
            state_root: "peer-exec-state-1".to_string(),
            timestamp_ms: 249_100,
        }),
    }));
    let network: Arc<dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync> =
        Arc::new(PeerHeadCheckpointNetwork {
            inner: Arc::new(TestInMemoryNetwork::default()),
            fetch_protocols: Arc::new(Mutex::new(Vec::new())),
            head,
            checkpoint_fetch_available: Arc::new(AtomicBool::new(false)),
            checkpoint_fetch_not_found: Arc::new(AtomicBool::new(true)),
            connected_peer_ids: vec!["node-a".to_string(), "node-c".to_string()],
        });
    network
        .register_handler(REPLICATION_FETCH_COMMIT_PROTOCOL, Box::new(|_| {
            serde_json::to_vec(&super::replication::FetchCommitResponse {
                found: false,
                message: None,
            })
            .map_err(|err| WorldError::DistributedValidationFailed {
                reason: format!("encode absent checkpoint response failed: {err}"),
            })
        }))
        .expect("register absent checkpoint response");
    let handle_b = NodeReplicationNetworkHandle::new(Arc::clone(&network));
    let mut endpoint_b = ReplicationNetworkEndpoint::new(
        &handle_b, world_id, true, &config_b.network_policy,
    )
    .expect("fresh observer endpoint");
    assert_eq!(endpoint_b.connected_peer_ids(), vec!["node-a", "node-c"]);
    let mut replication_b = ReplicationRuntime::new(
        config_b.replication.as_ref().expect("repl b"), "node-b",
    )
    .expect("fresh observer runtime");
    let mut engine_b = PosNodeEngine::new(&config_b).expect("fresh observer engine");
    assert!(engine_b.peer_heads.is_empty());
    let mut execution_hook = PackageProbeHeightOneFailureHook {
        incremental_commits: Vec::new(),
        rollback_heights: Vec::new(),
    };
    macro_rules! package_tick {
        ($now_ms:expr) => {
            engine_b.tick(
                "node-b", world_id, $now_ms, None, Some(&mut replication_b),
                Some(&mut endpoint_b), None, Vec::new(), Some(&mut execution_hook),
            )
        };
    }
    package_tick!(249_300).expect("initial stale world head must hold height zero");
    package_tick!(549_300).expect("300s unavailable checkpoint probe must hold height zero");
    assert_eq!(engine_b.committed_height, 0);
    assert!(execution_hook.incremental_commits.is_empty());
    assert!(execution_hook.rollback_heights.is_empty());
    endpoint_b
        .publish_replication(&height_one_a)
        .expect("publish first height-one candidate after probe window");
    let first_result = package_tick!(549_301);
    assert!(first_result.is_ok(), "first candidate should remain queued: {first_result:?}");
    endpoint_b
        .publish_replication(&height_one_c)
        .expect("publish second height-one candidate after probe window");
    let result = package_tick!(549_302);
    assert!(
        result.is_ok(),
        "post-merge testnet.249 observer must remain fail-closed after two candidates; live bug signature: result={result:?} incremental={:?} rollback={:?}",
        execution_hook.incremental_commits,
        execution_hook.rollback_heights
    );
    let snapshot = result.expect("height-zero post-merge snapshot");
    assert_eq!(snapshot.consensus_snapshot.committed_height, 0);
    assert!(execution_hook.incremental_commits.is_empty());
    assert!(execution_hook.rollback_heights.is_empty());
    assert_eq!(engine_b.committed_height, 0);
    assert_eq!(engine_b.replication_persisted_height, 0);
    let _ = fs::remove_dir_all(&dir_a);
    let _ = fs::remove_dir_all(&dir_b);
    let _ = fs::remove_dir_all(&dir_c);
}
