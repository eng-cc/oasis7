use super::*;

struct CheckpointInstallingExecutionHook {
    installed: Vec<u64>,
    sequential_commits: Vec<u64>,
}

impl NodeExecutionHook for CheckpointInstallingExecutionHook {
    fn on_commit(
        &mut self,
        context: NodeExecutionCommitContext,
    ) -> Result<NodeExecutionCommitResult, String> {
        self.sequential_commits.push(context.height);
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

struct BoundaryCheckpointExportHook {
    bundle: NodeExecutionCheckpointBundle,
}

impl NodeExecutionHook for BoundaryCheckpointExportHook {
    fn on_commit(
        &mut self,
        context: NodeExecutionCommitContext,
    ) -> Result<NodeExecutionCommitResult, String> {
        Ok(NodeExecutionCommitResult {
            execution_height: context.height,
            execution_block_hash: format!("exec-block-{}", context.height),
            execution_state_root: format!("exec-state-{}", context.height),
        })
    }

    fn export_checkpoint_bundle(
        &mut self,
        height: u64,
    ) -> Result<Option<NodeExecutionCheckpointBundle>, String> {
        Ok((self.bundle.height == height).then(|| self.bundle.clone()))
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
fn high_replication_checkpoint_candidates_prefer_release_retained_boundaries() {
    let candidates = PosNodeEngine::high_replication_checkpoint_candidates(16_715, 64);

    assert_eq!(
        &candidates[..4],
        &[16_715, 16_704, 16_640, 16_576],
        "the immediate 64-height boundary should follow the advertised head before older retained checkpoints"
    );
    let retained_index = candidates
        .iter()
        .position(|height| *height == 16_640)
        .expect("retained boundary candidate");
    let fallback_index = candidates
        .iter()
        .position(|height| *height == 16_672)
        .expect("32-height fallback boundary candidate");
    assert!(
        retained_index < fallback_index,
        "retained boundary should be preferred over nearer 32-height fallback boundary; candidates={candidates:?}"
    );
}

#[test]
fn high_replication_checkpoint_candidates_include_latest_aligned_checkpoint_after_live_head() {
    let candidates = PosNodeEngine::high_replication_checkpoint_candidates(18_574, 0);
    let latest_aligned_checkpoint = 18_560;
    let older_checkpoint = 18_496;

    assert_eq!(candidates.first().copied(), Some(18_574));
    let latest_index = candidates
        .iter()
        .position(|height| *height == latest_aligned_checkpoint)
        .expect("fresh observers must probe the latest completed checkpoint boundary");
    let older_index = candidates
        .iter()
        .position(|height| *height == older_checkpoint)
        .expect("older retained checkpoint boundary");
    assert!(
        latest_index < older_index,
        "the latest completed checkpoint must precede older fallbacks; candidates={candidates:?}"
    );
}

#[test]
fn fetch_commit_boundary_does_not_rescan_unrelated_retained_records_per_request() {
    let world_id = "world-fetch-commit-boundary-no-rescan";
    let dir = temp_dir("fetch-commit-boundary-no-rescan");
    let replication_config = signed_replication_config(dir.clone(), 249);
    let mut replication =
        ReplicationRuntime::new(&replication_config, "node-storage").expect("runtime");
    let boundary_height = 64;
    let decision = PosDecision {
        height: boundary_height,
        slot: boundary_height,
        epoch: 0,
        proposer_id: "node-storage".to_string(),
        status: PosConsensusStatus::Committed,
        block_hash: format!("block-{boundary_height}"),
        action_root: empty_action_root(),
        committed_actions: Vec::new(),
        approved_stake: 100,
        rejected_stake: 0,
        required_stake: 67,
        total_stake: 100,
    };
    let message = replication
        .build_local_commit_message(
            "node-storage",
            world_id,
            8_064,
            &decision,
            Some("exec-block-64"),
            Some("exec-state-64"),
        )
        .expect("build boundary message")
        .expect("boundary message");
    let unrelated_path = dir
        .join("replication_commit_messages")
        .join(format!("{:020}.json", 1));
    std::fs::write(&unrelated_path, b"unreadable-retained-record")
        .expect("write unrelated unreadable retained record");
    let execution_hook: Arc<Mutex<Box<dyn NodeExecutionHook>>> = Arc::new(Mutex::new(Box::new(
        BoundaryCheckpointExportHook {
            bundle: test_execution_checkpoint_bundle(
                boundary_height,
                "exec-block-64",
                "exec-state-64",
            ),
        },
    )));

    let served = attach_checkpoint_for_fetch_commit_if_boundary(
        Some(message),
        Some(&execution_hook),
        dir.as_path(),
        world_id,
        "node-storage",
        &replication_config,
        boundary_height,
    )
    .expect("boundary FetchCommit should not rescan unrelated retained records")
    .expect("boundary FetchCommit message");
    let payload = parse_replication_commit_payload(served.payload.as_slice())
        .expect("served boundary payload");
    assert!(
        payload.execution_checkpoint.is_some(),
        "valid boundary response must retain its exported checkpoint"
    );
    let _ = std::fs::remove_dir_all(&dir);
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
fn high_replication_checkpoint_probe_continues_after_fetch_commit_route_unavailable() {
    let err = NodeError::Replication {
        reason:
            "replication network route unavailable: /aw/node/replication/fetch-commit/1.0.0"
                .to_string(),
    };

    assert!(PosNodeEngine::high_replication_checkpoint_probe_can_continue(
        &err
    ));
}

#[test]
fn high_replication_checkpoint_probe_continues_after_missing_checkpoint_blob() {
    let blob_err = NodeError::Replication {
        reason: "execution checkpoint blob not found hash=missing-blob".to_string(),
    };
    let commit_blob_err = NodeError::Replication {
        reason: "gap sync height 16640 blob not found for hash missing-commit-payload"
            .to_string(),
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
    let rate_limited_blob = NodeError::Replication {
        reason:
            "replication network request failed: kind=rate_limited protocol=/aw/node/replication/fetch-blob/1.0.0 detail=fetch-blob response budget exhausted; retry after window reset"
                .to_string(),
    };

    assert!(PosNodeEngine::high_replication_checkpoint_probe_can_continue(
        &blob_err
    ));
    assert!(PosNodeEngine::high_replication_checkpoint_probe_can_continue(
        &commit_blob_err
    ));
    assert!(!PosNodeEngine::high_replication_checkpoint_probe_can_continue(
        &execution_err
    ));
    assert!(!PosNodeEngine::high_replication_checkpoint_probe_can_continue(
        &protocol_omitted_blob_timeout
    ));
    assert!(PosNodeEngine::high_replication_checkpoint_probe_can_continue(
        &rate_limited_blob
    ));
}

#[test]
fn storage_rehydrates_and_resigns_existing_checkpoint_descriptor_with_local_closure() {
    let world_id = "world-existing-checkpoint-descriptor-storage-refresh";
    let source_dir = temp_dir("existing-checkpoint-descriptor-source");
    let storage_dir = temp_dir("existing-checkpoint-descriptor-storage");
    let (source_private, source_public) = deterministic_keypair_hex(254);
    let (storage_private, storage_public) = deterministic_keypair_hex(255);
    let source_config = NodeReplicationConfig::new(&source_dir)
        .expect("source config")
        .with_signing_keypair(source_private, source_public)
        .expect("source signer");
    let storage_config = NodeReplicationConfig::new(&storage_dir)
        .expect("storage config")
        .with_signing_keypair(storage_private, storage_public.clone())
        .expect("storage signer");
    let mut source = ReplicationRuntime::new(&source_config, "node-source").expect("source runtime");
    let mut storage =
        ReplicationRuntime::new(&storage_config, "node-storage").expect("storage runtime");
    let height = 64;
    let decision = PosDecision {
        height,
        slot: height,
        epoch: 0,
        proposer_id: "node-source".to_string(),
        status: PosConsensusStatus::Committed,
        block_hash: format!("block-{height}"),
        action_root: empty_action_root(),
        committed_actions: Vec::new(),
        approved_stake: 100,
        rejected_stake: 0,
        required_stake: 67,
        total_stake: 100,
    };
    let message = source
        .build_local_commit_message(
            "node-source",
            world_id,
            8_000,
            &decision,
            Some("exec-block-64"),
            Some("exec-state-64"),
        )
        .expect("build source message")
        .expect("source message");
    let checkpoint = test_execution_checkpoint_bundle(height, "exec-block-64", "exec-state-64");
    let source_descriptor_message = source
        .attach_execution_checkpoint_descriptor_to_message("node-source", &message, &checkpoint)
        .expect("source attaches checkpoint descriptor");

    let refreshed = storage
        .attach_execution_checkpoint_descriptor_to_message(
            "node-storage",
            &source_descriptor_message,
            &checkpoint,
        )
        .expect("storage refreshes its local checkpoint closure");
    let payload = crate::replication_state_reconcile::parse_replication_commit_payload(
        refreshed.payload.as_slice(),
    )
    .expect("refreshed checkpoint payload");
    let descriptor = payload
        .execution_checkpoint
        .expect("refreshed message retains checkpoint descriptor");

    assert_eq!(refreshed.record.writer_id, storage_public);
    assert!(refreshed.signature_hex.is_some());
    assert!(
        storage
            .load_execution_checkpoint_bundle(&descriptor)
            .expect("load storage checkpoint closure")
            .is_some(),
        "storage must persist the complete local checkpoint closure before serving its refreshed descriptor"
    );

    let _ = fs::remove_dir_all(&source_dir);
    let _ = fs::remove_dir_all(&storage_dir);
}

#[test]
fn full_storage_publishes_execution_checkpoint_blob_providers() {
    let world_id = "world-checkpoint-provider-publish";
    let dir = temp_dir("checkpoint-provider-publish");
    let (_, public_key) = deterministic_keypair_hex(252);
    let config = NodeConfig::new("node-storage", world_id, NodeRole::Storage)
        .expect("config")
        .with_replication(
            signed_replication_config(dir.clone(), 252)
                .with_fetch_requester_allowlist(vec![public_key])
                .expect("fetch allowlist"),
        );
    let replication =
        ReplicationRuntime::new(config.replication.as_ref().expect("replication"), "node-storage")
            .expect("runtime");
    let bundle = test_execution_checkpoint_bundle(64, "exec-block-64", "exec-state-64");
    let manifest_hash = oasis7_distfs::blake3_hex(bundle.manifest_json.as_slice());
    let blob_hash = bundle
        .blobs
        .first()
        .expect("checkpoint blob")
        .content_hash
        .clone();
    let descriptor = replication
        .store_execution_checkpoint_bundle(&bundle)
        .expect("store checkpoint");
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(TestInMemoryNetwork::default());
    let dht = Arc::new(TestReplicaMaintenanceDht::new(
        "remote-provider",
        "storage-provider",
    ));
    let handle = NodeReplicationNetworkHandle::new(Arc::clone(&network))
        .with_dht(dht.clone())
        .with_local_provider_id("storage-provider");
    let endpoint =
        ReplicationNetworkEndpoint::new(&handle, world_id, false, &config.network_policy)
            .expect("endpoint");

    PosNodeEngine::publish_execution_checkpoint_descriptor_providers(
        &endpoint,
        world_id,
        &replication,
        &descriptor,
    )
    .expect("publish checkpoint providers");

    let published = dht.published_records();
    for hash in [manifest_hash, blob_hash] {
        assert!(
            published.iter().any(
                |(published_world, published_hash, provider)| published_world == world_id
                    && published_hash == &hash
                    && provider == "storage-provider"
            ),
            "expected checkpoint content hash={hash} to be published, published={published:?}"
        );
    }

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn checkpoint_provider_publication_rejects_partial_closure_before_advertising() {
    let world_id = "world-checkpoint-provider-partial-closure";
    let dir = temp_dir("checkpoint-provider-partial-closure");
    let config = NodeConfig::new("node-storage", world_id, NodeRole::Storage)
        .expect("config")
        .with_replication(signed_replication_config(dir.clone(), 246));
    let replication =
        ReplicationRuntime::new(config.replication.as_ref().expect("replication"), "node-storage")
            .expect("runtime");
    let bundle = test_execution_checkpoint_bundle(64, "exec-block-64", "exec-state-64");
    let descriptor = replication
        .store_execution_checkpoint_bundle(&bundle)
        .expect("store checkpoint");
    let missing_hash = descriptor
        .blobs
        .first()
        .expect("checkpoint blob ref")
        .content_hash
        .clone();
    fs::remove_file(
        dir.join("store")
            .join("blobs")
            .join(format!("{missing_hash}.blob")),
    )
    .expect("remove one closure blob");

    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(TestInMemoryNetwork::default());
    let dht = Arc::new(TestReplicaMaintenanceDht::new(
        "remote-provider",
        "storage-provider",
    ));
    let handle = NodeReplicationNetworkHandle::new(Arc::clone(&network))
        .with_dht(dht.clone())
        .with_local_provider_id("storage-provider");
    let endpoint =
        ReplicationNetworkEndpoint::new(&handle, world_id, false, &config.network_policy)
            .expect("endpoint");

    let err = PosNodeEngine::publish_execution_checkpoint_descriptor_providers(
        &endpoint,
        world_id,
        &replication,
        &descriptor,
    )
    .expect_err("partial closure must not be advertised");

    assert!(
        err.to_string()
            .contains("checkpoint provider publication closure incomplete hash="),
        "partial closure error must name the missing hash: {err}",
    );
    assert!(
        dht.published_records().is_empty(),
        "publication must be all-or-nothing when any closure member is absent",
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn fetch_commit_handler_publishes_commit_payload_provider_to_dht() {
    let world_id = "world-fetch-commit-provider-publish";
    let dir = temp_dir("fetch-commit-provider-publish");
    let (_, public_key) = deterministic_keypair_hex(197);
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", 197)],
    );
    let config = NodeConfig::new("node-a", world_id, NodeRole::Sequencer)
        .expect("config")
        .with_pos_config(pos_config)
        .expect("pos config")
        .with_replication(
            signed_replication_config(dir.clone(), 197)
                .with_fetch_requester_allowlist(vec![public_key])
                .expect("fetch allowlist"),
        );
    let mut replication =
        ReplicationRuntime::new(config.replication.as_ref().expect("replication"), "node-a")
            .expect("runtime");
    let decision = PosDecision {
        height: 1,
        slot: 1,
        epoch: 0,
        proposer_id: "node-a".to_string(),
        status: PosConsensusStatus::Committed,
        block_hash: "block-1".to_string(),
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
            7_000,
            &decision,
            Some("exec-block-1"),
            Some("exec-state-1"),
        )
        .expect("build local message")
        .expect("message");
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(TestInMemoryNetwork::default());
    let dht = Arc::new(TestReplicaMaintenanceDht::new("remote-provider", "peer-seq"));
    let handle = NodeReplicationNetworkHandle::new(Arc::clone(&network))
        .with_dht(dht.clone())
        .with_local_provider_id("peer-seq");
    register_replication_fetch_handlers(
        &handle,
        config.replication.as_ref().expect("replication"),
        world_id,
        &config.network_policy,
    )
    .expect("register fetch handlers");

    let request = signed_fetch_commit_request_for_test(world_id, 1, 197);
    let response_payload = network
        .request(
            REPLICATION_FETCH_COMMIT_PROTOCOL,
            serde_json::to_vec(&request).expect("encode request").as_slice(),
        )
        .expect("fetch commit");
    let response: super::replication::FetchCommitResponse =
        serde_json::from_slice(&response_payload).expect("decode response");

    assert!(response.found, "fetch-commit should serve local commit");
    let published = wait_until(Instant::now() + Duration::from_secs(2), || {
        dht.published_records().iter().any(
            |(published_world, content_hash, provider_id)| published_world == world_id
                && content_hash == &message.record.content_hash
                && provider_id == "peer-seq",
        )
    });
    assert!(
        published,
        "expected fetch-commit handler to publish provider for {}, got {:?}",
        message.record.content_hash,
        dht.published_records()
    );
    let _ = fs::remove_dir_all(&dir);
}

#[derive(Clone, Default)]
struct BlockingCheckpointProviderDht {
    entered_publish: Arc<(Mutex<bool>, Condvar)>,
    release_publish: Arc<(Mutex<bool>, Condvar)>,
}

impl BlockingCheckpointProviderDht {
    fn wait_until_publish_entered(&self, timeout: Duration) -> bool {
        let deadline = std::time::Instant::now() + timeout;
        let (lock, cvar) = &*self.entered_publish;
        let mut entered = lock.lock().expect("lock entered_publish");
        while !*entered {
            let Some(remaining) = deadline.checked_duration_since(std::time::Instant::now())
            else {
                return false;
            };
            let (next_entered, _) = cvar
                .wait_timeout(entered, remaining)
                .expect("wait entered_publish");
            entered = next_entered;
        }
        true
    }

    fn release(&self) {
        let (lock, cvar) = &*self.release_publish;
        *lock.lock().expect("lock release_publish") = true;
        cvar.notify_all();
    }
}

impl proto_dht::DistributedDht<WorldError> for BlockingCheckpointProviderDht {
    fn publish_provider(
        &self,
        _world_id: &str,
        _content_hash: &str,
        _provider_id: &str,
    ) -> Result<(), WorldError> {
        let (entered_lock, entered_cvar) = &*self.entered_publish;
        *entered_lock.lock().expect("lock entered_publish") = true;
        entered_cvar.notify_all();

        let (release_lock, release_cvar) = &*self.release_publish;
        let mut released = release_lock.lock().expect("lock release_publish");
        while !*released {
            released = release_cvar
                .wait(released)
                .expect("wait release_publish");
        }
        Ok(())
    }

    fn publish_provider_best_effort(
        &self,
        _world_id: &str,
        _content_hash: &str,
        _provider_id: &str,
    ) -> Result<(), WorldError> {
        let (entered_lock, entered_cvar) = &*self.entered_publish;
        *entered_lock.lock().expect("lock entered_publish") = true;
        entered_cvar.notify_all();
        Ok(())
    }

    fn get_providers(
        &self,
        _world_id: &str,
        _content_hash: &str,
    ) -> Result<Vec<ProviderRecord>, WorldError> {
        Ok(Vec::new())
    }

    fn put_world_head(&self, _world_id: &str, _head: &WorldHeadAnnounce) -> Result<(), WorldError> {
        Ok(())
    }

    fn get_world_head(&self, _world_id: &str) -> Result<Option<WorldHeadAnnounce>, WorldError> {
        Ok(None)
    }

    fn put_membership_directory(
        &self,
        _world_id: &str,
        _snapshot: &MembershipDirectorySnapshot,
    ) -> Result<(), WorldError> {
        Ok(())
    }

    fn get_membership_directory(
        &self,
        _world_id: &str,
    ) -> Result<Option<MembershipDirectorySnapshot>, WorldError> {
        Ok(None)
    }

    fn put_peer_record(&self, _world_id: &str, _record: &SignedPeerRecord) -> Result<(), WorldError> {
        Ok(())
    }

    fn get_peer_record(
        &self,
        _world_id: &str,
        _peer_id: &str,
    ) -> Result<Option<SignedPeerRecord>, WorldError> {
        Ok(None)
    }
}

#[test]
fn fetch_commit_handler_does_not_block_on_checkpoint_provider_publish() {
    let world_id = "world-fetch-commit-provider-publish-nonblocking";
    let dir = temp_dir("fetch-commit-provider-publish-nonblocking");
    let (_, public_key) = deterministic_keypair_hex(253);
    let replication_config = signed_replication_config(dir.clone(), 253)
        .with_fetch_requester_allowlist(vec![public_key])
        .expect("fetch allowlist");
    let config = NodeConfig::new("node-storage", world_id, NodeRole::Storage)
        .expect("config")
        .with_replication(replication_config.clone());
    let mut replication =
        ReplicationRuntime::new(config.replication.as_ref().expect("replication"), "node-storage")
            .expect("runtime");
    let height = 64;
    let decision = PosDecision {
        height,
        slot: height,
        epoch: 0,
            proposer_id: "node-a".to_string(),
        status: PosConsensusStatus::Committed,
        block_hash: "block-64".to_string(),
        action_root: empty_action_root(),
        committed_actions: Vec::new(),
        approved_stake: 100,
        rejected_stake: 0,
        required_stake: 67,
        total_stake: 100,
    };
    replication
        .build_local_commit_message_with_checkpoint(
            "node-storage",
            world_id,
            7_064,
            &decision,
            Some("exec-block-64"),
            Some("exec-state-64"),
            Some(test_execution_checkpoint_bundle(
                height,
                "exec-block-64",
                "exec-state-64",
            )),
        )
        .expect("build checkpoint message")
        .expect("message");

    let network_impl = Arc::new(TestInMemoryNetwork::default());
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = network_impl.clone();
    let dht = Arc::new(BlockingCheckpointProviderDht::default());
    let handle = NodeReplicationNetworkHandle::new(Arc::clone(&network))
        .with_dht(dht.clone())
        .with_local_provider_id("storage-provider");
    register_replication_fetch_handlers_with_checkpoint_export(
        &handle,
        &replication_config,
        "node-storage",
        world_id,
        &config.network_policy,
        None,
    )
    .expect("register fetch handlers");

    let request = replication
        .build_fetch_commit_request(world_id, height)
        .expect("fetch request");
    let payload = serde_json::to_vec(&request).expect("encode request");
    let started_at = std::time::Instant::now();
    let response = network
        .request(REPLICATION_FETCH_COMMIT_PROTOCOL, payload.as_slice())
        .expect("fetch response");

    assert!(
        started_at.elapsed() < Duration::from_millis(200),
        "fetch-commit handler should return before checkpoint provider publish completes"
    );
    let decoded: FetchCommitResponse = serde_json::from_slice(response.as_slice())
        .expect("decode fetch response");
    assert!(decoded.found);
    assert!(
        dht.wait_until_publish_entered(Duration::from_secs(1)),
        "checkpoint provider publish should still be scheduled in the background"
    );
    dht.release();

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn observer_gap_sync_recovers_local_state_block_with_retained_checkpoint() {
    let _nonce_lock =
        super::storage_replication_first_ready_checkpoint_tests::lock_checkpoint_probe_nonce();
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
    let mut provider_hashes = Vec::<String>::new();
    for height in [64_u64, 300_u64] {
        let decision = PosDecision {
            height,
            slot: height,
            epoch: 0,
            proposer_id: "node-a".to_string(),
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
        if let Some(bundle) = checkpoint.as_ref() {
            provider_hashes.push(oasis7_distfs::blake3_hex(bundle.manifest_json.as_slice()));
            provider_hashes.extend(
                bundle
                    .blobs
                    .iter()
                    .map(|blob| blob.content_hash.clone()),
            );
        }
        let message = replication_a
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
        provider_hashes.push(message.record.content_hash);
    }

    let network_impl = Arc::new(ProviderAwareTestNetwork::new(
        dir_a.clone(),
        "node-a-provider",
    ));
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = network_impl.clone();
    register_replication_fetch_handlers(
        &NodeReplicationNetworkHandle::new(Arc::clone(&network)),
        &replication_config_a,
        world_id,
        &config_a.network_policy,
    )
    .expect("register fetch handlers");
    let dht = Arc::new(TestReplicaMaintenanceDht::new(
        "node-a-provider",
        "node-b-provider",
    ));
    for hash in &provider_hashes {
        dht.seed_provider(hash, "node-a-provider");
    }
    let handle_b = NodeReplicationNetworkHandle::new(Arc::clone(&network))
        .with_dht(dht.clone())
        .with_local_provider_id("node-b-provider");
    let endpoint_b =
        ReplicationNetworkEndpoint::new(&handle_b, world_id, false, &config_b.network_policy)
            .expect("endpoint b");
    let mut replication_b =
        ReplicationRuntime::new(config_b.replication.as_ref().expect("repl b"), "node-b")
            .expect("runtime b");
    let mut engine_b = PosNodeEngine::new(&config_b).expect("engine b");
    engine_b.network_committed_height = 300;
    engine_b.last_replication_gap_sync_blocked_height = Some(1);
    engine_b.last_replication_gap_sync_blocked_reason = Some(
        "replication gap sync local state blocked: deterministic execution/state failure at height 1: execution driver peer mismatch"
            .to_string(),
    );
    let mut execution_hook = CheckpointInstallingExecutionHook {
        installed: Vec::new(),
        sequential_commits: Vec::new(),
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
    assert!(
        execution_hook.sequential_commits.is_empty(),
        "deterministic local-state block must still prevent sequential replay"
    );
    assert_eq!(engine_b.committed_height, 64);
    assert_eq!(engine_b.replication_persisted_height, 64);
    assert_eq!(engine_b.last_execution_height, 64);
    assert_eq!(
        engine_b.execution_binding_for_height(64),
        Some(("exec-block-64", "exec-state-64"))
    );
    assert_eq!(engine_b.last_replication_gap_sync_blocked_height, None);
    assert_eq!(engine_b.last_replication_gap_sync_blocked_reason, None);
    let provider_attempts = network_impl.provider_attempts();
    assert!(
        provider_attempts.iter().any(|providers| providers
            .iter()
            .any(|provider| provider == "node-a-provider")),
        "expected retained checkpoint sync to route blob fetches through DHT providers, attempts={provider_attempts:?}"
    );
    let _ = fs::remove_dir_all(&dir_a);
    let _ = fs::remove_dir_all(&dir_b);
}
