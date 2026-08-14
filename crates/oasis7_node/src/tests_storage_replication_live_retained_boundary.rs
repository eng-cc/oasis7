use std::fs;
use std::sync::Arc;

use ed25519_dalek::SigningKey;
use oasis7_proto::distributed_checkpoint_lineage::{
    checkpoint_lineage_descriptor_digest, CheckpointLineageCheckpointV1,
    CheckpointLineageEnvelopeV1, CheckpointLineageHeadV1,
    CHECKPOINT_LINEAGE_CLAIM_BOUNDARY_V1, CHECKPOINT_LINEAGE_ENVELOPE_SCHEMA_V1,
};

use super::*;
use super::storage_replication_first_ready_checkpoint_tests::{
    lock_checkpoint_probe_nonce, CheckpointProbeNonceGuard,
};

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
                lineage_envelope: None,
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

#[test]
fn fresh_observer_installs_latest_retained_checkpoint_below_non_aligned_live_head() {
    let _nonce_lock = lock_checkpoint_probe_nonce();
    let _probe_nonce = CheckpointProbeNonceGuard::install();
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
    assert_eq!(
        receipt["probe_nonce"],
        "probe-nonce-0123456789abcdef0123456789abcdef"
    );
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
    let _nonce_lock = lock_checkpoint_probe_nonce();
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

fn signed_lineage_head(
    world_id: &str,
    node_id: &str,
    signer_seed: u8,
    height: u64,
) -> GossipCommitMessage {
    let (private_hex, public_hex) = deterministic_keypair_hex(signer_seed);
    let signing_key = SigningKey::from_bytes(
        &hex::decode(private_hex)
            .expect("decode lineage signer")
            .try_into()
            .expect("lineage signer length"),
    );
    let signer = ConsensusMessageSigner::new(signing_key, public_hex).expect("lineage signer");
    let mut commit = GossipCommitMessage {
        version: 1,
        world_id: world_id.to_string(),
        node_id: node_id.to_string(),
        player_id: node_id.to_string(),
        height,
        slot: height,
        epoch: 0,
        block_hash: format!("head-block-{height}"),
        action_root: empty_action_root(),
        actions: Vec::new(),
        committed_at_ms: height as i64,
        execution_block_hash: Some(format!("head-execution-block-{height}")),
        execution_state_root: Some(format!("head-execution-state-{height}")),
        public_key_hex: None,
        signature_hex: None,
    };
    sign_commit_message(&mut commit, &signer).expect("sign lineage head");
    verify_commit_message_signature(&commit, true).expect("verify lineage head");
    commit
}

const CHECKPOINT_LINEAGE_MANIFEST_SCHEMA_V1: &str = "oasis7.checkpoint_manifest.v1";
fn lineage_checkpoint_bundle(
    world_id: &str,
    checkpoint_height: u64,
    checkpoint_block_hash: &str,
    execution_block_hash: &str,
    execution_state_root: &str,
) -> NodeExecutionCheckpointBundle {
    let bytes = format!("lineage-checkpoint-object-{checkpoint_height}").into_bytes();
    let blob_hash = oasis7_distfs::blake3_hex(bytes.as_slice());
    let manifest_json = serde_json::to_vec(&serde_json::json!({
        "schema": CHECKPOINT_LINEAGE_MANIFEST_SCHEMA_V1,
        "world_id": world_id,
        "height": checkpoint_height,
        "block_hash": checkpoint_block_hash,
        "execution_block_hash": execution_block_hash,
        "execution_state_root": execution_state_root,
        "blob_hash": blob_hash,
        "blob_size_bytes": bytes.len(),
    }))
    .expect("encode checkpoint manifest");
    NodeExecutionCheckpointBundle {
        height: checkpoint_height,
        execution_block_hash: execution_block_hash.to_string(),
        execution_state_root: execution_state_root.to_string(),
        manifest_json,
        blobs: vec![NodeExecutionCheckpointBlob {
            content_hash: oasis7_distfs::blake3_hex(bytes.as_slice()),
            bytes,
        }],
    }
}

fn production_lineage_envelope(
    world_id: &str,
    head: &GossipCommitMessage,
    payload: &super::replication_state_reconcile::ReplicationCommitPayload,
    descriptor: &NodeExecutionCheckpointDescriptor,
    engine_a: &PosNodeEngine,
    engine_c: &PosNodeEngine,
    divergent: bool,
) -> CheckpointLineageEnvelopeV1 {
    let descriptor_digest = checkpoint_lineage_descriptor_digest(
        world_id,
        descriptor.height,
        payload.block_hash.as_str(),
        descriptor.execution_block_hash.as_str(),
        descriptor.execution_state_root.as_str(),
        descriptor.manifest_ref.as_str(),
        descriptor.manifest_size_bytes,
        &descriptor
            .blobs
            .iter()
            .map(|blob| (blob.content_hash.clone(), blob.size_bytes))
            .collect::<Vec<_>>(),
    )
    .expect("production descriptor digest");
    let head = CheckpointLineageHeadV1 {
        height: head.height,
        block_hash: head.block_hash.clone(),
        state_root: head.execution_state_root.clone().expect("head state root"),
        execution_block_hash: head
            .execution_block_hash
            .clone()
            .expect("head execution block"),
        execution_state_root: head
            .execution_state_root
            .clone()
            .expect("head execution state"),
    };
    let checkpoint = CheckpointLineageCheckpointV1 {
        height: descriptor.height,
        block_hash: if divergent {
            "unbound-checkpoint-52032".to_string()
        } else {
            payload.block_hash.clone()
        },
        state_root: descriptor.execution_state_root.clone(),
        execution_block_hash: descriptor.execution_block_hash.clone(),
        execution_state_root: descriptor.execution_state_root.clone(),
        descriptor_digest,
        manifest_size: descriptor.manifest_size_bytes,
    };
    let round_id = format!("checkpoint-lineage-round-{}", descriptor.height);
    let vote_a = engine_a
        .build_local_checkpoint_lineage_vote(
            world_id,
            checkpoint.clone(),
            head.clone(),
            round_id.clone(),
        )
        .expect("node-a production lineage vote")
        .vote;
    let vote_c = engine_c
        .build_local_checkpoint_lineage_vote(world_id, checkpoint.clone(), head.clone(), round_id.clone())
        .expect("node-c production lineage vote")
        .vote;
    CheckpointLineageEnvelopeV1 {
        schema_version: CHECKPOINT_LINEAGE_ENVELOPE_SCHEMA_V1,
        claim_boundary: CHECKPOINT_LINEAGE_CLAIM_BOUNDARY_V1.to_string(),
        world_id: world_id.to_string(),
        checkpoint,
        head,
        validator_set_hash: engine_a.validator_set_hash.clone(),
        total_stake: engine_a.total_stake,
        required_stake: engine_a.required_stake,
        round_id,
        votes: vec![vote_a, vote_c],
    }
}

#[derive(Debug)]
struct LineageProbeResult {
    installed: Vec<u64>,
    incremental_commits: Vec<u64>,
    committed_height: u64,
    receipt_exists: bool,
}

fn run_authenticated_head_lineage_case(divergent: bool) -> LineageProbeResult {
    let world_id = if divergent {
        "world-authenticated-head-divergent-checkpoint"
    } else {
        "world-authenticated-head-bound-checkpoint"
    };
    let dir_a = temp_dir("authenticated-head-lineage-a");
    let dir_b = temp_dir("authenticated-head-lineage-b");
    let (_, public_key_a) = deterministic_keypair_hex(220);
    let (_, public_key_b) = deterministic_keypair_hex(221);
    let checkpoint_height = 52_032;
    let advertised_height = 52_079;
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![
            PosValidator {
                validator_id: "node-a".to_string(),
                stake: 70,
            },
            PosValidator {
                validator_id: "node-c".to_string(),
                stake: 30,
            },
        ],
        &[("node-a", 220), ("node-c", 222)],
    );
    let replication_config_a = signed_replication_config(dir_a.clone(), 220)
        .with_remote_writer_allowlist(vec![public_key_b.clone()])
        .expect("allowlist a")
        .with_fetch_requester_allowlist(vec![public_key_b.clone()])
        .expect("fetch allowlist a");
    let replication_config_b = signed_replication_config(dir_b.clone(), 221)
        .with_remote_writer_allowlist(vec![public_key_a.clone()])
        .expect("allowlist b")
        .with_fetch_requester_allowlist(vec![public_key_a])
        .expect("fetch allowlist b");
    let dir_c = temp_dir("authenticated-head-lineage-c");
    let replication_config_c = signed_replication_config(dir_c.clone(), 222);
    let config_a = NodeConfig::new("node-a", world_id, NodeRole::Sequencer)
        .expect("config a")
        .with_pos_config(pos_config.clone())
        .expect("pos config a")
        .with_replication(replication_config_a.clone());
    let config_b = NodeConfig::new("node-b", world_id, NodeRole::Observer)
        .expect("config b")
        .with_pos_config(pos_config.clone())
        .expect("pos config b")
        .with_require_execution_on_commit(true)
        .with_replication(replication_config_b.clone());
    let config_c = NodeConfig::new("node-c", world_id, NodeRole::Sequencer)
        .expect("config c")
        .with_pos_config(pos_config.clone())
        .expect("pos config c")
        .with_replication(replication_config_c);
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
    let head = signed_lineage_head(world_id, "node-a", 220, advertised_height);
    let checkpoint_block_hash = if divergent {
        "divergent-checkpoint-block-52032"
    } else {
        "checkpoint-block-52032"
    };
    let execution_block_hash = if divergent {
        "divergent-execution-block-52032"
    } else {
        "execution-block-52032"
    };
    let execution_state_root = if divergent {
        "divergent-execution-state-52032"
    } else {
        "execution-state-52032"
    };
    let checkpoint_message = replication_a
        .build_local_commit_message_with_checkpoint(
            "node-a",
            world_id,
            checkpoint_height as i64,
            &committed_decision_with_block_hash(checkpoint_height, checkpoint_block_hash),
            Some(execution_block_hash),
            Some(execution_state_root),
            Some(lineage_checkpoint_bundle(
                world_id,
                checkpoint_height,
                checkpoint_block_hash,
                execution_block_hash,
                execution_state_root,
            )),
        )
        .expect("build lineage checkpoint")
        .expect("lineage checkpoint message");
    let checkpoint_payload =
        super::replication_state_reconcile::parse_replication_commit_payload(
            checkpoint_message.payload.as_slice(),
        )
        .expect("parse source checkpoint payload");
    let descriptor = checkpoint_payload
        .execution_checkpoint
        .as_ref()
        .expect("source checkpoint descriptor")
        .clone();
    let engine_a = PosNodeEngine::new(&config_a).expect("lineage signer engine a");
    let engine_c = PosNodeEngine::new(&config_c).expect("lineage signer engine c");
    let lineage_envelope = production_lineage_envelope(
        world_id,
        &head,
        &checkpoint_payload,
        &descriptor,
        &engine_a,
        &engine_c,
        divergent,
    );
    let checkpoint_message = replication_a
        .attach_checkpoint_lineage_envelope("node-a", world_id, &lineage_envelope)
        .expect("attach source lineage sidecar")
        .expect("source lineage sidecar message");
    let attached_payload =
        super::replication_state_reconcile::parse_replication_commit_payload(
            checkpoint_message.payload.as_slice(),
        )
        .expect("parse amended source checkpoint payload");
    assert_eq!(
        attached_payload.lineage_envelope.as_ref(),
        Some(&lineage_envelope),
        "source signed replication payload must carry the production lineage sidecar"
    );
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(AdvertisedHeadRetainedCheckpointNetwork {
        inner: Arc::new(TestInMemoryNetwork::default()),
        advertised_head: super::replication::FetchHeadResponse {
            found: true,
            head: Some(super::replication::ReplicationHeadSummary {
                world_id: world_id.to_string(),
                height: advertised_height,
                block_hash: head.block_hash.clone(),
                state_root: head
                    .execution_state_root
                    .clone()
                    .expect("head execution state"),
                timestamp_ms: advertised_height as i64,
            }),
        },
    });
    register_replication_fetch_handlers(
        &NodeReplicationNetworkHandle::new(Arc::clone(&network)),
        &replication_config_a,
        world_id,
        &config_a.network_policy,
    )
    .expect("register lineage checkpoint provider");
    let handle_b = NodeReplicationNetworkHandle::new(Arc::clone(&network));
    let mut endpoint_b =
        ReplicationNetworkEndpoint::new(&handle_b, world_id, true, &config_b.network_policy)
            .expect("lineage observer endpoint");
    let mut replication_b =
        ReplicationRuntime::new(config_b.replication.as_ref().expect("repl b"), "node-b")
            .expect("runtime b");
    let fetch_request = replication_b
        .build_fetch_commit_request(world_id, checkpoint_height)
        .expect("build sidecar fetch request");
    let fetch_response_payload = network
        .request(
            super::replication::REPLICATION_FETCH_COMMIT_PROTOCOL,
            serde_json::to_vec(&fetch_request)
                .expect("encode sidecar fetch request")
                .as_slice(),
        )
        .expect("fetch source sidecar response");
    let fetch_response: super::replication::FetchCommitResponse =
        serde_json::from_slice(fetch_response_payload.as_slice())
            .expect("decode source sidecar response");
    assert_eq!(
        fetch_response.lineage_envelope.as_ref(),
        Some(&lineage_envelope),
        "fetch-commit response must expose the source-authored lineage sidecar"
    );
    let mut engine_b = PosNodeEngine::new(&config_b).expect("lineage observer engine");
    engine_b.observe_peer_commit_message(&head);
    assert!(engine_b.latest_validated_peer_commit.is_some());
    let cached_head = engine_b.peer_heads.get_mut("node-a").expect("cached head");
    cached_head.public_key_hex = None;
    cached_head.signature_hex = None;
    let mut execution_hook = RetainedBoundaryExecutionHook {
        installed: Vec::new(),
        incremental_commits: Vec::new(),
    };
    endpoint_b
        .publish_replication(&height_one)
        .expect("publish height-one tail");
    endpoint_b
        .publish_replication(&checkpoint_message)
        .expect("publish lineage checkpoint");
    engine_b
        .tick(
            "node-b",
            world_id,
            52_100,
            None,
            Some(&mut replication_b),
            Some(&mut endpoint_b),
            None,
            Vec::new(),
            Some(&mut execution_hook),
        )
        .expect("lineage probe tick");
    let receipt_exists = dir_b
        .join("checkpoint-verification")
        .join(format!("{checkpoint_height}.json"))
        .exists();
    let result = LineageProbeResult {
        installed: execution_hook.installed,
        incremental_commits: execution_hook.incremental_commits,
        committed_height: engine_b.committed_height,
        receipt_exists,
    };
    let _ = fs::remove_dir_all(&dir_a);
    let _ = fs::remove_dir_all(&dir_b);
    let _ = fs::remove_dir_all(&dir_c);
    result
}

#[test]
fn authenticated_head_rejects_same_signer_same_route_divergent_checkpoint() {
    let _nonce_lock = lock_checkpoint_probe_nonce();
    let _probe_nonce = CheckpointProbeNonceGuard::install();
    let result = run_authenticated_head_lineage_case(true);
    assert!(
        result.installed.is_empty(),
        "same-signer divergent checkpoint must fail closed before install: {result:?}"
    );
    assert!(
        result.incremental_commits.is_empty(),
        "divergent checkpoint must not re-enter height-one execution: {result:?}"
    );
    assert_eq!(result.committed_height, 0);
    assert!(!result.receipt_exists);
}

#[test]
fn authenticated_head_accepts_checkpoint_with_quorum_signed_lineage_envelope() {
    let _nonce_lock = lock_checkpoint_probe_nonce();
    let _probe_nonce = CheckpointProbeNonceGuard::install();
    let result = run_authenticated_head_lineage_case(false);
    assert_eq!(result.installed, vec![52_032], "valid lineage result={result:?}");
    assert!(result.incremental_commits.is_empty());
    assert_eq!(result.committed_height, 52_032);
    assert!(result.receipt_exists);
}
