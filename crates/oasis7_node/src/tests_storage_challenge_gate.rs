use super::*;

#[test]
fn runtime_replication_storage_challenge_gate_blocks_on_local_probe_failure() {
    let dir = temp_dir("challenge-gate-local");
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", 83)],
    );
    let config = NodeConfig::new("node-a", "world-challenge-local", NodeRole::Sequencer)
        .expect("config")
        .with_tick_interval(Duration::from_millis(10))
        .expect("tick")
        .with_pos_config(pos_config)
        .expect("pos config")
        .with_auto_attest_all_validators(true)
        .with_replication(signed_replication_config(dir.clone(), 83));
    let mut runtime = with_noop_execution_hook(NodeRuntime::new(config));

    runtime.start().expect("start runtime");
    let committed = wait_until(Instant::now() + Duration::from_secs(2), || {
        runtime.snapshot().consensus.committed_height >= 1
    });
    assert!(committed, "runtime did not produce first commit in time");

    let store = LocalCasStore::new(dir.join("store"));
    for entry in fs::read_dir(store.blobs_dir()).expect("list blobs") {
        let entry = entry.expect("blob entry");
        if entry.file_type().expect("blob type").is_file() {
            fs::write(entry.path(), b"tampered-local-blob").expect("tamper blob");
        }
    }

    let errored = wait_until(Instant::now() + Duration::from_secs(3), || {
        runtime
            .snapshot()
            .last_error
            .as_deref()
            .map(|reason| reason.contains("storage challenge gate failed"))
            .unwrap_or(false)
    });
    assert!(
        errored,
        "runtime did not report storage challenge gate failure"
    );

    runtime.stop().expect("stop runtime");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn runtime_replication_storage_challenge_gate_blocks_on_network_blob_mismatch() {
    let dir = temp_dir("challenge-gate-network");
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(TestInMemoryNetwork::default());
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", 84)],
    );
    let config = NodeConfig::new("node-a", "world-challenge-network", NodeRole::Sequencer)
        .expect("config")
        .with_tick_interval(Duration::from_millis(10))
        .expect("tick")
        .with_pos_config(pos_config)
        .expect("pos config")
        .with_auto_attest_all_validators(true)
        .with_replication(signed_replication_config(dir.clone(), 84));
    let mut runtime = with_noop_execution_hook(NodeRuntime::new(config))
        .with_replication_network(NodeReplicationNetworkHandle::new(Arc::clone(&network)));

    runtime.start().expect("start runtime");
    let committed = wait_until(Instant::now() + Duration::from_secs(2), || {
        runtime.snapshot().consensus.committed_height >= 1
    });
    assert!(committed, "runtime did not produce first commit in time");

    network
        .register_handler(
            super::replication::REPLICATION_FETCH_BLOB_PROTOCOL,
            Box::new(|payload| {
                let request =
                    serde_json::from_slice::<super::replication::FetchBlobRequest>(payload)
                        .map_err(|err| WorldError::DistributedValidationFailed {
                            reason: format!("decode fetch blob request failed: {err}"),
                        })?;
                let response = super::replication::FetchBlobResponse {
                    found: true,
                    blob: Some(format!("bad-{}", request.content_hash).into_bytes()),
                };
                serde_json::to_vec(&response).map_err(|err| {
                    WorldError::DistributedValidationFailed {
                        reason: format!("encode fetch blob response failed: {err}"),
                    }
                })
            }),
        )
        .expect("register mismatched blob handler");

    let errored = wait_until(Instant::now() + Duration::from_secs(3), || {
        runtime
            .snapshot()
            .last_error
            .as_deref()
            .map(|reason| {
                reason.contains("network threshold unmet")
                    && reason.contains("network blob hash mismatch")
            })
            .unwrap_or(false)
    });
    assert!(
        errored,
        "runtime did not report network blob mismatch gate failure"
    );

    runtime.stop().expect("stop runtime");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn runtime_replication_storage_challenge_gate_allows_when_network_matches_reach_threshold() {
    let dir = temp_dir("challenge-gate-threshold-pass");
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(TestInMemoryNetwork::default());
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", 86)],
    );
    let config = NodeConfig::new(
        "node-a",
        "world-challenge-threshold-pass",
        NodeRole::Sequencer,
    )
    .expect("config")
    .with_tick_interval(Duration::from_millis(10))
    .expect("tick")
    .with_pos_config(pos_config)
    .expect("pos config")
    .with_auto_attest_all_validators(true)
    .with_replication(signed_replication_config(dir.clone(), 86));
    let mut runtime = with_noop_execution_hook(NodeRuntime::new(config))
        .with_replication_network(NodeReplicationNetworkHandle::new(Arc::clone(&network)));

    let root_for_handler = dir.clone();
    let matched_hashes = Arc::new(Mutex::new(Vec::<String>::new()));
    let matched_hashes_for_handler = Arc::clone(&matched_hashes);
    network
        .register_handler(
            super::replication::REPLICATION_FETCH_BLOB_PROTOCOL,
            Box::new(move |payload| {
                let request =
                    serde_json::from_slice::<super::replication::FetchBlobRequest>(payload)
                        .map_err(|err| WorldError::DistributedValidationFailed {
                            reason: format!("decode fetch blob request failed: {err}"),
                        })?;
                let maybe_local = super::replication::load_blob_from_root(
                    root_for_handler.as_path(),
                    request.content_hash.as_str(),
                )
                .map_err(|err| WorldError::DistributedValidationFailed {
                    reason: format!("load local blob failed: {err}"),
                })?;
                let Some(local_blob) = maybe_local else {
                    let response = super::replication::FetchBlobResponse {
                        found: false,
                        blob: None,
                    };
                    return serde_json::to_vec(&response).map_err(|err| {
                        WorldError::DistributedValidationFailed {
                            reason: format!("encode fetch blob response failed: {err}"),
                        }
                    });
                };

                let mut matched_hashes = matched_hashes_for_handler
                    .lock()
                    .expect("lock matched hashes");
                if matched_hashes.len() < 2
                    && !matched_hashes
                        .iter()
                        .any(|hash| hash == &request.content_hash)
                {
                    matched_hashes.push(request.content_hash.clone());
                }
                let should_match = matched_hashes
                    .iter()
                    .any(|hash| hash == &request.content_hash);
                drop(matched_hashes);
                let response = super::replication::FetchBlobResponse {
                    found: true,
                    blob: Some(if should_match {
                        local_blob
                    } else {
                        format!("bad-{}", request.content_hash).into_bytes()
                    }),
                };
                serde_json::to_vec(&response).map_err(|err| {
                    WorldError::DistributedValidationFailed {
                        reason: format!("encode fetch blob response failed: {err}"),
                    }
                })
            }),
        )
        .expect("register threshold pass blob handler");

    runtime.start().expect("start runtime");
    let advanced = wait_until(Instant::now() + Duration::from_secs(2), || {
        runtime.snapshot().consensus.committed_height >= 5
    });
    assert!(
        advanced,
        "runtime did not continue committing under threshold-based gate"
    );

    let snapshot = runtime.snapshot();
    assert!(
        !snapshot
            .last_error
            .as_deref()
            .map(|reason| reason.contains("network threshold unmet"))
            .unwrap_or(false),
        "runtime should not report threshold unmet when enough matches are available"
    );

    runtime.stop().expect("stop runtime");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn runtime_replication_storage_challenge_gate_falls_back_to_older_samples_during_catchup() {
    let dir = temp_dir("challenge-gate-catchup-fallback");
    let world_id = "world-challenge-catchup-fallback";
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", 113)],
    );

    let seed_config = NodeConfig::new("node-a", world_id, NodeRole::Sequencer)
        .expect("seed config")
        .with_tick_interval(Duration::from_millis(10))
        .expect("seed tick")
        .with_pos_config(pos_config.clone())
        .expect("seed pos config")
        .with_auto_attest_all_validators(true)
        .with_replication(signed_replication_config(dir.clone(), 113));
    let mut seed_runtime = with_noop_execution_hook(NodeRuntime::new(seed_config));
    seed_runtime.start().expect("start seed runtime");
    let seeded = wait_until(Instant::now() + Duration::from_secs(2), || {
        seed_runtime.snapshot().consensus.committed_height >= 6
    });
    assert!(seeded, "seed runtime did not build enough local commits");
    let seeded_height = seed_runtime.snapshot().consensus.committed_height;
    seed_runtime.stop().expect("stop seed runtime");

    let replication_runtime = super::replication::ReplicationRuntime::new(
        &signed_replication_config(dir.clone(), 113),
        "node-a",
    )
    .expect("open local replication runtime");
    let dht = Arc::new(TestReplicaMaintenanceDht::new(
        "storage-provider-1",
        "node-a",
    ));
    let mut remotely_available_blobs = HashMap::<String, Vec<u8>>::new();
    for height in 1..=seeded_height {
        let message = replication_runtime
            .load_commit_message_by_height(world_id, height)
            .expect("load commit")
            .expect("commit payload");
        dht.seed_provider(message.record.content_hash.as_str(), "storage-provider-1");
        if height <= 3 {
            let blob = replication_runtime
                .load_blob_by_hash(message.record.content_hash.as_str())
                .expect("load local blob")
                .expect("blob payload");
            remotely_available_blobs.insert(message.record.content_hash.clone(), blob);
        }
    }

    let requested_hashes = Arc::new(Mutex::new(Vec::<String>::new()));
    let requested_hashes_for_handler = Arc::clone(&requested_hashes);
    let remote_blobs_for_handler = Arc::new(remotely_available_blobs);
    let remote_blobs = Arc::clone(&remote_blobs_for_handler);
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
                requested_hashes_for_handler
                    .lock()
                    .expect("lock requested hashes")
                    .push(request.content_hash.clone());
                let blob = remote_blobs.get(request.content_hash.as_str()).cloned();
                let response = super::replication::FetchBlobResponse {
                    found: blob.is_some(),
                    blob,
                };
                serde_json::to_vec(&response).map_err(|err| {
                    WorldError::DistributedValidationFailed {
                        reason: format!("encode fetch blob response failed: {err}"),
                    }
                })
            }),
        )
        .expect("register catchup blob handler");

    let config = NodeConfig::new("node-a", world_id, NodeRole::Sequencer)
        .expect("config")
        .with_pos_config(pos_config)
        .expect("pos config")
        .with_replication(signed_replication_config(dir.clone(), 113));
    let handle = NodeReplicationNetworkHandle::new(Arc::clone(&network))
        .with_dht(dht)
        .with_local_provider_id("node-a");
    let endpoint =
        ReplicationNetworkEndpoint::new(&handle, world_id, false, &config.network_policy)
            .expect("endpoint");
    let mut engine = PosNodeEngine::new(&config).expect("engine");
    engine.committed_height = STORAGE_GATE_NETWORK_WARMUP_HEIGHT;
    engine.network_committed_height = STORAGE_GATE_NETWORK_WARMUP_HEIGHT;
    engine.peer_heads.insert(
        "storage-provider-1".to_string(),
        PeerCommittedHead {
            height: STORAGE_GATE_NETWORK_WARMUP_HEIGHT,
            block_hash: "catchup-peer-head".to_string(),
            committed_at_ms: 1_234,
            execution_block_hash: None,
            execution_state_root: None,
        },
    );
    let replication = super::replication::ReplicationRuntime::new(
        &signed_replication_config(dir.clone(), 113),
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
    let requested_hashes_snapshot = requested_hashes
        .lock()
        .expect("lock requested hashes")
        .clone();
    assert!(
        gate_result.is_ok(),
        "storage challenge gate should accept older reachable samples during catch-up: seeded_height={} fallback_height={} requested_hashes={requested_hashes_snapshot:?} err={gate_result:?}",
        seeded_height,
        engine.storage_challenge_fallback_height,
    );
    assert_eq!(engine.storage_challenge_fallback_height, 3);

    assert!(
        requested_hashes_snapshot
            .iter()
            .any(|hash| !remote_blobs_for_handler.contains_key(hash)),
        "expected challenge gate to probe latest unavailable hashes first: {requested_hashes_snapshot:?}"
    );
    assert!(
        requested_hashes_snapshot
            .iter()
            .any(|hash| remote_blobs_for_handler.contains_key(hash)),
        "expected challenge gate to fall back to older reachable hashes: {requested_hashes_snapshot:?}"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn runtime_replication_storage_challenge_gate_allows_single_match_during_warmup() {
    let dir = temp_dir("challenge-gate-warmup-single-match");
    let world_id = "world-challenge-warmup-single-match";
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", 114)],
    );

    let seed_config = NodeConfig::new("node-a", world_id, NodeRole::Sequencer)
        .expect("seed config")
        .with_tick_interval(Duration::from_millis(10))
        .expect("seed tick")
        .with_pos_config(pos_config.clone())
        .expect("seed pos config")
        .with_auto_attest_all_validators(true)
        .with_replication(signed_replication_config(dir.clone(), 114));
    let mut seed_runtime = with_noop_execution_hook(NodeRuntime::new(seed_config));
    seed_runtime.start().expect("start seed runtime");
    let seeded = wait_until(Instant::now() + Duration::from_secs(2), || {
        seed_runtime.snapshot().consensus.committed_height >= 6
    });
    assert!(seeded, "seed runtime did not build enough local commits");
    let seeded_height = seed_runtime.snapshot().consensus.committed_height;
    seed_runtime.stop().expect("stop seed runtime");

    let replication_runtime = super::replication::ReplicationRuntime::new(
        &signed_replication_config(dir.clone(), 114),
        "node-a",
    )
    .expect("open local replication runtime");
    let dht = Arc::new(TestReplicaMaintenanceDht::new(
        "storage-provider-1",
        "node-a",
    ));
    let mut remotely_available_blobs = HashMap::<String, Vec<u8>>::new();
    for height in 1..=seeded_height {
        let message = replication_runtime
            .load_commit_message_by_height(world_id, height)
            .expect("load commit")
            .expect("commit payload");
        dht.seed_provider(message.record.content_hash.as_str(), "storage-provider-1");
        if height == 1 {
            let blob = replication_runtime
                .load_blob_by_hash(message.record.content_hash.as_str())
                .expect("load local blob")
                .expect("blob payload");
            remotely_available_blobs.insert(message.record.content_hash.clone(), blob);
        }
    }

    let requested_hashes = Arc::new(Mutex::new(Vec::<String>::new()));
    let requested_hashes_for_handler = Arc::clone(&requested_hashes);
    let remote_blobs_for_handler = Arc::new(remotely_available_blobs);
    let remote_blobs = Arc::clone(&remote_blobs_for_handler);
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
                requested_hashes_for_handler
                    .lock()
                    .expect("lock requested hashes")
                    .push(request.content_hash.clone());
                let blob = remote_blobs.get(request.content_hash.as_str()).cloned();
                let response = super::replication::FetchBlobResponse {
                    found: blob.is_some(),
                    blob,
                };
                serde_json::to_vec(&response).map_err(|err| {
                    WorldError::DistributedValidationFailed {
                        reason: format!("encode fetch blob response failed: {err}"),
                    }
                })
            }),
        )
        .expect("register warmup blob handler");

    let config = NodeConfig::new("node-a", world_id, NodeRole::Sequencer)
        .expect("config")
        .with_pos_config(pos_config)
        .expect("pos config")
        .with_replication(signed_replication_config(dir.clone(), 114));
    let handle = NodeReplicationNetworkHandle::new(Arc::clone(&network))
        .with_dht(dht)
        .with_local_provider_id("node-a");
    let endpoint =
        ReplicationNetworkEndpoint::new(&handle, world_id, false, &config.network_policy)
            .expect("endpoint");
    let mut engine = PosNodeEngine::new(&config).expect("engine");
    engine.committed_height = 11;
    engine.network_committed_height = 11;
    engine.peer_heads.insert(
        "storage-provider-1".to_string(),
        PeerCommittedHead {
            height: 11,
            block_hash: "warmup-peer-head".to_string(),
            committed_at_ms: 1_234,
            execution_block_hash: None,
            execution_state_root: None,
        },
    );
    let replication = super::replication::ReplicationRuntime::new(
        &signed_replication_config(dir.clone(), 114),
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
    let requested_hashes_snapshot = requested_hashes
        .lock()
        .expect("lock requested hashes")
        .clone();
    assert!(
        gate_result.is_ok(),
        "storage challenge gate should allow a single remote match during warmup: seeded_height={} requested_hashes={requested_hashes_snapshot:?} err={gate_result:?}",
        seeded_height,
    );
    assert_eq!(engine.storage_challenge_fallback_height, 2);
    assert!(
        requested_hashes_snapshot
            .iter()
            .any(|hash| !remote_blobs_for_handler.contains_key(hash)),
        "expected warmup gate to probe unavailable recent hashes first: {requested_hashes_snapshot:?}"
    );
    assert!(
        requested_hashes_snapshot
            .iter()
            .any(|hash| remote_blobs_for_handler.contains_key(hash)),
        "expected warmup gate to accept at least one reachable older hash: {requested_hashes_snapshot:?}"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn runtime_replication_storage_challenge_gate_skips_network_probe_during_warmup_without_peer_heads()
{
    let dir = temp_dir("challenge-gate-warmup-no-peer-heads");
    let world_id = "world-challenge-warmup-no-peer-heads";
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", 115)],
    );

    let seed_config = NodeConfig::new("node-a", world_id, NodeRole::Sequencer)
        .expect("seed config")
        .with_tick_interval(Duration::from_millis(10))
        .expect("seed tick")
        .with_pos_config(pos_config.clone())
        .expect("seed pos config")
        .with_auto_attest_all_validators(true)
        .with_replication(signed_replication_config(dir.clone(), 115));
    let mut seed_runtime = with_noop_execution_hook(NodeRuntime::new(seed_config));
    seed_runtime.start().expect("start seed runtime");
    let seeded = wait_until(Instant::now() + Duration::from_secs(2), || {
        seed_runtime.snapshot().consensus.committed_height >= 6
    });
    assert!(seeded, "seed runtime did not build enough local commits");
    seed_runtime.stop().expect("stop seed runtime");

    let request_count = Arc::new(Mutex::new(0usize));
    let request_count_for_handler = Arc::clone(&request_count);
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(TestInMemoryNetwork::default());
    network
        .register_handler(
            super::replication::REPLICATION_FETCH_BLOB_PROTOCOL,
            Box::new(move |_payload| {
                *request_count_for_handler
                    .lock()
                    .expect("lock request count") += 1;
                Err(WorldError::NetworkProtocolUnavailable {
                    protocol: "unexpected warmup fetch-blob request".to_string(),
                })
            }),
        )
        .expect("register warmup skip handler");

    let config = NodeConfig::new("node-a", world_id, NodeRole::Sequencer)
        .expect("config")
        .with_pos_config(pos_config)
        .expect("pos config")
        .with_replication(signed_replication_config(dir.clone(), 115));
    let handle = NodeReplicationNetworkHandle::new(Arc::clone(&network))
        .with_dht(Arc::new(TestReplicaMaintenanceDht::new(
            "storage-provider-1",
            "node-a",
        )))
        .with_local_provider_id("node-a");
    let endpoint =
        ReplicationNetworkEndpoint::new(&handle, world_id, false, &config.network_policy)
            .expect("endpoint");
    let mut engine = PosNodeEngine::new(&config).expect("engine");
    engine.committed_height = 11;
    engine.network_committed_height = 11;
    let replication = super::replication::ReplicationRuntime::new(
        &signed_replication_config(dir.clone(), 115),
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
        "storage challenge gate should skip network probing during warmup when peer heads are still empty: {gate_result:?}"
    );
    assert_eq!(*request_count.lock().expect("lock request count"), 0);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn runtime_replication_storage_challenge_gate_allows_single_match_without_peer_heads_after_warmup()
{
    let dir = temp_dir("challenge-gate-no-peer-heads-single-match");
    let world_id = "world-challenge-no-peer-heads-single-match";
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", 116)],
    );

    let seed_config = NodeConfig::new("node-a", world_id, NodeRole::Sequencer)
        .expect("seed config")
        .with_tick_interval(Duration::from_millis(10))
        .expect("seed tick")
        .with_pos_config(pos_config.clone())
        .expect("seed pos config")
        .with_auto_attest_all_validators(true)
        .with_replication(signed_replication_config(dir.clone(), 116));
    let mut seed_runtime = with_noop_execution_hook(NodeRuntime::new(seed_config));
    seed_runtime.start().expect("start seed runtime");
    let seeded = wait_until(Instant::now() + Duration::from_secs(4), || {
        seed_runtime.snapshot().consensus.committed_height >= 6
    });
    assert!(seeded, "seed runtime did not build enough local commits");
    let seeded_height = seed_runtime.snapshot().consensus.committed_height;
    seed_runtime.stop().expect("stop seed runtime");

    let replication_runtime = super::replication::ReplicationRuntime::new(
        &signed_replication_config(dir.clone(), 116),
        "node-a",
    )
    .expect("open local replication runtime");
    let dht = Arc::new(TestReplicaMaintenanceDht::new(
        "storage-provider-1",
        "node-a",
    ));
    let mut remotely_available_blobs = HashMap::<String, Vec<u8>>::new();
    for height in 1..=seeded_height {
        let message = replication_runtime
            .load_commit_message_by_height(world_id, height)
            .expect("load commit")
            .expect("commit payload");
        dht.seed_provider(message.record.content_hash.as_str(), "storage-provider-1");
        if height == 1 {
            let blob = replication_runtime
                .load_blob_by_hash(message.record.content_hash.as_str())
                .expect("load local blob")
                .expect("blob payload");
            remotely_available_blobs.insert(message.record.content_hash.clone(), blob);
        }
    }

    let requested_hashes = Arc::new(Mutex::new(Vec::<String>::new()));
    let requested_hashes_for_handler = Arc::clone(&requested_hashes);
    let remote_blobs_for_handler = Arc::new(remotely_available_blobs);
    let remote_blobs = Arc::clone(&remote_blobs_for_handler);
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
                requested_hashes_for_handler
                    .lock()
                    .expect("lock requested hashes")
                    .push(request.content_hash.clone());
                let blob = remote_blobs.get(request.content_hash.as_str()).cloned();
                let response = super::replication::FetchBlobResponse {
                    found: blob.is_some(),
                    blob,
                };
                serde_json::to_vec(&response).map_err(|err| {
                    WorldError::DistributedValidationFailed {
                        reason: format!("encode fetch blob response failed: {err}"),
                    }
                })
            }),
        )
        .expect("register no-peer-head blob handler");

    let config = NodeConfig::new("node-a", world_id, NodeRole::Sequencer)
        .expect("config")
        .with_pos_config(pos_config)
        .expect("pos config")
        .with_require_peer_execution_hashes(true)
        .with_replication(signed_replication_config(dir.clone(), 116));
    let handle = NodeReplicationNetworkHandle::new(Arc::clone(&network))
        .with_dht(dht)
        .with_local_provider_id("node-a");
    let endpoint =
        ReplicationNetworkEndpoint::new(&handle, world_id, false, &config.network_policy)
            .expect("endpoint");
    let mut engine = PosNodeEngine::new(&config).expect("engine");
    engine.committed_height = STORAGE_GATE_NETWORK_WARMUP_HEIGHT + 8;
    engine.network_committed_height = STORAGE_GATE_NETWORK_WARMUP_HEIGHT + 8;
    let replication = super::replication::ReplicationRuntime::new(
        &signed_replication_config(dir.clone(), 116),
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
    let requested_hashes_snapshot = requested_hashes
        .lock()
        .expect("lock requested hashes")
        .clone();
    assert!(
        gate_result.is_ok(),
        "storage challenge gate should allow a single remote match when peer heads remain empty after warmup: seeded_height={} requested_hashes={requested_hashes_snapshot:?} err={gate_result:?}",
        seeded_height,
    );
    assert_eq!(engine.storage_challenge_fallback_height, 2);
    assert!(
        requested_hashes_snapshot
            .iter()
            .any(|hash| !remote_blobs_for_handler.contains_key(hash)),
        "expected gate to probe unavailable recent hashes first: {requested_hashes_snapshot:?}"
    );
    assert!(
        requested_hashes_snapshot
            .iter()
            .any(|hash| remote_blobs_for_handler.contains_key(hash)),
        "expected gate to accept at least one reachable older hash: {requested_hashes_snapshot:?}"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn runtime_replication_storage_challenge_gate_prefers_dht_blob_providers() {
    let dir = temp_dir("challenge-gate-provider-selection");
    let network_impl = Arc::new(ProviderAwareTestNetwork::new(
        dir.clone(),
        "storage-provider-1",
    ));
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = network_impl.clone();
    let dht = Arc::new(TestReplicaMaintenanceDht::new(
        "storage-provider-1",
        "storage-provider-1",
    ));
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", 93)],
    );
    let config = NodeConfig::new(
        "node-a",
        "world-challenge-provider-selection",
        NodeRole::Sequencer,
    )
    .expect("config")
    .with_tick_interval(Duration::from_millis(10))
    .expect("tick")
    .with_pos_config(pos_config)
    .expect("pos config")
    .with_auto_attest_all_validators(true)
    .with_replication(signed_replication_config(dir.clone(), 93));
    let mut runtime = with_noop_execution_hook(NodeRuntime::new(config)).with_replication_network(
        NodeReplicationNetworkHandle::new(Arc::clone(&network))
            .with_dht(dht)
            .with_local_provider_id("node-a"),
    );

    runtime.start().expect("start runtime");
    let advanced = wait_until(Instant::now() + Duration::from_secs(2), || {
        runtime.snapshot().consensus.committed_height >= 4
    });
    let snapshot = runtime.snapshot();
    let attempts = network_impl.provider_attempts();
    assert!(
        advanced,
        "runtime did not continue committing when provider-aware fetch-blob was available: committed_height={} network_committed_height={} last_error={:?} attempts={attempts:?}",
        snapshot.consensus.committed_height,
        snapshot.consensus.network_committed_height,
        snapshot.last_error
    );

    assert!(
        attempts.iter().any(|providers| {
            providers
                .iter()
                .any(|provider| provider == "storage-provider-1")
        }),
        "expected storage challenge gate to request fetch-blob with DHT providers, attempts={attempts:?}"
    );

    assert!(
        !snapshot
            .last_error
            .as_deref()
            .map(|reason| reason.contains("storage challenge gate"))
            .unwrap_or(false),
        "runtime should not report storage challenge gate failure when provider-aware fetch-blob succeeds: {:?}",
        snapshot.last_error
    );

    runtime.stop().expect("stop runtime");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn runtime_local_replication_publishes_blob_provider_to_dht() {
    let dir = temp_dir("publish-local-provider");
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(TestInMemoryNetwork::default());
    let dht = Arc::new(TestReplicaMaintenanceDht::new("peer-local", "peer-local"));
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", 94)],
    );
    let config = NodeConfig::new(
        "node-a",
        "world-publish-local-provider",
        NodeRole::Sequencer,
    )
    .expect("config")
    .with_tick_interval(Duration::from_millis(10))
    .expect("tick")
    .with_pos_config(pos_config)
    .expect("pos config")
    .with_auto_attest_all_validators(true)
    .with_replication(signed_replication_config(dir.clone(), 94));
    let mut runtime = with_noop_execution_hook(NodeRuntime::new(config)).with_replication_network(
        NodeReplicationNetworkHandle::new(Arc::clone(&network))
            .with_dht(dht.clone())
            .with_local_provider_id("peer-local"),
    );

    runtime.start().expect("start runtime");
    let published = wait_until(Instant::now() + Duration::from_secs(2), || {
        !dht.published_records().is_empty()
    });
    assert!(published, "expected local commit to publish blob provider");

    let published_records = dht.published_records();
    assert!(
        published_records
            .iter()
            .any(|(_, _, provider_id)| provider_id == "peer-local"),
        "expected published provider id peer-local, got {published_records:?}"
    );

    runtime.stop().expect("stop runtime");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn runtime_replication_storage_challenge_gate_falls_back_after_provider_route_unavailable() {
    let dir = temp_dir("challenge-gate-provider-fallback");
    let network_impl = Arc::new(ProviderFallbackTestNetwork::new(dir.clone()));
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = network_impl.clone();
    let dht = Arc::new(TestReplicaMaintenanceDht::new(
        "storage-provider-1",
        "node-a",
    ));
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", 97)],
    );
    let config = NodeConfig::new(
        "node-a",
        "world-challenge-provider-fallback",
        NodeRole::Sequencer,
    )
    .expect("config")
    .with_tick_interval(Duration::from_millis(10))
    .expect("tick")
    .with_pos_config(pos_config)
    .expect("pos config")
    .with_auto_attest_all_validators(true)
    .with_replication(signed_replication_config(dir.clone(), 97));
    let mut runtime = with_noop_execution_hook(NodeRuntime::new(config)).with_replication_network(
        NodeReplicationNetworkHandle::new(Arc::clone(&network))
            .with_dht(dht)
            .with_local_provider_id("node-a"),
    );

    runtime.start().expect("start runtime");
    let advanced = wait_until(Instant::now() + Duration::from_secs(2), || {
        runtime.snapshot().consensus.committed_height >= 4
    });
    let snapshot = runtime.snapshot();
    let provider_attempts = network_impl.provider_attempts();
    let generic_attempts = network_impl.generic_attempts();
    assert!(
        advanced,
        "runtime did not continue committing after provider-route fallback: committed_height={} network_committed_height={} last_error={:?} provider_attempts={provider_attempts:?} generic_attempts={generic_attempts}",
        snapshot.consensus.committed_height,
        snapshot.consensus.network_committed_height,
        snapshot.last_error
    );
    assert!(
        provider_attempts.iter().any(|providers| {
            providers
                .iter()
                .any(|provider| provider == "storage-provider-1")
        }),
        "expected storage challenge gate to try DHT-selected provider before fallback: {provider_attempts:?}"
    );
    assert!(
        generic_attempts > 0,
        "expected storage challenge gate to fall back to generic lane request"
    );
    assert!(
        !snapshot
            .last_error
            .as_deref()
            .map(|reason| reason.contains("storage challenge gate"))
            .unwrap_or(false),
        "runtime should not report storage challenge gate failure when generic fallback succeeds: {:?}",
        snapshot.last_error
    );

    runtime.stop().expect("stop runtime");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn runtime_replication_storage_challenge_gate_falls_back_after_provider_route_not_found() {
    let dir = temp_dir("challenge-gate-provider-not-found");
    let network_impl = Arc::new(ProviderNotFoundFallbackTestNetwork::new(dir.clone()));
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = network_impl.clone();
    let dht = Arc::new(TestReplicaMaintenanceDht::new(
        "storage-provider-1",
        "node-a",
    ));
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", 112)],
    );
    let config = NodeConfig::new(
        "node-a",
        "world-challenge-provider-not-found",
        NodeRole::Sequencer,
    )
    .expect("config")
    .with_tick_interval(Duration::from_millis(10))
    .expect("tick")
    .with_pos_config(pos_config)
    .expect("pos config")
    .with_auto_attest_all_validators(true)
    .with_replication(signed_replication_config(dir.clone(), 112));
    let mut runtime = with_noop_execution_hook(NodeRuntime::new(config)).with_replication_network(
        NodeReplicationNetworkHandle::new(Arc::clone(&network))
            .with_dht(dht)
            .with_local_provider_id("node-a"),
    );

    runtime.start().expect("start runtime");
    let advanced = wait_until(Instant::now() + Duration::from_secs(2), || {
        runtime.snapshot().consensus.committed_height >= 4
    });
    let snapshot = runtime.snapshot();
    let provider_attempts = network_impl.provider_attempts();
    let generic_attempts = network_impl.generic_attempts();
    assert!(
        advanced,
        "runtime did not continue committing after provider-route not-found fallback: committed_height={} network_committed_height={} last_error={:?} provider_attempts={provider_attempts:?} generic_attempts={generic_attempts}",
        snapshot.consensus.committed_height,
        snapshot.consensus.network_committed_height,
        snapshot.last_error
    );
    assert!(
        provider_attempts.iter().any(|providers| {
            providers
                .iter()
                .any(|provider| provider == "storage-provider-1")
        }),
        "expected storage challenge gate to try DHT-selected provider before not-found fallback: {provider_attempts:?}"
    );
    assert!(
        generic_attempts > 0,
        "expected storage challenge gate to retry generic lane after not-found provider response"
    );
    assert!(
        !snapshot
            .last_error
            .as_deref()
            .map(|reason| reason.contains("storage challenge gate"))
            .unwrap_or(false),
        "runtime should not report storage challenge gate failure when not-found fallback succeeds: {:?}",
        snapshot.last_error
    );

    runtime.stop().expect("stop runtime");
    let _ = fs::remove_dir_all(&dir);
}
