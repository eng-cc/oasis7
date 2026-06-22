use super::*;

#[test]
fn persist_runtime_snapshot_records_committed_resource_chunk_and_delta() {
    let mut world = World::new();
    world.submit_action(Action::RegisterAgent {
        agent_id: "agent-resource-chunk".to_string(),
        pos: pos(0, 0),
    });
    world.step().expect("register agent");

    let dir = temp_dir("persist-runtime-resource-chunk");
    world.save_to_dir(&dir).expect("save world");

    let snapshot = Snapshot::load_json(dir.join("snapshot.json")).expect("load snapshot");
    assert!(snapshot.chain_resource_manifest.is_schema_current());
    assert_eq!(snapshot.chain_resource_manifest.generated_chunks.len(), 1);
    assert!(
        snapshot
            .chain_resource_manifest
            .generated_chunks
            .values()
            .any(|chunk| chunk.chunk_status == ChainChunkResourceStatus::Committed)
    );
    let delta = snapshot
        .latest_chain_resource_delta
        .expect("resource delta");
    assert!(delta.is_schema_current());
    assert!(
        !delta.entries.is_empty(),
        "runtime resource delta should expose a non-empty chunk commit stream"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn persist_empty_runtime_snapshot_records_starter_resource_chunk_and_chain_context() {
    let world = World::new();

    let dir = temp_dir("persist-empty-runtime-starter-resource-chunk");
    world
        .save_to_dir_with_chain_resource_context(
            &dir,
            ChainResourceDerivationContext {
                world_id: "testnet-world",
                chain_id: "testnet-chain",
                genesis_ref: Some("genesis-ref"),
                created_at_height: 7,
                manifest_height: 7,
                commit_block_hash: Some("block-h7"),
                tick: world.state().time,
            },
            "testnet-world-config",
            "testnet-generation-algorithm",
        )
        .expect("save empty world with chain context");

    let snapshot = Snapshot::load_json(dir.join("snapshot.json")).expect("load snapshot");
    let manifest = snapshot.chain_resource_manifest;
    assert!(manifest.is_schema_current());
    assert_eq!(manifest.world_id, "testnet-world");
    assert_eq!(manifest.chain_id, "testnet-chain");
    assert_eq!(manifest.genesis_ref.as_deref(), Some("genesis-ref"));
    assert_eq!(manifest.manifest_height, 7);
    assert_eq!(manifest.created_at_block_hash.as_deref(), Some("block-h7"));
    assert_ne!(manifest.world_seed, 0);
    assert_eq!(manifest.generated_chunks.len(), 1);
    let chunk = manifest
        .generated_chunks
        .values()
        .next()
        .expect("starter chunk");
    assert_eq!(chunk.chunk_status, ChainChunkResourceStatus::Committed);
    assert!(!chunk.total_by_element_g.is_empty());
    assert_eq!(chunk.total_by_element_g, chunk.remaining_by_element_g);
    assert!(chunk.total_by_element_g.values().any(|amount| *amount > 0));

    let delta = snapshot
        .latest_chain_resource_delta
        .expect("resource delta");
    assert!(delta.is_schema_current());
    assert_eq!(delta.world_id, "testnet-world");
    assert_eq!(delta.chain_id, "testnet-chain");
    assert_eq!(delta.block_height, 7);
    assert_eq!(delta.ordering_key.height, 7);
    assert_eq!(delta.commit_block_hash.as_deref(), Some("block-h7"));
    assert_eq!(delta.resulting_manifest_hash, manifest.manifest_hash);
    assert!(!delta.entries.is_empty());
    assert!(delta.entries.iter().any(|entry| match entry {
        ChainResourceDeltaEntry::ChunkResource {
            total_delta_g,
            remaining_delta_g,
            resulting_remaining_g,
            ..
        } => *total_delta_g > 0 || *remaining_delta_g > 0 || *resulting_remaining_g > 0,
        _ => false,
    }));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn persist_claimed_agent_runtime_snapshot_preserves_starter_chunk_resources() {
    let mut world = World::new();
    world.submit_action(Action::RegisterAgent {
        agent_id: "claimed-agent".to_string(),
        pos: pos(0, 0),
    });
    world.step().expect("register claimed agent");

    let dir = temp_dir("persist-claimed-agent-starter-resource-chunk");
    world
        .save_to_dir_with_chain_resource_context(
            &dir,
            ChainResourceDerivationContext {
                world_id: "testnet-world",
                chain_id: "testnet-world",
                genesis_ref: None,
                created_at_height: 1,
                manifest_height: 2,
                commit_block_hash: Some("block-h2"),
                tick: world.state().time,
            },
            "testnet-world-config",
            "testnet-generation-algorithm",
        )
        .expect("save claimed agent world with chain context");

    let snapshot = Snapshot::load_json(dir.join("snapshot.json")).expect("load snapshot");
    let chunk = snapshot
        .chain_resource_manifest
        .generated_chunks
        .values()
        .next()
        .expect("claimed agent chunk");
    assert_eq!(chunk.chunk_status, ChainChunkResourceStatus::Committed);
    assert!(!chunk.total_by_element_g.is_empty());
    assert_eq!(chunk.total_by_element_g, chunk.remaining_by_element_g);
    assert!(chunk.fragment_refs.iter().any(|fragment| {
        fragment
            .fragment_id
            .starts_with("runtime-starter-fragment:")
    }));
    assert!(
        chunk
            .fragment_refs
            .iter()
            .any(|fragment| fragment.fragment_id == "runtime-agent:claimed-agent")
    );
    let delta = snapshot
        .latest_chain_resource_delta
        .expect("resource delta");
    assert!(delta.is_schema_current());
    assert!(
        delta.entries.is_empty(),
        "non-genesis claimed-agent snapshot should not replay starter resources"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn persist_non_starter_agent_chunk_does_not_mint_extra_starter_budget() {
    let mut world = World::new();
    world.submit_action(Action::RegisterAgent {
        agent_id: "claimed-agent-far".to_string(),
        pos: pos(2_500_000, 4_200_000),
    });
    world.step().expect("register far claimed agent");

    let dir = temp_dir("persist-far-agent-no-extra-starter-budget");
    world
        .save_to_dir_with_chain_resource_context(
            &dir,
            ChainResourceDerivationContext {
                world_id: "testnet-world",
                chain_id: "testnet-world",
                genesis_ref: None,
                created_at_height: 1,
                manifest_height: 1,
                commit_block_hash: Some("block-h1"),
                tick: world.state().time,
            },
            "testnet-world-config",
            "testnet-generation-algorithm",
        )
        .expect("save far claimed agent world with chain context");

    let snapshot = Snapshot::load_json(dir.join("snapshot.json")).expect("load snapshot");
    assert_eq!(snapshot.chain_resource_manifest.generated_chunks.len(), 2);
    let starter_chunk = snapshot
        .chain_resource_manifest
        .generated_chunks
        .get("chunk:0:0:0")
        .expect("starter chunk");
    assert!(!starter_chunk.total_by_element_g.is_empty());
    let agent_chunk = snapshot
        .chain_resource_manifest
        .generated_chunks
        .get("chunk:1:2:0")
        .expect("agent chunk");
    assert!(agent_chunk.total_by_element_g.is_empty());
    assert!(agent_chunk.remaining_by_element_g.is_empty());
    assert!(
        agent_chunk
            .fragment_refs
            .iter()
            .any(|fragment| fragment.fragment_id == "runtime-agent:claimed-agent-far")
    );

    let delta = snapshot
        .latest_chain_resource_delta
        .expect("resource delta");
    assert!(delta.entries.iter().all(|entry| match entry {
        ChainResourceDeltaEntry::ChunkResource { coord, .. } => *coord == starter_chunk.coord,
        _ => true,
    }));

    let _ = std::fs::remove_dir_all(&dir);
}
