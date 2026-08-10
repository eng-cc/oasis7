fn assert_stale_dht_checkpoint_provenance(
    replication: &ReplicationRuntime,
    world_id: &str,
    checkpoint_height: u64,
    checkpoint_block_hash: &str,
    checkpoint_state_root: &str,
    replication_dir: &std::path::Path,
) {
    let persisted_checkpoint = replication
        .load_commit_message_by_height(world_id, checkpoint_height)
        .expect("load persisted checkpoint")
        .expect("checkpoint persists after bootstrap");
    let persisted_checkpoint =
        super::replication_state_reconcile::parse_replication_commit_payload(
            persisted_checkpoint.payload.as_slice(),
        )
        .expect("decode persisted checkpoint payload");
    assert_eq!(
        persisted_checkpoint.execution_block_hash.as_deref(),
        Some(checkpoint_block_hash)
    );
    assert_eq!(
        persisted_checkpoint.execution_state_root.as_deref(),
        Some(checkpoint_state_root)
    );

    let receipt_path = replication_dir
        .join("checkpoint-verification")
        .join(format!("{checkpoint_height}.json"));
    let receipt: serde_json::Value = serde_json::from_slice(
        fs::read(&receipt_path)
            .expect("read checkpoint verification receipt")
            .as_slice(),
    )
    .expect("decode checkpoint verification receipt");
    assert!(
        receipt["fetch_observations"]
            .as_array()
            .expect("receipt observations")
            .iter()
            .all(|observation| {
                observation["source"].as_str() == Some("network_fetch")
                    && observation["signed_request"].as_bool() == Some(true)
                    && observation["connected_peer_ids"]
                        .as_array()
                        .is_some_and(|candidates| candidates
                            .iter()
                            .any(|candidate| candidate.as_str() == Some("node-a")))
            }),
        "checkpoint receipt must retain signed connected-peer provenance: {receipt}"
    );
}

fn peer_head_checkpoint_before_height_one(
    initial_peer_head: InitialPeerHead,
    initial_checkpoint_fetch_available: bool,
) {
    peer_head_checkpoint_before_height_one_with_stale_dht(
        initial_peer_head,
        initial_checkpoint_fetch_available,
        false,
        false,
    );
}

#[test]
fn fresh_observer_fetches_peer_head_checkpoint_before_first_height_one_execution() {
    peer_head_checkpoint_before_height_one(InitialPeerHead::HighCheckpoint, true);
}

#[test]
fn fresh_observer_defers_height_one_until_transient_peer_head_preflight_can_bootstrap() {
    peer_head_checkpoint_before_height_one(InitialPeerHead::Unavailable, true);
}

#[test]
fn fresh_observer_defers_height_one_until_advertised_checkpoint_fetch_recovers() {
    peer_head_checkpoint_before_height_one(InitialPeerHead::HighCheckpoint, false);
}

#[test]
fn fresh_observer_defers_height_one_until_stale_peer_head_advances_to_high_checkpoint() {
    peer_head_checkpoint_before_height_one(InitialPeerHead::StaleHeightOne, true);
}

#[test]
fn fresh_observer_prefers_connected_high_checkpoint_over_stale_dht_height_one() {
    peer_head_checkpoint_before_height_one_with_stale_dht(
        InitialPeerHead::HighCheckpoint,
        true,
        true,
        false,
    );
}

#[test]
fn fresh_observer_defers_height_one_when_signed_checkpoint_is_temporarily_not_found() {
    peer_head_checkpoint_before_height_one_with_stale_dht(
        InitialPeerHead::HighCheckpoint,
        false,
        false,
        true,
    );
}

#[test]
fn fresh_observer_with_observed_high_head_defers_height_one_without_checkpoint_closure() {
    peer_head_checkpoint_before_height_one_with_stale_dht(
        InitialPeerHead::UnavailableWithObservedHigh,
        false,
        false,
        true,
    );
}
