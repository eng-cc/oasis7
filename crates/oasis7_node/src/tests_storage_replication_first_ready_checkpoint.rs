use std::fs;

use super::*;

#[derive(Clone)]
struct FirstReadyHeadCheckpointNetwork {
    inner: Arc<TestInMemoryNetwork>,
    fetch_protocols: Arc<Mutex<Vec<String>>>,
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
