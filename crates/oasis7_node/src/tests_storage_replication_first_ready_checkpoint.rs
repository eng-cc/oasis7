use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

use super::*;
use oasis7_proto::distributed::WorldHeadAnnounce;
use oasis7_proto::distributed_dht::DistributedDht;

#[derive(Clone)]
struct FirstReadyHeadCheckpointNetwork {
    inner: Arc<TestInMemoryNetwork>,
    fetch_protocols: Arc<Mutex<Vec<String>>>,
}

#[derive(Clone)]
struct PeerHeadCheckpointNetwork {
    inner: Arc<TestInMemoryNetwork>,
    fetch_protocols: Arc<Mutex<Vec<String>>>,
    head: Arc<Mutex<super::replication::FetchHeadResponse>>,
    checkpoint_fetch_available: Arc<AtomicBool>,
    checkpoint_fetch_not_found: Arc<AtomicBool>,
    connected_peer_ids: Vec<String>,
}

impl oasis7_proto::distributed_net::DistributedNetwork<WorldError>
    for FirstReadyHeadCheckpointNetwork
{
    fn publish(&self, topic: &str, payload: &[u8]) -> Result<(), WorldError> {
        self.inner.publish(topic, payload)
    }

    fn subscribe(&self, topic: &str) -> Result<NetworkSubscription, WorldError> {
        self.inner.subscribe(topic)
    }

    fn request(&self, protocol: &str, payload: &[u8]) -> Result<Vec<u8>, WorldError> {
        if protocol == REPLICATION_GET_HEAD_PROTOCOL {
            return serde_json::to_vec(&super::replication::FetchHeadResponse {
                found: false,
                head: None,
            })
            .map_err(|err| WorldError::DistributedValidationFailed {
                reason: format!("encode absent world head response failed: {err}"),
            });
        }
        self.fetch_protocols
            .lock()
            .expect("lock checkpoint fetch protocols")
            .push(protocol.to_string());
        self.inner.request(protocol, payload)
    }

    fn connected_peer_ids(&self) -> Vec<String> {
        vec!["node-a".to_string()]
    }

    fn known_peer_ids(&self) -> Vec<String> {
        vec!["node-a".to_string()]
    }

    fn register_handler(
        &self,
        protocol: &str,
        handler: Box<dyn Fn(&[u8]) -> Result<Vec<u8>, WorldError> + Send + Sync>,
    ) -> Result<(), WorldError> {
        self.inner.register_handler(protocol, handler)
    }
}

impl oasis7_proto::distributed_net::DistributedNetwork<WorldError> for PeerHeadCheckpointNetwork {
    fn publish(&self, topic: &str, payload: &[u8]) -> Result<(), WorldError> {
        self.inner.publish(topic, payload)
    }

    fn subscribe(&self, topic: &str) -> Result<NetworkSubscription, WorldError> {
        self.inner.subscribe(topic)
    }

    fn request(&self, protocol: &str, payload: &[u8]) -> Result<Vec<u8>, WorldError> {
        if protocol == REPLICATION_GET_HEAD_PROTOCOL {
            let head = self.head.lock().expect("lock peer checkpoint head").clone();
            return serde_json::to_vec(&head).map_err(|err| {
                WorldError::DistributedValidationFailed {
                    reason: format!("encode peer checkpoint head failed: {err}"),
                }
            });
        }
        if protocol == REPLICATION_FETCH_COMMIT_PROTOCOL
            && !self.checkpoint_fetch_available.load(Ordering::SeqCst)
        {
            if self.checkpoint_fetch_not_found.load(Ordering::SeqCst) {
                let request = serde_json::from_slice::<super::replication::FetchCommitRequest>(
                    payload,
                )
                .map_err(|err| WorldError::DistributedValidationFailed {
                    reason: format!("decode checkpoint fetch request failed: {err}"),
                })?;
                if request.height > 1 {
                    return serde_json::to_vec(&super::replication::FetchCommitResponse {
                        found: false,
                        message: None,
                    })
                    .map_err(|err| WorldError::DistributedValidationFailed {
                        reason: format!("encode unavailable checkpoint response failed: {err}"),
                    });
                }
            }
            if !self.checkpoint_fetch_not_found.load(Ordering::SeqCst) {
                return Err(WorldError::NetworkProtocolUnavailable {
                    protocol: protocol.to_string(),
                });
            }
        }
        self.fetch_protocols
            .lock()
            .expect("lock checkpoint fetch protocols")
            .push(protocol.to_string());
        self.inner.request(protocol, payload)
    }

    fn connected_peer_ids(&self) -> Vec<String> {
        self.connected_peer_ids.clone()
    }

    fn known_peer_ids(&self) -> Vec<String> {
        self.connected_peer_ids.clone()
    }

    fn register_handler(
        &self,
        protocol: &str,
        handler: Box<dyn Fn(&[u8]) -> Result<Vec<u8>, WorldError> + Send + Sync>,
    ) -> Result<(), WorldError> {
        self.inner.register_handler(protocol, handler)
    }
}

struct CheckpointInstallingExecutionHook {
    installed: Vec<u64>,
}

static CHECKPOINT_PROBE_NONCE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn lock_checkpoint_probe_nonce() -> std::sync::MutexGuard<'static, ()> {
    CHECKPOINT_PROBE_NONCE_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

struct CheckpointProbeNonceGuard;

impl CheckpointProbeNonceGuard {
    fn install() -> Self {
        // SAFETY: tests serialize access with CHECKPOINT_PROBE_NONCE_LOCK.
        unsafe {
            std::env::set_var(
                "OASIS7_CHECKPOINT_PROBE_NONCE",
                "probe-nonce-0123456789abcdef0123456789abcdef",
            );
        }
        Self
    }
}

impl Drop for CheckpointProbeNonceGuard {
    fn drop(&mut self) {
        // SAFETY: tests serialize access with CHECKPOINT_PROBE_NONCE_LOCK.
        unsafe {
            std::env::remove_var("OASIS7_CHECKPOINT_PROBE_NONCE");
        }
    }
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

struct BootstrapBeforeIncrementalHook {
    installed: Vec<u64>,
    incremental_commits: Vec<u64>,
    rollback_heights: Vec<u64>,
}

impl NodeExecutionHook for BootstrapBeforeIncrementalHook {
    fn on_commit(
        &mut self,
        context: NodeExecutionCommitContext,
    ) -> Result<NodeExecutionCommitResult, String> {
        self.incremental_commits.push(context.height);
        Ok(NodeExecutionCommitResult {
            execution_height: context.height,
            execution_block_hash: format!("local-execution-block-{}", context.height),
            execution_state_root: format!("local-execution-state-{}", context.height),
        })
    }

    fn restore_to_height(&mut self, _world_id: &str, height: u64) -> Result<bool, String> {
        self.rollback_heights.push(height);
        Ok(false)
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

fn checkpoint_bundle(
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

fn committed_decision(height: u64) -> PosDecision {
    PosDecision {
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
    }
}

#[test]
fn fresh_observer_discovers_checkpoint_after_first_ready_replication_head() {
    let _nonce_lock = lock_checkpoint_probe_nonce();
    let world_id = "world-gap-sync-first-ready-checkpoint-head";
    let dir_a = temp_dir("gap-sync-first-ready-checkpoint-head-a");
    let dir_b = temp_dir("gap-sync-first-ready-checkpoint-head-b");
    let (_, public_key_a) = deterministic_keypair_hex(182);
    let (_, public_key_b) = deterministic_keypair_hex(183);
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", 182)],
    );
    let replication_config_a = signed_replication_config(dir_a.clone(), 182)
        .with_remote_writer_allowlist(vec![public_key_b.clone()])
        .expect("allowlist a")
        .with_fetch_requester_allowlist(vec![public_key_b])
        .expect("fetch allowlist a");
    let replication_config_b = signed_replication_config(dir_b.clone(), 183)
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
    let checkpoint_height = 3;
    let execution_block_hash = format!("exec-block-{checkpoint_height}");
    let execution_state_root = format!("exec-state-{checkpoint_height}");
    let mut replication_a =
        ReplicationRuntime::new(config_a.replication.as_ref().expect("repl a"), "node-a")
            .expect("runtime a");
    let checkpoint_message = replication_a
        .build_local_commit_message_with_checkpoint(
            "node-a",
            world_id,
            5_800,
            &committed_decision(checkpoint_height),
            Some(execution_block_hash.as_str()),
            Some(execution_state_root.as_str()),
            Some(checkpoint_bundle(
                checkpoint_height,
                execution_block_hash.as_str(),
                execution_state_root.as_str(),
            )),
        )
        .expect("build first ready checkpoint head")
        .expect("checkpoint message");
    let fetch_protocols = Arc::new(Mutex::new(Vec::new()));
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(FirstReadyHeadCheckpointNetwork {
        inner: Arc::new(TestInMemoryNetwork::default()),
        fetch_protocols: Arc::clone(&fetch_protocols),
    });
    register_replication_fetch_handlers(
        &NodeReplicationNetworkHandle::new(Arc::clone(&network)),
        &replication_config_a,
        world_id,
        &config_a.network_policy,
    )
    .expect("register ready provider checkpoint handlers");
    let handle_b = NodeReplicationNetworkHandle::new(Arc::clone(&network));
    let mut endpoint_b =
        ReplicationNetworkEndpoint::new(&handle_b, world_id, true, &config_b.network_policy)
            .expect("fresh observer endpoint");
    let mut replication_b =
        ReplicationRuntime::new(config_b.replication.as_ref().expect("repl b"), "node-b")
            .expect("fresh observer runtime");
    let mut engine_b = PosNodeEngine::new(&config_b).expect("fresh observer engine");
    let mut execution_hook = CheckpointInstallingExecutionHook {
        installed: Vec::new(),
    };
    assert_eq!(engine_b.committed_height, 0, "fresh observer starts at height zero");
    assert_eq!(engine_b.replication_persisted_height, 0);
    endpoint_b
        .publish_replication(&checkpoint_message)
        .expect("publish first ready checkpoint head");
    engine_b
        .tick(
            "node-b",
            world_id,
            6_000,
            None,
            Some(&mut replication_b),
            Some(&mut endpoint_b),
            None,
            Vec::new(),
            Some(&mut execution_hook),
        )
        .expect("first ready head should trigger checkpoint discovery and fetch");
    assert_eq!(
        engine_b.network_committed_height, checkpoint_height,
        "the fresh observer must first record the ready network head"
    );
    assert_eq!(execution_hook.installed, vec![checkpoint_height]);
    assert_eq!(engine_b.committed_height, checkpoint_height);
    assert_eq!(engine_b.replication_persisted_height, checkpoint_height);
    assert_eq!(engine_b.last_execution_height, checkpoint_height);
    let fetch_protocols = fetch_protocols
        .lock()
        .expect("lock observed checkpoint fetch protocols")
        .clone();
    assert!(
        fetch_protocols
            .iter()
            .any(|protocol| protocol == REPLICATION_FETCH_COMMIT_PROTOCOL),
        "ready connected provider path must fetch the checkpoint descriptor: {fetch_protocols:?}"
    );
    assert!(
        fetch_protocols
            .iter()
            .any(|protocol| protocol == REPLICATION_FETCH_BLOB_PROTOCOL),
        "ready connected provider path must fetch the checkpoint closure: {fetch_protocols:?}"
    );
    assert!(
        replication_b
            .load_commit_message_by_height(world_id, 1)
            .expect("inspect absent low history")
            .is_none(),
        "checkpoint discovery must not require a locally replayable height-one commit"
    );
    let _ = fs::remove_dir_all(&dir_a);
    let _ = fs::remove_dir_all(&dir_b);
}

#[test]
fn checkpoint_receipt_keeps_connected_provider_provenance_without_reinstall_loop() {
    let _nonce_lock = lock_checkpoint_probe_nonce();
    let _probe_nonce = CheckpointProbeNonceGuard::install();
    let world_id = "world-checkpoint-connected-candidate-receipt";
    let dir_a = temp_dir("checkpoint-connected-candidate-receipt-a");
    let dir_b = temp_dir("checkpoint-connected-candidate-receipt-b");
    let (_, public_key_a) = deterministic_keypair_hex(186);
    let (_, public_key_b) = deterministic_keypair_hex(187);
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", 186)],
    );
    let replication_config_a = signed_replication_config(dir_a.clone(), 186)
        .with_remote_writer_allowlist(vec![public_key_b.clone()])
        .expect("allowlist a")
        .with_fetch_requester_allowlist(vec![public_key_b])
        .expect("fetch allowlist a");
    let replication_config_b = signed_replication_config(dir_b.clone(), 187)
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
    let checkpoint_height = 3;
    let execution_block_hash = format!("exec-block-{checkpoint_height}");
    let execution_state_root = format!("exec-state-{checkpoint_height}");
    let mut replication_a =
        ReplicationRuntime::new(config_a.replication.as_ref().expect("repl a"), "node-a")
            .expect("runtime a");
    let checkpoint_message = replication_a
        .build_local_commit_message_with_checkpoint(
            "node-a",
            world_id,
            6_100,
            &committed_decision(checkpoint_height),
            Some(execution_block_hash.as_str()),
            Some(execution_state_root.as_str()),
            Some(checkpoint_bundle(
                checkpoint_height,
                execution_block_hash.as_str(),
                execution_state_root.as_str(),
            )),
        )
        .expect("build checkpoint message")
        .expect("checkpoint message");
    let checkpoint_payload = super::replication_state_reconcile::parse_replication_commit_payload(
        checkpoint_message.payload.as_slice(),
    )
    .expect("decode checkpoint payload");
    let checkpoint_descriptor = checkpoint_payload
        .execution_checkpoint
        .expect("checkpoint descriptor");
    let fetch_protocols = Arc::new(Mutex::new(Vec::new()));
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(FirstReadyHeadCheckpointNetwork {
        inner: Arc::new(TestInMemoryNetwork::default()),
        fetch_protocols: Arc::clone(&fetch_protocols),
    });
    register_replication_fetch_handlers(
        &NodeReplicationNetworkHandle::new(Arc::clone(&network)),
        &replication_config_a,
        world_id,
        &config_a.network_policy,
    )
    .expect("register checkpoint fetch handlers");
    let dht = Arc::new(TestReplicaMaintenanceDht::new(
        "node-a-provider",
        "node-b-provider",
    ));
    dht.seed_provider(
        checkpoint_descriptor.manifest_ref.as_str(),
        "node-a-provider",
    );
    for blob in &checkpoint_descriptor.blobs {
        dht.seed_provider(blob.content_hash.as_str(), "node-a-provider");
    }
    let handle_b = NodeReplicationNetworkHandle::new(Arc::clone(&network))
        .with_dht(dht)
        .with_local_provider_id("node-b-provider");
    let endpoint_b =
        ReplicationNetworkEndpoint::new(&handle_b, world_id, false, &config_b.network_policy)
            .expect("fresh observer endpoint");
    let mut replication_b =
        ReplicationRuntime::new(config_b.replication.as_ref().expect("repl b"), "node-b")
            .expect("fresh observer runtime");
    let mut engine_b = PosNodeEngine::new(&config_b).expect("fresh observer engine");
    engine_b.network_committed_height = checkpoint_height;
    let mut execution_hook = CheckpointInstallingExecutionHook {
        installed: Vec::new(),
    };

    engine_b
        .sync_missing_replication_commits(
            &endpoint_b,
            "node-b",
            world_id,
            Some(&mut replication_b),
            Some(&mut execution_hook),
        )
        .expect("connected checkpoint fetch must persist its receipt");

    let receipt_path = dir_b
        .join("checkpoint-verification")
        .join(format!("{checkpoint_height}.json"));
    let receipt: serde_json::Value = serde_json::from_slice(
        fs::read(&receipt_path).expect("read checkpoint verification receipt").as_slice(),
    )
    .expect("decode checkpoint verification receipt");
    assert_eq!(execution_hook.installed, vec![checkpoint_height]);
    assert!(
        receipt["fetch_observations"]
            .as_array()
            .expect("receipt observations")
            .iter()
            .all(|observation| {
                observation["connected_candidate_ids"]
                    .as_array()
                    .is_some_and(|candidates| {
                        candidates.iter().any(|candidate| {
                            candidate.as_str() == Some("node-a-provider")
                        })
                    })
            }),
        "every signed network fetch must retain its connected provider candidate: {receipt}"
    );
    assert!(
        receipt["fetch_observations"]
            .as_array()
            .expect("receipt observations")
            .iter()
            .zip(receipt["objects"].as_array().expect("receipt closure objects"))
            .all(|(observation, object)| {
                observation["source"].as_str() == Some("network_fetch")
                    && observation["signed_request"].as_bool() == Some(true)
                    && observation["response_found"].as_bool() == Some(true)
                    && observation["content_hash"] == object["expected_content_hash"]
                    && observation["observed_content_hash"] == object["observed_content_hash"]
                    && observation["observed_size_bytes"] == object["observed_size_bytes"]
            }),
        "receipt observations must bind every fetched closure object to its signed response: {receipt}"
    );

    engine_b
        .sync_missing_replication_commits(
            &endpoint_b,
            "node-b",
            world_id,
            Some(&mut replication_b),
            Some(&mut execution_hook),
        )
        .expect("receipt-backed checkpoint must not reinstall on the next sync");
    assert_eq!(
        execution_hook.installed,
        vec![checkpoint_height],
        "a successful receipt must stop repeated checkpoint installs"
    );
    assert!(
        fetch_protocols
            .lock()
            .expect("lock fetch protocols")
            .iter()
            .any(|protocol| protocol == REPLICATION_FETCH_BLOB_PROTOCOL),
        "receipt must be based on fetched checkpoint closure"
    );
    let _ = fs::remove_dir_all(&dir_a);
    let _ = fs::remove_dir_all(&dir_b);
}

#[test]
fn checkpoint_receipt_recovers_finalization_failure_without_reinstalling_generic_connected_closure() {
    let _nonce_lock = lock_checkpoint_probe_nonce();
    let _probe_nonce = CheckpointProbeNonceGuard::install();
    let world_id = "world-checkpoint-generic-connected-candidate";
    let dir_a = temp_dir("checkpoint-generic-connected-candidate-a");
    let dir_b = temp_dir("checkpoint-generic-connected-candidate-b");
    let (_, public_key_a) = deterministic_keypair_hex(188);
    let (_, public_key_b) = deterministic_keypair_hex(189);
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", 188)],
    );
    let replication_config_a = signed_replication_config(dir_a.clone(), 188)
        .with_remote_writer_allowlist(vec![public_key_b.clone()])
        .expect("allowlist a")
        .with_fetch_requester_allowlist(vec![public_key_b])
        .expect("fetch allowlist a");
    let replication_config_b = signed_replication_config(dir_b.clone(), 189)
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
    let checkpoint_height = 4;
    let execution_block_hash = format!("exec-block-{checkpoint_height}");
    let execution_state_root = format!("exec-state-{checkpoint_height}");
    let mut replication_a =
        ReplicationRuntime::new(config_a.replication.as_ref().expect("repl a"), "node-a")
            .expect("runtime a");
    let checkpoint_message = replication_a
        .build_local_commit_message_with_checkpoint(
            "node-a",
            world_id,
            6_200,
            &committed_decision(checkpoint_height),
            Some(execution_block_hash.as_str()),
            Some(execution_state_root.as_str()),
            Some(checkpoint_bundle(
                checkpoint_height,
                execution_block_hash.as_str(),
                execution_state_root.as_str(),
            )),
        )
        .expect("build checkpoint message")
        .expect("checkpoint message");
    let checkpoint_descriptor = super::replication_state_reconcile::parse_replication_commit_payload(
        checkpoint_message.payload.as_slice(),
    )
    .expect("decode checkpoint payload")
    .execution_checkpoint
    .expect("checkpoint descriptor");
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(FirstReadyHeadCheckpointNetwork {
        inner: Arc::new(TestInMemoryNetwork::default()),
        fetch_protocols: Arc::new(Mutex::new(Vec::new())),
    });
    register_replication_fetch_handlers(
        &NodeReplicationNetworkHandle::new(Arc::clone(&network)),
        &replication_config_a,
        world_id,
        &config_a.network_policy,
    )
    .expect("register checkpoint fetch handlers");
    // The generic connected route succeeds even before a DHT provider record is visible.
    let handle_b = NodeReplicationNetworkHandle::new(Arc::clone(&network))
        .with_dht(Arc::new(TestReplicaMaintenanceDht::new("unused-provider", "node-b")))
        .with_local_provider_id("node-b");
    let endpoint_b =
        ReplicationNetworkEndpoint::new(&handle_b, world_id, false, &config_b.network_policy)
            .expect("fresh observer endpoint");
    let mut replication_b =
        ReplicationRuntime::new(config_b.replication.as_ref().expect("repl b"), "node-b")
            .expect("fresh observer runtime");
    let mut engine_b = PosNodeEngine::new(&config_b).expect("fresh observer engine");
    engine_b.network_committed_height = checkpoint_height;
    let mut execution_hook = CheckpointInstallingExecutionHook {
        installed: Vec::new(),
    };
    let receipt_path = dir_b
        .join("checkpoint-verification")
        .join(format!("{checkpoint_height}.json"));
    // A directory at the publication path deterministically makes the
    // no-overwrite hard-link finalization fail after the closure is installed.
    fs::create_dir_all(&receipt_path).expect("block initial receipt publication");

    let first_sync = engine_b
        .sync_missing_replication_commits(
            &endpoint_b,
            "node-b",
            world_id,
            Some(&mut replication_b),
            Some(&mut execution_hook),
        )
        .expect_err("initial receipt finalization must fail after checkpoint installation");
    assert!(
        first_sync.to_string().contains("publish checkpoint verification receipt"),
        "expected receipt publication failure, got: {first_sync}"
    );
    assert_eq!(
        engine_b.last_execution_height, checkpoint_height,
        "the failed finalization must retain the installed checkpoint identity for retry"
    );
    fs::remove_dir(&receipt_path).expect("unblock receipt publication for retry");
    let second_sync = engine_b
        .sync_missing_replication_commits(
            &endpoint_b,
            "node-b",
            world_id,
            Some(&mut replication_b),
            Some(&mut execution_hook),
        )
        .expect("retry must finalize the receipt without reinstalling the closure");

    let _ = second_sync;

    assert_eq!(
        execution_hook.installed,
        vec![checkpoint_height],
        "an installed checkpoint must not be installed again while provenance receipt finalization is pending"
    );
    let receipt: serde_json::Value = serde_json::from_slice(
        fs::read(&receipt_path).expect("read checkpoint verification receipt").as_slice(),
    )
    .expect("decode checkpoint verification receipt");
    assert!(
        receipt["fetch_observations"]
            .as_array()
            .expect("receipt observations")
            .iter()
            .all(|observation| {
                observation["source"].as_str() == Some("network_fetch")
                    && observation["signed_request"].as_bool() == Some(true)
                    && observation["connected_candidate_ids"]
                        .as_array()
                        .is_some_and(|candidates| candidates.iter().any(|id| id.as_str() == Some("node-a")))
            }),
        "generic fetches must retain the actual connected candidate, not only a DHT provider hint: {receipt}"
    );
    assert_eq!(
        receipt["objects"].as_array().expect("receipt objects").len(),
        checkpoint_descriptor.blobs.len() + 1,
        "receipt must bind the manifest and every checkpoint blob"
    );
    assert!(
        receipt["fetch_observations"]
            .as_array()
            .expect("receipt observations")
            .iter()
            .zip(receipt["objects"].as_array().expect("receipt closure objects"))
            .all(|(observation, object)| {
                observation["response_found"].as_bool() == Some(true)
                    && observation["content_hash"] == object["expected_content_hash"]
                    && observation["observed_content_hash"] == object["observed_content_hash"]
                    && observation["observed_size_bytes"] == object["observed_size_bytes"]
            }),
        "recovered runtime receipt must retain response and hash/size bindings: {receipt}"
    );
    let _ = fs::remove_dir_all(&dir_a);
    let _ = fs::remove_dir_all(&dir_b);
}

#[test]
fn fresh_observer_bootstraps_checkpoint_at_boundary_before_height_one_peer_mismatch() {
    let _nonce_lock = lock_checkpoint_probe_nonce();
    let world_id = "world-gap-sync-bootstrap-before-height-one";
    let dir_a = temp_dir("gap-sync-bootstrap-before-height-one-a");
    let dir_b = temp_dir("gap-sync-bootstrap-before-height-one-b");
    let (_, public_key_a) = deterministic_keypair_hex(184);
    let (_, public_key_b) = deterministic_keypair_hex(185);
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", 184)],
    );
    let replication_config_a = signed_replication_config(dir_a.clone(), 184)
        .with_remote_writer_allowlist(vec![public_key_b.clone()])
        .expect("allowlist a")
        .with_fetch_requester_allowlist(vec![public_key_b])
        .expect("fetch allowlist a");
    let replication_config_b = signed_replication_config(dir_b.clone(), 185)
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
        .with_require_execution_on_commit(true)
        .with_replication(replication_config_b.clone());
    let checkpoint_height = 64;
    let mut replication_a =
        ReplicationRuntime::new(config_a.replication.as_ref().expect("repl a"), "node-a")
            .expect("runtime a");
    let incremental_message = replication_a
        .build_local_commit_message(
            "node-a",
            world_id,
            5_900,
            &committed_decision(1),
            Some("exec-block-1"),
            Some("exec-state-1"),
        )
        .expect("build incompatible height-one tail")
        .expect("height-one message");
    let checkpoint_block_hash = format!("exec-block-{checkpoint_height}");
    let checkpoint_state_root = format!("exec-state-{checkpoint_height}");
    let checkpoint_message = replication_a
        .build_local_commit_message_with_checkpoint(
            "node-a",
            world_id,
            5_963,
            &committed_decision(checkpoint_height),
            Some(checkpoint_block_hash.as_str()),
            Some(checkpoint_state_root.as_str()),
            Some(checkpoint_bundle(
                checkpoint_height,
                checkpoint_block_hash.as_str(),
                checkpoint_state_root.as_str(),
            )),
        )
        .expect("build checkpoint boundary")
        .expect("checkpoint boundary message");
    let fetch_protocols = Arc::new(Mutex::new(Vec::new()));
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(FirstReadyHeadCheckpointNetwork {
        inner: Arc::new(TestInMemoryNetwork::default()),
        fetch_protocols: Arc::clone(&fetch_protocols),
    });
    register_replication_fetch_handlers(
        &NodeReplicationNetworkHandle::new(Arc::clone(&network)),
        &replication_config_a,
        world_id,
        &config_a.network_policy,
    )
    .expect("register ready checkpoint providers");
    let handle_b = NodeReplicationNetworkHandle::new(Arc::clone(&network));
    let mut endpoint_b =
        ReplicationNetworkEndpoint::new(&handle_b, world_id, true, &config_b.network_policy)
            .expect("fresh observer endpoint");
    let mut replication_b =
        ReplicationRuntime::new(config_b.replication.as_ref().expect("repl b"), "node-b")
            .expect("fresh observer runtime");
    let mut engine_b = PosNodeEngine::new(&config_b).expect("fresh observer engine");
    assert!(
        replication_b
            .load_commit_message_by_height(world_id, checkpoint_height)
            .expect("inspect fresh observer checkpoint state")
            .is_none(),
        "fresh observer must start without a locally persisted checkpoint"
    );
    assert_eq!(engine_b.last_execution_height, 0);
    let mut execution_hook = BootstrapBeforeIncrementalHook {
        installed: Vec::new(),
        incremental_commits: Vec::new(),
        rollback_heights: Vec::new(),
    };
    endpoint_b
        .publish_replication(&incremental_message)
        .expect("publish incompatible height-one tail");
    endpoint_b
        .publish_replication(&checkpoint_message)
        .expect("publish checkpoint boundary after incremental tail");

    engine_b
        .tick(
            "node-b",
            world_id,
            6_000,
            None,
            Some(&mut replication_b),
            Some(&mut endpoint_b),
            None,
            Vec::new(),
            Some(&mut execution_hook),
        )
        .expect("checkpoint bootstrap must precede incompatible height-one incremental replay");

    assert_eq!(engine_b.network_committed_height, checkpoint_height);
    assert_eq!(execution_hook.installed, vec![checkpoint_height]);
    assert!(
        execution_hook.incremental_commits.is_empty(),
        "fresh observer must not execute the height-one tail before checkpoint bootstrap: {:?}",
        execution_hook.incremental_commits
    );
    assert!(
        execution_hook.rollback_heights.is_empty(),
        "fresh observer must not attempt unavailable height-zero rollback: {:?}",
        execution_hook.rollback_heights
    );
    let fetch_protocols = fetch_protocols
        .lock()
        .expect("lock observed checkpoint fetch protocols")
        .clone();
    assert!(
        fetch_protocols
            .iter()
            .any(|protocol| protocol == REPLICATION_FETCH_COMMIT_PROTOCOL),
        "checkpoint bootstrap must fetch the checkpoint descriptor: {fetch_protocols:?}"
    );
    assert!(
        fetch_protocols
            .iter()
            .any(|protocol| protocol == REPLICATION_FETCH_BLOB_PROTOCOL),
        "checkpoint bootstrap must fetch the checkpoint closure: {fetch_protocols:?}"
    );
    assert_eq!(engine_b.committed_height, checkpoint_height);
    assert_eq!(engine_b.replication_persisted_height, checkpoint_height);
    assert!(
        replication_b
            .load_commit_message_by_height(world_id, 1)
            .expect("inspect height-one replication")
            .is_none(),
        "checkpoint bootstrap must not persist the incompatible height-one tail"
    );
    let _ = fs::remove_dir_all(&dir_a);
    let _ = fs::remove_dir_all(&dir_b);
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum InitialPeerHead {
    Unavailable,
    HighCheckpoint,
    StaleHeightOne,
}

fn peer_head_checkpoint_before_height_one_with_stale_dht(
    initial_peer_head: InitialPeerHead,
    initial_checkpoint_fetch_available: bool,
    stale_dht_height_one: bool,
    initial_checkpoint_fetch_not_found: bool,
    dual_connected_provider_heads: bool,
) {
    let _nonce_lock = lock_checkpoint_probe_nonce();
    let _probe_nonce = stale_dht_height_one.then(CheckpointProbeNonceGuard::install);
    let world_id = "world-peer-head-checkpoint-before-height-one";
    let dir_a = temp_dir("peer-head-checkpoint-before-height-one-a");
    let dir_b = temp_dir("peer-head-checkpoint-before-height-one-b");
    let (_, public_key_a) = deterministic_keypair_hex(186);
    let (_, public_key_b) = deterministic_keypair_hex(187);
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", 186)],
    );
    let replication_config_a = signed_replication_config(dir_a.clone(), 186)
        .with_remote_writer_allowlist(vec![public_key_b.clone()])
        .expect("allowlist a")
        .with_fetch_requester_allowlist(vec![public_key_b])
        .expect("fetch allowlist a");
    let replication_config_b = signed_replication_config(dir_b.clone(), 187)
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
    let checkpoint_height = 64;
    let mut replication_a =
        ReplicationRuntime::new(config_a.replication.as_ref().expect("repl a"), "node-a")
            .expect("runtime a");
    let height_one_message = replication_a
        .build_local_commit_message(
            "node-a",
            world_id,
            6_100,
            &committed_decision(1),
            Some("peer-exec-block-1"),
            Some("peer-exec-state-1"),
        )
        .expect("build incompatible height-one message")
        .expect("height-one message");
    let checkpoint_block_hash = format!("exec-block-{checkpoint_height}");
    let checkpoint_state_root = format!("exec-state-{checkpoint_height}");
    replication_a
        .build_local_commit_message_with_checkpoint(
            "node-a",
            world_id,
            6_164,
            &committed_decision(checkpoint_height),
            Some(checkpoint_block_hash.as_str()),
            Some(checkpoint_state_root.as_str()),
            Some(checkpoint_bundle(
                checkpoint_height,
                checkpoint_block_hash.as_str(),
                checkpoint_state_root.as_str(),
            )),
        )
        .expect("build checkpoint only discoverable through peer-head lookup")
        .expect("checkpoint message");
    let fetch_protocols = Arc::new(Mutex::new(Vec::new()));
    let checkpoint_fetch_available = Arc::new(AtomicBool::new(initial_checkpoint_fetch_available));
    let checkpoint_fetch_not_found = Arc::new(AtomicBool::new(initial_checkpoint_fetch_not_found));
    let peer_head = Arc::new(Mutex::new(match initial_peer_head {
        InitialPeerHead::Unavailable => super::replication::FetchHeadResponse {
            found: false,
            head: None,
        },
        InitialPeerHead::HighCheckpoint => super::replication::FetchHeadResponse {
            found: true,
            head: Some(super::replication::ReplicationHeadSummary {
                world_id: world_id.to_string(),
                height: checkpoint_height,
                block_hash: format!("block-{checkpoint_height}"),
                state_root: checkpoint_state_root.clone(),
                timestamp_ms: 6_164,
            }),
        },
        InitialPeerHead::StaleHeightOne => super::replication::FetchHeadResponse {
            found: true,
            head: Some(super::replication::ReplicationHeadSummary {
                world_id: world_id.to_string(),
                height: 1,
                block_hash: "block-1".to_string(),
                state_root: "peer-exec-state-1".to_string(),
                timestamp_ms: 6_100,
            }),
        },
    }));
    let high_peer_head = super::replication::FetchHeadResponse {
        found: true,
        head: Some(super::replication::ReplicationHeadSummary {
            world_id: world_id.to_string(),
            height: checkpoint_height,
            block_hash: format!("block-{checkpoint_height}"),
            state_root: checkpoint_state_root.clone(),
            timestamp_ms: 6_164,
        }),
    };
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(PeerHeadCheckpointNetwork {
        inner: Arc::new(TestInMemoryNetwork::default()),
        fetch_protocols: Arc::clone(&fetch_protocols),
        head: Arc::clone(&peer_head),
        checkpoint_fetch_available: Arc::clone(&checkpoint_fetch_available),
        checkpoint_fetch_not_found: Arc::clone(&checkpoint_fetch_not_found),
        connected_peer_ids: if dual_connected_provider_heads {
            vec!["node-a".to_string(), "node-c".to_string()]
        } else {
            vec!["node-a".to_string()]
        },
    });
    register_replication_fetch_handlers(
        &NodeReplicationNetworkHandle::new(Arc::clone(&network)),
        &replication_config_a,
        world_id,
        &config_a.network_policy,
    )
    .expect("register checkpoint providers");
    let handle_b = NodeReplicationNetworkHandle::new(Arc::clone(&network));
    let handle_b = if stale_dht_height_one {
        let dht = Arc::new(TestReplicaMaintenanceDht::new("node-a", "node-b"));
        dht.put_world_head(
            world_id,
            &WorldHeadAnnounce {
                world_id: world_id.to_string(),
                height: 1,
                block_hash: "block-1".to_string(),
                state_root: "peer-exec-state-1".to_string(),
                timestamp_ms: 6_100,
                signature: String::new(),
            },
        )
        .expect("seed stale DHT height-one candidate");
        handle_b.with_dht(dht)
    } else {
        handle_b
    };
    let mut endpoint_b =
        ReplicationNetworkEndpoint::new(&handle_b, world_id, true, &config_b.network_policy)
            .expect("fresh observer endpoint");
    let mut replication_b =
        ReplicationRuntime::new(config_b.replication.as_ref().expect("repl b"), "node-b")
            .expect("fresh observer runtime");
    let mut engine_b = PosNodeEngine::new(&config_b).expect("fresh observer engine");
    let mut execution_hook = BootstrapBeforeIncrementalHook {
        installed: Vec::new(),
        incremental_commits: Vec::new(),
        rollback_heights: Vec::new(),
    };
    endpoint_b
        .publish_replication(&height_one_message)
        .expect("publish only the incompatible height-one tail in tick one");

    engine_b
        .tick(
            "node-b",
            world_id,
            6_200,
            None,
            Some(&mut replication_b),
            Some(&mut endpoint_b),
            None,
            Vec::new(),
            Some(&mut execution_hook),
        )
        .expect("peer-head checkpoint bootstrap must precede first height-one execution");

    if initial_peer_head != InitialPeerHead::HighCheckpoint
        || !initial_checkpoint_fetch_available
    {
        assert!(
            execution_hook.incremental_commits.is_empty(),
            "fresh observer must defer height one while peer-head preflight is temporarily unavailable: {:?}",
            execution_hook.incremental_commits
        );
        assert_eq!(engine_b.committed_height, 0);
        assert_eq!(engine_b.replication_persisted_height, 0);

        if initial_peer_head != InitialPeerHead::HighCheckpoint {
            *peer_head.lock().expect("publish delayed peer checkpoint head") =
                high_peer_head;
        }
        checkpoint_fetch_available.store(true, Ordering::SeqCst);
        checkpoint_fetch_not_found.store(false, Ordering::SeqCst);

        engine_b
            .tick(
                "node-b",
                world_id,
                6_300,
                None,
                Some(&mut replication_b),
                Some(&mut endpoint_b),
                None,
                Vec::new(),
                Some(&mut execution_hook),
            )
            .expect("peer-head checkpoint bootstrap must precede deferred height-one execution");
    }

    assert_eq!(execution_hook.installed, vec![checkpoint_height]);
    assert!(
        execution_hook.incremental_commits.is_empty(),
        "fresh observer must not execute height one before a peer-head checkpoint bootstrap: {:?}",
        execution_hook.incremental_commits
    );
    assert!(
        execution_hook.rollback_heights.is_empty(),
        "fresh observer must not attempt unavailable height-zero rollback: {:?}",
        execution_hook.rollback_heights
    );
    assert_eq!(engine_b.committed_height, checkpoint_height);
    assert_eq!(engine_b.replication_persisted_height, checkpoint_height);
    if stale_dht_height_one {
        assert_stale_dht_checkpoint_provenance(
            &replication_b,
            world_id,
            checkpoint_height,
            checkpoint_block_hash.as_str(),
            checkpoint_state_root.as_str(),
            &dir_b,
        );
    }
    let fetch_protocols = fetch_protocols
        .lock()
        .expect("lock observed checkpoint fetch protocols")
        .clone();
    assert!(
        fetch_protocols
            .iter()
            .any(|protocol| protocol == REPLICATION_FETCH_COMMIT_PROTOCOL),
        "peer-head checkpoint bootstrap must fetch the checkpoint descriptor: {fetch_protocols:?}"
    );
    assert!(
        fetch_protocols
            .iter()
            .any(|protocol| protocol == REPLICATION_FETCH_BLOB_PROTOCOL),
        "peer-head checkpoint bootstrap must fetch the checkpoint closure: {fetch_protocols:?}"
    );
    let _ = fs::remove_dir_all(&dir_a);
    let _ = fs::remove_dir_all(&dir_b);
}

include!("tests_storage_replication_stale_dht_checkpoint.rs");
