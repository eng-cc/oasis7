use super::distributed::WorldHeadAnnounce;
use super::distributed_client::DistributedClient;
use super::distributed_dht::DistributedDht;
use super::distributed_validation::{validate_head_update, HeadValidationResult};
use super::error::WorldError;
use super::replay_flow::load_manifest_and_segments;
use oasis7_distfs::{blake3_hex, BlobStore};

pub fn replay_validate_head(
    world_id: &str,
    client: &DistributedClient,
    store: &impl BlobStore,
) -> Result<HeadValidationResult, WorldError> {
    let head = client.get_world_head(world_id)?;
    replay_validate_with_head(&head, client, store)
}

pub fn replay_validate_head_with_dht(
    world_id: &str,
    dht: &impl DistributedDht,
    client: &DistributedClient,
    store: &impl BlobStore,
) -> Result<HeadValidationResult, WorldError> {
    let head =
        dht.get_world_head(world_id)?
            .ok_or_else(|| WorldError::DistributedValidationFailed {
                reason: format!("world head not found for {world_id}"),
            })?;
    replay_validate_with_head_and_dht(&head, dht, client, store)
}

pub fn replay_validate_with_head(
    head: &WorldHeadAnnounce,
    client: &DistributedClient,
    store: &impl BlobStore,
) -> Result<HeadValidationResult, WorldError> {
    let block_response = client.get_block_response(&head.world_id, head.height)?;
    let block = block_response.block;
    let (manifest, segments) = load_manifest_and_segments(
        &block_response.snapshot_ref,
        &block_response.journal_ref,
        |content_hash| client.fetch_blob(content_hash),
        verify_blob_hash,
        |content_hash, bytes| store.put(content_hash, bytes),
        WorldError::from,
    )?;

    validate_head_update(head, &block, &manifest, &segments, store)
}

pub fn replay_validate_with_head_and_dht(
    head: &WorldHeadAnnounce,
    dht: &impl DistributedDht,
    client: &DistributedClient,
    store: &impl BlobStore,
) -> Result<HeadValidationResult, WorldError> {
    let block_response = client.get_block_response(&head.world_id, head.height)?;
    let block = block_response.block;
    let (manifest, segments) = load_manifest_and_segments(
        &block_response.snapshot_ref,
        &block_response.journal_ref,
        |content_hash| client.fetch_blob_from_dht(&head.world_id, content_hash, dht),
        verify_blob_hash,
        |content_hash, bytes| store.put(content_hash, bytes),
        WorldError::from,
    )?;

    validate_head_update(head, &block, &manifest, &segments, store)
}

fn verify_blob_hash(expected: &str, bytes: &[u8]) -> Result<(), WorldError> {
    let actual = blake3_hex(bytes);
    if actual != expected {
        return Err(WorldError::DistributedValidationFailed {
            reason: format!("blob hash mismatch: expected={expected}, actual={actual}"),
        });
    }
    Ok(())
}

#[cfg(all(test, feature = "self_tests"))]
mod tests {
    use std::fs;
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};

    use oasis7::runtime::{Action, World};
    use oasis7::GeoPos;
    use oasis7_distfs::{BlobStore as _, LocalCasStore};
    use oasis7_proto::distributed::{
        FetchBlobRequest, FetchBlobResponse, GetBlockRequest, GetBlockResponse,
        GetWorldHeadRequest, GetWorldHeadResponse, RR_FETCH_BLOB, RR_GET_BLOCK, RR_GET_WORLD_HEAD,
    };
    use oasis7_proto::distributed_dht::DistributedDht as _;

    use super::super::distributed_dht::InMemoryDht;
    use super::super::distributed_net::{DistributedNetwork, InMemoryNetwork};
    use super::super::distributed_storage::{store_execution_result, ExecutionWriteConfig};
    use super::super::util::to_canonical_cbor;
    use super::*;

    fn temp_dir(prefix: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("duration since epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("oasis7-net-{prefix}-{unique}"))
    }

    #[test]
    fn replay_validate_head_round_trip() {
        let dir = temp_dir("observer-replay");
        let store = LocalCasStore::new(&dir);
        let mut world = World::new();
        world.submit_action(Action::RegisterAgent {
            agent_id: "agent-1".to_string(),
            pos: GeoPos::new(0, 0, 0),
        });
        world.step().expect("step world");

        let snapshot = world.snapshot();
        let journal = world.journal().clone();
        let write = store_execution_result(
            "w1",
            1,
            "genesis",
            "exec-1",
            1,
            &snapshot,
            &journal,
            &store,
            ExecutionWriteConfig::default(),
        )
        .expect("write");

        let network: Arc<dyn DistributedNetwork + Send + Sync> = Arc::new(InMemoryNetwork::new());
        let head_announce = write.head_announce.clone();
        let expected_block_hash = head_announce.block_hash.clone();
        let write_block = write.block.clone();
        let write_snapshot_ref = write.snapshot_manifest_ref.content_hash.clone();
        let write_journal_ref = write.journal_segments_ref.content_hash.clone();
        let store_clone = store.clone();

        network
            .register_handler(
                RR_GET_WORLD_HEAD,
                Box::new(move |payload| {
                    let request: GetWorldHeadRequest =
                        serde_cbor::from_slice(payload).expect("decode request");
                    assert_eq!(request.world_id, "w1");
                    let response = GetWorldHeadResponse {
                        head: head_announce.clone(),
                    };
                    Ok(to_canonical_cbor(&response).expect("encode response"))
                }),
            )
            .expect("register head");

        network
            .register_handler(
                RR_GET_BLOCK,
                Box::new(move |payload| {
                    let request: GetBlockRequest =
                        serde_cbor::from_slice(payload).expect("decode request");
                    assert_eq!(request.world_id, "w1");
                    let response = GetBlockResponse {
                        block: write_block.clone(),
                        journal_ref: write_journal_ref.clone(),
                        snapshot_ref: write_snapshot_ref.clone(),
                    };
                    Ok(to_canonical_cbor(&response).expect("encode response"))
                }),
            )
            .expect("register block");

        network
            .register_handler(
                RR_FETCH_BLOB,
                Box::new(move |payload| {
                    let request: FetchBlobRequest =
                        serde_cbor::from_slice(payload).expect("decode request");
                    let bytes = store_clone.get(&request.content_hash).expect("load blob");
                    let response = FetchBlobResponse {
                        blob: bytes,
                        content_hash: request.content_hash,
                    };
                    Ok(to_canonical_cbor(&response).expect("encode response"))
                }),
            )
            .expect("register fetch blob");

        let client = DistributedClient::new(Arc::clone(&network));
        let result = replay_validate_head("w1", &client, &store).expect("replay");
        assert_eq!(result.block_hash, expected_block_hash);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn replay_validate_head_uses_dht_when_available() {
        let dir = temp_dir("observer-replay-dht");
        let store = LocalCasStore::new(&dir);
        let mut world = World::new();
        world.submit_action(Action::RegisterAgent {
            agent_id: "agent-1".to_string(),
            pos: GeoPos::new(0, 0, 0),
        });
        world.step().expect("step world");

        let snapshot = world.snapshot();
        let journal = world.journal().clone();
        let write = store_execution_result(
            "w1",
            1,
            "genesis",
            "exec-1",
            1,
            &snapshot,
            &journal,
            &store,
            ExecutionWriteConfig::default(),
        )
        .expect("write");

        let network: Arc<dyn DistributedNetwork + Send + Sync> = Arc::new(InMemoryNetwork::new());
        let write_block = write.block.clone();
        let write_snapshot_ref = write.snapshot_manifest_ref.content_hash.clone();
        let write_journal_ref = write.journal_segments_ref.content_hash.clone();
        let store_clone = store.clone();

        network
            .register_handler(
                RR_GET_BLOCK,
                Box::new(move |payload| {
                    let request: GetBlockRequest =
                        serde_cbor::from_slice(payload).expect("decode request");
                    assert_eq!(request.world_id, "w1");
                    let response = GetBlockResponse {
                        block: write_block.clone(),
                        journal_ref: write_journal_ref.clone(),
                        snapshot_ref: write_snapshot_ref.clone(),
                    };
                    Ok(to_canonical_cbor(&response).expect("encode response"))
                }),
            )
            .expect("register block");

        network
            .register_handler(
                RR_FETCH_BLOB,
                Box::new(move |payload| {
                    let request: FetchBlobRequest =
                        serde_cbor::from_slice(payload).expect("decode request");
                    let bytes = store_clone.get(&request.content_hash).expect("load blob");
                    let response = FetchBlobResponse {
                        blob: bytes,
                        content_hash: request.content_hash,
                    };
                    Ok(to_canonical_cbor(&response).expect("encode response"))
                }),
            )
            .expect("register fetch blob");

        let dht = InMemoryDht::new();
        dht.put_world_head("w1", &write.head_announce)
            .expect("put head");

        let client = DistributedClient::new(Arc::clone(&network));
        let result = replay_validate_head_with_dht("w1", &dht, &client, &store).expect("replay");
        assert_eq!(result.block_hash, write.head_announce.block_hash);

        let _ = fs::remove_dir_all(&dir);
    }
}
