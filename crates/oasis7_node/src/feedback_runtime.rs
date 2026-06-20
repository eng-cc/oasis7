use std::sync::Mutex;

use oasis7_distfs::{
    FeedbackAnnounce, FeedbackAnnounceBridge, FeedbackStore, blake3_hex,
    ingest_feedback_announce_with_fetcher,
};
use oasis7_proto::world_error::WorldError as ProtoWorldError;

use crate::network_bridge::ReplicationNetworkEndpoint;
use crate::replication::{
    FetchBlobRequest, FetchBlobResponse, REPLICATION_FETCH_BLOB_PROTOCOL, ReplicationRuntime,
};
use crate::{NodeError, NodeFeedbackP2pConfig};

pub(crate) fn maybe_publish_runtime_feedback_announces(
    config: Option<&NodeFeedbackP2pConfig>,
    pending_feedback_announces: &Mutex<Vec<FeedbackAnnounce>>,
    bridge: Option<&FeedbackAnnounceBridge>,
) -> Result<(), NodeError> {
    let Some(config) = config else {
        return Ok(());
    };
    let Some(bridge) = bridge else {
        return Err(NodeError::InvalidConfig {
            reason: "feedback_p2p announce bridge is unavailable".to_string(),
        });
    };

    let to_publish = {
        let mut pending = pending_feedback_announces
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let drain_count = pending
            .len()
            .min(config.max_outgoing_announces_per_tick.max(1));
        if drain_count == 0 {
            Vec::new()
        } else {
            pending.drain(..drain_count).collect::<Vec<_>>()
        }
    };
    if to_publish.is_empty() {
        return Ok(());
    }

    let mut failed_announces = Vec::new();
    let mut failures = Vec::new();
    for announce in to_publish {
        if let Err(err) = bridge.publish(&announce) {
            failed_announces.push(announce);
            failures.push(format!("publish failed: {:?}", err));
        }
    }
    if failed_announces.is_empty() {
        return Ok(());
    }

    let mut pending = pending_feedback_announces
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    failed_announces.append(&mut *pending);
    *pending = failed_announces;
    Err(NodeError::Replication {
        reason: format!(
            "feedback announce publish failures: count={} first={}",
            failures.len(),
            failures[0]
        ),
    })
}

pub(crate) fn maybe_ingest_runtime_feedback_announces(
    config: Option<&NodeFeedbackP2pConfig>,
    store: Option<&FeedbackStore>,
    bridge: Option<&FeedbackAnnounceBridge>,
    replication: Option<&ReplicationRuntime>,
    replication_network: Option<&ReplicationNetworkEndpoint>,
) -> Result<(), NodeError> {
    let Some(config) = config else {
        return Ok(());
    };
    let Some(store) = store else {
        return Err(NodeError::InvalidConfig {
            reason: "feedback_p2p store is unavailable".to_string(),
        });
    };
    let Some(bridge) = bridge else {
        return Err(NodeError::InvalidConfig {
            reason: "feedback_p2p announce bridge is unavailable".to_string(),
        });
    };
    let Some(replication) = replication else {
        return Err(NodeError::InvalidConfig {
            reason: "feedback_p2p requires replication runtime".to_string(),
        });
    };
    let Some(replication_network) = replication_network else {
        return Err(NodeError::InvalidConfig {
            reason: "feedback_p2p requires replication network endpoint".to_string(),
        });
    };

    let mut failures = Vec::new();
    for announce in bridge
        .drain()
        .into_iter()
        .take(config.max_incoming_announces_per_tick.max(1))
    {
        let announce = match announce {
            Ok(announce) => announce,
            Err(err) => {
                failures.push(format!("decode announce failed: {:?}", err));
                continue;
            }
        };
        let ingest_result =
            ingest_feedback_announce_with_fetcher(store, &announce, |content_hash| {
                fetch_feedback_blob_from_replication_network(
                    content_hash,
                    replication,
                    replication_network,
                )
            });
        if let Err(err) = ingest_result {
            failures.push(format!(
                "feedback_id={} event_id={} err={:?}",
                announce.feedback_id, announce.event_id, err
            ));
        }
    }

    if failures.is_empty() {
        return Ok(());
    }
    Err(NodeError::Replication {
        reason: format!(
            "feedback announce ingest failures: count={} first={}",
            failures.len(),
            failures[0]
        ),
    })
}

fn fetch_feedback_blob_from_replication_network(
    content_hash: &str,
    replication: &ReplicationRuntime,
    replication_network: &ReplicationNetworkEndpoint,
) -> Result<Vec<u8>, ProtoWorldError> {
    let request = replication
        .build_fetch_blob_request(content_hash)
        .map_err(node_error_to_feedback_world_error)?;
    let response = replication_network
        .request_json::<FetchBlobRequest, FetchBlobResponse>(
            REPLICATION_FETCH_BLOB_PROTOCOL,
            &request,
        )
        .map_err(node_error_to_feedback_world_error)?;
    if !response.found {
        return Err(ProtoWorldError::BlobNotFound {
            content_hash: content_hash.to_string(),
        });
    }
    let blob = response
        .blob
        .ok_or_else(|| ProtoWorldError::DistributedValidationFailed {
            reason: format!(
                "feedback fetch-blob response missing blob payload for hash={}",
                content_hash
            ),
        })?;
    let actual_hash = blake3_hex(blob.as_slice());
    if actual_hash != content_hash {
        return Err(ProtoWorldError::BlobHashMismatch {
            expected: content_hash.to_string(),
            actual: actual_hash,
        });
    }
    Ok(blob)
}

fn node_error_to_feedback_world_error(err: NodeError) -> ProtoWorldError {
    ProtoWorldError::DistributedValidationFailed {
        reason: err.to_string(),
    }
}
