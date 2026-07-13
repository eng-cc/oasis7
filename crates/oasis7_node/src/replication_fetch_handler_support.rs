use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::execution_hook::NodeExecutionHook;
use crate::replication::{
    FetchBlobRequest, FetchBlobResponse, FetchCommitRequest, REPLICATION_FETCH_BLOB_PROTOCOL,
    load_blob_range_from_root,
};
use crate::replication::{
    GossipReplicationMessage, NodeReplicationConfig, ReplicationRuntime,
    load_latest_commit_message_from_root,
};
use crate::replication_state_reconcile::parse_replication_commit_payload;
use crate::{NodeError, REPLICATION_GAP_SYNC_MAX_HEIGHTS_PER_POLL};
use crate::{network_bad_request, network_internal_error};
use oasis7_proto::distributed_net::DistributedNetwork;
use oasis7_proto::world_error::WorldError as ProtoWorldError;

pub(super) const FETCH_BLOB_MAX_RESPONSE_BYTES: u64 =
    oasis7_proto::distributed_net::FETCH_BLOB_MAX_RAW_CHUNK_BYTES as u64;

pub(super) fn admit_fetch_commit_request(
    payload: &[u8],
    world_id: &str,
    replication: &NodeReplicationConfig,
) -> Result<FetchCommitRequest, ProtoWorldError> {
    let request = serde_json::from_slice::<FetchCommitRequest>(payload)
        .map_err(|err| network_bad_request(format!("decode fetch-commit request failed: {err}")))?;
    if request.world_id != world_id {
        return Err(network_bad_request(format!(
            "fetch-commit world mismatch: expected={world_id}, got={}",
            request.world_id
        )));
    }
    replication
        .authorize_fetch_commit_request(&request)
        .map_err(|err| network_bad_request(format!("fetch-commit authorization failed: {err}")))?;
    Ok(request)
}

pub(super) fn validated_fetch_blob_range(
    offset_bytes: Option<u64>,
    limit_bytes: Option<u64>,
) -> Result<(u64, usize), String> {
    let offset = offset_bytes.unwrap_or(0);
    let limit = limit_bytes.unwrap_or(FETCH_BLOB_MAX_RESPONSE_BYTES);
    if limit == 0 || limit > FETCH_BLOB_MAX_RESPONSE_BYTES {
        return Err(format!(
            "fetch-blob requested response bytes must be within 1..={FETCH_BLOB_MAX_RESPONSE_BYTES}"
        ));
    }
    Ok((offset, usize::try_from(limit).unwrap_or(usize::MAX)))
}

pub(super) fn register_fetch_blob_handler(
    network: std::sync::Arc<dyn DistributedNetwork<ProtoWorldError> + Send + Sync>,
    replication: &NodeReplicationConfig,
) -> Result<(), ProtoWorldError> {
    let root = replication.root_dir.clone();
    let handler_config = replication.clone();
    let admission_config = replication.clone();
    network.register_handler_with_admission(
        REPLICATION_FETCH_BLOB_PROTOCOL,
        Box::new(move |payload| {
            let request = serde_json::from_slice::<FetchBlobRequest>(payload).map_err(|err| {
                network_bad_request(format!("decode fetch-blob request failed: {err}"))
            })?;
            admission_config
                .authorize_fetch_blob_request(&request)
                .map_err(|err| {
                    network_bad_request(format!("fetch-blob authorization failed: {err}"))
                })?;
            validated_fetch_blob_range(request.offset_bytes, request.limit_bytes)
                .map_err(network_bad_request)?;
            Ok(())
        }),
        Box::new(move |payload| {
            let request = serde_json::from_slice::<FetchBlobRequest>(payload).map_err(|err| {
                network_bad_request(format!("decode fetch-blob request failed: {err}"))
            })?;
            handler_config
                .authorize_fetch_blob_request(&request)
                .map_err(|err| {
                    network_bad_request(format!("fetch-blob authorization failed: {err}"))
                })?;
            let (offset, limit) =
                validated_fetch_blob_range(request.offset_bytes, request.limit_bytes)
                    .map_err(network_bad_request)?;
            let blob = load_blob_range_from_root(
                root.as_path(),
                request.content_hash.as_str(),
                offset,
                limit,
            )
            .map_err(network_internal_error)?;
            let (blob, range_complete) = match blob {
                Some((bytes, complete)) => (Some(bytes), Some(complete)),
                None => (None, None),
            };
            serde_json::to_vec(&FetchBlobResponse {
                found: blob.is_some(),
                range_offset_bytes: range_complete.map(|_| offset),
                range_complete,
                blob,
            })
            .map_err(|err| {
                network_internal_error(NodeError::Replication {
                    reason: format!("encode fetch-blob response failed: {err}"),
                })
            })
        }),
    )
}

pub(super) fn attach_checkpoint_for_fetch_commit_if_boundary(
    message: Option<GossipReplicationMessage>,
    execution_hook: Option<&Arc<Mutex<Box<dyn NodeExecutionHook>>>>,
    root_dir: &Path,
    world_id: &str,
    node_id: &str,
    replication: &NodeReplicationConfig,
    request_height: u64,
) -> Result<Option<GossipReplicationMessage>, NodeError> {
    let Some(execution_hook) = execution_hook else {
        return Ok(message);
    };
    let Some(message) = message else {
        return Ok(None);
    };
    let latest_commit_height = load_latest_commit_message_from_root(
        root_dir,
        world_id,
        replication.max_hot_commit_messages(),
    )?
    .and_then(|message| {
        parse_replication_commit_payload(message.payload.as_slice()).map(|payload| payload.height)
    });
    if !latest_commit_height
        .map(|latest_height| {
            should_export_checkpoint_for_fetch_commit(request_height, latest_height)
        })
        .unwrap_or(false)
    {
        return Ok(Some(message));
    }

    let checkpoint = execution_hook
        .lock()
        .map_err(|_| NodeError::Execution {
            reason: "execution hook lock poisoned".to_string(),
        })?
        .export_checkpoint_bundle(request_height)
        .map_err(|reason| NodeError::Execution { reason })?;
    let Some(checkpoint) = checkpoint else {
        return Ok(Some(message));
    };
    let mut runtime = ReplicationRuntime::new(replication, node_id)?;
    runtime
        .attach_execution_checkpoint_descriptor_to_message(node_id, &message, &checkpoint)
        .map(Some)
}

pub(super) fn should_export_checkpoint_for_fetch_commit(
    request_height: u64,
    latest_height: u64,
) -> bool {
    request_height == latest_height
        || request_height % REPLICATION_GAP_SYNC_MAX_HEIGHTS_PER_POLL == 0
        || request_height % (REPLICATION_GAP_SYNC_MAX_HEIGHTS_PER_POLL / 2) == 0
}
