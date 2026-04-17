use super::*;

fn build_storage_challenge_blob_cache_fixture(
    seed: u8,
    world_id: &str,
    dir: &std::path::Path,
) -> (
    NodeConfig,
    Arc<dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync>,
    Arc<Mutex<usize>>,
    ReplicationRuntime,
    ReplicationNetworkEndpoint,
) {
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", seed)],
    );
    let mut replication = super::replication::ReplicationRuntime::new(
        &signed_replication_config(dir.to_path_buf(), seed),
        "node-a",
    )
    .expect("open local replication runtime");
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
    let first_message = replication
        .build_local_commit_message("node-a", world_id, 1_000, &decision, None, None)
        .expect("build first commit")
        .expect("first commit payload");
    let blob = replication
        .load_blob_by_hash(first_message.record.content_hash.as_str())
        .expect("load first blob")
        .expect("first blob payload");

    let dht = Arc::new(TestReplicaMaintenanceDht::new(
        "storage-provider-1",
        "node-a",
    ));
    dht.seed_provider(
        first_message.record.content_hash.as_str(),
        "storage-provider-1",
    );

    let request_count = Arc::new(Mutex::new(0usize));
    let request_count_for_handler = Arc::clone(&request_count);
    let expected_hash = first_message.record.content_hash.clone();
    let expected_blob = blob.clone();
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(TestInMemoryNetwork::default());
    network
        .register_handler(
            super::replication::REPLICATION_FETCH_BLOB_PROTOCOL,
            Box::new(move |payload| {
                let request =
                    serde_json::from_slice::<super::replication::FetchBlobRequest>(payload)
                        .map_err(|err| WorldError::DistributedValidationFailed {
                            reason: format!("decode fetch blob request failed: {err}"),
                        })?;
                *request_count_for_handler
                    .lock()
                    .expect("lock request count") += 1;
                let response = super::replication::FetchBlobResponse {
                    found: request.content_hash == expected_hash,
                    blob: (request.content_hash == expected_hash).then(|| expected_blob.clone()),
                };
                serde_json::to_vec(&response).map_err(|err| {
                    WorldError::DistributedValidationFailed {
                        reason: format!("encode fetch blob response failed: {err}"),
                    }
                })
            }),
        )
        .expect("register blob handler");

    let config = NodeConfig::new("node-a", world_id, NodeRole::Sequencer)
        .expect("config")
        .with_pos_config(pos_config)
        .expect("pos config")
        .with_replication(signed_replication_config(dir.to_path_buf(), seed));
    let handle = NodeReplicationNetworkHandle::new(Arc::clone(&network))
        .with_dht(dht)
        .with_local_provider_id("node-a");
    let leaked_handle: &'static NodeReplicationNetworkHandle = Box::leak(Box::new(handle));
    let endpoint =
        ReplicationNetworkEndpoint::new(leaked_handle, world_id, false, &config.network_policy)
            .expect("endpoint");

    (config, network, request_count, replication, endpoint)
}

#[test]
fn storage_challenge_gate_reuses_recent_blob_success_cache() {
    let dir = temp_dir("storage-challenge-success-cache-hit");
    let world_id = "world-storage-challenge-success-cache-hit";
    let (config, _network, request_count, replication, endpoint) =
        build_storage_challenge_blob_cache_fixture(117, world_id, dir.as_path());

    let mut engine = PosNodeEngine::new(&config).expect("engine");
    engine.committed_height = STORAGE_GATE_NETWORK_WARMUP_HEIGHT + 8;
    engine.network_committed_height = STORAGE_GATE_NETWORK_WARMUP_HEIGHT + 8;
    engine.peer_heads.insert(
        "storage-provider-1".to_string(),
        PeerCommittedHead {
            height: STORAGE_GATE_NETWORK_WARMUP_HEIGHT + 8,
            block_hash: "cached-peer-head".to_string(),
            committed_at_ms: 1_234,
            execution_block_hash: None,
            execution_state_root: None,
        },
    );

    engine
        .enforce_storage_challenge_gate(&replication, Some(&endpoint), "node-a", world_id, 1_234)
        .expect("first gate");
    engine
        .enforce_storage_challenge_gate(&replication, Some(&endpoint), "node-a", world_id, 1_235)
        .expect("second gate");

    assert_eq!(
        *request_count.lock().expect("lock request count"),
        1,
        "second gate should reuse recent success cache instead of refetching the same blob"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn storage_challenge_gate_reuses_recent_blob_success_cache_expires() {
    let dir = temp_dir("storage-challenge-success-cache-expiry");
    let world_id = "world-storage-challenge-success-cache-expiry";
    let (config, _network, request_count, replication, endpoint) =
        build_storage_challenge_blob_cache_fixture(118, world_id, dir.as_path());

    let mut engine = PosNodeEngine::new(&config).expect("engine");
    engine.committed_height = STORAGE_GATE_NETWORK_WARMUP_HEIGHT + 8;
    engine.network_committed_height = STORAGE_GATE_NETWORK_WARMUP_HEIGHT + 8;
    engine.peer_heads.insert(
        "storage-provider-1".to_string(),
        PeerCommittedHead {
            height: STORAGE_GATE_NETWORK_WARMUP_HEIGHT + 8,
            block_hash: "expiry-peer-head".to_string(),
            committed_at_ms: 1_234,
            execution_block_hash: None,
            execution_state_root: None,
        },
    );

    engine
        .enforce_storage_challenge_gate(&replication, Some(&endpoint), "node-a", world_id, 1_234)
        .expect("first gate");
    engine.committed_height += STORAGE_CHALLENGE_SUCCESS_CACHE_MAX_AGE_HEIGHTS + 1;
    engine.network_committed_height = engine.committed_height;
    engine
        .enforce_storage_challenge_gate(&replication, Some(&endpoint), "node-a", world_id, 1_235)
        .expect("second gate after cache expiry");

    assert_eq!(
        *request_count.lock().expect("lock request count"),
        2,
        "expired success cache should trigger a fresh network probe"
    );
    let _ = fs::remove_dir_all(&dir);
}
