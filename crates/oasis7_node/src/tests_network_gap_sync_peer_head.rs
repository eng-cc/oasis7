use super::network_gap_sync_tests::build_fetch_commit_success_cache_fixture;
use super::*;

#[test]
fn gap_sync_fetch_commit_records_peer_head_for_readiness() {
    let dir_remote = temp_dir("gap-sync-peer-head-observe-remote");
    let dir_local = temp_dir("gap-sync-peer-head-observe-local");
    let world_id = "world-gap-sync-peer-head-observe";
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(TestInMemoryNetwork::default());
    let (mut engine, mut replication, endpoint, message) = build_fetch_commit_success_cache_fixture(
        world_id,
        dir_remote.as_path(),
        dir_local.as_path(),
        132,
        133,
        Arc::clone(&network),
    );
    engine.network_committed_height = 1;
    let expected_hash = message.record.content_hash.clone();
    let expected_blob = message.payload.clone();
    network
        .register_handler(
            super::replication::REPLICATION_FETCH_COMMIT_PROTOCOL,
            Box::new({
                let message = message.clone();
                move |_payload| {
                    serde_json::to_vec(&super::replication::FetchCommitResponse {
                        found: true,
                        message: Some(message.clone()),
                    })
                    .map_err(|err| WorldError::DistributedValidationFailed {
                        reason: format!("encode fetch commit response failed: {err}"),
                    })
                }
            }),
        )
        .expect("register fetch commit handler");
    network
        .register_handler(
            super::replication::REPLICATION_FETCH_BLOB_PROTOCOL,
            Box::new(move |payload| {
                let request =
                    serde_json::from_slice::<super::replication::FetchBlobRequest>(payload)
                        .map_err(|err| WorldError::DistributedValidationFailed {
                            reason: format!("decode fetch blob request failed: {err}"),
                        })?;
                serde_json::to_vec(&test_fetch_blob_response(
                    request.content_hash == expected_hash,
                    (request.content_hash == expected_hash).then(|| expected_blob.clone()),
                ))
                .map_err(|err| WorldError::DistributedValidationFailed {
                    reason: format!("encode fetch blob response failed: {err}"),
                })
            }),
        )
        .expect("register fetch blob handler");

    engine
        .sync_missing_replication_commits(&endpoint, "node-b", world_id, Some(&mut replication), None)
        .expect("gap sync missing commits");

    let peer_head = engine.peer_heads.get("node-a").expect("node-a peer head");
    assert_eq!(peer_head.height, 1);
    assert_eq!(peer_head.block_hash, "block-1");
    assert_eq!(peer_head.public_key_hex, None);
    assert_eq!(peer_head.signature_hex, None);
    assert_eq!(engine.committed_height, 1);
    assert_eq!(engine.replication_persisted_height, 1);
    assert_eq!(engine.network_committed_height, 1);

    let _ = fs::remove_dir_all(&dir_remote);
    let _ = fs::remove_dir_all(&dir_local);
}
