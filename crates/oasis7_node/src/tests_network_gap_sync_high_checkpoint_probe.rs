use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, Mutex};

use oasis7_proto::world_error::WorldError;

use super::*;
use super::network_gap_sync_tests::{
    build_fetch_commit_success_cache_fixture, PeerDirectedFetchCommitTestNetwork,
};

#[test]
fn high_checkpoint_probe_uses_single_fetch_commit_probe_for_not_found_candidate() {
    let dir_remote = temp_dir("high-checkpoint-single-probe-remote");
    let dir_local = temp_dir("high-checkpoint-single-probe-local");
    let world_id = "world-high-checkpoint-single-probe";
    let generic_attempts = Arc::new(Mutex::new(0usize));
    let provider_attempts = Arc::new(Mutex::new(Vec::<Vec<String>>::new()));
    let peer_responses = Arc::new(Mutex::new(HashMap::new()));
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(PeerDirectedFetchCommitTestNetwork {
        generic_response: super::replication::FetchCommitResponse {
            found: false,
            message: None,
        },
        generic_unsupported: false,
        peer_responses,
        connected_peer_ids: Vec::new(),
        generic_attempts: Arc::clone(&generic_attempts),
        provider_attempts,
    });
    let (mut engine, mut replication, endpoint, _) = build_fetch_commit_success_cache_fixture(
        world_id,
        dir_remote.as_path(),
        dir_local.as_path(),
        146,
        147,
        network,
    );
    let mut execution_hook = None;
    let mut progress_callback = None;

    let synced = engine
        .try_sync_high_replication_checkpoint_boundary(
            &endpoint,
            "node-b",
            world_id,
            &mut replication,
            16_640,
            64,
            None,
            &mut execution_hook,
            &mut progress_callback,
        )
        .expect("high checkpoint probe not-found response");

    assert!(!synced);
    assert_eq!(
        *generic_attempts.lock().expect("lock generic attempts"),
        1,
        "high checkpoint candidate scans should not spend the full gap-sync retry budget"
    );

    let _ = fs::remove_dir_all(&dir_remote);
    let _ = fs::remove_dir_all(&dir_local);
}
