use super::*;
use crate::node_engine_replication_provider_route::REPLICATION_GAP_SYNC_FETCH_BLOB_RATE_LIMIT_COOLDOWN_MS;
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

struct TimeoutThenCheckpointNetwork {
    messages: Mutex<BTreeMap<u64, replication::GossipReplicationMessage>>,
    blobs: Mutex<BTreeMap<String, Vec<u8>>>,
    attempts: Mutex<Vec<u64>>,
    provider_attempts: Mutex<Vec<String>>,
    rate_limited_providers: Mutex<BTreeSet<String>>,
    not_found_providers: Mutex<BTreeSet<String>>,
    connected_peers: Mutex<Vec<String>>,
    advertised_height: u64,
}

impl Default for TimeoutThenCheckpointNetwork {
    fn default() -> Self {
        Self {
            messages: Mutex::default(),
            blobs: Mutex::default(),
            attempts: Mutex::default(),
            provider_attempts: Mutex::default(),
            rate_limited_providers: Mutex::default(),
            not_found_providers: Mutex::default(),
            connected_peers: Mutex::new(vec!["peer-a".to_string()]),
            advertised_height: 0,
        }
    }
}

#[derive(Default)]
struct TimeoutWorldHeadDht;

struct CheckpointProviderDht {
    providers: Mutex<Vec<String>>,
}

impl CheckpointProviderDht {
    fn set_providers(&self, providers: &[&str]) {
        *self.providers.lock().expect("lock checkpoint providers") = providers
            .iter()
            .map(|provider| (*provider).to_string())
            .collect();
    }
}

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

impl DistributedDht<WorldError> for CheckpointProviderDht {
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
        let providers = self.providers.lock().expect("lock checkpoint providers");
        Ok(providers
            .iter()
            .enumerate()
            .map(|(index, provider_id)| ProviderRecord {
                provider_id: provider_id.clone(),
                last_seen_ms: i64::try_from(providers.len() - index)
                    .expect("provider index fits i64"),
                storage_total_bytes: None,
                storage_available_bytes: None,
                uptime_ratio_per_mille: None,
                challenge_pass_ratio_per_mille: None,
                load_ratio_per_mille: None,
                p50_read_latency_ms: None,
            })
            .collect())
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
        providers: &[String],
    ) -> Result<Vec<u8>, WorldError> {
        if protocol == replication::REPLICATION_GET_HEAD_PROTOCOL {
            return Err(WorldError::NetworkProtocolUnavailable {
                protocol: "request failed: Timeout".to_string(),
            });
        }
        if protocol == replication::REPLICATION_FETCH_BLOB_PROTOCOL {
            if let Some(provider_id) = providers.first() {
                self.provider_attempts
                    .lock()
                    .expect("lock provider attempts")
                    .push(provider_id.clone());
                if self
                    .not_found_providers
                    .lock()
                    .expect("lock not-found providers")
                    .contains(provider_id)
                {
                    return self.fetch_blob_not_found_response();
                }
                if self
                    .rate_limited_providers
                    .lock()
                    .expect("lock rate-limited providers")
                    .contains(provider_id)
                {
                    return Err(WorldError::NetworkRequestFailed {
                        code: DistributedErrorCode::ErrRateLimited,
                        message: format!(
                            "fetch-blob response budget exhausted for provider={provider_id}"
                        ),
                        retryable: true,
                    });
                }
            }
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
        self.connected_peers
            .lock()
            .expect("lock connected peers")
            .clone()
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
    fn set_rate_limited_providers(&self, providers: &[&str]) {
        *self
            .rate_limited_providers
            .lock()
            .expect("lock rate-limited providers") = providers
            .iter()
            .map(|provider| (*provider).to_string())
            .collect();
    }

    fn set_not_found_providers(&self, providers: &[&str]) {
        *self
            .not_found_providers
            .lock()
            .expect("lock not-found providers") = providers
            .iter()
            .map(|provider| (*provider).to_string())
            .collect();
    }

    fn set_connected_peers(&self, peers: &[&str]) {
        *self.connected_peers.lock().expect("lock connected peers") =
            peers.iter().map(|peer| (*peer).to_string()).collect();
    }

    fn provider_attempts(&self) -> Vec<String> {
        self.provider_attempts
            .lock()
            .expect("lock provider attempts")
            .clone()
    }

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

    fn fetch_blob_not_found_response(&self) -> Result<Vec<u8>, WorldError> {
        serde_json::to_vec(&replication::FetchBlobResponse {
            found: false,
            range_offset_bytes: None,
            range_complete: None,
            blob: None,
        })
        .map_err(|err| WorldError::DistributedValidationFailed {
            reason: format!("encode fetch-blob not-found response failed: {err}"),
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
            proposer_id: "node-a".to_string(),
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

struct ProviderRateLimitCheckpointFixture {
    dir_a: PathBuf,
    dir_b: PathBuf,
    network: Arc<TimeoutThenCheckpointNetwork>,
    dht: Arc<CheckpointProviderDht>,
    endpoint: ReplicationNetworkEndpoint,
    replication: ReplicationRuntime,
    engine: PosNodeEngine,
    checkpoint_height: u64,
    manifest_hash: String,
    manifest_bytes: Vec<u8>,
}

impl ProviderRateLimitCheckpointFixture {
    fn cleanup(&self) {
        let _ = fs::remove_dir_all(&self.dir_a);
        let _ = fs::remove_dir_all(&self.dir_b);
    }
}

fn provider_rate_limit_checkpoint_fixture(
    world_id: &str,
    seed_a: u8,
    seed_b: u8,
) -> ProviderRateLimitCheckpointFixture {
    let dir_a = temp_dir("gap-sync-provider-rate-limit-a");
    let dir_b = temp_dir("gap-sync-provider-rate-limit-b");
    let (_, public_key_a) = deterministic_keypair_hex(seed_a);
    let (_, public_key_b) = deterministic_keypair_hex(seed_b);
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", seed_a)],
    );
    let replication_config_a = signed_replication_config(dir_a.clone(), seed_a)
        .with_remote_writer_allowlist(vec![public_key_b.clone()])
        .expect("allowlist a")
        .with_fetch_requester_allowlist(vec![public_key_b.clone()])
        .expect("fetch allowlist a");
    let replication_config_b = signed_replication_config(dir_b.clone(), seed_b)
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
    let advertised_height = checkpoint_height + 1;
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
    let manifest_hash = oasis7_distfs::blake3_hex(checkpoint_bundle.manifest_json.as_slice());
    let message = replication_a
        .build_local_commit_message_with_checkpoint(
            "node-a",
            world_id,
            8_000,
            &committed_decision(checkpoint_height, 100, 67),
            Some(execution_block_hash.as_str()),
            Some(execution_state_root.as_str()),
            Some(checkpoint_bundle.clone()),
        )
        .expect("build checkpoint message")
        .expect("checkpoint message");
    let network = Arc::new(TimeoutThenCheckpointNetwork {
        advertised_height,
        connected_peers: Mutex::new(vec!["peer-a".to_string()]),
        ..Default::default()
    });
    network
        .messages
        .lock()
        .expect("lock messages")
        .insert(checkpoint_height, message.clone());
    let mut blobs = network.blobs.lock().expect("lock blobs");
    blobs.insert(message.record.content_hash.clone(), message.payload.clone());
    blobs.insert(manifest_hash.clone(), checkpoint_bundle.manifest_json.clone());
    for blob in &checkpoint_bundle.blobs {
        blobs.insert(blob.content_hash.clone(), blob.bytes.clone());
    }
    drop(blobs);
    let distributed_network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = network.clone();
    let dht = Arc::new(CheckpointProviderDht {
        providers: Mutex::new(vec!["provider-a".to_string(), "provider-b".to_string()]),
    });
    let endpoint = ReplicationNetworkEndpoint::new(
        &NodeReplicationNetworkHandle::new(distributed_network).with_dht(dht.clone()),
        world_id,
        false,
        &config_b.network_policy,
    )
    .expect("endpoint b");

    ProviderRateLimitCheckpointFixture {
        dir_a,
        dir_b,
        network,
        dht,
        endpoint,
        replication: ReplicationRuntime::new(
            config_b.replication.as_ref().expect("repl b"),
            "node-b",
        )
        .expect("runtime b"),
        engine: {
            let mut engine = PosNodeEngine::new(&config_b).expect("engine b");
            engine.network_committed_height = advertised_height;
            engine
        },
        checkpoint_height,
        manifest_hash,
        manifest_bytes: checkpoint_bundle.manifest_json,
    }
}

#[test]
fn high_checkpoint_rate_limited_provider_falls_through_to_next_advertised_provider() {
    let _nonce_lock =
        super::storage_replication_first_ready_checkpoint_tests::lock_checkpoint_probe_nonce();
    let world_id = "world-gap-sync-provider-rate-limit-failover";
    let mut fixture = provider_rate_limit_checkpoint_fixture(world_id, 246, 247);
    fixture.network.set_rate_limited_providers(&["provider-a"]);
    let mut install_hook = CheckpointInstallingExecutionHook { installed: Vec::new() };

    fixture
        .engine
        .sync_missing_replication_commits(
            &fixture.endpoint,
            "node-b",
            world_id,
            Some(&mut fixture.replication),
            Some(&mut install_hook),
        )
        .expect("provider-b should complete checkpoint install after provider-a rate limit");

    assert_eq!(
        install_hook.installed,
        vec![fixture.checkpoint_height],
        "checkpoint sync did not install; committed_height={} persisted_height={} provider_attempts={:?}",
        fixture.engine.committed_height,
        fixture.engine.replication_persisted_height,
        fixture.network.provider_attempts(),
    );
    assert!(
        fixture
            .network
            .provider_attempts()
            .chunks(2)
            .all(|attempts| attempts == ["provider-a", "provider-b"]),
        "structured provider-a rate limit must not prevent trying provider-b: {:?}",
        fixture.network.provider_attempts(),
    );
    assert_eq!(fixture.engine.last_replication_gap_sync_blocked_height, None);
    fixture.cleanup();
}

#[test]
fn high_checkpoint_stale_dht_provider_falls_through_to_connected_complete_peer() {
    let _nonce_lock =
        super::storage_replication_first_ready_checkpoint_tests::lock_checkpoint_probe_nonce();
    let world_id = "world-gap-sync-stale-dht-provider-failover";
    let mut fixture = provider_rate_limit_checkpoint_fixture(world_id, 249, 250);
    fixture.dht.set_providers(&["provider-a"]);
    fixture.network.set_not_found_providers(&["provider-a"]);
    fixture.network.set_connected_peers(&["provider-b"]);
    let mut install_hook = CheckpointInstallingExecutionHook { installed: Vec::new() };

    fixture
        .engine
        .sync_missing_replication_commits(
            &fixture.endpoint,
            "node-b",
            world_id,
            Some(&mut fixture.replication),
            Some(&mut install_hook),
        )
        .expect("connected provider-b must complete the verified checkpoint closure after stale provider-a returns not-found");

    assert_eq!(install_hook.installed, vec![fixture.checkpoint_height]);
    assert!(
        fixture
            .network
            .provider_attempts()
            .contains(&"provider-b".to_string()),
        "routing must attempt connected provider-b after stale DHT provider-a: {:?}",
        fixture.network.provider_attempts(),
    );
    assert_eq!(
        fixture.engine.last_replication_gap_sync_blocked_height,
        None
    );
    fixture.cleanup();
}

#[test]
fn high_checkpoint_rate_limited_sole_dht_provider_falls_through_to_connected_complete_peer() {
    let _nonce_lock =
        super::storage_replication_first_ready_checkpoint_tests::lock_checkpoint_probe_nonce();
    let world_id = "world-gap-sync-rate-limited-sole-dht-provider-failover";
    let mut fixture = provider_rate_limit_checkpoint_fixture(world_id, 250, 251);
    fixture.dht.set_providers(&["provider-a"]);
    fixture.network.set_rate_limited_providers(&["provider-a"]);
    fixture.network.set_connected_peers(&["provider-b"]);
    let mut install_hook = CheckpointInstallingExecutionHook { installed: Vec::new() };

    fixture
        .engine
        .sync_missing_replication_commits(
            &fixture.endpoint,
            "node-b",
            world_id,
            Some(&mut fixture.replication),
            Some(&mut install_hook),
        )
        .expect("connected provider-b must complete checkpoint closure after sole DHT provider-a is rate-limited");

    assert!(
        fixture
            .network
            .provider_attempts()
            .contains(&"provider-b".to_string()),
        "structured fetch-blob rate limit from sole advertised provider-a must not prevent connected provider-b fallback: {:?}",
        fixture.network.provider_attempts(),
    );
    assert_eq!(install_hook.installed, vec![fixture.checkpoint_height]);
    assert_eq!(
        fixture.engine.last_replication_gap_sync_blocked_height,
        None,
        "a healthy connected peer must avoid entering checkpoint rate-limit cooldown",
    );
    fixture.cleanup();
}

#[test]
fn high_checkpoint_rate_limited_sole_dht_and_connected_peer_enters_cooldown() {
    let _nonce_lock =
        super::storage_replication_first_ready_checkpoint_tests::lock_checkpoint_probe_nonce();
    let world_id = "world-gap-sync-rate-limited-sole-dht-and-connected-peer";
    let mut fixture = provider_rate_limit_checkpoint_fixture(world_id, 252, 253);
    fixture.dht.set_providers(&["provider-a"]);
    fixture
        .network
        .set_rate_limited_providers(&["provider-a", "provider-b"]);
    fixture.network.set_connected_peers(&["provider-b"]);
    let mut install_hook = CheckpointInstallingExecutionHook { installed: Vec::new() };

    fixture
        .engine
        .sync_missing_replication_commits(
            &fixture.endpoint,
            "node-b",
            world_id,
            Some(&mut fixture.replication),
            Some(&mut install_hook),
        )
        .expect("all eligible fetch routes being rate-limited should enter cooldown");

    assert_eq!(
        fixture.network.provider_attempts(),
        vec!["provider-a".to_string(), "provider-b".to_string()],
        "the sole advertised route and distinct connected fallback must both be attempted",
    );
    assert!(install_hook.installed.is_empty());
    assert_eq!(fixture.engine.last_replication_gap_sync_blocked_height, Some(1));

    fixture
        .engine
        .sync_missing_replication_commits(
            &fixture.endpoint,
            "node-b",
            world_id,
            Some(&mut fixture.replication),
            Some(&mut install_hook),
        )
        .expect("bounded cooldown should suppress an immediate retry");
    assert_eq!(fixture.network.provider_attempts().len(), 2);
    fixture.cleanup();
}

#[test]
fn high_checkpoint_without_complete_provider_stays_fail_closed_with_blob_hash() {
    let _nonce_lock =
        super::storage_replication_first_ready_checkpoint_tests::lock_checkpoint_probe_nonce();
    let world_id = "world-gap-sync-no-complete-checkpoint-provider";
    let mut fixture = provider_rate_limit_checkpoint_fixture(world_id, 251, 252);
    fixture.dht.set_providers(&["provider-a"]);
    fixture
        .network
        .set_not_found_providers(&["provider-a", "provider-b"]);
    fixture.network.set_connected_peers(&["provider-b"]);
    let mut install_hook = CheckpointInstallingExecutionHook { installed: Vec::new() };

    let err = fixture
        .engine
        .sync_missing_replication_commits(
            &fixture.endpoint,
            "node-b",
            world_id,
            Some(&mut fixture.replication),
            Some(&mut install_hook),
        )
        .expect_err(
            "checkpoint install must fail closed when no route can serve a complete closure",
        );

    assert!(install_hook.installed.is_empty());
    assert!(
        err.to_string()
            .contains("execution checkpoint blob not found hash="),
        "failure must retain an actionable missing-closure signature: {err}",
    );
    fixture.cleanup();
}

#[test]
fn high_checkpoint_all_rate_limited_providers_cool_down_then_resume_from_cached_blobs() {
    let _nonce_lock =
        super::storage_replication_first_ready_checkpoint_tests::lock_checkpoint_probe_nonce();
    let world_id = "world-gap-sync-provider-rate-limit-resume";
    let mut fixture = provider_rate_limit_checkpoint_fixture(world_id, 248, 249);
    fixture
        .replication
        .store_blob_by_hash(
            fixture.manifest_hash.as_str(),
            fixture.manifest_bytes.as_slice(),
        )
        .expect("cache checkpoint manifest");
    fixture
        .network
        .set_rate_limited_providers(&["provider-a", "provider-b"]);
    let mut install_hook = CheckpointInstallingExecutionHook {
        installed: Vec::new(),
    };

    fixture
        .engine
        .sync_missing_replication_commits(
            &fixture.endpoint,
            "node-b",
            world_id,
            Some(&mut fixture.replication),
            Some(&mut install_hook),
        )
        .expect("all-provider rate limit should enter cooldown without failing sync");
    assert!(install_hook.installed.is_empty());
    assert_eq!(
        fixture.network.provider_attempts(),
        vec!["provider-a".to_string(), "provider-b".to_string()],
        "all advertised providers should be exhausted before cooldown"
    );
    assert_eq!(
        fixture.engine.last_replication_gap_sync_blocked_height,
        Some(1),
        "checkpoint rate limit cooldown belongs to the blocked gap height"
    );

    fixture
        .engine
        .sync_missing_replication_commits(
            &fixture.endpoint,
            "node-b",
            world_id,
            Some(&mut fixture.replication),
            Some(&mut install_hook),
        )
        .expect("cooldown should safely suppress immediate retry");
    assert_eq!(fixture.network.provider_attempts().len(), 2);

    fixture.network.set_rate_limited_providers(&[]);
    fixture.engine.last_replication_gap_sync_blocked_at_ms = Some(
        crate::runtime_util::now_unix_ms()
            - REPLICATION_GAP_SYNC_FETCH_BLOB_RATE_LIMIT_COOLDOWN_MS,
    );
    fixture
        .engine
        .sync_missing_replication_commits(
            &fixture.endpoint,
            "node-b",
            world_id,
            Some(&mut fixture.replication),
            Some(&mut install_hook),
        )
        .expect("checkpoint install should resume after the cooldown using cached manifest blob");

    assert_eq!(install_hook.installed, vec![fixture.checkpoint_height]);
    assert_eq!(
        fixture.network.provider_attempts(),
        vec![
            "provider-a".to_string(),
            "provider-b".to_string(),
            "provider-a".to_string(),
            "provider-a".to_string(),
        ],
        "resume should reuse the cached manifest instead of fetching it again"
    );
    assert_eq!(fixture.engine.last_replication_gap_sync_blocked_height, None);
    fixture.cleanup();
}

#[test]
fn observer_gap_sync_continues_checkpoint_candidates_after_protocol_omitted_timeout() {
    let _nonce_lock =
        super::storage_replication_first_ready_checkpoint_tests::lock_checkpoint_probe_nonce();
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
    let _nonce_lock =
        super::storage_replication_first_ready_checkpoint_tests::lock_checkpoint_probe_nonce();
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
