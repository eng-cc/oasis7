use super::*;

#[derive(Clone)]
struct PeerDirectedHeadTestNetwork {
    inner: Arc<TestInMemoryNetwork>,
    connected_peer_ids: Vec<String>,
    known_peer_ids: Vec<String>,
    configured_static_bootstrap_peer_ids: Vec<String>,
    peer_heads: Arc<Mutex<HashMap<String, super::replication::FetchHeadResponse>>>,
    head_request_errors: Arc<Mutex<HashMap<String, WorldError>>>,
    unsupported_head_peers: Vec<String>,
    waitable_fetch_commit_heights: Arc<Mutex<Vec<u64>>>,
    provider_attempts: Arc<Mutex<Vec<Vec<String>>>>,
    provider_budgets: Arc<Mutex<Vec<(u64, u64)>>>,
}

impl oasis7_proto::distributed_net::DistributedNetwork<WorldError> for PeerDirectedHeadTestNetwork {
    fn publish(&self, topic: &str, payload: &[u8]) -> Result<(), WorldError> {
        self.inner.publish(topic, payload)
    }

    fn subscribe(&self, topic: &str) -> Result<NetworkSubscription, WorldError> {
        self.inner.subscribe(topic)
    }

    fn request(&self, protocol: &str, payload: &[u8]) -> Result<Vec<u8>, WorldError> {
        if self.fetch_commit_height_is_waitable(protocol, payload)? {
            return Err(waitable_fetch_commit_error(protocol));
        }
        self.inner.request(protocol, payload)
    }

    fn connected_peer_ids(&self) -> Vec<String> {
        self.connected_peer_ids.clone()
    }

    fn known_peer_ids(&self) -> Vec<String> {
        self.known_peer_ids.clone()
    }

    fn configured_static_bootstrap_peer_ids(&self) -> Vec<String> {
        self.configured_static_bootstrap_peer_ids.clone()
    }

    fn request_with_providers_budget(
        &self,
        protocol: &str,
        payload: &[u8],
        providers: &[String],
        request_timeout_ms: u64,
        retry_budget_ms: u64,
    ) -> Result<Vec<u8>, WorldError> {
        self.provider_budgets
            .lock()
            .expect("lock provider budgets")
            .push((request_timeout_ms, retry_budget_ms));
        self.request_with_providers(protocol, payload, providers)
    }

    fn request_with_providers(
        &self,
        protocol: &str,
        payload: &[u8],
        providers: &[String],
    ) -> Result<Vec<u8>, WorldError> {
        if self.fetch_commit_height_is_waitable(protocol, payload)? {
            return Err(waitable_fetch_commit_error(protocol));
        }
        if protocol != REPLICATION_GET_HEAD_PROTOCOL {
            return self.inner.request_with_providers(protocol, payload, providers);
        }
        self.provider_attempts
            .lock()
            .expect("lock provider attempts")
            .push(providers.to_vec());
        let Some(provider) = providers.first() else {
            return Err(WorldError::NetworkProtocolUnavailable {
                protocol: protocol.to_string(),
            });
        };
        if self
            .unsupported_head_peers
            .iter()
            .any(|unsupported| unsupported == provider)
        {
            return Err(WorldError::NetworkRequestFailed {
                code: DistributedErrorCode::ErrUnsupported,
                message: protocol.to_string(),
                retryable: false,
            });
        }
        if let Some(err) = self
            .head_request_errors
            .lock()
            .expect("lock head request errors")
            .get(provider)
            .cloned()
        {
            return Err(err);
        }
        let response = self
            .peer_heads
            .lock()
            .expect("lock peer heads")
            .get(provider)
            .cloned()
            .unwrap_or(super::replication::FetchHeadResponse {
                found: false,
                head: None,
            });
        serde_json::to_vec(&response).map_err(|err| WorldError::DistributedValidationFailed {
            reason: format!("encode peer-directed head response failed: {err}"),
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

impl PeerDirectedHeadTestNetwork {
    fn fetch_commit_height_is_waitable(
        &self,
        protocol: &str,
        payload: &[u8],
    ) -> Result<bool, WorldError> {
        if protocol != super::replication::REPLICATION_FETCH_COMMIT_PROTOCOL {
            return Ok(false);
        }
        let request = serde_json::from_slice::<super::replication::FetchCommitRequest>(payload)
            .map_err(|err| WorldError::DistributedValidationFailed {
                reason: format!("decode fetch commit request failed: {err}"),
            })?;
        Ok(self
            .waitable_fetch_commit_heights
            .lock()
            .expect("lock waitable fetch heights")
            .contains(&request.height))
    }
}

fn waitable_fetch_commit_error(protocol: &str) -> WorldError {
    WorldError::NetworkProtocolUnavailable {
        protocol: format!("libp2p-replication no connected peers for protocol {protocol}"),
    }
}

fn peer_directed_head_endpoint(
    world_id: &str,
    connected_peer_ids: Vec<String>,
    known_peer_ids: Vec<String>,
    configured_static_bootstrap_peer_ids: Vec<String>,
    peer_heads: Arc<Mutex<HashMap<String, super::replication::FetchHeadResponse>>>,
    head_request_errors: Arc<Mutex<HashMap<String, WorldError>>>,
) -> (
    ReplicationNetworkEndpoint,
    Arc<Mutex<Vec<Vec<String>>>>,
    Arc<Mutex<Vec<(u64, u64)>>>,
) {
    let provider_attempts = Arc::new(Mutex::new(Vec::<Vec<String>>::new()));
    let provider_budgets = Arc::new(Mutex::new(Vec::<(u64, u64)>::new()));
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(PeerDirectedHeadTestNetwork {
        inner: Arc::new(TestInMemoryNetwork::default()),
        connected_peer_ids,
        known_peer_ids,
        configured_static_bootstrap_peer_ids,
        peer_heads,
        head_request_errors,
        unsupported_head_peers: Vec::new(),
        waitable_fetch_commit_heights: Arc::new(Mutex::new(Vec::new())),
        provider_attempts: Arc::clone(&provider_attempts),
        provider_budgets: Arc::clone(&provider_budgets),
    });
    let handle = NodeReplicationNetworkHandle::new(network);
    let endpoint = ReplicationNetworkEndpoint::new(
        &handle,
        world_id,
        false,
        &NodeNetworkPolicy::for_runtime_role(NodeRole::Observer),
    )
    .expect("peer-directed head endpoint");
    (endpoint, provider_attempts, provider_budgets)
}

#[test]
fn peer_world_head_probe_continues_after_connected_peer_timeout() {
    let world_id = "world-peer-head-timeout-fallback";
    let peer_heads = Arc::new(Mutex::new(HashMap::new()));
    peer_heads.lock().expect("lock peer heads").insert(
        "peer-b".to_string(),
        super::replication::FetchHeadResponse {
            found: true,
            head: Some(super::replication::ReplicationHeadSummary {
                world_id: world_id.to_string(),
                height: 1133,
                block_hash: "block-1133".to_string(),
                state_root: "state-1133".to_string(),
                timestamp_ms: 4_133,
            }),
        },
    );
    let head_request_errors = Arc::new(Mutex::new(HashMap::new()));
    head_request_errors.lock().expect("lock head errors").insert(
        "peer-a".to_string(),
        WorldError::NetworkRequestFailed {
            code: DistributedErrorCode::ErrTimeout,
            message: REPLICATION_GET_HEAD_PROTOCOL.to_string(),
            retryable: true,
        },
    );
    let (endpoint, provider_attempts, _) = peer_directed_head_endpoint(
        world_id,
        vec!["peer-a".to_string(), "peer-b".to_string()],
        vec!["peer-a".to_string(), "peer-b".to_string()],
        Vec::new(),
        peer_heads,
        head_request_errors,
    );

    let head = endpoint
        .lookup_world_head(world_id)
        .expect("timeout should fall back to later peer")
        .expect("later peer head");

    assert_eq!(head.height, 1133);
    assert_eq!(head.block_hash, "block-1133");
    assert_eq!(
        provider_attempts
            .lock()
            .expect("lock provider attempts")
            .as_slice(),
        &[vec!["peer-a".to_string()], vec!["peer-b".to_string()]]
    );
}

#[test]
fn peer_world_head_probe_uses_candidate_static_peer_after_active_peer_timeout() {
    let world_id = "world-peer-head-static-candidate-fallback";
    let peer_heads = Arc::new(Mutex::new(HashMap::from([(
        "static-204".to_string(),
        super::replication::FetchHeadResponse {
            found: true,
            head: Some(super::replication::ReplicationHeadSummary {
                world_id: world_id.to_string(),
                height: 204,
                block_hash: "block-204".to_string(),
                state_root: "state-204".to_string(),
                timestamp_ms: 4_204,
            }),
        },
    )])));
    let head_request_errors = Arc::new(Mutex::new(HashMap::from([(
        "static-205".to_string(),
        WorldError::NetworkRequestFailed {
            code: DistributedErrorCode::ErrTimeout,
            message: REPLICATION_GET_HEAD_PROTOCOL.to_string(),
            retryable: true,
        },
    )])));
    let (endpoint, provider_attempts, _) = peer_directed_head_endpoint(
        world_id,
        vec!["static-205".to_string()],
        vec!["static-205".to_string(), "static-204".to_string()],
        vec!["static-205".to_string(), "static-204".to_string()],
        peer_heads,
        head_request_errors,
    );

    let head = endpoint
        .lookup_world_head(world_id)
        .expect("active timeout should fall through to candidate static peer")
        .expect("candidate static peer head");

    assert_eq!(head.height, 204);
    assert_eq!(
        provider_attempts
            .lock()
            .expect("lock provider attempts")
            .as_slice(),
        &[vec!["static-205".to_string()], vec!["static-204".to_string()]]
    );
}

#[test]
fn peer_world_head_probe_prioritizes_static_bootstrap_before_earlier_dht_candidate() {
    let world_id = "world-peer-head-static-bootstrap-priority";
    let peer_heads = Arc::new(Mutex::new(HashMap::from([(
        "static-204".to_string(),
        super::replication::FetchHeadResponse {
            found: true,
            head: Some(super::replication::ReplicationHeadSummary {
                world_id: world_id.to_string(),
                height: 204,
                block_hash: "block-204".to_string(),
                state_root: "state-204".to_string(),
                timestamp_ms: 4_204,
            }),
        },
    )])));
    let head_request_errors = Arc::new(Mutex::new(HashMap::from([(
        "static-205".to_string(),
        WorldError::NetworkRequestFailed {
            code: DistributedErrorCode::ErrTimeout,
            message: REPLICATION_GET_HEAD_PROTOCOL.to_string(),
            retryable: true,
        },
    )])));
    let (endpoint, provider_attempts, provider_budgets) = peer_directed_head_endpoint(
        world_id,
        vec!["static-205".to_string()],
        vec![
            "dht-001".to_string(),
            "static-204".to_string(),
            "static-205".to_string(),
        ],
        vec!["static-205".to_string(), "static-204".to_string()],
        peer_heads,
        head_request_errors,
    );

    let head = endpoint
        .lookup_world_head(world_id)
        .expect("active timeout should fall through to static bootstrap provider")
        .expect("static bootstrap provider head");

    assert_eq!(head.height, 204);
    assert_eq!(
        provider_attempts
            .lock()
            .expect("lock provider attempts")
            .as_slice(),
        &[vec!["static-205".to_string()], vec!["static-204".to_string()]]
    );
    assert_eq!(
        provider_budgets.lock().expect("lock provider budgets").as_slice(),
        &[(1_500, 3_000), (1_500, 3_000)]
    );
}

#[test]
fn peer_world_head_probe_continues_after_protocol_omitted_libp2p_timeout() {
    let world_id = "world-peer-head-libp2p-timeout-fallback";
    let peer_heads = Arc::new(Mutex::new(HashMap::new()));
    peer_heads.lock().expect("lock peer heads").insert(
        "peer-b".to_string(),
        super::replication::FetchHeadResponse {
            found: true,
            head: Some(super::replication::ReplicationHeadSummary {
                world_id: world_id.to_string(),
                height: 1133,
                block_hash: "block-1133".to_string(),
                state_root: "state-1133".to_string(),
                timestamp_ms: 4_133,
            }),
        },
    );
    let head_request_errors = Arc::new(Mutex::new(HashMap::new()));
    head_request_errors.lock().expect("lock head errors").insert(
        "peer-a".to_string(),
        WorldError::NetworkProtocolUnavailable {
            protocol: "libp2p-replication outbound request failed: request failed: Timeout"
                .to_string(),
        },
    );
    let (endpoint, provider_attempts, _) = peer_directed_head_endpoint(
        world_id,
        vec!["peer-a".to_string(), "peer-b".to_string()],
        vec!["peer-a".to_string(), "peer-b".to_string()],
        Vec::new(),
        peer_heads,
        head_request_errors,
    );

    let head = endpoint
        .lookup_world_head(world_id)
        .expect("protocol-omitted libp2p timeout should fall back to later peer")
        .expect("later peer head");

    assert_eq!(head.height, 1133);
    assert_eq!(
        provider_attempts
            .lock()
            .expect("lock provider attempts")
            .as_slice(),
        &[vec!["peer-a".to_string()], vec!["peer-b".to_string()]]
    );
}

#[test]
fn peer_world_head_probe_rejects_mismatched_world_without_fallback() {
    let world_id = "world-peer-head-mismatch-fail-closed";
    let peer_heads = Arc::new(Mutex::new(HashMap::new()));
    peer_heads.lock().expect("lock peer heads").insert(
        "peer-a".to_string(),
        super::replication::FetchHeadResponse {
            found: true,
            head: Some(super::replication::ReplicationHeadSummary {
                world_id: "other-world".to_string(),
                height: 1132,
                block_hash: "other-block".to_string(),
                state_root: "other-state".to_string(),
                timestamp_ms: 4_132,
            }),
        },
    );
    peer_heads.lock().expect("lock peer heads").insert(
        "peer-b".to_string(),
        super::replication::FetchHeadResponse {
            found: true,
            head: Some(super::replication::ReplicationHeadSummary {
                world_id: world_id.to_string(),
                height: 1133,
                block_hash: "block-1133".to_string(),
                state_root: "state-1133".to_string(),
                timestamp_ms: 4_133,
            }),
        },
    );
    let (endpoint, provider_attempts, _) = peer_directed_head_endpoint(
        world_id,
        vec!["peer-a".to_string(), "peer-b".to_string()],
        vec!["peer-a".to_string(), "peer-b".to_string()],
        Vec::new(),
        peer_heads,
        Arc::new(Mutex::new(HashMap::new())),
    );

    let err = endpoint
        .lookup_world_head(world_id)
        .expect_err("mismatched peer head must fail closed");

    assert!(
        matches!(
            err,
            NodeError::Replication { ref reason }
                if reason.contains("replication peer head mismatch")
                    && reason.contains("expected=world-peer-head-mismatch-fail-closed")
                    && reason.contains("actual=other-world")
        ),
        "unexpected error: {err:?}"
    );
    assert_eq!(
        provider_attempts
            .lock()
            .expect("lock provider attempts")
            .as_slice(),
        &[vec!["peer-a".to_string()]]
    );
}

#[test]
fn observer_gap_sync_continues_peer_world_head_probe_after_peer_without_head() {
    let world_id = "world-gap-sync-peer-head-retry";
    let dir_a = temp_dir("gap-sync-peer-head-retry-a");
    let dir_b = temp_dir("gap-sync-peer-head-retry-b");
    let (_, public_key_a) = deterministic_keypair_hex(218);
    let (_, public_key_b) = deterministic_keypair_hex(219);
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", 218)],
    );
    let replication_config_a = signed_replication_config(dir_a.clone(), 218)
        .with_remote_writer_allowlist(vec![public_key_b.clone()])
        .expect("allowlist a")
        .with_fetch_requester_allowlist(vec![public_key_b])
        .expect("fetch allowlist a");
    let replication_config_b = signed_replication_config(dir_b.clone(), 219)
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

    let mut replication_a =
        ReplicationRuntime::new(config_a.replication.as_ref().expect("repl a"), "node-a")
            .expect("runtime a");
    for height in 1..=3 {
        let decision = PosDecision {
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
        };
        replication_a
            .build_local_commit_message(
                "node-a",
                world_id,
                2_900 + i64::try_from(height).expect("height fits i64"),
                &decision,
                Some(format!("exec-block-{height}").as_str()),
                Some(format!("exec-state-{height}").as_str()),
            )
            .expect("build local message")
            .expect("message");
    }
    std::fs::remove_file(dir_a.join("replication_commit_messages/00000000000000000001.json"))
        .expect("remove low commit 1");
    std::fs::remove_file(dir_a.join("replication_commit_messages/00000000000000000002.json"))
        .expect("remove low commit 2");

    let peer_heads = Arc::new(Mutex::new(HashMap::new()));
    peer_heads.lock().expect("lock peer heads").insert(
        "peer-a".to_string(),
        super::replication::FetchHeadResponse {
            found: false,
            head: None,
        },
    );
    peer_heads.lock().expect("lock peer heads").insert(
        "peer-b".to_string(),
        super::replication::FetchHeadResponse {
            found: true,
            head: Some(super::replication::ReplicationHeadSummary {
                world_id: world_id.to_string(),
                height: 3,
                block_hash: "block-3".to_string(),
                state_root: "exec-state-3".to_string(),
                timestamp_ms: 2_903,
            }),
        },
    );
    let provider_attempts = Arc::new(Mutex::new(Vec::<Vec<String>>::new()));
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(PeerDirectedHeadTestNetwork {
        inner: Arc::new(TestInMemoryNetwork::default()),
        connected_peer_ids: vec!["peer-a".to_string(), "peer-b".to_string()],
        known_peer_ids: vec!["peer-a".to_string(), "peer-b".to_string()],
        configured_static_bootstrap_peer_ids: Vec::new(),
        peer_heads: Arc::clone(&peer_heads),
        head_request_errors: Arc::new(Mutex::new(HashMap::new())),
        unsupported_head_peers: vec!["peer-a".to_string()],
        waitable_fetch_commit_heights: Arc::new(Mutex::new(Vec::new())),
        provider_attempts: Arc::clone(&provider_attempts),
        provider_budgets: Arc::new(Mutex::new(Vec::new())),
    });
    register_replication_fetch_handlers(
        &NodeReplicationNetworkHandle::new(Arc::clone(&network)),
        &replication_config_a,
        world_id,
        &config_a.network_policy,
    )
    .expect("register fetch handlers");

    let handle_b = NodeReplicationNetworkHandle::new(Arc::clone(&network));
    let endpoint_b =
        ReplicationNetworkEndpoint::new(&handle_b, world_id, false, &config_b.network_policy)
            .expect("endpoint b");
    let mut replication_b =
        ReplicationRuntime::new(config_b.replication.as_ref().expect("repl b"), "node-b")
            .expect("runtime b");
    let mut engine_b = PosNodeEngine::new(&config_b).expect("engine b");

    engine_b
        .sync_missing_replication_commits(
            &endpoint_b,
            "node-b",
            world_id,
            Some(&mut replication_b),
            None,
        )
        .expect("peer-head retry high checkpoint sync");

    assert_eq!(engine_b.network_committed_height, 3);
    assert_eq!(engine_b.committed_height, 3);
    assert_eq!(
        provider_attempts
            .lock()
            .expect("lock provider attempts")
            .as_slice(),
        &[vec!["peer-a".to_string()], vec!["peer-b".to_string()]]
    );

    let _ = fs::remove_dir_all(&dir_a);
    let _ = fs::remove_dir_all(&dir_b);
}

#[test]
fn peer_world_head_fallback_retains_high_network_height_after_partial_sync() {
    let world_id = "world-gap-sync-peer-head-retains-high";
    let dir_a = temp_dir("gap-sync-peer-head-retains-high-a");
    let dir_b = temp_dir("gap-sync-peer-head-retains-high-b");
    let (_, public_key_a) = deterministic_keypair_hex(220);
    let (_, public_key_b) = deterministic_keypair_hex(221);
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", 220)],
    );
    let replication_config_a = signed_replication_config(dir_a.clone(), 220)
        .with_remote_writer_allowlist(vec![public_key_b.clone()])
        .expect("allowlist a")
        .with_fetch_requester_allowlist(vec![public_key_b])
        .expect("fetch allowlist a");
    let replication_config_b = signed_replication_config(dir_b.clone(), 221)
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

    let mut replication_a =
        ReplicationRuntime::new(config_a.replication.as_ref().expect("repl a"), "node-a")
            .expect("runtime a");
    let decision = PosDecision {
        height: 1,
        slot: 1,
        epoch: 0,
        proposer_id: "node-a".to_string(),
        status: PosConsensusStatus::Committed,
        block_hash: "block-1".to_string(),
        action_root: empty_action_root(),
        committed_actions: Vec::new(),
        approved_stake: 100,
        rejected_stake: 0,
        required_stake: 67,
        total_stake: 100,
    };
    replication_a
        .build_local_commit_message(
            "node-a",
            world_id,
            3_100,
            &decision,
            Some("exec-block-1"),
            Some("exec-state-1"),
        )
        .expect("build local message")
        .expect("message");

    let high_height = REPLICATION_GAP_SYNC_MAX_HEIGHTS_PER_POLL + 3;
    let peer_heads = Arc::new(Mutex::new(HashMap::new()));
    peer_heads.lock().expect("lock peer heads").insert(
        "peer-a".to_string(),
        super::replication::FetchHeadResponse {
            found: true,
            head: Some(super::replication::ReplicationHeadSummary {
                world_id: world_id.to_string(),
                height: high_height,
                block_hash: format!("block-{high_height}"),
                state_root: format!("exec-state-{high_height}"),
                timestamp_ms: 3_100 + i64::try_from(high_height).expect("height fits i64"),
            }),
        },
    );
    let provider_attempts = Arc::new(Mutex::new(Vec::<Vec<String>>::new()));
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(PeerDirectedHeadTestNetwork {
        inner: Arc::new(TestInMemoryNetwork::default()),
        connected_peer_ids: vec!["peer-a".to_string()],
        known_peer_ids: vec!["peer-a".to_string()],
        configured_static_bootstrap_peer_ids: Vec::new(),
        peer_heads,
        head_request_errors: Arc::new(Mutex::new(HashMap::new())),
        unsupported_head_peers: Vec::new(),
        waitable_fetch_commit_heights: Arc::new(Mutex::new(Vec::new())),
        provider_attempts,
        provider_budgets: Arc::new(Mutex::new(Vec::new())),
    });
    register_replication_fetch_handlers(
        &NodeReplicationNetworkHandle::new(Arc::clone(&network)),
        &replication_config_a,
        world_id,
        &config_a.network_policy,
    )
    .expect("register fetch handlers");

    let handle_b = NodeReplicationNetworkHandle::new(Arc::clone(&network));
    let endpoint_b =
        ReplicationNetworkEndpoint::new(&handle_b, world_id, false, &config_b.network_policy)
            .expect("endpoint b");
    let mut replication_b =
        ReplicationRuntime::new(config_b.replication.as_ref().expect("repl b"), "node-b")
            .expect("runtime b");
    let mut engine_b = PosNodeEngine::new(&config_b).expect("engine b");

    engine_b
        .sync_missing_replication_commits(
            &endpoint_b,
            "node-b",
            world_id,
            Some(&mut replication_b),
            None,
        )
        .expect("partial peer-head sync");

    assert_eq!(engine_b.committed_height, 1);
    assert_eq!(engine_b.replication_persisted_height, 1);
    assert_eq!(engine_b.network_committed_height, high_height);
    assert_eq!(engine_b.last_replication_gap_sync_blocked_height, Some(2));

    let _ = fs::remove_dir_all(&dir_a);
    let _ = fs::remove_dir_all(&dir_b);
}

#[test]
fn peer_world_head_fallback_retains_high_network_height_after_waitable_gap() {
    let world_id = "world-gap-sync-peer-head-retains-high-wait";
    let dir_a = temp_dir("gap-sync-peer-head-retains-high-wait-a");
    let dir_b = temp_dir("gap-sync-peer-head-retains-high-wait-b");
    let (_, public_key_a) = deterministic_keypair_hex(222);
    let (_, public_key_b) = deterministic_keypair_hex(223);
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", 222)],
    );
    let replication_config_a = signed_replication_config(dir_a.clone(), 222)
        .with_remote_writer_allowlist(vec![public_key_b.clone()])
        .expect("allowlist a")
        .with_fetch_requester_allowlist(vec![public_key_b])
        .expect("fetch allowlist a");
    let replication_config_b = signed_replication_config(dir_b.clone(), 223)
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

    let mut replication_a =
        ReplicationRuntime::new(config_a.replication.as_ref().expect("repl a"), "node-a")
            .expect("runtime a");
    let decision = PosDecision {
        height: 1,
        slot: 1,
        epoch: 0,
        proposer_id: "node-a".to_string(),
        status: PosConsensusStatus::Committed,
        block_hash: "block-1".to_string(),
        action_root: empty_action_root(),
        committed_actions: Vec::new(),
        approved_stake: 100,
        rejected_stake: 0,
        required_stake: 67,
        total_stake: 100,
    };
    replication_a
        .build_local_commit_message(
            "node-a",
            world_id,
            3_200,
            &decision,
            Some("exec-block-1"),
            Some("exec-state-1"),
        )
        .expect("build local message")
        .expect("message");

    let high_height = REPLICATION_GAP_SYNC_MAX_HEIGHTS_PER_POLL + 4;
    let peer_heads = Arc::new(Mutex::new(HashMap::new()));
    peer_heads.lock().expect("lock peer heads").insert(
        "peer-a".to_string(),
        super::replication::FetchHeadResponse {
            found: true,
            head: Some(super::replication::ReplicationHeadSummary {
                world_id: world_id.to_string(),
                height: high_height,
                block_hash: format!("block-{high_height}"),
                state_root: format!("exec-state-{high_height}"),
                timestamp_ms: 3_200 + i64::try_from(high_height).expect("height fits i64"),
            }),
        },
    );
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(PeerDirectedHeadTestNetwork {
        inner: Arc::new(TestInMemoryNetwork::default()),
        connected_peer_ids: vec!["peer-a".to_string()],
        known_peer_ids: vec!["peer-a".to_string()],
        configured_static_bootstrap_peer_ids: Vec::new(),
        peer_heads,
        head_request_errors: Arc::new(Mutex::new(HashMap::new())),
        unsupported_head_peers: Vec::new(),
        waitable_fetch_commit_heights: Arc::new(Mutex::new(vec![2])),
        provider_attempts: Arc::new(Mutex::new(Vec::new())),
        provider_budgets: Arc::new(Mutex::new(Vec::new())),
    });
    register_replication_fetch_handlers(
        &NodeReplicationNetworkHandle::new(Arc::clone(&network)),
        &replication_config_a,
        world_id,
        &config_a.network_policy,
    )
    .expect("register fetch handlers");

    let handle_b = NodeReplicationNetworkHandle::new(Arc::clone(&network));
    let endpoint_b =
        ReplicationNetworkEndpoint::new(&handle_b, world_id, false, &config_b.network_policy)
            .expect("endpoint b");
    let mut replication_b =
        ReplicationRuntime::new(config_b.replication.as_ref().expect("repl b"), "node-b")
            .expect("runtime b");
    let mut engine_b = PosNodeEngine::new(&config_b).expect("engine b");

    engine_b
        .sync_missing_replication_commits(
            &endpoint_b,
            "node-b",
            world_id,
            Some(&mut replication_b),
            None,
        )
        .expect("partial peer-head sync with waitable gap");

    assert_eq!(engine_b.committed_height, 1);
    assert_eq!(engine_b.replication_persisted_height, 1);
    assert_eq!(engine_b.network_committed_height, high_height);

    let _ = fs::remove_dir_all(&dir_a);
    let _ = fs::remove_dir_all(&dir_b);
}
