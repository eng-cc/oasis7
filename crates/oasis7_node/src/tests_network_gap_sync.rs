use super::*;

fn build_fetch_commit_success_cache_fixture(
    world_id: &str,
    dir_remote: &std::path::Path,
    dir_local: &std::path::Path,
    remote_seed: u8,
    local_seed: u8,
    network: Arc<dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync>,
) -> (
    PosNodeEngine,
    ReplicationRuntime,
    ReplicationNetworkEndpoint,
    super::replication::GossipReplicationMessage,
) {
    let (_, remote_public_key_hex) = deterministic_keypair_hex(remote_seed);
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", remote_seed)],
    );
    let local_replication_config = signed_replication_config(dir_local.to_path_buf(), local_seed)
        .with_remote_writer_allowlist(vec![remote_public_key_hex])
        .expect("local remote writer allowlist");
    let config = NodeConfig::new("node-b", world_id, NodeRole::Observer)
        .expect("config")
        .with_tick_interval(Duration::from_millis(10))
        .expect("tick")
        .with_pos_config(pos_config)
        .expect("pos config")
        .with_replication(local_replication_config.clone());
    let handle = NodeReplicationNetworkHandle::new(network);
    let endpoint =
        ReplicationNetworkEndpoint::new(&handle, world_id, false, &config.network_policy)
            .expect("endpoint");
    let mut remote_replication = super::replication::ReplicationRuntime::new(
        &signed_replication_config(dir_remote.to_path_buf(), remote_seed),
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
    let replication =
        super::replication::ReplicationRuntime::new(&local_replication_config, "node-b")
            .expect("local replication runtime");
    (
        PosNodeEngine::new(&config).expect("engine"),
        replication,
        endpoint,
        message,
    )
}

fn build_gap_sync_endpoint_with_policy(
    world_id: &str,
    dir_local: &std::path::Path,
    local_seed: u8,
    network: Arc<dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync>,
    network_policy: NodeNetworkPolicy,
) -> ReplicationNetworkEndpoint {
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", local_seed)],
    );
    let local_replication_config = signed_replication_config(dir_local.to_path_buf(), local_seed);
    let config = NodeConfig::new("node-b", world_id, NodeRole::Observer)
        .expect("config")
        .with_tick_interval(Duration::from_millis(10))
        .expect("tick")
        .with_pos_config(pos_config)
        .expect("pos config")
        .with_replication(local_replication_config)
        .with_network_policy(network_policy)
        .expect("network policy");
    let handle = NodeReplicationNetworkHandle::new(network);
    ReplicationNetworkEndpoint::new(&handle, world_id, false, &config.network_policy)
        .expect("endpoint")
}

#[test]
fn successor_probe_at_genesis_syncs_height_one_before_local_proposal() {
    let dir_remote = temp_dir("successor-probe-genesis-remote");
    let dir_local = temp_dir("successor-probe-genesis-local");
    let world_id = "world-successor-probe-genesis";
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(TestInMemoryNetwork::default());
    let (mut engine, mut replication, endpoint, message) = build_fetch_commit_success_cache_fixture(
        world_id,
        dir_remote.as_path(),
        dir_local.as_path(),
        130,
        131,
        Arc::clone(&network),
    );
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

    assert_eq!(engine.committed_height, 0);
    let hold = engine
        .maybe_hold_proposal_for_replication_successor_probe(
            &endpoint,
            "node-b",
            world_id,
            1_000,
            Some(&mut replication),
            None,
        )
        .expect("probe genesis successor");
    assert!(
        hold,
        "genesis probe should hold proposals while syncing height 1"
    );
    assert_eq!(engine.committed_height, 1);
    assert_eq!(engine.replication_persisted_height, 1);

    let _ = fs::remove_dir_all(&dir_remote);
    let _ = fs::remove_dir_all(&dir_local);
}

#[test]
fn runtime_network_replication_gap_sync_fetch_commit_success_cache_reuses_validated_response() {
    let dir_remote = temp_dir("gap-sync-fetch-commit-success-cache-remote");
    let dir_local = temp_dir("gap-sync-fetch-commit-success-cache-local");
    let world_id = "world-gap-sync-fetch-commit-success-cache";
    let request_count = Arc::new(Mutex::new(0usize));
    let blob_count = Arc::new(Mutex::new(0usize));
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(TestInMemoryNetwork::default());
    let (engine, mut replication, endpoint, message) = build_fetch_commit_success_cache_fixture(
        world_id,
        dir_remote.as_path(),
        dir_local.as_path(),
        120,
        121,
        Arc::clone(&network),
    );
    let mut endpoint = endpoint;
    endpoint.set_fetch_commit_success_cache_after_for_testing(Duration::from_millis(250));
    let expected_hash = message.record.content_hash.clone();
    let expected_blob = message.payload.clone();
    network
        .register_handler(
            super::replication::REPLICATION_FETCH_COMMIT_PROTOCOL,
            Box::new({
                let request_count = Arc::clone(&request_count);
                let message = message.clone();
                move |_payload| {
                    *request_count.lock().expect("lock request count") += 1;
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
            Box::new({
                let blob_count = Arc::clone(&blob_count);
                move |payload| {
                    *blob_count.lock().expect("lock blob count") += 1;
                    let request =
                        serde_json::from_slice::<super::replication::FetchBlobRequest>(payload)
                            .map_err(|err| WorldError::DistributedValidationFailed {
                                reason: format!("decode fetch blob request failed: {err}"),
                            })?;
                    serde_json::to_vec(&super::replication::FetchBlobResponse {
                        found: request.content_hash == expected_hash,
                        blob: (request.content_hash == expected_hash)
                            .then(|| expected_blob.clone()),
                    })
                    .map_err(|err| WorldError::DistributedValidationFailed {
                        reason: format!("encode fetch blob response failed: {err}"),
                    })
                }
            }),
        )
        .expect("register fetch blob handler");

    let first = engine
        .sync_replication_height_once(&endpoint, "node-b", world_id, &mut replication, 1)
        .expect("first sync");
    let second = engine
        .sync_replication_height_once(&endpoint, "node-b", world_id, &mut replication, 1)
        .expect("second sync");

    assert!(matches!(first, GapSyncHeightOutcome::Synced { .. }));
    assert!(matches!(second, GapSyncHeightOutcome::Synced { .. }));
    assert_eq!(*request_count.lock().expect("lock request count"), 1);
    assert_eq!(*blob_count.lock().expect("lock blob count"), 2);

    let _ = fs::remove_dir_all(&dir_remote);
    let _ = fs::remove_dir_all(&dir_local);
}

#[test]
fn successor_probe_cooldown_suppresses_same_height_not_found_retry() {
    let dir_remote = temp_dir("successor-probe-cooldown-remote");
    let dir_local = temp_dir("successor-probe-cooldown-local");
    let world_id = "world-successor-probe-cooldown";
    let request_count = Arc::new(Mutex::new(0usize));
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(TestInMemoryNetwork::default());
    let (mut engine, mut replication, endpoint, message) = build_fetch_commit_success_cache_fixture(
        world_id,
        dir_remote.as_path(),
        dir_local.as_path(),
        124,
        125,
        Arc::clone(&network),
    );
    let expected_hash = message.record.content_hash.clone();
    let expected_blob = message.payload.clone();
    network
        .register_handler(
            super::replication::REPLICATION_FETCH_COMMIT_PROTOCOL,
            Box::new({
                let request_count = Arc::clone(&request_count);
                let message = message.clone();
                move |payload| {
                    *request_count.lock().expect("lock request count") += 1;
                    let request =
                        serde_json::from_slice::<super::replication::FetchCommitRequest>(payload)
                            .map_err(|err| WorldError::DistributedValidationFailed {
                            reason: format!("decode fetch commit request failed: {err}"),
                        })?;
                    let response = if request.height == 1 {
                        super::replication::FetchCommitResponse {
                            found: true,
                            message: Some(message.clone()),
                        }
                    } else {
                        super::replication::FetchCommitResponse {
                            found: false,
                            message: None,
                        }
                    };
                    serde_json::to_vec(&response).map_err(|err| {
                        WorldError::DistributedValidationFailed {
                            reason: format!("encode fetch commit response failed: {err}"),
                        }
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

    let synced = engine
        .sync_replication_height_once(&endpoint, "node-b", world_id, &mut replication, 1)
        .expect("sync height 1");
    let GapSyncHeightOutcome::Synced { payload, .. } = synced else {
        panic!("expected synced outcome for height 1");
    };
    engine
        .record_synced_replication_height(1, payload.block_hash, payload.committed_at_ms)
        .expect("record synced height 1");
    *request_count.lock().expect("lock request count") = 0;

    let first = engine
        .maybe_hold_proposal_for_replication_successor_probe(
            &endpoint,
            "node-b",
            world_id,
            1_000,
            Some(&mut replication),
            None,
        )
        .expect("first successor probe");
    assert!(
        !first,
        "not-found successor probe should not hold proposals"
    );
    assert_eq!(*request_count.lock().expect("lock request count"), 1);

    let second = engine
        .maybe_hold_proposal_for_replication_successor_probe(
            &endpoint,
            "node-b",
            world_id,
            1_200,
            Some(&mut replication),
            None,
        )
        .expect("second successor probe");
    assert!(!second, "cooldown skip should keep proposal hold disabled");
    assert_eq!(
        *request_count.lock().expect("lock request count"),
        1,
        "same successor height should not be re-probed inside cooldown"
    );

    let third = engine
        .maybe_hold_proposal_for_replication_successor_probe(
            &endpoint,
            "node-b",
            world_id,
            2_100,
            Some(&mut replication),
            None,
        )
        .expect("third successor probe");
    assert!(
        !third,
        "not-found after cooldown should still keep proposal hold disabled"
    );
    assert_eq!(*request_count.lock().expect("lock request count"), 2);

    let _ = fs::remove_dir_all(&dir_remote);
    let _ = fs::remove_dir_all(&dir_local);
}

#[test]
fn successor_probe_cooldown_preserves_waitable_hold_decision() {
    let dir_remote = temp_dir("successor-probe-cooldown-hold-remote");
    let dir_local = temp_dir("successor-probe-cooldown-hold-local");
    let world_id = "world-successor-probe-cooldown-hold";
    let request_count = Arc::new(Mutex::new(0usize));
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(TestInMemoryNetwork::default());
    let (mut engine, mut replication, endpoint, message) = build_fetch_commit_success_cache_fixture(
        world_id,
        dir_remote.as_path(),
        dir_local.as_path(),
        124,
        125,
        Arc::clone(&network),
    );
    let expected_hash = message.record.content_hash.clone();
    let expected_blob = message.payload.clone();
    network
        .register_handler(
            super::replication::REPLICATION_FETCH_COMMIT_PROTOCOL,
            Box::new({
                let request_count = Arc::clone(&request_count);
                let message = message.clone();
                move |payload| {
                    *request_count.lock().expect("lock request count") += 1;
                    let request =
                        serde_json::from_slice::<super::replication::FetchCommitRequest>(payload)
                            .map_err(|err| WorldError::DistributedValidationFailed {
                                reason: format!("decode fetch commit request failed: {err}"),
                            })?;
                    let response = if request.height == 1 {
                        super::replication::FetchCommitResponse {
                            found: true,
                            message: Some(message.clone()),
                        }
                    } else {
                        Err(WorldError::NetworkProtocolUnavailable {
                            protocol: "libp2p-replication no connected peers for protocol /aw/node/replication/fetch-commit/1.0.0".to_string(),
                        })?
                    };
                    serde_json::to_vec(&response).map_err(|err| {
                        WorldError::DistributedValidationFailed {
                            reason: format!("encode fetch commit response failed: {err}"),
                        }
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

    let synced = engine
        .sync_replication_height_once(&endpoint, "node-b", world_id, &mut replication, 1)
        .expect("sync height 1");
    let GapSyncHeightOutcome::Synced { payload, .. } = synced else {
        panic!("expected synced outcome for height 1");
    };
    engine
        .record_synced_replication_height(1, payload.block_hash, payload.committed_at_ms)
        .expect("record synced height 1");
    *request_count.lock().expect("lock request count") = 0;

    let first = engine
        .maybe_hold_proposal_for_replication_successor_probe(
            &endpoint,
            "node-b",
            world_id,
            1_000,
            Some(&mut replication),
            None,
        )
        .expect("first successor probe");
    assert!(first, "waitable connection-gap should hold proposals");
    assert_eq!(*request_count.lock().expect("lock request count"), 1);

    let second = engine
        .maybe_hold_proposal_for_replication_successor_probe(
            &endpoint,
            "node-b",
            world_id,
            1_200,
            Some(&mut replication),
            None,
        )
        .expect("second successor probe");
    assert!(
        second,
        "cooldown skip should preserve previous hold decision for waitable gaps"
    );
    assert_eq!(
        *request_count.lock().expect("lock request count"),
        1,
        "same successor height should not be re-probed inside cooldown"
    );

    let _ = fs::remove_dir_all(&dir_remote);
    let _ = fs::remove_dir_all(&dir_local);
}

#[test]
fn runtime_network_replication_gap_sync_fetch_commit_success_cache_skips_invalid_commit() {
    let dir_remote = temp_dir("gap-sync-fetch-commit-invalid-cache-remote");
    let dir_local = temp_dir("gap-sync-fetch-commit-invalid-cache-local");
    let world_id = "world-gap-sync-fetch-commit-invalid-cache";
    let request_count = Arc::new(Mutex::new(0usize));
    let blob_count = Arc::new(Mutex::new(0usize));
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(TestInMemoryNetwork::default());
    let (engine, mut replication, endpoint, valid_message) =
        build_fetch_commit_success_cache_fixture(
            world_id,
            dir_remote.as_path(),
            dir_local.as_path(),
            122,
            123,
            Arc::clone(&network),
        );
    let mut invalid_message = valid_message.clone();
    invalid_message.world_id = "wrong-world".to_string();
    let expected_hash = valid_message.record.content_hash.clone();
    let expected_blob = valid_message.payload.clone();
    network
        .register_handler(
            super::replication::REPLICATION_FETCH_COMMIT_PROTOCOL,
            Box::new({
                let request_count = Arc::clone(&request_count);
                let invalid_message = invalid_message.clone();
                let valid_message = valid_message.clone();
                move |_payload| {
                    let mut count = request_count.lock().expect("lock request count");
                    *count += 1;
                    let message = if *count == 1 {
                        invalid_message.clone()
                    } else {
                        valid_message.clone()
                    };
                    serde_json::to_vec(&super::replication::FetchCommitResponse {
                        found: true,
                        message: Some(message),
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
            Box::new({
                let blob_count = Arc::clone(&blob_count);
                move |payload| {
                    *blob_count.lock().expect("lock blob count") += 1;
                    let request =
                        serde_json::from_slice::<super::replication::FetchBlobRequest>(payload)
                            .map_err(|err| WorldError::DistributedValidationFailed {
                                reason: format!("decode fetch blob request failed: {err}"),
                            })?;
                    serde_json::to_vec(&super::replication::FetchBlobResponse {
                        found: request.content_hash == expected_hash,
                        blob: (request.content_hash == expected_hash)
                            .then(|| expected_blob.clone()),
                    })
                    .map_err(|err| WorldError::DistributedValidationFailed {
                        reason: format!("encode fetch blob response failed: {err}"),
                    })
                }
            }),
        )
        .expect("register fetch blob handler");

    let first =
        engine.sync_replication_height_once(&endpoint, "node-b", world_id, &mut replication, 1);
    assert!(matches!(
        first,
        Err(NodeError::Replication { reason }) if reason.contains("world mismatch")
    ));
    let second = engine
        .sync_replication_height_once(&endpoint, "node-b", world_id, &mut replication, 1)
        .expect("second sync");

    assert!(matches!(second, GapSyncHeightOutcome::Synced { .. }));
    assert_eq!(*request_count.lock().expect("lock request count"), 2);
    assert_eq!(
        *blob_count.lock().expect("lock blob count"),
        1,
        "invalid fetch-commit response should fail before any fetch-blob request",
    );

    let _ = fs::remove_dir_all(&dir_remote);
    let _ = fs::remove_dir_all(&dir_local);
}

#[test]
fn runtime_network_replication_gap_sync_fetch_commit_success_cache_expires() {
    let dir_remote = temp_dir("gap-sync-fetch-commit-cache-expiry-remote");
    let dir_local = temp_dir("gap-sync-fetch-commit-cache-expiry-local");
    let world_id = "world-gap-sync-fetch-commit-cache-expiry";
    let request_count = Arc::new(Mutex::new(0usize));
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(TestInMemoryNetwork::default());
    let (engine, mut replication, endpoint, message) = build_fetch_commit_success_cache_fixture(
        world_id,
        dir_remote.as_path(),
        dir_local.as_path(),
        124,
        125,
        Arc::clone(&network),
    );
    let mut endpoint = endpoint;
    endpoint.set_fetch_commit_success_cache_after_for_testing(Duration::from_millis(250));
    let expected_hash = message.record.content_hash.clone();
    let expected_blob = message.payload.clone();
    network
        .register_handler(
            super::replication::REPLICATION_FETCH_COMMIT_PROTOCOL,
            Box::new({
                let request_count = Arc::clone(&request_count);
                let message = message.clone();
                move |_payload| {
                    *request_count.lock().expect("lock request count") += 1;
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

    engine
        .sync_replication_height_once(&endpoint, "node-b", world_id, &mut replication, 1)
        .expect("first sync");
    engine
        .sync_replication_height_once(&endpoint, "node-b", world_id, &mut replication, 1)
        .expect("second sync");
    assert_eq!(*request_count.lock().expect("lock request count"), 1);

    thread::sleep(Duration::from_millis(300));
    engine
        .sync_replication_height_once(&endpoint, "node-b", world_id, &mut replication, 1)
        .expect("post-expiry sync");
    assert_eq!(*request_count.lock().expect("lock request count"), 2);

    let _ = fs::remove_dir_all(&dir_remote);
    let _ = fs::remove_dir_all(&dir_local);
}

#[test]
fn runtime_network_replication_gap_sync_fetch_commit_cache_still_enforces_lane_access() {
    let dir_local = temp_dir("gap-sync-fetch-commit-cache-policy-local");
    let world_id = "world-gap-sync-fetch-commit-cache-policy";
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(TestInMemoryNetwork::default());
    let mut endpoint = build_gap_sync_endpoint_with_policy(
        world_id,
        dir_local.as_path(),
        126,
        Arc::clone(&network),
        NodeNetworkPolicy {
            deployment_mode: oasis7_proto::distributed_dht::PeerDeploymentMode::Public,
            node_role_claim: oasis7_proto::distributed_dht::PeerNodeRole::Relay,
        },
    );
    endpoint.set_fetch_commit_success_cache_after_for_testing(Duration::from_secs(5));
    let request = signed_fetch_commit_request_for_test(world_id, 7, 126);
    endpoint.remember_validated_fetch_commit_success(
        &request,
        &super::replication::FetchCommitResponse {
            found: true,
            message: Some(super::replication::GossipReplicationMessage {
                version: 1,
                world_id: world_id.to_string(),
                node_id: "node-a".to_string(),
                record: oasis7_distfs::FileReplicationRecord {
                    world_id: world_id.to_string(),
                    writer_id: "writer-a".to_string(),
                    writer_epoch: 1,
                    sequence: 1,
                    path: "consensus/commits/00000000000000000007.json".to_string(),
                    content_hash: "hash-7".to_string(),
                    size_bytes: 7,
                    updated_at_ms: 7,
                },
                payload: b"payload".to_vec(),
                public_key_hex: None,
                signature_hex: None,
            }),
        },
    );

    let result = endpoint.request_fetch_commit_for_gap_sync(&request);
    assert!(matches!(
        result,
        Err(NodeError::InvalidConfig { reason })
            if reason.contains("cannot Request")
                && reason.contains(super::replication::REPLICATION_FETCH_COMMIT_PROTOCOL)
                && reason.contains("lane=sync")
    ));

    let _ = fs::remove_dir_all(&dir_local);
}

#[test]
fn runtime_network_replication_gap_sync_fetches_missing_commits() {
    let world_id = "world-network-gap";
    let dir_a = temp_dir("network-gap-a");
    let dir_b = temp_dir("network-gap-b");
    let validators = vec![
        PosValidator {
            validator_id: "node-a".to_string(),
            stake: 60,
        },
        PosValidator {
            validator_id: "node-b".to_string(),
            stake: 40,
        },
    ];
    let pos_config =
        signed_pos_config_with_signer_seeds(validators, &[("node-a", 78), ("node-b", 79)]);
    let network_impl = Arc::new(TestInMemoryNetwork::default());
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = network_impl.clone();

    let config_a = NodeConfig::new("node-a", world_id, NodeRole::Sequencer)
        .expect("config a")
        .with_tick_interval(Duration::from_millis(10))
        .expect("tick a")
        .with_pos_config(pos_config.clone())
        .expect("pos config a")
        .with_auto_attest_all_validators(true)
        .with_replication(signed_replication_config(dir_a.clone(), 78));
    let config_b = NodeConfig::new("node-b", world_id, NodeRole::Observer)
        .expect("config b")
        .with_tick_interval(Duration::from_millis(10))
        .expect("tick b")
        .with_pos_config(pos_config)
        .expect("pos config b")
        .with_replication(signed_replication_config(dir_b.clone(), 79));

    let mut runtime_a = with_noop_execution_hook(NodeRuntime::new(config_a))
        .with_replication_network(NodeReplicationNetworkHandle::new(Arc::clone(&network)));
    runtime_a.start().expect("start a");
    let reached = wait_until(Instant::now() + Duration::from_secs(2), || {
        runtime_a.snapshot().consensus.committed_height >= 3
    });
    assert!(reached, "sequencer did not reach target height in time");
    let target_height = runtime_a.snapshot().consensus.committed_height;
    runtime_a.stop().expect("stop a");

    let mut commit_map = HashMap::<u64, super::replication::GossipReplicationMessage>::new();
    let mut blob_map = HashMap::<String, Vec<u8>>::new();
    for height in 1..=target_height {
        let request = signed_fetch_commit_request_for_test(world_id, height, 78);
        let payload = serde_json::to_vec(&request).expect("encode commit request");
        let response_payload = network
            .request(
                super::replication::REPLICATION_FETCH_COMMIT_PROTOCOL,
                payload.as_slice(),
            )
            .expect("fetch commit");
        let response: super::replication::FetchCommitResponse =
            serde_json::from_slice(&response_payload).expect("decode commit response");
        assert!(response.found, "missing fetched commit at height {height}");
        let message = response.message.expect("commit payload");
        blob_map.insert(message.record.content_hash.clone(), message.payload.clone());
        commit_map.insert(height, message);
    }
    assert_eq!(commit_map.len() as u64, target_height);
    let high_message = commit_map
        .get(&target_height)
        .cloned()
        .expect("high commit message");

    let topic = super::network_bridge::default_replication_topic(world_id);
    network_impl.clear_topic(topic.as_str());
    network_impl
        .clear_topic(super::network_bridge::default_consensus_proposal_topic(world_id).as_str());
    network_impl
        .clear_topic(super::network_bridge::default_consensus_attestation_topic(world_id).as_str());
    network_impl
        .clear_topic(super::network_bridge::default_consensus_commit_topic(world_id).as_str());
    let high_payload = serde_json::to_vec(&high_message).expect("encode high message");
    network
        .publish(topic.as_str(), high_payload.as_slice())
        .expect("publish high message");

    let mut runtime_b = NodeRuntime::new(config_b)
        .with_replication_network(NodeReplicationNetworkHandle::new(Arc::clone(&network)))
        .with_replication_network_consensus_enabled(false);
    runtime_b.start().expect("start b");

    let commit_map = Arc::new(commit_map);
    let blob_map = Arc::new(blob_map);
    let commit_world_id = world_id.to_string();
    network
        .register_handler(
            super::replication::REPLICATION_FETCH_COMMIT_PROTOCOL,
            Box::new(move |payload| {
                let request =
                    serde_json::from_slice::<super::replication::FetchCommitRequest>(payload)
                        .map_err(|err| WorldError::DistributedValidationFailed {
                            reason: format!("decode fetch commit request failed: {err}"),
                        })?;
                if request.world_id != commit_world_id {
                    return Err(WorldError::DistributedValidationFailed {
                        reason: format!(
                            "world mismatch expected={} actual={}",
                            commit_world_id, request.world_id
                        ),
                    });
                }
                let response = super::replication::FetchCommitResponse {
                    found: commit_map.contains_key(&request.height),
                    message: commit_map.get(&request.height).cloned(),
                };
                serde_json::to_vec(&response).map_err(|err| {
                    WorldError::DistributedValidationFailed {
                        reason: format!("encode fetch commit response failed: {err}"),
                    }
                })
            }),
        )
        .expect("register commit handler");
    network
        .register_handler(
            super::replication::REPLICATION_FETCH_BLOB_PROTOCOL,
            Box::new(move |payload| {
                let request =
                    serde_json::from_slice::<super::replication::FetchBlobRequest>(payload)
                        .map_err(|err| WorldError::DistributedValidationFailed {
                            reason: format!("decode fetch blob request failed: {err}"),
                        })?;
                let response = super::replication::FetchBlobResponse {
                    found: blob_map.contains_key(request.content_hash.as_str()),
                    blob: blob_map.get(request.content_hash.as_str()).cloned(),
                };
                serde_json::to_vec(&response).map_err(|err| {
                    WorldError::DistributedValidationFailed {
                        reason: format!("encode fetch blob response failed: {err}"),
                    }
                })
            }),
        )
        .expect("register blob handler");

    let synced = wait_until(Instant::now() + Duration::from_secs(8), || {
        runtime_b.snapshot().consensus.committed_height >= target_height
    });
    assert!(synced, "observer did not sync missing commits in time");

    runtime_b.stop().expect("stop b");
    let snapshot_b = runtime_b.snapshot();
    assert!(snapshot_b.last_error.is_none());
    assert!(snapshot_b.consensus.committed_height >= target_height);

    let store_b = LocalCasStore::new(dir_b.join("store"));
    let files = store_b.list_files().expect("list files");
    assert!(files
        .iter()
        .any(|item| item.path == "consensus/commits/00000000000000000001.json"));
    assert!(files
        .iter()
        .any(|item| { item.path == format!("consensus/commits/{:020}.json", target_height) }));

    let _ = fs::remove_dir_all(&dir_a);
    let _ = fs::remove_dir_all(&dir_b);
}

#[test]
fn runtime_network_replication_gap_sync_not_found_is_non_fatal() {
    let world_id = "world-network-gap-not-found";
    let dir_a = temp_dir("network-gap-not-found-a");
    let dir_b = temp_dir("network-gap-not-found-b");
    let validators = vec![
        PosValidator {
            validator_id: "node-a".to_string(),
            stake: 60,
        },
        PosValidator {
            validator_id: "node-b".to_string(),
            stake: 40,
        },
    ];
    let pos_config =
        signed_pos_config_with_signer_seeds(validators, &[("node-a", 87), ("node-b", 88)]);
    let network_impl = Arc::new(TestInMemoryNetwork::default());
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = network_impl.clone();

    let config_a = NodeConfig::new("node-a", world_id, NodeRole::Sequencer)
        .expect("config a")
        .with_tick_interval(Duration::from_millis(10))
        .expect("tick a")
        .with_pos_config(pos_config.clone())
        .expect("pos config a")
        .with_auto_attest_all_validators(true)
        .with_replication(signed_replication_config(dir_a.clone(), 87));
    let config_b = NodeConfig::new("node-b", world_id, NodeRole::Observer)
        .expect("config b")
        .with_tick_interval(Duration::from_millis(10))
        .expect("tick b")
        .with_pos_config(pos_config)
        .expect("pos config b")
        .with_replication(signed_replication_config(dir_b.clone(), 88));

    let mut runtime_a = with_noop_execution_hook(NodeRuntime::new(config_a))
        .with_replication_network(NodeReplicationNetworkHandle::new(Arc::clone(&network)));
    runtime_a.start().expect("start a");
    let reached = wait_until(Instant::now() + Duration::from_secs(2), || {
        runtime_a.snapshot().consensus.committed_height >= 3
    });
    assert!(reached, "sequencer did not reach target height in time");
    let target_height = runtime_a.snapshot().consensus.committed_height;
    runtime_a.stop().expect("stop a");

    let request = signed_fetch_commit_request_for_test(world_id, target_height, 87);
    let payload = serde_json::to_vec(&request).expect("encode commit request");
    let response_payload = network
        .request(
            super::replication::REPLICATION_FETCH_COMMIT_PROTOCOL,
            payload.as_slice(),
        )
        .expect("fetch commit");
    let response: super::replication::FetchCommitResponse =
        serde_json::from_slice(&response_payload).expect("decode commit response");
    assert!(response.found, "missing high commit");
    let high_message = response.message.expect("high commit payload");

    let topic = super::network_bridge::default_replication_topic(world_id);
    network_impl.clear_topic(topic.as_str());
    network_impl
        .clear_topic(super::network_bridge::default_consensus_proposal_topic(world_id).as_str());
    network_impl
        .clear_topic(super::network_bridge::default_consensus_attestation_topic(world_id).as_str());
    network_impl
        .clear_topic(super::network_bridge::default_consensus_commit_topic(world_id).as_str());
    let high_payload = serde_json::to_vec(&high_message).expect("encode high message");
    network
        .publish(topic.as_str(), high_payload.as_slice())
        .expect("publish high message");

    network
        .register_handler(
            super::replication::REPLICATION_FETCH_COMMIT_PROTOCOL,
            Box::new(move |_payload| {
                let response = super::replication::FetchCommitResponse {
                    found: false,
                    message: None,
                };
                serde_json::to_vec(&response).map_err(|err| {
                    WorldError::DistributedValidationFailed {
                        reason: format!("encode fetch commit response failed: {err}"),
                    }
                })
            }),
        )
        .expect("register commit not found handler");

    let mut runtime_b = NodeRuntime::new(config_b)
        .with_replication_network(NodeReplicationNetworkHandle::new(Arc::clone(&network)));
    runtime_b.start().expect("start b");
    thread::sleep(Duration::from_millis(250));

    let snapshot_b = runtime_b.snapshot();
    assert!(
        !snapshot_b
            .last_error
            .as_deref()
            .map(|reason| reason.contains("gap sync height"))
            .unwrap_or(false),
        "not found gap sync should not be reported as fatal error"
    );
    assert!(
        snapshot_b.consensus.committed_height < target_height,
        "observer should keep waiting when target height is not found"
    );

    runtime_b.stop().expect("stop b");
    let _ = fs::remove_dir_all(&dir_a);
    let _ = fs::remove_dir_all(&dir_b);
}
