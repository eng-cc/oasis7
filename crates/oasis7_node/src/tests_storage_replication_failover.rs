use super::*;

#[derive(Clone)]
struct FailoverFetchCommitNetwork {
    inner: TestInMemoryNetwork,
    provider_id: String,
    provider_root: PathBuf,
}

impl FailoverFetchCommitNetwork {
    fn new(provider_id: impl Into<String>, provider_root: PathBuf) -> Self {
        Self {
            inner: TestInMemoryNetwork::default(),
            provider_id: provider_id.into(),
            provider_root,
        }
    }

    fn fetch_blob_from_provider(&self, payload: &[u8]) -> Result<Vec<u8>, WorldError> {
        let request = serde_json::from_slice::<super::replication::FetchBlobRequest>(payload)
            .map_err(|err| WorldError::DistributedValidationFailed {
                reason: format!("decode fetch blob request failed: {err}"),
            })?;
        let blob = super::replication::load_blob_from_root(
            self.provider_root.as_path(),
            request.content_hash.as_str(),
        )
        .map_err(|err| WorldError::DistributedValidationFailed {
            reason: format!("load provider blob failed: {err}"),
        })?;
        let response = super::replication::FetchBlobResponse {
            found: blob.is_some(),
            blob,
        };
        serde_json::to_vec(&response).map_err(|err| WorldError::DistributedValidationFailed {
            reason: format!("encode fetch blob response failed: {err}"),
        })
    }
}

impl oasis7_proto::distributed_net::DistributedNetwork<WorldError> for FailoverFetchCommitNetwork {
    fn publish(&self, topic: &str, payload: &[u8]) -> Result<(), WorldError> {
        self.inner.publish(topic, payload)
    }

    fn subscribe(
        &self,
        topic: &str,
    ) -> Result<oasis7_proto::distributed_net::NetworkSubscription, WorldError> {
        self.inner.subscribe(topic)
    }

    fn request(&self, protocol: &str, payload: &[u8]) -> Result<Vec<u8>, WorldError> {
        if protocol == super::replication::REPLICATION_FETCH_BLOB_PROTOCOL {
            return self.fetch_blob_from_provider(payload);
        }
        self.inner.request(protocol, payload)
    }

    fn connected_peer_ids(&self) -> Vec<String> {
        vec![self.provider_id.clone()]
    }

    fn request_with_providers(
        &self,
        protocol: &str,
        payload: &[u8],
        providers: &[String],
    ) -> Result<Vec<u8>, WorldError> {
        let routed_to_provider = providers
            .iter()
            .any(|provider| provider == &self.provider_id);
        if protocol == super::replication::REPLICATION_FETCH_BLOB_PROTOCOL {
            return self.fetch_blob_from_provider(payload);
        }
        if protocol != super::replication::REPLICATION_FETCH_COMMIT_PROTOCOL || !routed_to_provider
        {
            return self
                .inner
                .request_with_providers(protocol, payload, providers);
        }
        let request = serde_json::from_slice::<super::replication::FetchCommitRequest>(payload)
            .map_err(|err| WorldError::DistributedValidationFailed {
                reason: format!("decode fetch commit request failed: {err}"),
            })?;
        let message = super::replication::load_commit_message_from_root(
            self.provider_root.as_path(),
            request.world_id.as_str(),
            request.height,
        )
        .map_err(|err| WorldError::DistributedValidationFailed {
            reason: format!("load provider commit failed: {err}"),
        })?;
        let response = super::replication::FetchCommitResponse {
            found: message.is_some(),
            message,
        };
        serde_json::to_vec(&response).map_err(|err| WorldError::DistributedValidationFailed {
            reason: format!("encode fetch commit response failed: {err}"),
        })
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
fn runtime_network_replication_accepts_writer_failover_with_epoch_rotation() {
    let dir_a = temp_dir("failover-a");
    let dir_b = temp_dir("failover-b");
    let dir_c = temp_dir("failover-c");
    let validators = vec![
        PosValidator {
            validator_id: "node-a".to_string(),
            stake: 34,
        },
        PosValidator {
            validator_id: "node-b".to_string(),
            stake: 33,
        },
        PosValidator {
            validator_id: "node-c".to_string(),
            stake: 33,
        },
    ];
    let pos_config = signed_pos_config_with_signer_seeds(
        validators,
        &[("node-a", 91), ("node-b", 92), ("node-c", 93)],
    );
    let network_impl = Arc::new(FailoverFetchCommitNetwork::new(
        "observer-provider",
        dir_b.clone(),
    ));
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = network_impl.clone();

    let build_observer = || {
        NodeConfig::new("node-b", "world-failover-repl", NodeRole::Observer)
            .expect("observer config")
            .with_tick_interval(Duration::from_millis(10))
            .expect("observer tick")
            .with_pos_config(pos_config.clone())
            .expect("observer pos config")
            .with_replication(signed_replication_config(dir_b.clone(), 92))
    };
    let build_sequencer_a = || {
        NodeConfig::new("node-a", "world-failover-repl", NodeRole::Sequencer)
            .expect("sequencer a config")
            .with_tick_interval(Duration::from_millis(10))
            .expect("sequencer a tick")
            .with_pos_config(pos_config.clone())
            .expect("sequencer a pos config")
            .with_auto_attest_all_validators(true)
            .with_replication(signed_replication_config(dir_a.clone(), 91))
    };
    let build_sequencer_c = || {
        NodeConfig::new("node-c", "world-failover-repl", NodeRole::Sequencer)
            .expect("sequencer c config")
            .with_tick_interval(Duration::from_millis(10))
            .expect("sequencer c tick")
            .with_pos_config(pos_config.clone())
            .expect("sequencer c pos config")
            .with_auto_attest_all_validators(true)
            .with_replication(signed_replication_config(dir_c.clone(), 93))
    };

    let mut runtime_a = with_noop_execution_hook(NodeRuntime::new(build_sequencer_a()))
        .with_replication_network(NodeReplicationNetworkHandle::new(Arc::clone(&network)));
    let mut runtime_b = NodeRuntime::new(build_observer()).with_replication_network(
        NodeReplicationNetworkHandle::new(Arc::clone(&network))
            .with_local_provider_id("observer-provider"),
    );
    runtime_a.start().expect("start a");
    runtime_b.start().expect("start b with a");
    let guard_path = dir_b.join("replication_remote_guards.json");
    let guard_ready = wait_until(Instant::now() + Duration::from_secs(3), || {
        fs::read(&guard_path)
            .ok()
            .and_then(|bytes| {
                serde_json::from_slice::<BTreeMap<String, SingleWriterReplicationGuard>>(&bytes)
                    .ok()
            })
            .and_then(|guards| guards.values().next().cloned())
            .is_some_and(|guard| guard.last_sequence >= 1 && guard.writer_epoch >= 1)
    });
    runtime_a.stop().expect("stop a");
    runtime_b.stop().expect("stop b after a");
    assert!(
        guard_ready,
        "initial replication guard did not persist before failover snapshot"
    );

    let guard_before = serde_json::from_slice::<BTreeMap<String, SingleWriterReplicationGuard>>(
        &fs::read(&guard_path).expect("read guard before"),
    )
    .expect("parse guard before")
    .values()
    .next()
    .cloned()
    .expect("remote guard before");
    assert!(guard_before.last_sequence >= 1);
    assert!(guard_before.writer_epoch >= 1);
    let writer_before = guard_before.writer_id.clone();
    let height_before = runtime_b.snapshot().consensus.committed_height;

    let mut runtime_c = with_noop_execution_hook(NodeRuntime::new(build_sequencer_c()))
        .with_replication_network(NodeReplicationNetworkHandle::new(Arc::clone(&network)));
    let mut runtime_b = NodeRuntime::new(build_observer()).with_replication_network(
        NodeReplicationNetworkHandle::new(Arc::clone(&network))
            .with_local_provider_id("observer-provider"),
    );
    runtime_b.start().expect("start b with c");
    let observer_ready = wait_until(Instant::now() + Duration::from_secs(2), || {
        let snapshot = runtime_b.snapshot();
        snapshot.consensus.committed_height >= height_before && snapshot.last_error.is_none()
    });
    assert!(
        observer_ready,
        "observer did not restore replicated head before failover: before_height={} snapshot={:?}",
        height_before,
        runtime_b.snapshot()
    );
    runtime_c.start().expect("start c");
    let failover_ready = wait_until(Instant::now() + Duration::from_secs(3), || {
        fs::read(&guard_path)
            .ok()
            .and_then(|bytes| {
                serde_json::from_slice::<BTreeMap<String, SingleWriterReplicationGuard>>(&bytes)
                    .ok()
            })
            .is_some_and(|guards| {
                guards
                    .values()
                    .any(|guard| guard.writer_id != writer_before && guard.last_sequence >= 1)
                    && runtime_b.snapshot().consensus.committed_height > height_before
            })
    });
    let snapshot_c_after_failover = runtime_c.snapshot();
    let snapshot_b_after_failover = runtime_b.snapshot();
    let guards_after_failover = fs::read(&guard_path).ok().and_then(|bytes| {
        serde_json::from_slice::<BTreeMap<String, SingleWriterReplicationGuard>>(&bytes).ok()
    });
    runtime_c.stop().expect("stop c");
    runtime_b.stop().expect("stop b after c");
    assert!(
        failover_ready,
        "failover guard did not persist before second stop: before_height={} c_snapshot={:?} b_snapshot={:?} guards={:?}",
        height_before,
        snapshot_c_after_failover,
        snapshot_b_after_failover,
        guards_after_failover
    );

    let guard_after = serde_json::from_slice::<BTreeMap<String, SingleWriterReplicationGuard>>(
        &fs::read(&guard_path).expect("read guard after"),
    )
    .expect("parse guard after")
    .values()
    .find(|guard| guard.writer_id != writer_before)
    .cloned()
    .expect("failover remote guard after");
    assert!(guard_after.last_sequence >= 1);
    assert_ne!(guard_after.writer_id, writer_before);

    let _ = fs::remove_dir_all(&dir_a);
    let _ = fs::remove_dir_all(&dir_b);
    let _ = fs::remove_dir_all(&dir_c);
}
