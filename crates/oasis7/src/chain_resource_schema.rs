//! Compatibility exports for shared resource contracts.
#[cfg(test)]
use crate::simulator::chain_resource::*;
#[cfg(test)]
use crate::simulator::*;
pub use oasis7_agent_api::world_resource::*;
#[cfg(test)]
use serde::Serialize;
#[cfg(test)]
use sha2::{Digest, Sha256};
#[cfg(test)]
fn hash_json<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
    let bytes = serde_json::to_vec(value)?;
    Ok(sha256_hex(&bytes))
}

#[cfg(test)]
fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_resource_manifest_hash_is_stable_across_roundtrip() {
        let mut manifest = ChainResourceManifest::empty_at_height("world-a", 42, 7);
        manifest.chain_id = "chain-a".to_string();
        manifest.world_config_hash = "world-config-hash".to_string();
        manifest.generation_algorithm_hash = "algorithm-hash".to_string();
        manifest.manifest_hash = manifest.canonical_hash();

        let encoded = serde_json::to_string(&manifest).expect("encode manifest");
        let decoded: ChainResourceManifest =
            serde_json::from_str(encoded.as_str()).expect("decode manifest");

        assert_eq!(decoded.schema_version, CHAIN_RESOURCE_MANIFEST_SCHEMA_V1);
        assert_eq!(
            decoded.chunk_generation_schema_version,
            CHUNK_GENERATION_SCHEMA_V1
        );
        assert_eq!(decoded.manifest_hash, manifest.manifest_hash);
        assert!(decoded.is_schema_current());
    }

    #[test]
    fn chain_resource_delta_requires_world_resource_schema_and_commit_hashes() {
        let delta = ChainResourceDelta::empty_for_manifest("world-a", "manifest-hash", 9, 11);

        assert_eq!(delta.schema_version, CHAIN_RESOURCE_DELTA_SCHEMA_V1);
        assert_eq!(delta.chain_id, "unbound");
        assert_eq!(delta.ordering_key.height, 9);
        assert!(delta.is_schema_current());
    }

    #[test]
    fn chunk_generation_delta_base_hash_uses_pre_event_manifest() {
        let coord = ChunkCoord { x: 1, y: 2, z: 3 };
        let mut budget = crate::simulator::ChunkResourceBudget::default();
        budget
            .total_by_element_g
            .insert(FragmentElementKind::Iron, 100);
        budget
            .remaining_by_element_g
            .insert(FragmentElementKind::Iron, 80);

        let mut manifest = ChainResourceManifest::empty_at_height("world-a", 42, 3);
        manifest.chain_id = "chain-a".to_string();
        manifest.manifest_height = 3;
        manifest.generated_chunks.insert(
            chunk_key(coord),
            ChainChunkResourceManifestEntry {
                schema_version: CHAIN_RESOURCE_MANIFEST_SCHEMA_V1.to_string(),
                world_id: "world-a".to_string(),
                chain_id: "chain-a".to_string(),
                world_seed: 42,
                chunk_generation_schema_version: CHUNK_GENERATION_SCHEMA_V1.to_string(),
                coord,
                seed: 99,
                chunk_status: ChainChunkResourceStatus::Committed,
                fragment_count: 0,
                block_count: 0,
                fragment_refs: Vec::new(),
                chunk_budget_total_hash: hash_json(&budget.total_by_element_g).unwrap(),
                chunk_budget_remaining_hash: hash_json(&budget.remaining_by_element_g).unwrap(),
                total_by_element_g: budget.total_by_element_g.clone(),
                remaining_by_element_g: budget.remaining_by_element_g.clone(),
                commit_ref: ChainResourceCommitRef {
                    height: 3,
                    block_hash: Some("block-3".to_string()),
                    event_id: Some(7),
                    action_id: None,
                },
                manifest_hash: String::new(),
            },
        );
        manifest.refresh_hashes();

        let mut expected_base = manifest.clone();
        expected_base.generated_chunks.remove(&chunk_key(coord));
        expected_base.manifest_height = 2;
        expected_base.refresh_hashes();

        let event = WorldEvent {
            id: 7,
            time: 9,
            kind: WorldEventKind::ChunkGenerated {
                coord,
                seed: 99,
                fragment_count: 0,
                block_count: 0,
                chunk_budget: budget,
                cause: ChunkGenerationCause::Action,
            },
            runtime_event: None,
        };
        let delta = delta_from_simulator_event(
            ChainResourceDerivationContext {
                world_id: "world-a",
                chain_id: "chain-a",
                genesis_ref: None,
                created_at_height: 0,
                manifest_height: 3,
                commit_block_hash: Some("block-3"),
                tick: 9,
            },
            &manifest,
            &event,
        )
        .expect("chunk delta");

        assert_eq!(delta.base_manifest_hash, expected_base.manifest_hash);
        assert_ne!(delta.base_manifest_hash, manifest.manifest_hash);
        assert_eq!(delta.resulting_manifest_hash, manifest.manifest_hash);
    }
}
