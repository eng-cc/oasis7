use super::*;
use crate::node_engine_gap_sync_outcome::GapSyncHeightOutcome;

pub(super) fn build_fetch_commit_success_cache_fixture(
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

#[derive(Clone)]
pub(super) struct PeerDirectedFetchCommitTestNetwork {
    pub(super) generic_response: super::replication::FetchCommitResponse,
    pub(super) generic_unsupported: bool,
    pub(super) peer_responses: Arc<Mutex<HashMap<String, super::replication::FetchCommitResponse>>>,
    pub(super) connected_peer_ids: Vec<String>,
    pub(super) generic_attempts: Arc<Mutex<usize>>,
    pub(super) provider_attempts: Arc<Mutex<Vec<Vec<String>>>>,
}

impl oasis7_proto::distributed_net::DistributedNetwork<WorldError>
    for PeerDirectedFetchCommitTestNetwork
{
    fn publish(&self, _topic: &str, _payload: &[u8]) -> Result<(), WorldError> {
        Ok(())
    }

    fn subscribe(&self, topic: &str) -> Result<NetworkSubscription, WorldError> {
        Ok(NetworkSubscription::new(
            topic.to_string(),
            Arc::new(Mutex::new(HashMap::new())),
        ))
    }

    fn request(&self, protocol: &str, _payload: &[u8]) -> Result<Vec<u8>, WorldError> {
        *self.generic_attempts.lock().expect("lock generic attempts") += 1;
        if protocol != super::replication::REPLICATION_FETCH_COMMIT_PROTOCOL {
            return Err(WorldError::NetworkProtocolUnavailable {
                protocol: protocol.to_string(),
            });
        }
        if self.generic_unsupported {
            return Err(WorldError::NetworkRequestFailed {
                code: DistributedErrorCode::ErrUnsupported,
                message: super::replication::REPLICATION_FETCH_COMMIT_PROTOCOL.to_string(),
                retryable: false,
            });
        }
        serde_json::to_vec(&self.generic_response).map_err(|err| {
            WorldError::DistributedValidationFailed {
                reason: format!("encode generic fetch commit response failed: {err}"),
            }
        })
    }

    fn connected_peer_ids(&self) -> Vec<String> {
        self.connected_peer_ids.clone()
    }

    fn request_with_providers(
        &self,
        protocol: &str,
        _payload: &[u8],
        providers: &[String],
    ) -> Result<Vec<u8>, WorldError> {
        self.provider_attempts
            .lock()
            .expect("lock provider attempts")
            .push(providers.to_vec());
        if protocol != super::replication::REPLICATION_FETCH_COMMIT_PROTOCOL {
            return Err(WorldError::NetworkProtocolUnavailable {
                protocol: protocol.to_string(),
            });
        }
        let peer_id =
            providers
                .first()
                .cloned()
                .ok_or_else(|| WorldError::NetworkProtocolUnavailable {
                    protocol: "missing provider peer id".to_string(),
                })?;
        let response = self
            .peer_responses
            .lock()
            .expect("lock peer responses")
            .get(peer_id.as_str())
            .cloned()
            .unwrap_or(super::replication::FetchCommitResponse {
                found: false,
                message: None,
            lineage_envelope: None,
            });
        serde_json::to_vec(&response).map_err(|err| WorldError::DistributedValidationFailed {
            reason: format!("encode peer-directed fetch commit response failed: {err}"),
        })
    }

    fn register_handler(
        &self,
        _protocol: &str,
        _handler: Box<dyn Fn(&[u8]) -> Result<Vec<u8>, WorldError> + Send + Sync>,
    ) -> Result<(), WorldError> {
        Ok(())
    }
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
                        lineage_envelope: None,
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
                serde_json::to_vec(&test_fetch_blob_response(
                    request.content_hash == expected_hash,
                    (request.content_hash == expected_hash).then(|| expected_blob.clone()),
                ))
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
                        lineage_envelope: None,
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
                    serde_json::to_vec(&test_fetch_blob_response(
                        request.content_hash == expected_hash,
                        (request.content_hash == expected_hash).then(|| expected_blob.clone()),
                    ))
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
                            lineage_envelope: None,
                        }
                    } else {
                        super::replication::FetchCommitResponse {
                            found: false,
                            message: None,
                        lineage_envelope: None,
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
                serde_json::to_vec(&test_fetch_blob_response(
                    request.content_hash == expected_hash,
                    (request.content_hash == expected_hash).then(|| expected_blob.clone()),
                ))
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
                            lineage_envelope: None,
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
                serde_json::to_vec(&test_fetch_blob_response(
                    request.content_hash == expected_hash,
                    (request.content_hash == expected_hash).then(|| expected_blob.clone()),
                ))
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
                        lineage_envelope: None,
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
                    serde_json::to_vec(&test_fetch_blob_response(
                        request.content_hash == expected_hash,
                        (request.content_hash == expected_hash).then(|| expected_blob.clone()),
                    ))
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
                        lineage_envelope: None,
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
                serde_json::to_vec(&test_fetch_blob_response(
                    request.content_hash == expected_hash,
                    (request.content_hash == expected_hash).then(|| expected_blob.clone()),
                ))
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
            lineage_envelope: None,
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
fn gap_sync_fetch_commit_retries_after_not_found_response() {
    let dir_remote = temp_dir("gap-sync-fetch-commit-not-found-remote");
    let dir_local = temp_dir("gap-sync-fetch-commit-not-found-local");
    let world_id = "world-gap-sync-fetch-commit-not-found";
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(TestInMemoryNetwork::default());
    let (_, _, endpoint, message) = build_fetch_commit_success_cache_fixture(
        world_id,
        dir_remote.as_path(),
        dir_local.as_path(),
        140,
        141,
        Arc::clone(&network),
    );
    let request = signed_fetch_commit_request_for_test(world_id, 1, 141);
    let request_count = Arc::new(Mutex::new(0usize));
    network
        .register_handler(
            super::replication::REPLICATION_FETCH_COMMIT_PROTOCOL,
            Box::new({
                let message = message.clone();
                let request_count = Arc::clone(&request_count);
                move |_payload| {
                    let mut attempts = request_count.lock().expect("lock request count");
                    *attempts += 1;
                    let response = if *attempts == 1 {
                        super::replication::FetchCommitResponse {
                            found: false,
                            message: None,
                        lineage_envelope: None,
                        }
                    } else {
                        super::replication::FetchCommitResponse {
                            found: true,
                            message: Some(message.clone()),
                            lineage_envelope: None,
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

    let response = endpoint
        .request_fetch_commit_for_gap_sync(&request)
        .expect("gap-sync fetch commit response");
    assert!(
        response.response.found,
        "expected retry to return found=true"
    );
    assert_eq!(
        *request_count.lock().expect("lock request count"),
        2,
        "expected gap sync fetch commit to retry once after found=false"
    );

    let _ = fs::remove_dir_all(&dir_remote);
    let _ = fs::remove_dir_all(&dir_local);
}

#[test]
fn gap_sync_fetch_commit_tries_connected_peers_after_generic_not_found() {
    let dir_remote = temp_dir("gap-sync-fetch-commit-peer-route-remote");
    let dir_local = temp_dir("gap-sync-fetch-commit-peer-route-local");
    let world_id = "world-gap-sync-fetch-commit-peer-route";
    let generic_attempts = Arc::new(Mutex::new(0usize));
    let provider_attempts = Arc::new(Mutex::new(Vec::<Vec<String>>::new()));
    let peer_responses = Arc::new(Mutex::new(HashMap::new()));
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(PeerDirectedFetchCommitTestNetwork {
        generic_response: super::replication::FetchCommitResponse {
            found: false,
            message: None,
        lineage_envelope: None,
        },
        generic_unsupported: false,
        peer_responses: Arc::clone(&peer_responses),
        connected_peer_ids: vec!["peer-a".to_string(), "peer-b".to_string()],
        generic_attempts: Arc::clone(&generic_attempts),
        provider_attempts: Arc::clone(&provider_attempts),
    });
    let (_, _, endpoint, message) = build_fetch_commit_success_cache_fixture(
        world_id,
        dir_remote.as_path(),
        dir_local.as_path(),
        142,
        143,
        Arc::clone(&network),
    );
    peer_responses.lock().expect("lock peer responses").insert(
        "peer-b".to_string(),
        super::replication::FetchCommitResponse {
            found: true,
            message: Some(message),
            lineage_envelope: None,
        },
    );
    let request = signed_fetch_commit_request_for_test(world_id, 1, 143);

    let response = endpoint
        .request_fetch_commit_for_gap_sync(&request)
        .expect("peer-directed gap-sync fetch commit response");
    assert!(
        response.response.found,
        "expected connected peer fallback to return found=true"
    );
    assert_eq!(
        *generic_attempts.lock().expect("lock generic attempts"),
        1,
        "expected one generic attempt before peer-directed fallback"
    );
    assert_eq!(
        provider_attempts
            .lock()
            .expect("lock provider attempts")
            .as_slice(),
        &[vec!["peer-a".to_string()], vec!["peer-b".to_string()],],
        "expected gap sync to try each connected peer until one returned the commit"
    );

    let _ = fs::remove_dir_all(&dir_remote);
    let _ = fs::remove_dir_all(&dir_local);
}

#[test]
fn gap_sync_fetch_commit_tries_connected_peers_after_generic_unsupported() {
    let dir_remote = temp_dir("gap-sync-fetch-commit-peer-route-unsupported-remote");
    let dir_local = temp_dir("gap-sync-fetch-commit-peer-route-unsupported-local");
    let world_id = "world-gap-sync-fetch-commit-peer-route-unsupported";
    let generic_attempts = Arc::new(Mutex::new(0usize));
    let provider_attempts = Arc::new(Mutex::new(Vec::<Vec<String>>::new()));
    let peer_responses = Arc::new(Mutex::new(HashMap::new()));
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(PeerDirectedFetchCommitTestNetwork {
        generic_response: super::replication::FetchCommitResponse {
            found: false,
            message: None,
        lineage_envelope: None,
        },
        generic_unsupported: true,
        peer_responses: Arc::clone(&peer_responses),
        connected_peer_ids: vec!["peer-a".to_string(), "peer-b".to_string()],
        generic_attempts: Arc::clone(&generic_attempts),
        provider_attempts: Arc::clone(&provider_attempts),
    });
    let (_, _, endpoint, message) = build_fetch_commit_success_cache_fixture(
        world_id,
        dir_remote.as_path(),
        dir_local.as_path(),
        144,
        145,
        Arc::clone(&network),
    );
    peer_responses.lock().expect("lock peer responses").insert(
        "peer-b".to_string(),
        super::replication::FetchCommitResponse {
            found: true,
            message: Some(message),
            lineage_envelope: None,
        },
    );
    let request = signed_fetch_commit_request_for_test(world_id, 1, 145);

    let response = endpoint
        .request_fetch_commit_for_gap_sync(&request)
        .expect("peer-directed gap-sync fetch commit response");
    assert!(
        response.response.found,
        "expected connected peer fallback to recover from generic unsupported"
    );
    assert_eq!(
        *generic_attempts.lock().expect("lock generic attempts"),
        1,
        "expected one generic attempt before peer-directed fallback"
    );
    assert_eq!(
        provider_attempts
            .lock()
            .expect("lock provider attempts")
            .as_slice(),
        &[vec!["peer-a".to_string()], vec!["peer-b".to_string()],],
        "expected gap sync to keep trying connected peers after generic unsupported"
    );

    let _ = fs::remove_dir_all(&dir_remote);
    let _ = fs::remove_dir_all(&dir_local);
}

#[path = "tests_network_gap_sync_replication_fetch.rs"]
mod network_gap_sync_replication_fetch_tests;
