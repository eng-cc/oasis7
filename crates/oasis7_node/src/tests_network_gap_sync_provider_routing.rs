use std::fs;

use super::*;

#[test]
fn runtime_network_replication_gap_sync_prefers_dht_blob_providers() {
    let world_id = "world-network-gap-provider-selection";
    let dir_a = temp_dir("network-gap-provider-selection-a");
    let dir_b = temp_dir("network-gap-provider-selection-b");
    let validators = vec![PosValidator {
        validator_id: "node-a".to_string(),
        stake: 100,
    }];
    let pos_config = signed_pos_config_with_signer_seeds(validators, &[("node-a", 98)]);
    let network_impl = Arc::new(ProviderAwareTestNetwork::new(
        dir_a.clone(),
        "storage-provider-1",
    ));
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = network_impl.clone();

    let config_b = NodeConfig::new("node-b", world_id, NodeRole::Observer)
        .expect("config b")
        .with_tick_interval(Duration::from_millis(10))
        .expect("tick b")
        .with_pos_config(pos_config)
        .expect("pos config b")
        .with_replication(signed_replication_config(dir_b.clone(), 99));

    let mut commit_map = HashMap::<u64, super::replication::GossipReplicationMessage>::new();
    let mut replication_runtime_a = super::replication::ReplicationRuntime::new(
        &signed_replication_config(dir_a.clone(), 98),
        "node-a",
    )
    .expect("open replication runtime a");
    let target_height = 2;
    for height in 1..=target_height {
        let decision = PosDecision {
            height,
            slot: height.saturating_sub(1),
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
        let message = replication_runtime_a
            .build_local_commit_message(
                "node-a",
                world_id,
                1_000 + height as i64,
                &decision,
                None,
                None,
            )
            .expect("build local commit message")
            .expect("commit payload");
        commit_map.insert(height, message);
    }
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

    let dht = Arc::new(TestReplicaMaintenanceDht::new(
        "storage-provider-1",
        "observer-provider",
    ));
    for message in commit_map.values() {
        dht.seed_provider(message.record.content_hash.as_str(), "storage-provider-1");
    }
    let mut runtime_b = NodeRuntime::new(config_b)
        .with_replication_network(
            NodeReplicationNetworkHandle::new(Arc::clone(&network))
                .with_dht(dht)
                .with_local_provider_id("observer-provider"),
        )
        .with_replication_network_consensus_enabled(false);
    runtime_b.start().expect("start b");

    let commit_map = Arc::new(commit_map);
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

    let synced = wait_until(Instant::now() + Duration::from_secs(3), || {
        runtime_b.snapshot().consensus.committed_height >= target_height
    });
    let snapshot_b = runtime_b.snapshot();
    let attempts = network_impl.provider_attempts();
    assert!(
        synced,
        "observer did not sync missing commits with provider-aware gap sync: committed_height={} network_committed_height={} last_error={:?} attempts={attempts:?}",
        snapshot_b.consensus.committed_height,
        snapshot_b.consensus.network_committed_height,
        snapshot_b.last_error
    );
    assert!(
        attempts.iter().any(|providers| {
            providers
                .iter()
                .any(|provider| provider == "storage-provider-1")
        }),
        "expected gap sync fetch-blob to use DHT-selected providers, attempts={attempts:?}"
    );

    runtime_b.stop().expect("stop b");
    let _ = fs::remove_dir_all(&dir_a);
    let _ = fs::remove_dir_all(&dir_b);
}

#[test]
fn runtime_network_replication_gap_sync_falls_back_after_provider_route_unavailable() {
    let world_id = "world-network-gap-provider-fallback";
    let dir_a = temp_dir("network-gap-provider-fallback-a");
    let dir_b = temp_dir("network-gap-provider-fallback-b");
    let validators = vec![PosValidator {
        validator_id: "node-a".to_string(),
        stake: 100,
    }];
    let pos_config = signed_pos_config_with_signer_seeds(validators, &[("node-a", 100)]);
    let network_impl = Arc::new(ProviderFallbackTestNetwork::new(dir_a.clone()));
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
        .with_replication(signed_replication_config(dir_a.clone(), 100));
    let config_b = NodeConfig::new("node-b", world_id, NodeRole::Observer)
        .expect("config b")
        .with_tick_interval(Duration::from_millis(10))
        .expect("tick b")
        .with_pos_config(pos_config)
        .expect("pos config b")
        .with_replication(signed_replication_config(dir_b.clone(), 101));

    let mut runtime_a = with_noop_execution_hook(NodeRuntime::new(config_a));
    runtime_a.start().expect("start a");
    let reached = wait_until(Instant::now() + Duration::from_secs(2), || {
        runtime_a.snapshot().consensus.committed_height >= 3
    });
    assert!(reached, "sequencer did not reach target height in time");
    let target_height = runtime_a.snapshot().consensus.committed_height;
    runtime_a.stop().expect("stop a");

    let mut commit_map = HashMap::<u64, super::replication::GossipReplicationMessage>::new();
    let replication_runtime_a = super::replication::ReplicationRuntime::new(
        &signed_replication_config(dir_a.clone(), 100),
        "node-a",
    )
    .expect("open replication runtime a");
    for height in 1..=target_height {
        let message = replication_runtime_a
            .load_commit_message_by_height(world_id, height)
            .expect("load commit by height")
            .expect("commit payload");
        commit_map.insert(height, message);
    }
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

    let dht = Arc::new(TestReplicaMaintenanceDht::new(
        "storage-provider-1",
        "observer-provider",
    ));
    let mut runtime_b = NodeRuntime::new(config_b)
        .with_replication_network(
            NodeReplicationNetworkHandle::new(Arc::clone(&network))
                .with_dht(dht)
                .with_local_provider_id("observer-provider"),
        )
        .with_replication_network_consensus_enabled(false);
    runtime_b.start().expect("start b");

    let commit_map = Arc::new(commit_map);
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

    let synced = wait_until(Instant::now() + Duration::from_secs(3), || {
        runtime_b.snapshot().consensus.committed_height >= target_height
    });
    let snapshot_b = runtime_b.snapshot();
    let provider_attempts = network_impl.provider_attempts();
    let generic_attempts = network_impl.generic_attempts();
    assert!(
        synced,
        "observer did not sync missing commits after provider-route fallback: committed_height={} network_committed_height={} last_error={:?} provider_attempts={provider_attempts:?} generic_attempts={generic_attempts}",
        snapshot_b.consensus.committed_height,
        snapshot_b.consensus.network_committed_height,
        snapshot_b.last_error
    );
    assert!(
        provider_attempts.iter().any(|providers| {
            providers
                .iter()
                .any(|provider| provider == "storage-provider-1")
        }),
        "expected gap sync to try DHT-selected provider before fallback: {provider_attempts:?}"
    );
    assert!(
        generic_attempts > 0,
        "expected gap sync fetch-blob to fall back to generic request"
    );

    runtime_b.stop().expect("stop b");
    let _ = fs::remove_dir_all(&dir_a);
    let _ = fs::remove_dir_all(&dir_b);
}

#[test]
fn runtime_network_replication_gap_sync_falls_back_after_provider_route_not_found() {
    let world_id = "world-network-gap-provider-not-found";
    let dir_a = temp_dir("network-gap-provider-not-found-a");
    let dir_b = temp_dir("network-gap-provider-not-found-b");
    let validators = vec![PosValidator {
        validator_id: "node-a".to_string(),
        stake: 100,
    }];
    let pos_config = signed_pos_config_with_signer_seeds(validators, &[("node-a", 110)]);
    let network_impl = Arc::new(ProviderNotFoundFallbackTestNetwork::new(dir_a.clone()));
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
        .with_replication(signed_replication_config(dir_a.clone(), 110));
    let config_b = NodeConfig::new("node-b", world_id, NodeRole::Observer)
        .expect("config b")
        .with_tick_interval(Duration::from_millis(10))
        .expect("tick b")
        .with_pos_config(pos_config)
        .expect("pos config b")
        .with_replication(signed_replication_config(dir_b.clone(), 111));

    let mut runtime_a = with_noop_execution_hook(NodeRuntime::new(config_a));
    runtime_a.start().expect("start a");
    let reached = wait_until(Instant::now() + Duration::from_secs(2), || {
        runtime_a.snapshot().consensus.committed_height >= 3
    });
    assert!(reached, "sequencer did not reach target height in time");
    let target_height = runtime_a.snapshot().consensus.committed_height;
    runtime_a.stop().expect("stop a");

    let mut commit_map = HashMap::<u64, super::replication::GossipReplicationMessage>::new();
    let replication_runtime_a = super::replication::ReplicationRuntime::new(
        &signed_replication_config(dir_a.clone(), 110),
        "node-a",
    )
    .expect("open replication runtime a");
    for height in 1..=target_height {
        let message = replication_runtime_a
            .load_commit_message_by_height(world_id, height)
            .expect("load commit by height")
            .expect("commit payload");
        commit_map.insert(height, message);
    }
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

    let dht = Arc::new(TestReplicaMaintenanceDht::new(
        "storage-provider-1",
        "observer-provider",
    ));
    let mut runtime_b = NodeRuntime::new(config_b)
        .with_replication_network(
            NodeReplicationNetworkHandle::new(Arc::clone(&network))
                .with_dht(dht)
                .with_local_provider_id("observer-provider"),
        )
        .with_replication_network_consensus_enabled(false);
    runtime_b.start().expect("start b");

    let commit_map = Arc::new(commit_map);
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

    let synced = wait_until(Instant::now() + Duration::from_secs(3), || {
        runtime_b.snapshot().consensus.committed_height >= target_height
    });
    let snapshot_b = runtime_b.snapshot();
    let provider_attempts = network_impl.provider_attempts();
    let generic_attempts = network_impl.generic_attempts();
    assert!(
        synced,
        "observer did not sync missing commits after provider-route not-found fallback: committed_height={} network_committed_height={} last_error={:?} provider_attempts={provider_attempts:?} generic_attempts={generic_attempts}",
        snapshot_b.consensus.committed_height,
        snapshot_b.consensus.network_committed_height,
        snapshot_b.last_error
    );
    assert!(
        provider_attempts.iter().any(|providers| {
            providers
                .iter()
                .any(|provider| provider == "storage-provider-1")
        }),
        "expected gap sync to try DHT-selected provider before not-found fallback: {provider_attempts:?}"
    );
    assert!(
        generic_attempts > 0,
        "expected gap sync fetch-blob to retry on generic lane after not-found provider response"
    );

    runtime_b.stop().expect("stop b");
    let _ = fs::remove_dir_all(&dir_a);
    let _ = fs::remove_dir_all(&dir_b);
}
