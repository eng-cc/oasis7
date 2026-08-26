use super::*;
use oasis7_proto::distributed_net::DistributedNetwork;

struct CurrentWindowCheckpointExportHook {
    bundle: NodeExecutionCheckpointBundle,
}
impl NodeExecutionHook for CurrentWindowCheckpointExportHook {
    fn on_commit(
        &mut self,
        context: NodeExecutionCommitContext,
    ) -> Result<NodeExecutionCommitResult, String> {
        Ok(NodeExecutionCommitResult {
            execution_height: context.height,
            execution_block_hash: format!("exec-block-{}", context.height),
            execution_state_root: format!("exec-state-{}", context.height),
        })
    }

    fn export_checkpoint_bundle(
        &mut self,
        height: u64,
    ) -> Result<Option<NodeExecutionCheckpointBundle>, String> {
        Ok((self.bundle.height == height).then(|| self.bundle.clone()))
    }
}

#[test]
fn production_checkpoint_replication_persists_local_source_without_self_echo_and_reaches_quorum() {
    let world_id = "world-production-lineage-no-self-echo";
    let source_dir = temp_dir("production-lineage-no-self-echo-source");
    let peer_dir = temp_dir("production-lineage-no-self-echo-peer");
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
    let pos_config = signed_pos_config_with_signer_seeds(
        validators,
        &[("node-a", 241), ("node-b", 242)],
    );
    let (_, source_public_key) = deterministic_keypair_hex(241);
    let source_config = NodeConfig::new("node-a", world_id, NodeRole::Sequencer)
        .expect("source config")
        .with_pos_config(pos_config.clone())
        .expect("source pos config")
        .with_replication(signed_replication_config(source_dir.clone(), 241));
    let peer_replication_config = signed_replication_config(peer_dir.clone(), 242)
        .with_remote_writer_allowlist(vec![source_public_key])
        .expect("peer remote writer allowlist");
    let peer_config = NodeConfig::new("node-b", world_id, NodeRole::Storage)
        .expect("peer config")
        .with_network_policy(NodeNetworkPolicy {
            deployment_mode: oasis7_proto::distributed_dht::PeerDeploymentMode::Private,
            node_role_claim: oasis7_proto::distributed_dht::PeerNodeRole::ValidatorCore,
        })
        .expect("peer validator-core policy")
        .with_pos_config(pos_config)
        .expect("peer pos config")
        .with_replication(peer_replication_config);

    let network_impl = Arc::new(TestInMemoryNetwork::default());
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = network_impl.clone();
    let source_handle = NodeReplicationNetworkHandle::new(Arc::clone(&network));
    let source_replication_endpoint = ReplicationNetworkEndpoint::new(
        &source_handle,
        world_id,
        false,
        &source_config.network_policy,
    )
    .expect("source replication endpoint");
    let source_consensus_endpoint = ConsensusNetworkEndpoint::new(
        &source_handle,
        world_id,
        false,
        &source_config.network_policy,
    )
    .expect("source consensus endpoint");
    let lineage_subscription = network_impl
        .subscribe(super::network_bridge::default_consensus_lineage_topic(world_id).as_str())
        .expect("lineage subscription");
    let mut source_replication = ReplicationRuntime::new(
        source_config.replication.as_ref().expect("source replication config"),
        "node-a",
    )
    .expect("source replication runtime");
    let mut source_engine = PosNodeEngine::new(&source_config).expect("source engine");
    let source_slot = (0..256)
        .find(|slot| source_engine.expected_proposer(*slot).as_deref() == Some("node-a"))
        .expect("source proposer slot");
    let checkpoint_height = 64;
    for height in 1..checkpoint_height {
        let block_hash = format!("production-history-block-{height}");
        let execution_block_hash = format!("production-history-execution-block-{height}");
        let execution_state_root = format!("production-history-execution-state-{height}");
        let message = source_replication
            .build_local_commit_message(
                "node-a",
                world_id,
                height as i64,
                &committed_decision_with_block_hash(height, block_hash.as_str()),
                Some(execution_block_hash.as_str()),
                Some(execution_state_root.as_str()),
            )
            .expect("build production contiguous history")
            .expect("production contiguous history message");
        source_replication_endpoint
            .publish_replication(&message)
            .expect("publish production contiguous history");
    }
    let checkpoint_block_hash = "production-checkpoint-block-64";
    let execution_block_hash = "production-execution-block-64";
    let execution_state_root = "production-execution-state-64";
    source_engine.last_execution_height = checkpoint_height;
    source_engine.last_execution_block_hash = Some(execution_block_hash.to_string());
    source_engine.last_execution_state_root = Some(execution_state_root.to_string());
    let decision = PosDecision {
        height: checkpoint_height,
        slot: source_slot,
        epoch: 0,
        proposer_id: "node-a".to_string(),
        status: PosConsensusStatus::Committed,
        block_hash: checkpoint_block_hash.to_string(),
        action_root: empty_action_root(),
        committed_actions: Vec::new(),
        approved_stake: 100,
        rejected_stake: 0,
        required_stake: 67,
        total_stake: 100,
    };
    let mut export_hook = CurrentWindowCheckpointExportHook {
        bundle: lineage_checkpoint_bundle(
            world_id,
            checkpoint_height,
            checkpoint_block_hash,
            execution_block_hash,
            execution_state_root,
        ),
    };
    source_engine
        .broadcast_local_replication(
            None,
            Some(&source_replication_endpoint),
            "node-a",
            world_id,
            64_000,
            &decision,
            Some(&mut source_replication),
            Some(&mut export_hook),
        )
        .expect("production source replication");

    assert!(
        source_replication
            .load_commit_message_by_height(world_id, checkpoint_height)
            .expect("load local production checkpoint")
            .is_some(),
        "production replication must persist the signed local checkpoint message"
    );
    let source_lineage_path = source_dir
        .join("checkpoint-lineage")
        .join(format!("source-{checkpoint_height}.json"));
    assert!(
        source_lineage_path.exists(),
        "local signed checkpoint source must persist without relying on a self-echo: path={} ",
        source_lineage_path.display()
    );

    let peer_handle = NodeReplicationNetworkHandle::new(Arc::clone(&network));
    let peer_replication_endpoint = ReplicationNetworkEndpoint::new(
        &peer_handle,
        world_id,
        true,
        &peer_config.network_policy,
    )
    .expect("peer replication endpoint");
    let peer_consensus_endpoint = ConsensusNetworkEndpoint::new(
        &peer_handle,
        world_id,
        false,
        &peer_config.network_policy,
    )
    .expect("peer consensus endpoint");
    let mut peer_replication = ReplicationRuntime::new(
        peer_config.replication.as_ref().expect("peer replication config"),
        "node-b",
    )
    .expect("peer replication runtime");
    peer_replication
        .store_execution_checkpoint_bundle(&lineage_checkpoint_bundle(
            world_id,
            checkpoint_height,
            checkpoint_block_hash,
            execution_block_hash,
            execution_state_root,
        ))
        .expect("seed peer checkpoint blob closure");
    let mut peer_engine = PosNodeEngine::new(&peer_config).expect("peer engine");
    peer_engine
        .ingest_network_replications(
            &peer_replication_endpoint,
            "node-b",
            world_id,
            Some(&mut peer_replication),
            None,
        )
        .expect("peer ingest source checkpoint");
    let head = signed_lineage_head(world_id, "node-b", 242, 128);
    source_engine.observe_peer_commit_message(&head);
    peer_engine.observe_peer_commit_message(&head);
    source_engine
        .maybe_publish_local_checkpoint_lineage_vote(
            Some(&source_consensus_endpoint),
            None,
            "node-a",
            world_id,
            Some(&mut source_replication),
        )
        .expect("source lineage vote");
    peer_engine
        .maybe_publish_local_checkpoint_lineage_vote(
            Some(&peer_consensus_endpoint),
            None,
            "node-b",
            world_id,
            Some(&mut peer_replication),
        )
        .expect("peer lineage vote");
    let peer_vote = lineage_subscription
        .drain()
        .into_iter()
        .filter_map(|payload| serde_json::from_slice::<GossipMessage>(payload.as_slice()).ok())
        .find_map(|message| match message {
            GossipMessage::CheckpointLineageVote(vote)
                if vote.vote.validator_id == "node-b" =>
            {
                Some(vote)
            }
            _ => None,
        })
        .expect("peer signed lineage vote");
    source_engine
        .ingest_checkpoint_lineage_vote_message(
            "node-a",
            world_id,
            &peer_vote,
            Some(&mut source_replication),
        )
        .expect("attach quorum lineage envelope");
    let attached = source_replication
        .load_commit_message_by_height(world_id, checkpoint_height)
        .expect("reload source checkpoint with quorum envelope")
        .expect("source checkpoint remains persisted");
    let attached_payload =
        super::replication_state_reconcile::parse_replication_commit_payload(attached.payload.as_slice())
            .expect("parse quorum-attached checkpoint");
    assert!(
        attached_payload.lineage_envelope.is_some(),
        "quorum lineage must attach to the persisted signed local checkpoint"
    );

    let _ = fs::remove_dir_all(source_dir);
    let _ = fs::remove_dir_all(peer_dir);
}

#[test]
fn production_checkpoint_lineage_fails_closed_without_authenticated_head() {
    let world_id = "world-production-lineage-no-authenticated-head";
    let source_dir = temp_dir("production-lineage-no-authenticated-head");
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", 243)],
    );
    let config = NodeConfig::new("node-a", world_id, NodeRole::Sequencer)
        .expect("config")
        .with_pos_config(pos_config)
        .expect("pos config")
        .with_replication(signed_replication_config(source_dir.clone(), 243));
    let mut replication = ReplicationRuntime::new(
        config.replication.as_ref().expect("replication config"),
        "node-a",
    )
    .expect("replication runtime");
    let message = replication
        .build_local_commit_message_with_checkpoint(
            "node-a",
            world_id,
            64,
            &committed_decision_with_block_hash(64, "no-head-checkpoint-block"),
            Some("no-head-execution-block"),
            Some("no-head-execution-state"),
            Some(lineage_checkpoint_bundle(
                world_id,
                64,
                "no-head-checkpoint-block",
                "no-head-execution-block",
                "no-head-execution-state",
            )),
        )
        .expect("build signed source")
        .expect("signed source message");
    replication
        .persist_local_checkpoint_message_for_lineage("node-a", world_id, &message)
        .expect("persist signed source");
    let network_impl = Arc::new(TestInMemoryNetwork::default());
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = network_impl.clone();
    let handle = NodeReplicationNetworkHandle::new(network);
    let endpoint = ConsensusNetworkEndpoint::new(&handle, world_id, false, &config.network_policy)
        .expect("consensus endpoint");
    let lineage_subscription = network_impl
        .subscribe(super::network_bridge::default_consensus_lineage_topic(world_id).as_str())
        .expect("lineage subscription");
    let mut engine = PosNodeEngine::new(&config).expect("engine");
    engine
        .maybe_publish_local_checkpoint_lineage_vote(
            Some(&endpoint),
            None,
            "node-a",
            world_id,
            Some(&mut replication),
        )
        .expect("missing authenticated head must fail closed without error");
    assert!(
        lineage_subscription.drain().is_empty(),
        "missing authenticated head must not publish a lineage vote"
    );
    assert!(
        replication
            .load_commit_message_by_height(world_id, 64)
            .expect("reload source")
            .and_then(|message| {
                super::replication_state_reconcile::parse_replication_commit_payload(
                    message.payload.as_slice(),
                )
            })
            .is_some_and(|payload| payload.lineage_envelope.is_none()),
        "missing authenticated head must not attach an envelope"
    );
    let _ = fs::remove_dir_all(source_dir);
}

#[test]
fn production_checkpoint_lineage_fails_closed_without_valid_signed_source() {
    let world_id = "world-production-lineage-no-valid-source";
    let source_dir = temp_dir("production-lineage-no-valid-source");
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![
            PosValidator {
                validator_id: "node-a".to_string(),
                stake: 60,
            },
            PosValidator {
                validator_id: "node-b".to_string(),
                stake: 40,
            },
        ],
        &[("node-a", 244), ("node-b", 245)],
    );
    let config = NodeConfig::new("node-a", world_id, NodeRole::Sequencer)
        .expect("config")
        .with_pos_config(pos_config)
        .expect("pos config")
        .with_replication(signed_replication_config(source_dir.clone(), 244));
    let network_impl = Arc::new(TestInMemoryNetwork::default());
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = network_impl.clone();
    let handle = NodeReplicationNetworkHandle::new(network);
    let endpoint = ConsensusNetworkEndpoint::new(&handle, world_id, false, &config.network_policy)
        .expect("consensus endpoint");
    let lineage_subscription = network_impl
        .subscribe(super::network_bridge::default_consensus_lineage_topic(world_id).as_str())
        .expect("lineage subscription");
    let mut replication = ReplicationRuntime::new(
        config.replication.as_ref().expect("replication config"),
        "node-a",
    )
    .expect("replication runtime");
    let mut engine = PosNodeEngine::new(&config).expect("engine");
    engine.observe_peer_commit_message(&signed_lineage_head(world_id, "node-b", 245, 128));
    engine
        .maybe_publish_local_checkpoint_lineage_vote(
            Some(&endpoint),
            None,
            "node-a",
            world_id,
            Some(&mut replication),
        )
        .expect("missing valid signed source must fail closed without error");
    assert!(
        lineage_subscription.drain().is_empty(),
        "missing valid signed source must not publish a lineage vote"
    );
    let source_sidecars = fs::read_dir(source_dir.join("checkpoint-lineage"))
        .expect("lineage directory")
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_str()
                .is_some_and(|name| name.starts_with("source-"))
        })
        .count();
    assert_eq!(
        source_sidecars, 0,
        "missing valid signed source must not create lineage sidecars"
    );
    let _ = fs::remove_dir_all(source_dir);
}
