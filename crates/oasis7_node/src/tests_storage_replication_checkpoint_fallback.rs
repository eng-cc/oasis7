use std::fs;
use std::sync::{Arc, Mutex};

use oasis7_proto::distributed_checkpoint_lineage::CheckpointLineageHeadV1;

use super::*;
use super::storage_replication_first_ready_checkpoint_tests::{
    checkpoint_bundle, committed_decision, BootstrapBeforeIncrementalHook,
    FirstReadyHeadCheckpointNetwork, CheckpointProbeNonceGuard, lock_checkpoint_probe_nonce,
};

#[derive(Clone)]
struct FetchHeadOnlyCheckpointNetwork {
    inner: Arc<TestInMemoryNetwork>,
    advertised_head: super::replication::FetchHeadResponse,
}

impl oasis7_proto::distributed_net::DistributedNetwork<WorldError>
    for FetchHeadOnlyCheckpointNetwork
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
                    reason: format!("encode FetchHead-only response failed: {err}"),
                }
            });
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

#[derive(Clone)]
struct SequencedFetchHeadCheckpointNetwork {
    inner: Arc<TestInMemoryNetwork>,
    advertised_heads: Arc<Mutex<Vec<super::replication::FetchHeadResponse>>>,
}

impl oasis7_proto::distributed_net::DistributedNetwork<WorldError>
    for SequencedFetchHeadCheckpointNetwork
{
    fn publish(&self, topic: &str, payload: &[u8]) -> Result<(), WorldError> {
        self.inner.publish(topic, payload)
    }

    fn subscribe(&self, topic: &str) -> Result<NetworkSubscription, WorldError> {
        self.inner.subscribe(topic)
    }

    fn request(&self, protocol: &str, payload: &[u8]) -> Result<Vec<u8>, WorldError> {
        if protocol == REPLICATION_GET_HEAD_PROTOCOL {
            let mut advertised_heads = self
                .advertised_heads
                .lock()
                .expect("advertised FetchHead responses");
            let response = if !advertised_heads.is_empty() {
                advertised_heads.remove(0)
            } else {
                super::replication::FetchHeadResponse {
                    found: false,
                    head: None,
                }
            };
            return serde_json::to_vec(&response).map_err(|err| {
                WorldError::DistributedValidationFailed {
                    reason: format!("encode sequenced FetchHead response failed: {err}"),
                }
            });
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

#[test]
fn fresh_observer_execution_mismatch_falls_back_to_authenticated_retained_checkpoint() {
    let _nonce_lock = lock_checkpoint_probe_nonce();
    let _probe_nonce = CheckpointProbeNonceGuard::install();
    let world_id = "world-fresh-observer-execution-mismatch-checkpoint-fallback";
    let dir_a = temp_dir("fresh-observer-execution-mismatch-fallback-a");
    let dir_b = temp_dir("fresh-observer-execution-mismatch-fallback-b");
    let (_, public_key_a) = deterministic_keypair_hex(192);
    let (_, public_key_b) = deterministic_keypair_hex(193);
    let checkpoint_height = REPLICATION_GAP_SYNC_MAX_HEIGHTS_PER_POLL;
    let peer_head_height = REPLICATION_GAP_SYNC_MAX_HEIGHTS_PER_POLL + 36;
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", 192)],
    );
    let replication_config_a = signed_replication_config(dir_a.clone(), 192)
        .with_remote_writer_allowlist(vec![public_key_b.clone()])
        .expect("allowlist a")
        .with_fetch_requester_allowlist(vec![public_key_b.clone()])
        .expect("fetch allowlist a");
    let replication_config_b = signed_replication_config(dir_b.clone(), 193)
        .with_remote_writer_allowlist(vec![public_key_a.clone()])
        .expect("allowlist b")
        .with_fetch_requester_allowlist(vec![public_key_a.clone()])
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
            .expect("remote replication runtime");
    replication_a
        .build_local_commit_message(
            "node-a",
            world_id,
            7_100,
            &committed_decision(1),
            Some("peer-execution-block-1"),
            Some("peer-execution-state-1"),
        )
        .expect("build valid height-one commit")
        .expect("height-one commit");
    let checkpoint_execution_block_hash = format!("execution-block-{checkpoint_height}");
    let checkpoint_execution_state_root = format!("execution-state-{checkpoint_height}");
    replication_a
        .build_local_commit_message_with_checkpoint(
            "node-a",
            world_id,
            7_164,
            &committed_decision(checkpoint_height),
            Some(checkpoint_execution_block_hash.as_str()),
            Some(checkpoint_execution_state_root.as_str()),
            Some(checkpoint_bundle(
                checkpoint_height,
                checkpoint_execution_block_hash.as_str(),
                checkpoint_execution_state_root.as_str(),
            )),
        )
        .expect("build retained checkpoint commit")
        .expect("retained checkpoint commit");
    let engine_a = PosNodeEngine::new(&config_a).expect("lineage authority engine");
    let peer_head_block_hash = format!("block-{peer_head_height}");
    let peer_head_execution_block_hash = format!("execution-block-{peer_head_height}");
    let peer_head_execution_state_root = format!("execution-state-{peer_head_height}");
    let checkpoint_head = CheckpointLineageHeadV1 {
        height: peer_head_height,
        block_hash: peer_head_block_hash.clone(),
        state_root: peer_head_execution_state_root.clone(),
        execution_block_hash: peer_head_execution_block_hash.clone(),
        execution_state_root: peer_head_execution_state_root.clone(),
    };
    let lineage_envelope = super::storage_replication_live_retained_boundary_tests::
        attach_production_lineage_envelope(
            &mut replication_a,
            world_id,
            checkpoint_height,
            checkpoint_head.clone(),
            &[&engine_a],
        );
    assert_eq!(lineage_envelope.head, checkpoint_head);
    let persisted_checkpoint = replication_a
        .load_commit_message_by_height(world_id, checkpoint_height)
        .expect("load retained checkpoint source")
        .expect("retained checkpoint source");
    assert!(
        parse_replication_commit_payload(persisted_checkpoint.payload.as_slice())
            .expect("decode retained checkpoint source")
            .lineage_envelope
            .is_some(),
        "the fallback fixture must retain the signed lineage sidecar"
    );

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
    .expect("register authenticated checkpoint provider");
    let handle_b = NodeReplicationNetworkHandle::new(Arc::clone(&network));
    let endpoint_b =
        ReplicationNetworkEndpoint::new(&handle_b, world_id, true, &config_b.network_policy)
            .expect("fresh observer endpoint");
    let mut replication_b =
        ReplicationRuntime::new(config_b.replication.as_ref().expect("repl b"), "node-b")
            .expect("fresh observer replication runtime");
    let mut engine_b = PosNodeEngine::new(&config_b).expect("fresh observer engine");
    // Model the probe's stale low network cursor plus an authenticated high
    // peer head.  The current implementation enters height-one replay from
    // this state and returns the mismatch before trying the retained boundary.
    engine_b.network_committed_height = 1;
    engine_b.peer_heads.insert(
        "node-a".to_string(),
        PeerCommittedHead {
            height: peer_head_height,
            block_hash: peer_head_block_hash,
            committed_at_ms: 7_164,
            observed_at_ms: 7_200,
            execution_block_hash: Some(peer_head_execution_block_hash),
            execution_state_root: Some(peer_head_execution_state_root),
            action_root: empty_action_root(),
            public_key_hex: Some(public_key_a),
            signature_hex: Some(format!("signed-node-a-{peer_head_height}")),
        },
    );
    let mut execution_hook = BootstrapBeforeIncrementalHook {
        installed: Vec::new(),
        incremental_commits: Vec::new(),
        rollback_heights: Vec::new(),
    };

    // Keep the valid height-one commit available and make its clean-local
    // execution result disagree with the signed peer binding.  Height zero
    // rollback is deliberately unavailable; the expected recovery is the
    // authenticated retained checkpoint closure above.
    let result = engine_b.sync_missing_replication_commits(
        &endpoint_b,
        "node-b",
        world_id,
        Some(&mut replication_b),
        Some(&mut execution_hook),
    );
    assert!(
        result.is_ok(),
        "fresh observer must recover through the retained checkpoint after height-one execution mismatch: result={result:?} installed={:?} incremental={:?} rollback={:?}",
        execution_hook.installed,
        execution_hook.incremental_commits,
        execution_hook.rollback_heights
    );
    assert_eq!(execution_hook.incremental_commits, vec![1]);
    assert_eq!(execution_hook.rollback_heights, vec![0]);
    assert_eq!(execution_hook.installed, vec![checkpoint_height]);
    assert_eq!(engine_b.committed_height, checkpoint_height);
    assert_eq!(engine_b.replication_persisted_height, checkpoint_height);
    assert_eq!(engine_b.last_execution_height, checkpoint_height);
    assert!(
        replication_b
            .load_commit_message_by_height(world_id, 1)
            .expect("inspect height-one persistence")
            .is_none(),
        "mismatched height-one commit must not be persisted before checkpoint recovery"
    );
    assert!(
        replication_b
            .load_commit_message_by_height(world_id, checkpoint_height)
            .expect("inspect checkpoint persistence")
            .is_some(),
        "verified retained checkpoint must be persisted after fallback"
    );
    assert!(
        dir_b
            .join("checkpoint-verification")
            .join(format!("{checkpoint_height}.json"))
            .exists(),
        "checkpoint probe receipt must be retained"
    );
    let _ = fs::remove_dir_all(&dir_a);
    let _ = fs::remove_dir_all(&dir_b);
}

#[test]
fn fresh_observer_checkpoint_candidate_discovery_from_fetch_head_only_without_cached_peer_head()
{
    let _nonce_lock = lock_checkpoint_probe_nonce();
    let _probe_nonce = CheckpointProbeNonceGuard::install();
    let world_id = "world-fetch-head-only-retained-checkpoint-784";
    let dir_a = temp_dir("fetch-head-only-retained-checkpoint-784-a");
    let dir_b = temp_dir("fetch-head-only-retained-checkpoint-784-b");
    let (_, public_key_a) = deterministic_keypair_hex(194);
    let (_, public_key_b) = deterministic_keypair_hex(195);
    let advertised_height = 784;
    let checkpoint_height = 768;
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", 194)],
    );
    let replication_config_a = signed_replication_config(dir_a.clone(), 194)
        .with_remote_writer_allowlist(vec![public_key_b.clone()])
        .expect("allowlist a")
        .with_fetch_requester_allowlist(vec![public_key_b.clone()])
        .expect("fetch allowlist a");
    let replication_config_b = signed_replication_config(dir_b.clone(), 195)
        .with_remote_writer_allowlist(vec![public_key_a.clone()])
        .expect("allowlist b")
        .with_fetch_requester_allowlist(vec![public_key_a.clone()])
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
            .expect("source replication runtime");
    replication_a
        .build_local_commit_message(
            "node-a",
            world_id,
            7_784,
            &committed_decision(1),
            Some("peer-execution-block-1"),
            Some("peer-execution-state-1"),
        )
        .expect("build height-one commit")
        .expect("height-one commit");
    let checkpoint_execution_block_hash = format!("execution-block-{checkpoint_height}");
    let checkpoint_execution_state_root = format!("execution-state-{checkpoint_height}");
    replication_a
        .build_local_commit_message_with_checkpoint(
            "node-a",
            world_id,
            7_784,
            &committed_decision(checkpoint_height),
            Some(checkpoint_execution_block_hash.as_str()),
            Some(checkpoint_execution_state_root.as_str()),
            Some(checkpoint_bundle(
                checkpoint_height,
                checkpoint_execution_block_hash.as_str(),
                checkpoint_execution_state_root.as_str(),
            )),
        )
        .expect("build retained checkpoint")
        .expect("retained checkpoint");
    let engine_a = PosNodeEngine::new(&config_a).expect("lineage authority engine");
    let checkpoint_head = CheckpointLineageHeadV1 {
        height: advertised_height,
        block_hash: format!("block-{advertised_height}"),
        state_root: format!("execution-state-{advertised_height}"),
        execution_block_hash: format!("execution-block-{advertised_height}"),
        execution_state_root: format!("execution-state-{advertised_height}"),
    };
    let lineage_envelope =
        super::storage_replication_live_retained_boundary_tests::attach_production_lineage_envelope(
            &mut replication_a,
            world_id,
            checkpoint_height,
            checkpoint_head.clone(),
            &[&engine_a],
        );
    assert_eq!(lineage_envelope.head, checkpoint_head);
    let persisted_checkpoint = replication_a
        .load_commit_message_by_height(world_id, checkpoint_height)
        .expect("load retained checkpoint")
        .expect("retained checkpoint persisted");
    assert!(
        parse_replication_commit_payload(persisted_checkpoint.payload.as_slice())
            .expect("decode retained checkpoint")
            .lineage_envelope
            .is_some(),
        "fixture must carry the authenticated C-to-H lineage envelope"
    );

    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(FetchHeadOnlyCheckpointNetwork {
        inner: Arc::new(TestInMemoryNetwork::default()),
        advertised_head: super::replication::FetchHeadResponse {
            found: true,
            head: Some(super::replication::ReplicationHeadSummary {
                world_id: world_id.to_string(),
                height: advertised_height,
                block_hash: format!("block-{advertised_height}"),
                state_root: format!("execution-state-{advertised_height}"),
                timestamp_ms: 7_784,
            }),
        },
    });
    register_replication_fetch_handlers(
        &NodeReplicationNetworkHandle::new(Arc::clone(&network)),
        &replication_config_a,
        world_id,
        &config_a.network_policy,
    )
    .expect("register authenticated checkpoint provider");
    let handle_b = NodeReplicationNetworkHandle::new(Arc::clone(&network));
    let endpoint_b =
        ReplicationNetworkEndpoint::new(&handle_b, world_id, true, &config_b.network_policy)
            .expect("fresh observer endpoint");
    let mut replication_b =
        ReplicationRuntime::new(config_b.replication.as_ref().expect("repl b"), "node-b")
            .expect("fresh observer replication runtime");
    let mut engine_b = PosNodeEngine::new(&config_b).expect("fresh observer engine");
    // Match the public-testnet probe: the observer already saw the height-one
    // tail, but only FetchHead exposes the current high head. No usable
    // authenticated peer-head cache is available for lineage lookup.
    engine_b.network_committed_height = 1;
    assert!(engine_b.peer_heads.is_empty());
    let mut execution_hook = BootstrapBeforeIncrementalHook {
        installed: Vec::new(),
        incremental_commits: Vec::new(),
        rollback_heights: Vec::new(),
    };

    let result = engine_b.sync_missing_replication_commits(
        &endpoint_b,
        "node-b",
        world_id,
        Some(&mut replication_b),
        Some(&mut execution_hook),
    );
    assert!(
        result.is_ok(),
        "FetchHead-only discovery must install the authenticated retained checkpoint before height-one replay: result={result:?} installed={:?} incremental={:?}",
        execution_hook.installed,
        execution_hook.incremental_commits
    );
    assert_eq!(execution_hook.installed, vec![checkpoint_height]);
    assert!(execution_hook.incremental_commits.is_empty());
    assert_eq!(engine_b.committed_height, checkpoint_height);
    assert_eq!(engine_b.replication_persisted_height, checkpoint_height);
    assert_eq!(engine_b.last_execution_height, checkpoint_height);
    assert!(
        replication_b
            .load_commit_message_by_height(world_id, checkpoint_height)
            .expect("inspect checkpoint persistence")
            .is_some(),
        "verified retained checkpoint must be persisted"
    );
    assert!(
        dir_b
            .join("checkpoint-verification")
            .join(format!("{checkpoint_height}.json"))
            .exists(),
        "checkpoint verification receipt must be retained"
    );
    let _ = fs::remove_dir_all(&dir_a);
    let _ = fs::remove_dir_all(&dir_b);
}

#[test]
fn fresh_observer_fetch_head_lower_checkpoint_without_lineage_fails_closed() {
    let _nonce_lock = lock_checkpoint_probe_nonce();
    let _probe_nonce = CheckpointProbeNonceGuard::install();
    let world_id = "world-fetch-head-only-lower-checkpoint-without-lineage";
    let dir_a = temp_dir("fetch-head-only-lower-without-lineage-a");
    let dir_b = temp_dir("fetch-head-only-lower-without-lineage-b");
    let (_, public_key_a) = deterministic_keypair_hex(196);
    let (_, public_key_b) = deterministic_keypair_hex(197);
    let advertised_height = 784;
    let checkpoint_height = 768;
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", 196)],
    );
    let replication_config_a = signed_replication_config(dir_a.clone(), 196)
        .with_remote_writer_allowlist(vec![public_key_b.clone()])
        .expect("allowlist a")
        .with_fetch_requester_allowlist(vec![public_key_b.clone()])
        .expect("fetch allowlist a");
    let replication_config_b = signed_replication_config(dir_b.clone(), 197)
        .with_remote_writer_allowlist(vec![public_key_a.clone()])
        .expect("allowlist b")
        .with_fetch_requester_allowlist(vec![public_key_a.clone()])
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
            .expect("source replication runtime");
    replication_a
        .build_local_commit_message(
            "node-a",
            world_id,
            7_784,
            &committed_decision(1),
            Some("peer-execution-block-1"),
            Some("peer-execution-state-1"),
        )
        .expect("build height-one commit")
        .expect("height-one commit");
    replication_a
        .build_local_commit_message_with_checkpoint(
            "node-a",
            world_id,
            7_784,
            &committed_decision(checkpoint_height),
            Some("execution-block-768"),
            Some("execution-state-768"),
            Some(checkpoint_bundle(
                checkpoint_height,
                "execution-block-768",
                "execution-state-768",
            )),
        )
        .expect("build retained checkpoint without lineage")
        .expect("retained checkpoint without lineage");

    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(FetchHeadOnlyCheckpointNetwork {
        inner: Arc::new(TestInMemoryNetwork::default()),
        advertised_head: super::replication::FetchHeadResponse {
            found: true,
            head: Some(super::replication::ReplicationHeadSummary {
                world_id: world_id.to_string(),
                height: advertised_height,
                block_hash: format!("block-{advertised_height}"),
                state_root: format!("execution-state-{advertised_height}"),
                timestamp_ms: 7_784,
            }),
        },
    });
    register_replication_fetch_handlers(
        &NodeReplicationNetworkHandle::new(Arc::clone(&network)),
        &replication_config_a,
        world_id,
        &config_a.network_policy,
    )
    .expect("register checkpoint provider");
    let handle_b = NodeReplicationNetworkHandle::new(Arc::clone(&network));
    let endpoint_b =
        ReplicationNetworkEndpoint::new(&handle_b, world_id, true, &config_b.network_policy)
            .expect("fresh observer endpoint");
    let mut replication_b =
        ReplicationRuntime::new(config_b.replication.as_ref().expect("repl b"), "node-b")
            .expect("fresh observer replication runtime");
    let mut engine_b = PosNodeEngine::new(&config_b).expect("fresh observer engine");
    engine_b.network_committed_height = 1;
    assert!(engine_b.peer_heads.is_empty());
    let mut execution_hook = BootstrapBeforeIncrementalHook {
        installed: Vec::new(),
        incremental_commits: Vec::new(),
        rollback_heights: Vec::new(),
    };

    let result = engine_b.sync_missing_replication_commits(
        &endpoint_b,
        "node-b",
        world_id,
        Some(&mut replication_b),
        Some(&mut execution_hook),
    );
    let error = result.expect_err(
        "a lower FetchHead candidate without C-to-H lineage must remain fail-closed",
    );
    assert!(
        error.to_string().contains("execution hash validation failed"),
        "height-one mismatch must remain the terminal failure: {error}"
    );
    assert!(
        execution_hook.installed.is_empty(),
        "no lower checkpoint may install without lineage: {:?}",
        execution_hook.installed
    );
    assert_eq!(engine_b.committed_height, 0);
    assert_eq!(engine_b.replication_persisted_height, 0);
    assert_eq!(engine_b.last_execution_height, 0);
    assert!(
        replication_b
            .load_commit_message_by_height(world_id, checkpoint_height)
            .expect("inspect observer checkpoint persistence")
            .is_none(),
        "a rejected lower checkpoint must not be persisted"
    );
    assert!(
        !dir_b
            .join("checkpoint-verification")
            .join(format!("{checkpoint_height}.json"))
            .exists(),
        "a rejected lower checkpoint must not emit a verification receipt"
    );
    let _ = fs::remove_dir_all(&dir_a);
    let _ = fs::remove_dir_all(&dir_b);
}

#[test]
fn fresh_observer_fetch_head_lower_checkpoint_without_lineage_fails_closed_with_network_cursor()
{
    let _nonce_lock = lock_checkpoint_probe_nonce();
    let _probe_nonce = CheckpointProbeNonceGuard::install();
    let world_id = "world-fetch-head-only-lower-checkpoint-network-cursor";
    let dir_a = temp_dir("fetch-head-only-lower-network-cursor-a");
    let dir_b = temp_dir("fetch-head-only-lower-network-cursor-b");
    let (_, public_key_a) = deterministic_keypair_hex(200);
    let (_, public_key_b) = deterministic_keypair_hex(201);
    let advertised_height = 784;
    let checkpoint_height = 768;
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", 200)],
    );
    let replication_config_a = signed_replication_config(dir_a.clone(), 200)
        .with_remote_writer_allowlist(vec![public_key_b.clone()])
        .expect("allowlist a")
        .with_fetch_requester_allowlist(vec![public_key_b.clone()])
        .expect("fetch allowlist a");
    let replication_config_b = signed_replication_config(dir_b.clone(), 201)
        .with_remote_writer_allowlist(vec![public_key_a.clone()])
        .expect("allowlist b")
        .with_fetch_requester_allowlist(vec![public_key_a.clone()])
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
            .expect("source replication runtime");
    replication_a
        .build_local_commit_message(
            "node-a",
            world_id,
            7_784,
            &committed_decision(1),
            Some("peer-execution-block-1"),
            Some("peer-execution-state-1"),
        )
        .expect("build height-one commit")
        .expect("height-one commit");
    replication_a
        .build_local_commit_message_with_checkpoint(
            "node-a",
            world_id,
            7_784,
            &committed_decision(checkpoint_height),
            Some("execution-block-768"),
            Some("execution-state-768"),
            Some(checkpoint_bundle(
                checkpoint_height,
                "execution-block-768",
                "execution-state-768",
            )),
        )
        .expect("build retained checkpoint without lineage")
        .expect("retained checkpoint without lineage");

    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(FetchHeadOnlyCheckpointNetwork {
        inner: Arc::new(TestInMemoryNetwork::default()),
        advertised_head: super::replication::FetchHeadResponse {
            found: true,
            head: Some(super::replication::ReplicationHeadSummary {
                world_id: world_id.to_string(),
                height: advertised_height,
                block_hash: format!("block-{advertised_height}"),
                state_root: format!("execution-state-{advertised_height}"),
                timestamp_ms: 7_784,
            }),
        },
    });
    register_replication_fetch_handlers(
        &NodeReplicationNetworkHandle::new(Arc::clone(&network)),
        &replication_config_a,
        world_id,
        &config_a.network_policy,
    )
    .expect("register checkpoint provider");
    let handle_b = NodeReplicationNetworkHandle::new(Arc::clone(&network));
    let endpoint_b =
        ReplicationNetworkEndpoint::new(&handle_b, world_id, true, &config_b.network_policy)
            .expect("fresh observer endpoint");
    let mut replication_b =
        ReplicationRuntime::new(config_b.replication.as_ref().expect("repl b"), "node-b")
            .expect("fresh observer replication runtime");
    let mut engine_b = PosNodeEngine::new(&config_b).expect("fresh observer engine");
    // Keep local execution clean while modelling a previously observed network cursor above one.
    engine_b.network_committed_height = 64;
    assert_eq!(engine_b.committed_height, 0);
    assert_eq!(engine_b.replication_persisted_height, 0);
    assert_eq!(engine_b.last_execution_height, 0);
    assert!(engine_b.peer_heads.is_empty());
    let mut execution_hook = BootstrapBeforeIncrementalHook {
        installed: Vec::new(),
        incremental_commits: Vec::new(),
        rollback_heights: Vec::new(),
    };

    let result = engine_b.sync_missing_replication_commits(
        &endpoint_b,
        "node-b",
        world_id,
        Some(&mut replication_b),
        Some(&mut execution_hook),
    );
    assert!(
        result.is_err(),
        "a lower FetchHead candidate without C-to-H lineage must remain fail-closed above cursor one: result={result:?} installed={:?} committed={} persisted={} executed={}",
        execution_hook.installed,
        engine_b.committed_height,
        engine_b.replication_persisted_height,
        engine_b.last_execution_height
    );
    let error = result.expect_err(
        "a lower FetchHead candidate without C-to-H lineage must remain fail-closed above cursor one",
    );
    assert!(
        error.to_string().contains("execution hash validation failed"),
        "height-one mismatch must remain the terminal failure: {error}"
    );
    assert!(
        execution_hook.installed.is_empty(),
        "no lower checkpoint may install without lineage: {:?}",
        execution_hook.installed
    );
    assert_eq!(engine_b.committed_height, 0);
    assert_eq!(engine_b.replication_persisted_height, 0);
    assert_eq!(engine_b.last_execution_height, 0);
    assert_eq!(engine_b.network_committed_height, 64);
    assert!(
        replication_b
            .load_commit_message_by_height(world_id, checkpoint_height)
            .expect("inspect observer checkpoint persistence")
            .is_none(),
        "a rejected lower checkpoint must not be persisted"
    );
    assert!(
        !dir_b
            .join("checkpoint-verification")
            .join(format!("{checkpoint_height}.json"))
            .exists(),
        "a rejected lower checkpoint must not emit a verification receipt"
    );
    let _ = fs::remove_dir_all(&dir_a);
    let _ = fs::remove_dir_all(&dir_b);
}

#[test]
fn fresh_observer_fetch_head_refreshes_after_height_one_execution_mismatch() {
    let _nonce_lock = lock_checkpoint_probe_nonce();
    let _probe_nonce = CheckpointProbeNonceGuard::install();
    let world_id = "world-fetch-head-refresh-after-height-one-mismatch";
    let dir_a = temp_dir("fetch-head-refresh-after-mismatch-a");
    let dir_b = temp_dir("fetch-head-refresh-after-mismatch-b");
    let (_, public_key_a) = deterministic_keypair_hex(198);
    let (_, public_key_b) = deterministic_keypair_hex(199);
    let advertised_height = 784;
    let checkpoint_height = 768;
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", 198)],
    );
    let replication_config_a = signed_replication_config(dir_a.clone(), 198)
        .with_remote_writer_allowlist(vec![public_key_b.clone()])
        .expect("allowlist a")
        .with_fetch_requester_allowlist(vec![public_key_b.clone()])
        .expect("fetch allowlist a");
    let replication_config_b = signed_replication_config(dir_b.clone(), 199)
        .with_remote_writer_allowlist(vec![public_key_a.clone()])
        .expect("allowlist b")
        .with_fetch_requester_allowlist(vec![public_key_a.clone()])
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
            .expect("source replication runtime");
    replication_a
        .build_local_commit_message(
            "node-a",
            world_id,
            7_784,
            &committed_decision(1),
            Some("peer-execution-block-1"),
            Some("peer-execution-state-1"),
        )
        .expect("build height-one commit")
        .expect("height-one commit");
    replication_a
        .build_local_commit_message_with_checkpoint(
            "node-a",
            world_id,
            7_784,
            &committed_decision(checkpoint_height),
            Some("execution-block-768"),
            Some("execution-state-768"),
            Some(checkpoint_bundle(
                checkpoint_height,
                "execution-block-768",
                "execution-state-768",
            )),
        )
        .expect("build retained checkpoint")
        .expect("retained checkpoint");
    let engine_a = PosNodeEngine::new(&config_a).expect("lineage authority engine");
    let checkpoint_head = CheckpointLineageHeadV1 {
        height: advertised_height,
        block_hash: format!("block-{advertised_height}"),
        state_root: format!("execution-state-{advertised_height}"),
        execution_block_hash: format!("execution-block-{advertised_height}"),
        execution_state_root: format!("execution-state-{advertised_height}"),
    };
    let lineage_envelope =
        super::storage_replication_live_retained_boundary_tests::attach_production_lineage_envelope(
            &mut replication_a,
            world_id,
            checkpoint_height,
            checkpoint_head.clone(),
            &[&engine_a],
        );
    assert_eq!(lineage_envelope.head, checkpoint_head);

    let advertised_heads = Arc::new(Mutex::new(vec![
        super::replication::FetchHeadResponse {
            found: false,
            head: None,
        },
        super::replication::FetchHeadResponse {
            found: true,
            head: Some(super::replication::ReplicationHeadSummary {
                world_id: world_id.to_string(),
                height: advertised_height,
                block_hash: format!("block-{advertised_height}"),
                state_root: format!("execution-state-{advertised_height}"),
                timestamp_ms: 7_784,
            }),
        },
    ]));
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(SequencedFetchHeadCheckpointNetwork {
        inner: Arc::new(TestInMemoryNetwork::default()),
        advertised_heads: Arc::clone(&advertised_heads),
    });
    register_replication_fetch_handlers(
        &NodeReplicationNetworkHandle::new(Arc::clone(&network)),
        &replication_config_a,
        world_id,
        &config_a.network_policy,
    )
    .expect("register authenticated checkpoint provider");
    let handle_b = NodeReplicationNetworkHandle::new(Arc::clone(&network));
    let endpoint_b =
        ReplicationNetworkEndpoint::new(&handle_b, world_id, true, &config_b.network_policy)
            .expect("fresh observer endpoint");
    let mut replication_b =
        ReplicationRuntime::new(config_b.replication.as_ref().expect("repl b"), "node-b")
            .expect("fresh observer replication runtime");
    let mut engine_b = PosNodeEngine::new(&config_b).expect("fresh observer engine");
    engine_b.network_committed_height = 1;
    assert!(engine_b.peer_heads.is_empty());
    let mut execution_hook = BootstrapBeforeIncrementalHook {
        installed: Vec::new(),
        incremental_commits: Vec::new(),
        rollback_heights: Vec::new(),
    };

    let persisted_height_one = replication_a
        .load_commit_message_by_height(world_id, 1)
        .expect("inspect source height-one commit")
        .expect("source height-one commit");
    let height_one_payload =
        super::replication_state_reconcile::parse_replication_commit_payload(
            persisted_height_one.payload.as_slice(),
        )
        .expect("decode source height-one commit");
    let structured_error = engine_b
        .execute_synced_replication_commit(world_id, &height_one_payload, Some(&mut execution_hook))
        .expect_err("height-one execution mismatch must be structured before fallback");
    assert!(
        matches!(
            structured_error,
            NodeError::ExecutionMismatchRollbackUnavailable {
                payload_height: 1,
                rollback_height: 0,
                ..
            }
        ),
        "height-one mismatch must preserve its structured rollback-unavailable type: {structured_error:?}"
    );
    assert!(
        structured_error
            .to_string()
            .contains("execution hash validation failed"),
        "structured mismatch must retain the execution validation diagnostic: {structured_error}"
    );
    execution_hook.incremental_commits.clear();
    execution_hook.rollback_heights.clear();

    let result = engine_b.sync_missing_replication_commits(
        &endpoint_b,
        "node-b",
        world_id,
        Some(&mut replication_b),
        Some(&mut execution_hook),
    );
    assert!(
        result.is_ok(),
        "FetchHead refresh must recover after height-one mismatch: result={result:?} installed={:?} incremental={:?} rollback={:?}",
        execution_hook.installed,
        execution_hook.incremental_commits,
        execution_hook.rollback_heights
    );
    assert_eq!(execution_hook.incremental_commits, vec![1]);
    assert_eq!(execution_hook.rollback_heights, vec![0]);
    assert_eq!(execution_hook.installed, vec![checkpoint_height]);
    assert_eq!(engine_b.committed_height, checkpoint_height);
    assert_eq!(engine_b.replication_persisted_height, checkpoint_height);
    assert_eq!(engine_b.last_execution_height, checkpoint_height);
    assert!(
        replication_b
            .load_commit_message_by_height(world_id, 1)
            .expect("inspect height-one persistence")
            .is_none(),
        "mismatched height-one commit must not be persisted before checkpoint recovery"
    );
    assert!(
        replication_b
            .load_commit_message_by_height(world_id, checkpoint_height)
            .expect("inspect checkpoint persistence")
            .is_some(),
        "verified retained checkpoint must be persisted after fallback"
    );
    assert!(
        dir_b
            .join("checkpoint-verification")
            .join(format!("{checkpoint_height}.json"))
            .exists(),
        "checkpoint verification receipt must be retained after fallback"
    );
    assert!(
        advertised_heads
            .lock()
            .expect("advertised FetchHead responses")
            .is_empty(),
        "the fixture must consume the stale/absent and refreshed FetchHead responses"
    );
    let _ = fs::remove_dir_all(&dir_a);
    let _ = fs::remove_dir_all(&dir_b);
}

#[test]
fn rendered_replication_or_execution_error_cannot_qualify_checkpoint_fallback() {
    // The recovery decision must consume a structured execution outcome.  A
    // caller-controlled Display string that happens to contain the old
    // diagnostic phrases must not become checkpoint authority.
    let rendered_errors = [
        NodeError::Replication {
            reason: "execution hash validation failed; rollback record for height 0 is unavailable"
                .to_string(),
        },
        NodeError::Execution {
            reason: "execution driver peer mismatch; rollback record for height 0 is unavailable"
                .to_string(),
        },
    ];

    for error in rendered_errors {
        assert!(
            !PosNodeEngine::replication_gap_sync_local_state_blocked_reason(
                error.to_string().as_str()
            ),
            "rendered error text must not qualify checkpoint recovery: {error}"
        );
    }
}

struct FreshObserverWithoutCheckpointFixture {
    world_id: String,
    dir_a: std::path::PathBuf,
    dir_b: std::path::PathBuf,
    endpoint_b: ReplicationNetworkEndpoint,
    replication_b: ReplicationRuntime,
    engine_b: PosNodeEngine,
}

fn fresh_observer_without_authenticated_checkpoint_fixture() ->
    FreshObserverWithoutCheckpointFixture {
    let world_id = "world-fresh-observer-no-authenticated-checkpoint".to_string();
    let dir_a = temp_dir("fresh-observer-no-authenticated-checkpoint-a");
    let dir_b = temp_dir("fresh-observer-no-authenticated-checkpoint-b");
    let (_, public_key_a) = deterministic_keypair_hex(194);
    let (_, public_key_b) = deterministic_keypair_hex(195);
    let peer_head_height = REPLICATION_GAP_SYNC_MAX_HEIGHTS_PER_POLL + 36;
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", 194)],
    );
    let replication_config_a = signed_replication_config(dir_a.clone(), 194)
        .with_remote_writer_allowlist(vec![public_key_b.clone()])
        .expect("allowlist a")
        .with_fetch_requester_allowlist(vec![public_key_b])
        .expect("fetch allowlist a");
    let replication_config_b = signed_replication_config(dir_b.clone(), 195)
        .with_remote_writer_allowlist(vec![public_key_a.clone()])
        .expect("allowlist b")
        .with_fetch_requester_allowlist(vec![public_key_a.clone()])
        .expect("fetch allowlist b");
    let config_a = NodeConfig::new("node-a", world_id.as_str(), NodeRole::Sequencer)
        .expect("config a")
        .with_pos_config(pos_config.clone())
        .expect("pos config a")
        .with_replication(replication_config_a.clone());
    let config_b = NodeConfig::new("node-b", world_id.as_str(), NodeRole::Observer)
        .expect("config b")
        .with_pos_config(pos_config)
        .expect("pos config b")
        .with_require_execution_on_commit(true)
        .with_replication(replication_config_b.clone());
    let mut replication_a =
        ReplicationRuntime::new(config_a.replication.as_ref().expect("repl a"), "node-a")
            .expect("remote replication runtime");
    replication_a
        .build_local_commit_message(
            "node-a",
            world_id.as_str(),
            7_100,
            &committed_decision(1),
            Some("peer-execution-block-1"),
            Some("peer-execution-state-1"),
        )
        .expect("build valid height-one commit")
        .expect("height-one commit");
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(FirstReadyHeadCheckpointNetwork {
        inner: Arc::new(TestInMemoryNetwork::default()),
        fetch_protocols: Arc::new(Mutex::new(Vec::new())),
    });
    register_replication_fetch_handlers(
        &NodeReplicationNetworkHandle::new(Arc::clone(&network)),
        &replication_config_a,
        world_id.as_str(),
        &config_a.network_policy,
    )
    .expect("register authenticated provider without checkpoint");
    let handle_b = NodeReplicationNetworkHandle::new(Arc::clone(&network));
    let endpoint_b =
        ReplicationNetworkEndpoint::new(&handle_b, world_id.as_str(), true, &config_b.network_policy)
            .expect("fresh observer endpoint");
    let replication_b =
        ReplicationRuntime::new(config_b.replication.as_ref().expect("repl b"), "node-b")
            .expect("fresh observer replication runtime");
    let mut engine_b = PosNodeEngine::new(&config_b).expect("fresh observer engine");
    engine_b.network_committed_height = 1;
    engine_b.peer_heads.insert(
        "node-a".to_string(),
        PeerCommittedHead {
            height: peer_head_height,
            block_hash: format!("block-{peer_head_height}"),
            committed_at_ms: 7_164,
            observed_at_ms: 7_200,
            execution_block_hash: Some(format!("execution-block-{peer_head_height}")),
            execution_state_root: Some(format!("execution-state-{peer_head_height}")),
            action_root: empty_action_root(),
            public_key_hex: Some(public_key_a),
            signature_hex: Some(format!("signed-node-a-{peer_head_height}")),
        },
    );
    engine_b.checkpoint_bootstrap_enabled = true;
    FreshObserverWithoutCheckpointFixture {
        world_id,
        dir_a,
        dir_b,
        endpoint_b,
        replication_b,
        engine_b,
    }
}

#[test]
fn execution_mismatch_without_authenticated_checkpoint_remains_fail_closed() {
    let _nonce_lock = lock_checkpoint_probe_nonce();
    let fixture = fresh_observer_without_authenticated_checkpoint_fixture();
    let mut replication_b = fixture.replication_b;
    let mut engine_b = fixture.engine_b;
    let mut execution_hook = BootstrapBeforeIncrementalHook {
        installed: Vec::new(),
        incremental_commits: Vec::new(),
        rollback_heights: Vec::new(),
    };

    let result = engine_b.sync_missing_replication_commits(
        &fixture.endpoint_b,
        "node-b",
        fixture.world_id.as_str(),
        Some(&mut replication_b),
        Some(&mut execution_hook),
    );
    let error = result.expect_err(
        "a target mismatch without an authenticated checkpoint must remain fail-closed",
    );
    let rendered = error.to_string();
    assert!(
        rendered.contains("execution hash validation failed"),
        "original execution mismatch must be preserved: {rendered}"
    );
    assert!(
        rendered.contains("rollback record for height 0 is unavailable"),
        "original rollback failure must be preserved: {rendered}"
    );
    assert!(
        execution_hook.installed.is_empty(),
        "no unauthenticated checkpoint may be installed: {:?}",
        execution_hook.installed
    );
    assert_eq!(engine_b.committed_height, 0);
    assert_eq!(engine_b.replication_persisted_height, 0);
    assert!(
        !fixture.dir_b.join("checkpoint-verification").exists(),
        "no checkpoint receipt may be emitted without authenticated closure"
    );
    let _ = fs::remove_dir_all(&fixture.dir_a);
    let _ = fs::remove_dir_all(&fixture.dir_b);
}
