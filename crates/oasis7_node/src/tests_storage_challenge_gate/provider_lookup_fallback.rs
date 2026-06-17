use super::*;

#[test]
fn runtime_replication_storage_challenge_gate_provider_lookup_timeout_fallback_wrong_blob_degrades()
{
    let dir = temp_dir("challenge-gate-provider-lookup-fallback-wrong");
    let world_id = "world-challenge-provider-lookup-fallback-wrong";
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", 121)],
    );

    let seed_config = NodeConfig::new("node-a", world_id, NodeRole::Sequencer)
        .expect("seed config")
        .with_tick_interval(Duration::from_millis(10))
        .expect("seed tick")
        .with_pos_config(pos_config.clone())
        .expect("seed pos config")
        .with_auto_attest_all_validators(true)
        .with_replication(signed_replication_config(dir.clone(), 121));
    let mut seed_runtime = with_noop_execution_hook(NodeRuntime::new(seed_config));
    seed_runtime.start().expect("start seed runtime");
    let seeded = wait_until(Instant::now() + Duration::from_secs(2), || {
        seed_runtime.snapshot().consensus.committed_height >= 6
    });
    assert!(seeded, "seed runtime did not build enough local commits");
    seed_runtime.stop().expect("stop seed runtime");

    let network_impl = Arc::new(ProviderLookupFailureConnectedPeerTrapNetwork::new());
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = network_impl.clone();
    let config = NodeConfig::new("node-a", world_id, NodeRole::Sequencer)
        .expect("config")
        .with_pos_config(pos_config)
        .expect("pos config")
        .with_replication(signed_replication_config(dir.clone(), 121));
    let handle = NodeReplicationNetworkHandle::new(Arc::clone(&network))
        .with_dht(Arc::new(ProviderLookupFailureDht))
        .with_local_provider_id("node-a");
    let endpoint =
        ReplicationNetworkEndpoint::new(&handle, world_id, false, &config.network_policy)
            .expect("endpoint");
    let mut engine = PosNodeEngine::new(&config).expect("engine");
    engine.committed_height = STORAGE_GATE_NETWORK_WARMUP_HEIGHT + 8;
    engine.network_committed_height = STORAGE_GATE_NETWORK_WARMUP_HEIGHT + 8;
    engine.peer_heads.insert(
        "observer-light-peer".to_string(),
        PeerCommittedHead {
            height: 1,
            block_hash: "observer-light-peer-head".to_string(),
            committed_at_ms: 1_234,
            observed_at_ms: 1_234,
            execution_block_hash: None,
            execution_state_root: None,
            action_root: empty_action_root(),
            public_key_hex: None,
            signature_hex: None,
        },
    );
    let replication = super::replication::ReplicationRuntime::new(
        &signed_replication_config(dir.clone(), 121),
        "node-a",
    )
    .expect("restart replication runtime");

    let gate_result = engine.enforce_storage_challenge_gate(
        &replication,
        Some(&endpoint),
        "node-a",
        world_id,
        1_234,
    );

    assert!(
        gate_result.is_ok(),
        "provider lookup fallback wrong blob should degrade, not hard-block consensus: {gate_result:?}"
    );
    assert!(
        network_impl.connected_peer_provider_attempts() > 0,
        "storage challenge should try bounded fallback after retryable provider lookup failure"
    );
    let snapshot = engine.snapshot_from_decision(&engine.idle_pending_decision().expect("decision"));
    assert!(
        snapshot
            .storage_challenge_network_degraded_reason
            .as_deref()
            .map(|reason| {
                reason.contains("provider lookup failed")
                    && reason.contains("fallback blob hash mismatch")
            })
            .unwrap_or(false),
        "expected provider lookup failure and fallback mismatch to remain observable as degraded, got {:?}",
        snapshot.storage_challenge_network_degraded_reason
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn runtime_replication_storage_challenge_gate_provider_lookup_timeout_fallback_correct_blob_matches()
{
    let dir = temp_dir("challenge-gate-provider-lookup-fallback-correct");
    let world_id = "world-challenge-provider-lookup-fallback-correct";
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", 124)],
    );

    let seed_config = NodeConfig::new("node-a", world_id, NodeRole::Sequencer)
        .expect("seed config")
        .with_tick_interval(Duration::from_millis(10))
        .expect("seed tick")
        .with_pos_config(pos_config.clone())
        .expect("seed pos config")
        .with_auto_attest_all_validators(true)
        .with_replication(signed_replication_config(dir.clone(), 124));
    let mut seed_runtime = with_noop_execution_hook(NodeRuntime::new(seed_config));
    seed_runtime.start().expect("start seed runtime");
    let seeded = wait_until(Instant::now() + Duration::from_secs(2), || {
        seed_runtime.snapshot().consensus.committed_height >= 6
    });
    assert!(seeded, "seed runtime did not build enough local commits");
    seed_runtime.stop().expect("stop seed runtime");

    let replication = super::replication::ReplicationRuntime::new(
        &signed_replication_config(dir.clone(), 124),
        "node-a",
    )
    .expect("restart replication runtime");
    let mut fallback_blobs = HashMap::<String, Vec<u8>>::new();
    for (_, content_hash) in replication
        .recent_replicated_content_refs(world_id, STORAGE_GATE_NETWORK_SAMPLES_PER_CHECK)
        .expect("load recent replicated refs")
    {
        let blob = replication
            .load_blob_by_hash(content_hash.as_str())
            .expect("load local blob")
            .expect("blob payload");
        fallback_blobs.insert(content_hash, blob);
    }

    let network_impl = Arc::new(ProviderLookupFailureGenericBlobNetwork::new(fallback_blobs));
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = network_impl.clone();
    let config = NodeConfig::new("node-a", world_id, NodeRole::Sequencer)
        .expect("config")
        .with_pos_config(pos_config)
        .expect("pos config")
        .with_replication(signed_replication_config(dir.clone(), 124));
    let handle = NodeReplicationNetworkHandle::new(Arc::clone(&network))
        .with_dht(Arc::new(ProviderLookupFailureDht))
        .with_local_provider_id("node-a");
    let endpoint =
        ReplicationNetworkEndpoint::new(&handle, world_id, false, &config.network_policy)
            .expect("endpoint");
    let mut engine = PosNodeEngine::new(&config).expect("engine");
    engine.committed_height = STORAGE_GATE_NETWORK_WARMUP_HEIGHT + 8;
    engine.network_committed_height = STORAGE_GATE_NETWORK_WARMUP_HEIGHT + 8;
    engine.peer_heads.insert(
        "observer-fallback-peer".to_string(),
        PeerCommittedHead {
            height: STORAGE_GATE_NETWORK_WARMUP_HEIGHT + 8,
            block_hash: "observer-fallback-peer-head".to_string(),
            committed_at_ms: 1_234,
            observed_at_ms: 1_234,
            execution_block_hash: None,
            execution_state_root: None,
            action_root: empty_action_root(),
            public_key_hex: None,
            signature_hex: None,
        },
    );

    let gate_result = engine.enforce_storage_challenge_gate(
        &replication,
        Some(&endpoint),
        "node-a",
        world_id,
        1_234,
    );

    assert!(
        gate_result.is_ok(),
        "provider lookup fallback correct blob should satisfy storage challenge: {gate_result:?}"
    );
    assert!(
        network_impl.generic_attempts() > 0,
        "storage challenge should use generic fallback after retryable provider lookup failure"
    );
    let snapshot = engine.snapshot_from_decision(&engine.idle_pending_decision().expect("decision"));
    assert_eq!(snapshot.storage_challenge_network_degraded_height, None);
    assert_eq!(snapshot.storage_challenge_network_degraded_reason, None);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn runtime_replication_storage_challenge_gate_provider_lookup_timeout_fallback_local_corruption_hard_fails(
) {
    let dir = temp_dir("challenge-gate-provider-lookup-fallback-local-corrupt");
    let world_id = "world-challenge-provider-lookup-fallback-local-corrupt";
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", 125)],
    );

    let seed_config = NodeConfig::new("node-a", world_id, NodeRole::Sequencer)
        .expect("seed config")
        .with_tick_interval(Duration::from_millis(10))
        .expect("seed tick")
        .with_pos_config(pos_config.clone())
        .expect("seed pos config")
        .with_auto_attest_all_validators(true)
        .with_replication(signed_replication_config(dir.clone(), 125));
    let mut seed_runtime = with_noop_execution_hook(NodeRuntime::new(seed_config));
    seed_runtime.start().expect("start seed runtime");
    let seeded = wait_until(Instant::now() + Duration::from_secs(2), || {
        seed_runtime.snapshot().consensus.committed_height >= 6
    });
    assert!(seeded, "seed runtime did not build enough local commits");
    seed_runtime.stop().expect("stop seed runtime");

    let replication = super::replication::ReplicationRuntime::new(
        &signed_replication_config(dir.clone(), 125),
        "node-a",
    )
    .expect("restart replication runtime");
    let mut fallback_blobs = HashMap::<String, Vec<u8>>::new();
    for (_, content_hash) in replication
        .recent_replicated_content_refs(world_id, STORAGE_GATE_NETWORK_SAMPLES_PER_CHECK)
        .expect("load recent replicated refs")
    {
        let blob = replication
            .load_blob_by_hash(content_hash.as_str())
            .expect("load local blob")
            .expect("blob payload");
        fallback_blobs.insert(content_hash, blob);
    }
    let store = LocalCasStore::new(dir.join("store"));
    for entry in fs::read_dir(store.blobs_dir()).expect("list blobs") {
        let entry = entry.expect("blob entry");
        if entry.file_type().expect("blob type").is_file() {
            fs::write(entry.path(), b"tampered-local-blob").expect("tamper blob");
        }
    }

    let network_impl = Arc::new(ProviderLookupFailureGenericBlobNetwork::new(fallback_blobs));
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = network_impl.clone();
    let config = NodeConfig::new("node-a", world_id, NodeRole::Sequencer)
        .expect("config")
        .with_pos_config(pos_config)
        .expect("pos config")
        .with_replication(signed_replication_config(dir.clone(), 125));
    let handle = NodeReplicationNetworkHandle::new(Arc::clone(&network))
        .with_dht(Arc::new(ProviderLookupFailureDht))
        .with_local_provider_id("node-a");
    let endpoint =
        ReplicationNetworkEndpoint::new(&handle, world_id, false, &config.network_policy)
            .expect("endpoint");
    let mut engine = PosNodeEngine::new(&config).expect("engine");
    engine.committed_height = STORAGE_GATE_NETWORK_WARMUP_HEIGHT + 8;
    engine.network_committed_height = STORAGE_GATE_NETWORK_WARMUP_HEIGHT + 8;
    engine.peer_heads.insert(
        "observer-fallback-peer".to_string(),
        PeerCommittedHead {
            height: STORAGE_GATE_NETWORK_WARMUP_HEIGHT + 8,
            block_hash: "observer-fallback-peer-head".to_string(),
            committed_at_ms: 1_234,
            observed_at_ms: 1_234,
            execution_block_hash: None,
            execution_state_root: None,
            action_root: empty_action_root(),
            public_key_hex: None,
            signature_hex: None,
        },
    );

    let gate_result = engine.enforce_storage_challenge_gate(
        &replication,
        Some(&endpoint),
        "node-a",
        world_id,
        1_234,
    );

    gate_result.expect_err("local CAS corruption should hard-fail");
    let _ = fs::remove_dir_all(&dir);
}
