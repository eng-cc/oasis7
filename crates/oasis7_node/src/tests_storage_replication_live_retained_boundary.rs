use std::fs;
use std::sync::{Arc, Mutex, OnceLock};

use super::*;

#[derive(Clone)]
struct AdvertisedHeadRetainedCheckpointNetwork {
    inner: Arc<TestInMemoryNetwork>,
    advertised_head: super::replication::FetchHeadResponse,
}

impl oasis7_proto::distributed_net::DistributedNetwork<WorldError>
    for AdvertisedHeadRetainedCheckpointNetwork
{
    fn publish(&self, topic: &str, payload: &[u8]) -> Result<(), WorldError> {
        self.inner.publish(topic, payload)
    }

    fn subscribe(&self, topic: &str) -> Result<NetworkSubscription, WorldError> {
        self.inner.subscribe(topic)
    }

    fn request(&self, protocol: &str, payload: &[u8]) -> Result<Vec<u8>, WorldError> {
        if protocol == REPLICATION_GET_HEAD_PROTOCOL {
            return serde_json::to_vec(&self.advertised_head).map_err(|err| {
                WorldError::DistributedValidationFailed {
                    reason: format!("encode advertised live head failed: {err}"),
                }
            });
        }
        if protocol == REPLICATION_FETCH_COMMIT_PROTOCOL {
            let request = serde_json::from_slice::<super::replication::FetchCommitRequest>(payload)
                .map_err(|err| WorldError::DistributedValidationFailed {
                    reason: format!("decode retained-boundary fetch request failed: {err}"),
                })?;
            if request.height == 52_079 {
                return serde_json::to_vec(&super::replication::FetchCommitResponse {
                    found: false,
                    message: None,
                })
                .map_err(|err| WorldError::DistributedValidationFailed {
                    reason: format!("encode advertised-head miss failed: {err}"),
                });
            }
        }
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

struct RetainedBoundaryExecutionHook {
    installed: Vec<u64>,
    incremental_commits: Vec<u64>,
}

impl NodeExecutionHook for RetainedBoundaryExecutionHook {
    fn on_commit(
        &mut self,
        context: NodeExecutionCommitContext,
    ) -> Result<NodeExecutionCommitResult, String> {
        self.incremental_commits.push(context.height);
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
            return Err("retained checkpoint closure binding mismatch".to_string());
        }
        self.installed.push(context.height);
        Ok(NodeExecutionCommitResult {
            execution_height: context.height,
            execution_block_hash: context.execution_block_hash,
            execution_state_root: context.execution_state_root,
        })
    }
}

fn retained_boundary_bundle(
    height: u64,
    execution_block_hash: &str,
    execution_state_root: &str,
) -> NodeExecutionCheckpointBundle {
    let bytes = format!("retained-boundary-snapshot-{height}").into_bytes();
    NodeExecutionCheckpointBundle {
        height,
        execution_block_hash: execution_block_hash.to_string(),
        execution_state_root: execution_state_root.to_string(),
        manifest_json: br#"{"test":"retained-boundary"}"#.to_vec(),
        blobs: vec![NodeExecutionCheckpointBlob {
            content_hash: oasis7_distfs::blake3_hex(bytes.as_slice()),
            bytes,
        }],
    }
}

fn committed_decision(height: u64) -> PosDecision {
    committed_decision_with_block_hash(height, format!("block-{height}").as_str())
}

fn committed_decision_with_block_hash(height: u64, block_hash: &str) -> PosDecision {
    PosDecision {
        height,
        slot: height,
        epoch: 0,
        proposer_id: "node-a".to_string(),
        status: PosConsensusStatus::Committed,
        block_hash: block_hash.to_string(),
        action_root: empty_action_root(),
        committed_actions: Vec::new(),
        approved_stake: 100,
        rejected_stake: 0,
        required_stake: 67,
        total_stake: 100,
    }
}

static PROBE_NONCE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

struct ProbeNonceGuard;

impl ProbeNonceGuard {
    fn install() -> Self {
        // SAFETY: this test holds PROBE_NONCE_LOCK for its full duration.
        unsafe {
            std::env::set_var(
                "OASIS7_CHECKPOINT_PROBE_NONCE",
                "retained-boundary-probe-nonce-0123456789abcdef0123456789abcdef",
            );
        }
        Self
    }
}

impl Drop for ProbeNonceGuard {
    fn drop(&mut self) {
        // SAFETY: this test holds PROBE_NONCE_LOCK for its full duration.
        unsafe {
            std::env::remove_var("OASIS7_CHECKPOINT_PROBE_NONCE");
        }
    }
}

#[test]
fn fresh_observer_installs_latest_retained_checkpoint_below_non_aligned_live_head() {
    let _nonce_lock = PROBE_NONCE_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let _probe_nonce = ProbeNonceGuard::install();
    let world_id = "world-live-head-retained-boundary-52079";
    let dir_a = temp_dir("live-head-retained-boundary-a");
    let dir_b = temp_dir("live-head-retained-boundary-b");
    let (_, public_key_a) = deterministic_keypair_hex(208);
    let (_, public_key_b) = deterministic_keypair_hex(209);
    let advertised_height = 52_079;
    let checkpoint_height = 52_032;
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", 208)],
    );
    let replication_config_a = signed_replication_config(dir_a.clone(), 208)
        .with_remote_writer_allowlist(vec![public_key_b.clone()])
        .expect("allowlist a")
        .with_fetch_requester_allowlist(vec![public_key_b])
        .expect("fetch allowlist a");
    let replication_config_b = signed_replication_config(dir_b.clone(), 209)
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
    let mut replication_a =
        ReplicationRuntime::new(config_a.replication.as_ref().expect("repl a"), "node-a")
            .expect("runtime a");
    let height_one = replication_a
        .build_local_commit_message(
            "node-a",
            world_id,
            52_080,
            &committed_decision(1),
            Some("exec-block-1"),
            Some("exec-state-1"),
        )
        .expect("build height-one tail")
        .expect("height-one tail message");
    let checkpoint_block_hash = format!("exec-block-{checkpoint_height}");
    let checkpoint_state_root = format!("exec-state-{checkpoint_height}");
    let checkpoint_message = replication_a
        .build_local_commit_message_with_checkpoint(
            "node-a",
            world_id,
            52_032,
            &committed_decision(checkpoint_height),
            Some(checkpoint_block_hash.as_str()),
            Some(checkpoint_state_root.as_str()),
            Some(retained_boundary_bundle(
                checkpoint_height,
                checkpoint_block_hash.as_str(),
                checkpoint_state_root.as_str(),
            )),
        )
        .expect("build retained boundary checkpoint")
        .expect("retained boundary checkpoint message");
    let network: Arc<dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync> =
        Arc::new(AdvertisedHeadRetainedCheckpointNetwork {
            inner: Arc::new(TestInMemoryNetwork::default()),
            advertised_head: super::replication::FetchHeadResponse {
                found: true,
                head: Some(super::replication::ReplicationHeadSummary {
                    world_id: world_id.to_string(),
                    height: advertised_height,
                    block_hash: format!("block-{advertised_height}"),
                    state_root: format!("state-{advertised_height}"),
                    timestamp_ms: 52_079,
                }),
            },
        });
    register_replication_fetch_handlers(
        &NodeReplicationNetworkHandle::new(Arc::clone(&network)),
        &replication_config_a,
        world_id,
        &config_a.network_policy,
    )
    .expect("register retained checkpoint provider");
    let handle_b = NodeReplicationNetworkHandle::new(Arc::clone(&network));
    let mut endpoint_b =
        ReplicationNetworkEndpoint::new(&handle_b, world_id, true, &config_b.network_policy)
            .expect("fresh observer endpoint");
    let mut replication_b =
        ReplicationRuntime::new(config_b.replication.as_ref().expect("repl b"), "node-b")
            .expect("runtime b");
    let mut engine_b = PosNodeEngine::new(&config_b).expect("fresh observer engine");
    let mut execution_hook = RetainedBoundaryExecutionHook {
        installed: Vec::new(),
        incremental_commits: Vec::new(),
    };
    endpoint_b
        .publish_replication(&height_one)
        .expect("publish height-one tail before checkpoint preflight");
    endpoint_b
        .publish_replication(&checkpoint_message)
        .expect("publish retained checkpoint provider message");

    let result = engine_b.tick(
        "node-b",
        world_id,
        52_100,
        None,
        Some(&mut replication_b),
        Some(&mut endpoint_b),
        None,
        Vec::new(),
        Some(&mut execution_hook),
    );
    assert!(
        result.is_ok(),
        "fresh observer must use retained checkpoint below advertised live head before height-one execution: result={result:?} incremental={:?}",
        execution_hook.incremental_commits
    );
    assert_eq!(execution_hook.installed, vec![checkpoint_height]);
    assert!(execution_hook.incremental_commits.is_empty());
    assert_eq!(engine_b.committed_height, checkpoint_height);
    assert_eq!(engine_b.replication_persisted_height, checkpoint_height);
    let receipt_path = dir_b
        .join("checkpoint-verification")
        .join(format!("{checkpoint_height}.json"));
    let receipt: serde_json::Value = serde_json::from_slice(
        fs::read(&receipt_path)
            .expect("retained checkpoint closure receipt")
            .as_slice(),
    )
    .expect("decode retained checkpoint closure receipt");
    assert_eq!(receipt["height"], checkpoint_height);
    assert_eq!(receipt["probe_nonce"], "retained-boundary-probe-nonce-0123456789abcdef0123456789abcdef");
    assert!(
        receipt["fetch_observations"]
            .as_array()
            .is_some_and(|observations| !observations.is_empty()),
        "closure receipt must retain signed fetch observations: {receipt}"
    );
    assert!(
        replication_b
            .load_commit_message_by_height(world_id, 1)
            .expect("inspect height-one persistence")
            .is_none(),
        "height-one tail must not be persisted before retained checkpoint closure"
    );
    let _ = fs::remove_dir_all(&dir_a);
    let _ = fs::remove_dir_all(&dir_b);
}

#[test]
fn fresh_observer_rejects_lower_retained_checkpoint_without_head_lineage() {
    let _nonce_lock = PROBE_NONCE_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let _probe_nonce = ProbeNonceGuard::install();
    let world_id = "world-live-head-retained-fork-lineage-52079";
    let dir_a = temp_dir("live-head-retained-fork-lineage-a");
    let dir_b = temp_dir("live-head-retained-fork-lineage-b");
    let (_, public_key_a) = deterministic_keypair_hex(210);
    let (_, public_key_b) = deterministic_keypair_hex(211);
    let (_, public_key_c) = deterministic_keypair_hex(212);
    let advertised_height = 52_079;
    let checkpoint_height = 52_032;
    let advertised_block_hash = "fork-chain-head-52079";
    let checkpoint_block_hash = "other-chain-retained-52032";
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![
            PosValidator {
                validator_id: "node-a".to_string(),
                stake: 60,
            },
            PosValidator {
                validator_id: "node-c".to_string(),
                stake: 40,
            },
        ],
        &[("node-a", 210), ("node-c", 212)],
    );
    let replication_config_a = signed_replication_config(dir_a.clone(), 210)
        .with_remote_writer_allowlist(vec![public_key_b.clone()])
        .expect("allowlist a")
        .with_fetch_requester_allowlist(vec![public_key_b.clone()])
        .expect("fetch allowlist a");
    let replication_config_b = signed_replication_config(dir_b.clone(), 211)
        .with_remote_writer_allowlist(vec![public_key_a.clone(), public_key_c.clone()])
        .expect("allowlist b")
        .with_fetch_requester_allowlist(vec![public_key_a.clone(), public_key_c.clone()])
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
    let mut replication_a =
        ReplicationRuntime::new(config_a.replication.as_ref().expect("repl a"), "node-a")
            .expect("runtime a");
    let height_one = replication_a
        .build_local_commit_message(
            "node-a",
            world_id,
            52_080,
            &committed_decision(1),
            Some("other-chain-exec-block-1"),
            Some("other-chain-exec-state-1"),
        )
        .expect("build height-one tail")
        .expect("height-one tail message");
    let checkpoint_state_root = "other-chain-state-52032";
    let checkpoint_message = replication_a
        .build_local_commit_message_with_checkpoint(
            "node-a",
            world_id,
            52_032,
            &committed_decision_with_block_hash(checkpoint_height, checkpoint_block_hash),
            Some("other-chain-exec-block-52032"),
            Some(checkpoint_state_root),
            Some(retained_boundary_bundle(
                checkpoint_height,
                "other-chain-exec-block-52032",
                checkpoint_state_root,
            )),
        )
        .expect("build forked retained checkpoint")
        .expect("forked retained checkpoint message");
    assert!(checkpoint_message.public_key_hex.is_some());
    assert!(checkpoint_message.signature_hex.is_some());
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(AdvertisedHeadRetainedCheckpointNetwork {
        inner: Arc::new(TestInMemoryNetwork::default()),
        advertised_head: super::replication::FetchHeadResponse {
            found: true,
            head: Some(super::replication::ReplicationHeadSummary {
                world_id: world_id.to_string(),
                height: advertised_height,
                block_hash: advertised_block_hash.to_string(),
                state_root: "fork-chain-state-52079".to_string(),
                timestamp_ms: 52_079,
            }),
        },
    });
    register_replication_fetch_handlers(
        &NodeReplicationNetworkHandle::new(Arc::clone(&network)),
        &replication_config_a,
        world_id,
        &config_a.network_policy,
    )
    .expect("register forked checkpoint provider");
    let handle_b = NodeReplicationNetworkHandle::new(Arc::clone(&network));
    let mut endpoint_b =
        ReplicationNetworkEndpoint::new(&handle_b, world_id, true, &config_b.network_policy)
            .expect("fresh observer endpoint");
    let mut replication_b =
        ReplicationRuntime::new(config_b.replication.as_ref().expect("repl b"), "node-b")
            .expect("runtime b");
    let mut engine_b = PosNodeEngine::new(&config_b).expect("fresh observer engine");
    for (node_id, public_key_hex) in [("node-a", public_key_a), ("node-c", public_key_c)] {
        engine_b.peer_heads.insert(
            node_id.to_string(),
            PeerCommittedHead {
                height: advertised_height,
                block_hash: advertised_block_hash.to_string(),
                committed_at_ms: 52_079,
                observed_at_ms: 52_080,
                execution_block_hash: Some("fork-chain-exec-block-52079".to_string()),
                execution_state_root: Some("fork-chain-exec-state-52079".to_string()),
                action_root: empty_action_root(),
                public_key_hex: Some(public_key_hex),
                signature_hex: Some(format!("signed-{node_id}-{advertised_height}")),
            },
        );
    }
    assert_eq!(engine_b.peer_heads.len(), 2);
    assert!(engine_b
        .peer_heads
        .values()
        .all(|head| head.public_key_hex.is_some() && head.signature_hex.is_some()));
    assert_eq!(endpoint_b.connected_peer_ids(), vec!["node-a"]);
    let mut execution_hook = RetainedBoundaryExecutionHook {
        installed: Vec::new(),
        incremental_commits: Vec::new(),
    };
    endpoint_b
        .publish_replication(&height_one)
        .expect("publish height-one tail before lineage preflight");
    endpoint_b
        .publish_replication(&checkpoint_message)
        .expect("publish forked retained checkpoint provider message");

    let result = engine_b.tick(
        "node-b",
        world_id,
        52_100,
        None,
        Some(&mut replication_b),
        Some(&mut endpoint_b),
        None,
        Vec::new(),
        Some(&mut execution_hook),
    );
    assert!(
        result.is_ok(),
        "forked lower checkpoint must fail closed before height-one execution: result={result:?} installed={:?} incremental={:?}",
        execution_hook.installed,
        execution_hook.incremental_commits
    );
    assert!(
        execution_hook.installed.is_empty(),
        "checkpoint C must not install when signed head H has no same-chain lineage: installed={:?}",
        execution_hook.installed
    );
    assert!(execution_hook.incremental_commits.is_empty());
    assert_eq!(engine_b.committed_height, 0);
    assert_eq!(engine_b.replication_persisted_height, 0);
    assert!(!dir_b
        .join("checkpoint-verification")
        .join(format!("{checkpoint_height}.json"))
        .exists());
    let _ = fs::remove_dir_all(&dir_a);
    let _ = fs::remove_dir_all(&dir_b);
}
