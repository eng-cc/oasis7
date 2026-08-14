use super::*;
use super::network_gap_sync_tests::{
    build_fetch_commit_success_cache_fixture, PeerDirectedFetchCommitTestNetwork,
};

#[derive(Clone)]
struct WaitableFetchCommitGapNetwork {
    request_count: Arc<Mutex<usize>>,
}

impl oasis7_proto::distributed_net::DistributedNetwork<WorldError> for WaitableFetchCommitGapNetwork {
    fn publish(&self, _topic: &str, _payload: &[u8]) -> Result<(), WorldError> {
        Ok(())
    }

    fn subscribe(&self, topic: &str) -> Result<NetworkSubscription, WorldError> {
        Ok(NetworkSubscription::new(
            topic.to_string(),
            Arc::new(Mutex::new(HashMap::new())),
        ))
    }

    fn request(&self, protocol: &str, _payload: &[u8]) -> Result<Vec<u8>, WorldError> {
        *self.request_count.lock().expect("lock request count") += 1;
        Err(WorldError::NetworkProtocolUnavailable {
            protocol: format!("libp2p-replication no connected peers for protocol {protocol}"),
        })
    }

    fn register_handler(
        &self,
        _protocol: &str,
        _handler: Box<dyn Fn(&[u8]) -> Result<Vec<u8>, WorldError> + Send + Sync>,
    ) -> Result<(), WorldError> {
        Ok(())
    }
}

#[test]
fn successor_probe_clean_genesis_waitable_gap_keeps_local_proposals_enabled() {
    let dir_remote = temp_dir("successor-probe-clean-genesis-gap-remote");
    let dir_local = temp_dir("successor-probe-clean-genesis-gap-local");
    let world_id = "world-successor-probe-clean-genesis-gap";
    let request_count = Arc::new(Mutex::new(0usize));
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(WaitableFetchCommitGapNetwork {
        request_count: Arc::clone(&request_count),
    });
    let (mut engine, mut replication, endpoint, _) = build_fetch_commit_success_cache_fixture(
        world_id,
        dir_remote.as_path(),
        dir_local.as_path(),
        136,
        137,
        Arc::clone(&network),
    );

    let hold = engine
        .maybe_hold_proposal_for_replication_successor_probe(
            &endpoint,
            "node-b",
            world_id,
            1_000,
            Some(&mut replication),
            None,
        )
        .expect("waitable genesis gap should not be fatal");

    assert!(
        !hold,
        "clean genesis with no peer heads should not wait forever for a possible remote height 1"
    );
    assert_eq!(*request_count.lock().expect("lock request count"), 1);
    assert_eq!(engine.committed_height, 0);
    assert_eq!(engine.replication_persisted_height, 0);

    let _ = fs::remove_dir_all(&dir_remote);
    let _ = fs::remove_dir_all(&dir_local);
}

#[test]
fn successor_probe_unsupported_fetch_commit_keeps_local_proposals_enabled() {
    let dir_remote = temp_dir("successor-probe-unsupported-remote");
    let dir_local = temp_dir("successor-probe-unsupported-local");
    let world_id = "world-successor-probe-unsupported";
    let generic_attempts = Arc::new(Mutex::new(0usize));
    let provider_attempts = Arc::new(Mutex::new(Vec::<Vec<String>>::new()));
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(PeerDirectedFetchCommitTestNetwork {
        generic_response: super::replication::FetchCommitResponse {
            found: false,
            message: None,
        lineage_envelope: None,
        },
        generic_unsupported: true,
        peer_responses: Arc::new(Mutex::new(HashMap::new())),
        connected_peer_ids: Vec::new(),
        generic_attempts: Arc::clone(&generic_attempts),
        provider_attempts: Arc::clone(&provider_attempts),
    });
    let (mut engine, mut replication, endpoint, _) = build_fetch_commit_success_cache_fixture(
        world_id,
        dir_remote.as_path(),
        dir_local.as_path(),
        132,
        133,
        Arc::clone(&network),
    );

    let hold = engine
        .maybe_hold_proposal_for_replication_successor_probe(
            &endpoint,
            "node-b",
            world_id,
            1_000,
            Some(&mut replication),
            None,
        )
        .expect("unsupported fetch commit should not be fatal for successor probe");

    assert!(
        !hold,
        "unsupported fetch-commit probe should leave local proposal path enabled"
    );
    assert_eq!(*generic_attempts.lock().expect("lock generic attempts"), 1);
    assert!(
        provider_attempts
            .lock()
            .expect("lock provider attempts")
            .is_empty(),
        "no connected peers should skip provider-directed attempts"
    );

    let _ = fs::remove_dir_all(&dir_remote);
    let _ = fs::remove_dir_all(&dir_local);
}

#[test]
fn successor_probe_found_commit_missing_blob_does_not_enable_local_proposals() {
    let dir_remote = temp_dir("successor-probe-missing-blob-remote");
    let dir_local = temp_dir("successor-probe-missing-blob-local");
    let world_id = "world-successor-probe-missing-blob";
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(TestInMemoryNetwork::default());
    let (mut engine, mut replication, endpoint, message) = build_fetch_commit_success_cache_fixture(
        world_id,
        dir_remote.as_path(),
        dir_local.as_path(),
        134,
        135,
        Arc::clone(&network),
    );
    network
        .register_handler(
            super::replication::REPLICATION_FETCH_COMMIT_PROTOCOL,
            Box::new({
                let message = message.clone();
                move |_payload| {
                    serde_json::to_vec(&super::replication::FetchCommitResponse {
                        found: true,
                        message: Some(message.clone()),
                        lineage_envelope: None,
                    })
                    .map_err(|err| WorldError::DistributedValidationFailed {
                        reason: format!("encode fetch commit response failed: {err}"),
                    })
                }
            }),
        )
        .expect("register fetch commit handler");

    let err = engine
        .maybe_hold_proposal_for_replication_successor_probe(
            &endpoint,
            "node-b",
            world_id,
            1_000,
            Some(&mut replication),
            None,
        )
        .expect_err("missing blob after found commit must stay fatal for successor probe");
    let reason = err.to_string();
    assert!(
        reason.contains(super::replication::REPLICATION_FETCH_BLOB_PROTOCOL),
        "expected blob fetch route error, got {reason}"
    );

    let _ = fs::remove_dir_all(&dir_remote);
    let _ = fs::remove_dir_all(&dir_local);
}
