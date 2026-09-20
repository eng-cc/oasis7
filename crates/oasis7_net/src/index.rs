use std::collections::HashSet;

use oasis7_proto::distributed::WorldHeadAnnounce;

use super::distributed_dht::{DistributedDht, ProviderRecord};
use super::distributed_provider_cache::ProviderCache;
use super::distributed_storage::ExecutionWriteResult;
use super::error::WorldError;

#[derive(Debug, Clone)]
pub struct IndexPublishResult {
    pub world_id: String,
    pub published: usize,
}

pub fn publish_world_head(
    dht: &impl DistributedDht,
    head: &WorldHeadAnnounce,
) -> Result<(), WorldError> {
    dht.put_world_head(&head.world_id, head)
}

pub fn publish_execution_providers(
    dht: &impl DistributedDht,
    world_id: &str,
    provider_id: &str,
    result: &ExecutionWriteResult,
) -> Result<IndexPublishResult, WorldError> {
    let hashes = collect_execution_hashes(result);

    let mut published = 0usize;
    for hash in hashes {
        dht.publish_provider(world_id, &hash, provider_id)?;
        published = published.saturating_add(1);
    }

    Ok(IndexPublishResult {
        world_id: world_id.to_string(),
        published,
    })
}

pub fn publish_execution_providers_cached(
    cache: &ProviderCache,
    world_id: &str,
    result: &ExecutionWriteResult,
) -> Result<IndexPublishResult, WorldError> {
    let hashes = collect_execution_hashes(result);

    let mut published = 0usize;
    for hash in hashes {
        cache.register_local_content(world_id, &hash)?;
        published = published.saturating_add(1);
    }

    Ok(IndexPublishResult {
        world_id: world_id.to_string(),
        published,
    })
}

pub fn query_providers(
    dht: &impl DistributedDht,
    world_id: &str,
    content_hash: &str,
) -> Result<Vec<ProviderRecord>, WorldError> {
    dht.get_providers(world_id, content_hash)
}

fn collect_execution_hashes(result: &ExecutionWriteResult) -> HashSet<String> {
    let mut hashes: HashSet<String> = HashSet::new();
    hashes.insert(result.block_ref.content_hash.clone());
    hashes.insert(result.snapshot_manifest_ref.content_hash.clone());
    hashes.insert(result.journal_segments_ref.content_hash.clone());
    for chunk in &result.snapshot_manifest.chunks {
        hashes.insert(chunk.content_hash.clone());
    }
    for segment in &result.journal_segments {
        hashes.insert(segment.content_hash.clone());
    }
    hashes
}
