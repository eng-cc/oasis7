struct CheckpointInstallingExecutionHook {
    installed: Vec<u64>,
}

static CHECKPOINT_PROBE_NONCE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

pub(super) fn lock_checkpoint_probe_nonce() -> std::sync::MutexGuard<'static, ()> {
    CHECKPOINT_PROBE_NONCE_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub(super) struct CheckpointProbeNonceGuard;

impl CheckpointProbeNonceGuard {
    pub(super) fn install() -> Self {
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

#[test]
fn authenticated_remote_checkpoint_refetches_same_size_corrupt_local_member_from_provider() {
    let world_id = "world-authenticated-remote-checkpoint-refetch-corrupt-local-member";
    let dir_a = temp_dir("authenticated-remote-checkpoint-refetch-corrupt-local-member-a");
    let dir_b = temp_dir("authenticated-remote-checkpoint-refetch-corrupt-local-member-b");
    let (_, public_key_a) = deterministic_keypair_hex(230);
    let (_, public_key_b) = deterministic_keypair_hex(231);
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", 230)],
    );
    let replication_config_a = signed_replication_config(dir_a.clone(), 230)
        .with_remote_writer_allowlist(vec![public_key_b.clone()])
        .expect("allowlist a")
        .with_fetch_requester_allowlist(vec![public_key_b])
        .expect("fetch allowlist a");
    let replication_config_b = signed_replication_config(dir_b.clone(), 231)
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
    let bundle = checkpoint_bundle(64, "exec-block-64", "exec-state-64");
    let replication_a =
        ReplicationRuntime::new(config_a.replication.as_ref().expect("repl a"), "node-a")
            .expect("runtime a");
    let descriptor = replication_a
        .store_execution_checkpoint_bundle(&bundle)
        .expect("store remote checkpoint closure");
    let corrupt_hash = descriptor
        .blobs
        .first()
        .expect("checkpoint blob ref")
        .content_hash
        .clone();
    let corrupt_bytes = vec![0_u8; bundle.blobs.first().expect("checkpoint blob").bytes.len()];
    assert_ne!(
        oasis7_distfs::blake3_hex(corrupt_bytes.as_slice()),
        corrupt_hash,
        "fixture must contain a same-size, wrong-hash local CAS member"
    );

    let replication_b =
        ReplicationRuntime::new(config_b.replication.as_ref().expect("repl b"), "node-b")
            .expect("runtime b");
    fs::create_dir_all(dir_b.join("store").join("blobs")).expect("create local CAS blob directory");
    fs::write(
        dir_b
            .join("store")
            .join("blobs")
            .join(format!("{corrupt_hash}.blob")),
        corrupt_bytes.as_slice(),
    )
    .expect("seed corrupt local CAS member");

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
    .expect("register healthy authenticated provider handlers");
    let dht = Arc::new(TestReplicaMaintenanceDht::new(
        "node-a-provider",
        "node-b-provider",
    ));
    dht.seed_provider(descriptor.manifest_ref.as_str(), "node-a");
    for blob in &descriptor.blobs {
        dht.seed_provider(blob.content_hash.as_str(), "node-a");
    }
    let handle_b = NodeReplicationNetworkHandle::new(Arc::clone(&network))
        .with_dht(dht)
        .with_local_provider_id("node-b-provider");
    let endpoint_b =
        ReplicationNetworkEndpoint::new(&handle_b, world_id, false, &config_b.network_policy)
            .expect("fresh observer endpoint");
    let mut engine_b = PosNodeEngine::new(&config_b).expect("fresh observer engine");

    let (fetched_bundle, observations) = engine_b
        .fetch_execution_checkpoint_bundle(&endpoint_b, world_id, &replication_b, &descriptor, true)
        .expect("healthy authenticated provider must repair corrupt local CAS before closure");
    assert_eq!(
        fetched_bundle, bundle,
        "authenticated refetch must materialize the complete hash-valid checkpoint closure"
    );
    let repaired = replication_b
        .load_blob_by_hash(corrupt_hash.as_str())
        .expect("load repaired local checkpoint member")
        .expect("repaired checkpoint member");
    assert_eq!(
        repaired,
        bundle.blobs.first().expect("checkpoint blob").bytes,
        "successful authenticated refetch must replace the corrupt active CAS member"
    );
    assert!(
        observations.iter().any(|observation| {
            observation["content_hash"].as_str() == Some(corrupt_hash.as_str())
                && observation["source"].as_str() == Some("network_fetch")
                && observation["signed_request"].as_bool() == Some(true)
                && observation["observed_content_hash"].as_str() == Some(corrupt_hash.as_str())
        }),
        "repair must retain signed network-fetch provenance for the corrupt member: {observations:?}"
    );
    assert!(
        fetch_protocols
            .lock()
            .expect("lock checkpoint fetch protocols")
            .iter()
            .any(|protocol| protocol == REPLICATION_FETCH_BLOB_PROTOCOL),
        "healthy provider recovery must request checkpoint blobs through the network"
    );

    let _ = fs::remove_dir_all(&dir_a);
    let _ = fs::remove_dir_all(&dir_b);
}

#[test]
fn authenticated_remote_checkpoint_rejects_descriptor_size_mismatch_before_fetch() {
    let world_id = "world-authenticated-remote-checkpoint-size-mismatch";
    let dir_a = temp_dir("authenticated-remote-checkpoint-size-mismatch-a");
    let dir_b = temp_dir("authenticated-remote-checkpoint-size-mismatch-b");
    let (_, public_key_a) = deterministic_keypair_hex(232);
    let (_, public_key_b) = deterministic_keypair_hex(233);
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", 232)],
    );
    let replication_config_a = signed_replication_config(dir_a.clone(), 232)
        .with_remote_writer_allowlist(vec![public_key_b.clone()])
        .expect("allowlist a")
        .with_fetch_requester_allowlist(vec![public_key_b])
        .expect("fetch allowlist a");
    let replication_config_b = signed_replication_config(dir_b.clone(), 233)
        .with_remote_writer_allowlist(vec![public_key_a.clone()])
        .expect("allowlist b")
        .with_fetch_requester_allowlist(vec![public_key_a])
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
        .with_require_execution_on_commit(true)
        .with_replication(replication_config_b.clone());
    let bundle = checkpoint_bundle(65, "exec-block-65", "exec-state-65");
    let replication_a =
        ReplicationRuntime::new(config_a.replication.as_ref().expect("repl a"), "node-a")
            .expect("runtime a");
    let descriptor = replication_a
        .store_execution_checkpoint_bundle(&bundle)
        .expect("store remote checkpoint closure");
    let blob = bundle.blobs.first().expect("checkpoint blob");
    let blob_path = dir_b
        .join("store")
        .join("blobs")
        .join(format!("{}.blob", blob.content_hash));
    let replication_b =
        ReplicationRuntime::new(config_b.replication.as_ref().expect("repl b"), "node-b")
            .expect("runtime b");
    replication_b
        .store_blob_by_hash(descriptor.manifest_ref.as_str(), bundle.manifest_json.as_slice())
        .expect("seed valid local checkpoint manifest");
    replication_b
        .store_blob_by_hash(blob.content_hash.as_str(), blob.bytes.as_slice())
        .expect("seed hash-valid local checkpoint blob");
    assert_eq!(fs::read(&blob_path).expect("read local checkpoint blob"), blob.bytes);

    let mut mismatched_descriptor = descriptor.clone();
    mismatched_descriptor
        .blobs
        .first_mut()
        .expect("checkpoint blob ref")
        .size_bytes += 1;
    let fetch_protocols = Arc::new(Mutex::new(Vec::new()));
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(FirstReadyHeadCheckpointNetwork {
        inner: Arc::new(TestInMemoryNetwork::default()),
        fetch_protocols: Arc::clone(&fetch_protocols),
    });
    let dht = Arc::new(TestReplicaMaintenanceDht::new(
        "remote-checkpoint-provider",
        "local-checkpoint-provider",
    ));
    let handle_b = NodeReplicationNetworkHandle::new(Arc::clone(&network))
        .with_dht(dht.clone())
        .with_local_provider_id("local-checkpoint-provider");
    let endpoint_b =
        ReplicationNetworkEndpoint::new(&handle_b, world_id, false, &config_b.network_policy)
            .expect("fresh observer endpoint");
    let mut engine_b = PosNodeEngine::new(&config_b).expect("fresh observer engine");

    let result = engine_b.fetch_execution_checkpoint_bundle(
        &endpoint_b,
        world_id,
        &replication_b,
        &mismatched_descriptor,
        true,
    );
    assert!(
        result.is_err(),
        "descriptor size mismatch must fail closed before provider fetch: {result:?}"
    );
    assert_eq!(
        fs::read(&blob_path).expect("read preserved local checkpoint blob"),
        blob.bytes,
        "hash-valid local CAS bytes/path must be preserved on descriptor mismatch"
    );
    let observed_fetch_protocols = fetch_protocols
        .lock()
        .expect("lock checkpoint fetch protocols")
        .clone();
    assert!(
        observed_fetch_protocols.is_empty(),
        "descriptor size mismatch must not request a provider: {observed_fetch_protocols:?}"
    );
    assert!(
        dht.published_records().is_empty(),
        "descriptor size mismatch must not advertise any provider records: {:?}",
        dht.published_records()
    );

    let _ = fs::remove_dir_all(&dir_a);
    let _ = fs::remove_dir_all(&dir_b);
}
