use super::*;
use oasis7_proto::distributed::WorldHeadAnnounce;
use oasis7_proto::distributed_dht::{
    DistributedDht, MembershipDirectorySnapshot, ProviderRecord, SignedPeerRecord,
};

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

#[derive(Default)]
struct TimeoutThenCheckpointNetwork {
    messages: Mutex<BTreeMap<u64, replication::GossipReplicationMessage>>,
    blobs: Mutex<BTreeMap<String, Vec<u8>>>,
    attempts: Mutex<Vec<u64>>,
    advertised_height: u64,
}

#[derive(Default)]
struct TimeoutWorldHeadDht;

impl DistributedDht<WorldError> for TimeoutWorldHeadDht {
    fn publish_provider(
        &self,
        _world_id: &str,
        _content_hash: &str,
        _provider_id: &str,
    ) -> Result<(), WorldError> {
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
        Err(WorldError::NetworkProtocolUnavailable {
            protocol: "request failed: Timeout".to_string(),
        })
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

impl oasis7_proto::distributed_net::DistributedNetwork<WorldError> for TimeoutThenCheckpointNetwork {
    fn publish(&self, _topic: &str, _payload: &[u8]) -> Result<(), WorldError> {
        Ok(())
    }

    fn subscribe(&self, topic: &str) -> Result<NetworkSubscription, WorldError> {
        Ok(NetworkSubscription::new(
            topic.to_string(),
            Arc::new(Mutex::new(HashMap::new())),
        ))
    }

    fn request(&self, protocol: &str, payload: &[u8]) -> Result<Vec<u8>, WorldError> {
        if protocol == replication::REPLICATION_GET_HEAD_PROTOCOL {
            return Err(WorldError::NetworkProtocolUnavailable {
                protocol: "request failed: Timeout".to_string(),
            });
        }
        if protocol == replication::REPLICATION_FETCH_COMMIT_PROTOCOL {
            let request =
                serde_json::from_slice::<replication::FetchCommitRequest>(payload).map_err(
                    |err| WorldError::DistributedValidationFailed {
                        reason: format!("decode fetch-commit request failed: {err}"),
                    },
                )?;
            if request.height == self.advertised_height {
                return Err(WorldError::NetworkProtocolUnavailable {
                    protocol: "request failed: Timeout".to_string(),
                });
            }
            return encode_fetch_commit_response(None);
        }
        if protocol == replication::REPLICATION_FETCH_BLOB_PROTOCOL {
            return self.fetch_blob_response(payload);
        }
        Err(WorldError::NetworkProtocolUnavailable {
            protocol: protocol.to_string(),
        })
    }

    fn request_with_providers(
        &self,
        protocol: &str,
        payload: &[u8],
        _providers: &[String],
    ) -> Result<Vec<u8>, WorldError> {
        if protocol == replication::REPLICATION_GET_HEAD_PROTOCOL {
            return Err(WorldError::NetworkProtocolUnavailable {
                protocol: "request failed: Timeout".to_string(),
            });
        }
        if protocol == replication::REPLICATION_FETCH_BLOB_PROTOCOL {
            return self.fetch_blob_response(payload);
        }
        if protocol != replication::REPLICATION_FETCH_COMMIT_PROTOCOL {
            return Err(WorldError::NetworkProtocolUnavailable {
                protocol: protocol.to_string(),
            });
        }
        let request =
            serde_json::from_slice::<replication::FetchCommitRequest>(payload).map_err(|err| {
                WorldError::DistributedValidationFailed {
                    reason: format!("decode fetch-commit request failed: {err}"),
                }
            })?;
        self.attempts
            .lock()
            .expect("lock attempts")
            .push(request.height);
        if request.height == self.advertised_height {
            return Err(WorldError::NetworkProtocolUnavailable {
                protocol: "request failed: Timeout".to_string(),
            });
        }
        let message = self
            .messages
            .lock()
            .expect("lock messages")
            .get(&request.height)
            .cloned();
        encode_fetch_commit_response(message)
    }

    fn connected_peer_ids(&self) -> Vec<String> {
        vec!["peer-a".to_string()]
    }

    fn register_handler(
        &self,
        _protocol: &str,
        _handler: Box<dyn Fn(&[u8]) -> Result<Vec<u8>, WorldError> + Send + Sync>,
    ) -> Result<(), WorldError> {
        Ok(())
    }
}

impl TimeoutThenCheckpointNetwork {
    fn fetch_blob_response(&self, payload: &[u8]) -> Result<Vec<u8>, WorldError> {
        let request =
            serde_json::from_slice::<replication::FetchBlobRequest>(payload).map_err(|err| {
                WorldError::DistributedValidationFailed {
                    reason: format!("decode fetch-blob request failed: {err}"),
                }
            })?;
        let blob = self
            .blobs
            .lock()
            .expect("lock blobs")
            .get(request.content_hash.as_str())
            .cloned();
        serde_json::to_vec(&replication::FetchBlobResponse {
            found: blob.is_some(),
            range_offset_bytes: None,
            range_complete: None,
            blob,
        })
        .map_err(|err| WorldError::DistributedValidationFailed {
            reason: format!("encode fetch-blob response failed: {err}"),
        })
    }
}

fn encode_fetch_commit_response(
    message: Option<replication::GossipReplicationMessage>,
) -> Result<Vec<u8>, WorldError> {
    serde_json::to_vec(&replication::FetchCommitResponse {
        found: message.is_some(),
        message,
    })
    .map_err(|err| WorldError::DistributedValidationFailed {
        reason: format!("encode fetch-commit response failed: {err}"),
    })
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

fn committed_decision(height: u64, approved_stake: u64, required_stake: u64) -> PosDecision {
    PosDecision {
        height,
        slot: height,
        epoch: 0,
        status: PosConsensusStatus::Committed,
        block_hash: format!("block-{height}"),
        action_root: empty_action_root(),
        committed_actions: Vec::new(),
        approved_stake,
        rejected_stake: 0,
        required_stake,
        total_stake: 100,
    }
}

#[test]
fn observer_gap_sync_continues_checkpoint_candidates_after_protocol_omitted_timeout() {
    let world_id = "world-gap-sync-timeout-then-checkpoint";
    let dir_a = temp_dir("gap-sync-timeout-then-checkpoint-a");
    let dir_b = temp_dir("gap-sync-timeout-then-checkpoint-b");
    let (_, public_key_a) = deterministic_keypair_hex(252);
    let (_, public_key_b) = deterministic_keypair_hex(253);
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", 252)],
    );
    let replication_config_a = signed_replication_config(dir_a.clone(), 252)
        .with_remote_writer_allowlist(vec![public_key_b.clone()])
        .expect("allowlist a")
        .with_fetch_requester_allowlist(vec![public_key_b.clone()])
        .expect("fetch allowlist a");
    let replication_config_b = signed_replication_config(dir_b.clone(), 253)
        .with_remote_writer_allowlist(vec![public_key_a])
        .expect("allowlist b")
        .with_fetch_requester_allowlist(vec![public_key_b])
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
        .with_replication(replication_config_b.clone());

    let checkpoint_height = REPLICATION_GAP_SYNC_MAX_HEIGHTS_PER_POLL;
    let advertised_height = checkpoint_height + 2;
    let mut replication_a =
        ReplicationRuntime::new(config_a.replication.as_ref().expect("repl a"), "node-a")
            .expect("runtime a");
    let execution_block_hash = format!("exec-block-{checkpoint_height}");
    let execution_state_root = format!("exec-state-{checkpoint_height}");
    let checkpoint_bundle = test_execution_checkpoint_bundle(
        checkpoint_height,
        execution_block_hash.as_str(),
        execution_state_root.as_str(),
    );
    let message = replication_a
        .build_local_commit_message_with_checkpoint(
            "node-a",
            world_id,
            7_000,
            &committed_decision(checkpoint_height, 100, 67),
            Some(execution_block_hash.as_str()),
            Some(execution_state_root.as_str()),
            Some(checkpoint_bundle.clone()),
        )
        .expect("build checkpoint message")
        .expect("checkpoint message");

    let network_impl = Arc::new(TimeoutThenCheckpointNetwork {
        advertised_height,
        ..Default::default()
    });
    network_impl
        .messages
        .lock()
        .expect("lock messages")
        .insert(checkpoint_height, message.clone());
    let mut blobs = network_impl.blobs.lock().expect("lock blobs");
    blobs.insert(message.record.content_hash.clone(), message.payload.clone());
    blobs.insert(
        oasis7_distfs::blake3_hex(message.payload.as_slice()),
        message.payload.clone(),
    );
    blobs.insert(
        oasis7_distfs::blake3_hex(checkpoint_bundle.manifest_json.as_slice()),
        checkpoint_bundle.manifest_json.clone(),
    );
    for blob in &checkpoint_bundle.blobs {
        blobs.insert(blob.content_hash.clone(), blob.bytes.clone());
    }
    drop(blobs);

    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = network_impl.clone();
    let endpoint_b = ReplicationNetworkEndpoint::new(
        &NodeReplicationNetworkHandle::new(network),
        world_id,
        false,
        &config_b.network_policy,
    )
    .expect("endpoint b");
    let mut replication_b =
        ReplicationRuntime::new(config_b.replication.as_ref().expect("repl b"), "node-b")
            .expect("runtime b");
    let mut engine_b = PosNodeEngine::new(&config_b).expect("engine b");
    engine_b.network_committed_height = advertised_height;
    let mut install_hook = CheckpointInstallingExecutionHook {
        installed: Vec::new(),
    };

    engine_b
        .sync_missing_replication_commits(
            &endpoint_b,
            "node-b",
            world_id,
            Some(&mut replication_b),
            Some(&mut install_hook),
        )
        .expect("checkpoint candidate fallback after protocol-omitted timeout");

    assert_eq!(install_hook.installed, vec![checkpoint_height]);
    assert_eq!(engine_b.committed_height, checkpoint_height);
    assert_eq!(engine_b.replication_persisted_height, checkpoint_height);
    let attempts = network_impl.attempts.lock().expect("lock attempts").clone();
    assert!(
        attempts.starts_with(&[advertised_height, checkpoint_height]),
        "expected advertised height timeout before checkpoint candidate, got {attempts:?}"
    );

    let _ = fs::remove_dir_all(&dir_a);
    let _ = fs::remove_dir_all(&dir_b);
}

#[test]
fn observer_gap_sync_uses_network_height_when_dht_head_lookup_times_out() {
    let world_id = "world-gap-sync-dht-timeout-then-checkpoint";
    let dir_a = temp_dir("gap-sync-dht-timeout-then-checkpoint-a");
    let dir_b = temp_dir("gap-sync-dht-timeout-then-checkpoint-b");
    let (_, public_key_a) = deterministic_keypair_hex(254);
    let (_, public_key_b) = deterministic_keypair_hex(255);
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", 254)],
    );
    let replication_config_a = signed_replication_config(dir_a.clone(), 254)
        .with_remote_writer_allowlist(vec![public_key_b.clone()])
        .expect("allowlist a")
        .with_fetch_requester_allowlist(vec![public_key_b.clone()])
        .expect("fetch allowlist a");
    let replication_config_b = signed_replication_config(dir_b.clone(), 255)
        .with_remote_writer_allowlist(vec![public_key_a])
        .expect("allowlist b")
        .with_fetch_requester_allowlist(vec![public_key_b])
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
        .with_replication(replication_config_b.clone());

    let checkpoint_height = REPLICATION_GAP_SYNC_MAX_HEIGHTS_PER_POLL;
    let advertised_height = checkpoint_height + 2;
    let mut replication_a =
        ReplicationRuntime::new(config_a.replication.as_ref().expect("repl a"), "node-a")
            .expect("runtime a");
    let execution_block_hash = format!("exec-block-{checkpoint_height}");
    let execution_state_root = format!("exec-state-{checkpoint_height}");
    let checkpoint_bundle = test_execution_checkpoint_bundle(
        checkpoint_height,
        execution_block_hash.as_str(),
        execution_state_root.as_str(),
    );
    let message = replication_a
        .build_local_commit_message_with_checkpoint(
            "node-a",
            world_id,
            7_500,
            &committed_decision(checkpoint_height, 100, 67),
            Some(execution_block_hash.as_str()),
            Some(execution_state_root.as_str()),
            Some(checkpoint_bundle.clone()),
        )
        .expect("build checkpoint message")
        .expect("checkpoint message");

    let network_impl = Arc::new(TimeoutThenCheckpointNetwork {
        advertised_height,
        ..Default::default()
    });
    network_impl
        .messages
        .lock()
        .expect("lock messages")
        .insert(checkpoint_height, message.clone());
    let mut blobs = network_impl.blobs.lock().expect("lock blobs");
    blobs.insert(message.record.content_hash.clone(), message.payload.clone());
    blobs.insert(
        oasis7_distfs::blake3_hex(message.payload.as_slice()),
        message.payload.clone(),
    );
    blobs.insert(
        oasis7_distfs::blake3_hex(checkpoint_bundle.manifest_json.as_slice()),
        checkpoint_bundle.manifest_json.clone(),
    );
    for blob in &checkpoint_bundle.blobs {
        blobs.insert(blob.content_hash.clone(), blob.bytes.clone());
    }
    drop(blobs);

    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = network_impl.clone();
    let dht: Arc<dyn DistributedDht<WorldError> + Send + Sync> =
        Arc::new(TimeoutWorldHeadDht);
    let endpoint_b = ReplicationNetworkEndpoint::new(
        &NodeReplicationNetworkHandle::new(network).with_dht(dht),
        world_id,
        false,
        &config_b.network_policy,
    )
    .expect("endpoint b");
    let mut replication_b =
        ReplicationRuntime::new(config_b.replication.as_ref().expect("repl b"), "node-b")
            .expect("runtime b");
    let mut engine_b = PosNodeEngine::new(&config_b).expect("engine b");
    engine_b.network_committed_height = advertised_height;
    let mut install_hook = CheckpointInstallingExecutionHook {
        installed: Vec::new(),
    };

    engine_b
        .sync_missing_replication_commits(
            &endpoint_b,
            "node-b",
            world_id,
            Some(&mut replication_b),
            Some(&mut install_hook),
        )
        .expect("network-height high checkpoint sync after dht timeout");

    assert_eq!(install_hook.installed, vec![checkpoint_height]);
    assert_eq!(engine_b.committed_height, checkpoint_height);
    assert_eq!(engine_b.replication_persisted_height, checkpoint_height);
    let attempts = network_impl.attempts.lock().expect("lock attempts").clone();
    assert!(
        attempts.starts_with(&[advertised_height, checkpoint_height]),
        "expected network height probe before checkpoint candidate after dht timeout, got {attempts:?}"
    );

    let _ = fs::remove_dir_all(&dir_a);
    let _ = fs::remove_dir_all(&dir_b);
}
