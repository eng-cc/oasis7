use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use oasis7_distfs::blake3_hex;
use serde::{Deserialize, Serialize};

use super::commit_retention::{
    LatestCommitHeadIndex, hot_commit_message_heights_from_root,
    latest_commit_head_index_directory_from_root, latest_hot_commit_message_height_from_root,
    load_commit_message_cold_index_from_root, load_latest_commit_head_index_from_root,
    write_latest_commit_head_index_to_root,
};
use super::{
    COMMIT_FILE_PREFIX, GossipReplicationMessage, NodeError, REPLICATION_VERSION,
    load_commit_message_from_root, load_commit_message_record_from_root,
    verify_replication_message_signature,
};

const LATEST_COMMIT_WORLD_INVENTORY_SCHEMA: u8 = 1;
const LATEST_COMMIT_WORLD_INVENTORY_FILE: &str = "replication_commit_worlds.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct LatestCommitWorldInventory {
    schema: u8,
    worlds: BTreeMap<String, LatestCommitHeadIndex>,
}

impl Default for LatestCommitWorldInventory {
    fn default() -> Self {
        Self {
            schema: LATEST_COMMIT_WORLD_INVENTORY_SCHEMA,
            worlds: BTreeMap::new(),
        }
    }
}

pub(crate) fn load_latest_commit_message_from_root(
    root_dir: &Path,
    world_id: &str,
    _max_hot_commit_messages: usize,
) -> Result<Option<GossipReplicationMessage>, NodeError> {
    if let Some(index) = load_latest_commit_head_index_from_root(root_dir, world_id)? {
        return load_indexed_latest_commit_message(root_dir, world_id, &index);
    }

    // Legacy roots have no index yet. Choose by filename/cold metadata only, then
    // publish the durable index so subsequent head reads stay O(1) and never
    // parse the entire hot window.
    let hot_height = latest_hot_commit_message_height_from_root(root_dir)?.unwrap_or(0);
    let cold_height = load_commit_message_cold_index_from_root(root_dir)?
        .by_height
        .keys()
        .next_back()
        .copied()
        .unwrap_or(0);
    let candidate = hot_height.max(cold_height);
    if candidate == 0 {
        return Ok(None);
    }
    let Some(message) = load_commit_message_from_root(root_dir, world_id, candidate)? else {
        return Ok(None);
    };
    let index = latest_commit_head_index_for_message(candidate, &message)?;
    write_latest_commit_head_index_to_root(root_dir, &index)?;
    Ok(Some(message))
}

pub(crate) fn load_latest_commit_head_index_height_from_root(
    root_dir: &Path,
    world_id: &str,
) -> Result<Option<u64>, NodeError> {
    Ok(load_latest_commit_head_index_from_root(root_dir, world_id)?.map(|index| index.height))
}

pub(super) fn reconcile_latest_commit_head_indexes(root_dir: &Path) -> Result<(), NodeError> {
    let index_dir = latest_commit_head_index_directory_from_root(root_dir);
    let hot_heights = hot_commit_message_heights_from_root(root_dir)?;
    let cold_heights = load_commit_message_cold_index_from_root(root_dir)?
        .by_height
        .keys()
        .copied()
        .collect::<Vec<_>>();
    let latest_metadata_height = hot_heights
        .iter()
        .copied()
        .chain(cold_heights.iter().copied())
        .max();

    let mut current_by_world = BTreeMap::<String, LatestCommitHeadIndex>::new();
    if index_dir.exists() {
        let entries = fs::read_dir(index_dir.as_path()).map_err(|err| NodeError::Replication {
            reason: format!("list latest commit head indexes failed: {err}"),
        })?;
        for entry in entries {
            let entry = entry.map_err(|err| NodeError::Replication {
                reason: format!("read latest commit head index entry failed: {err}"),
            })?;
            let path = entry.path();
            if !path.is_file() || path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }
            let index = load_index_from_path(path.as_path())?;
            let expected_path =
                latest_commit_head_index_path_for_world(root_dir, index.world_id.as_str());
            if path != expected_path {
                return Err(NodeError::Replication {
                    reason: format!(
                        "latest commit head index filename/world mismatch: {}",
                        path.display()
                    ),
                });
            }

            // Validate every existing sidecar before considering promotion. A
            // corrupt current index must never be hidden by a newer candidate.
            load_indexed_latest_commit_message(root_dir, index.world_id.as_str(), &index)?;
            current_by_world.insert(index.world_id.clone(), index);
        }
    }

    let mut inventory = load_latest_commit_world_inventory(root_dir)?;
    let mut inventory_changed = false;
    for (world_id, candidate) in inventory.worlds.clone() {
        if world_id != candidate.world_id {
            return Err(NodeError::Replication {
                reason: format!(
                    "latest commit world inventory key/world mismatch: {}",
                    world_id
                ),
            });
        }
        match current_by_world.get(world_id.as_str()) {
            Some(current) if current.height > candidate.height => {}
            Some(current)
                if current.height == candidate.height
                    && current.record_content_hash == candidate.record_content_hash
                    && current.message_hash == candidate.message_hash => {}
            Some(current) if current.height == candidate.height => {
                return Err(NodeError::Replication {
                    reason: format!(
                        "latest commit head conflict with world inventory world={} height={}",
                        world_id, candidate.height
                    ),
                });
            }
            _ => {
                load_indexed_latest_commit_message(root_dir, world_id.as_str(), &candidate)?;
                write_latest_commit_head_index_to_root(root_dir, &candidate)?;
                current_by_world.insert(world_id, candidate);
            }
        }
    }

    for (world_id, current) in &current_by_world {
        match inventory.worlds.get(world_id) {
            Some(existing)
                if existing.height > current.height
                    || (existing.height == current.height
                        && existing.record_content_hash == current.record_content_hash
                        && existing.message_hash == current.message_hash) => {}
            Some(existing) if existing.height == current.height => {
                return Err(NodeError::Replication {
                    reason: format!(
                        "latest commit head conflict with current index world={} height={}",
                        world_id, current.height
                    ),
                });
            }
            _ => {
                inventory.worlds.insert(world_id.clone(), current.clone());
                inventory_changed = true;
            }
        }
    }

    // Existing indexes are the durable authority for their worlds. Only parse
    // the newest filename/cold-index candidate when metadata proves that the
    // indexes do not already cover the global newest height. This keeps startup
    // recovery bounded and avoids parsing unrelated retained payloads after a
    // valid indexed head has already been persisted.
    let Some(latest_metadata_height) = latest_metadata_height else {
        if inventory_changed {
            write_latest_commit_world_inventory(root_dir, &inventory)?;
        }
        return Ok(());
    };
    let indexes_cover_latest = current_by_world
        .values()
        .any(|index| index.height == latest_metadata_height);
    if indexes_cover_latest {
        if inventory_changed {
            write_latest_commit_world_inventory(root_dir, &inventory)?;
        }
        return Ok(());
    }

    let mut newest_by_world = BTreeMap::<String, LatestCommitHeadIndex>::new();
    let Some(message) = load_commit_message_record_from_root(root_dir, latest_metadata_height)?
    else {
        return Ok(());
    };
    if message.version != REPLICATION_VERSION
        || message.world_id.is_empty()
        || message.record.world_id != message.world_id
    {
        return Err(NodeError::Replication {
            reason: format!(
                "invalid commit message world/version binding at height {}",
                latest_metadata_height
            ),
        });
    }
    let candidate = latest_commit_head_index_for_message(latest_metadata_height, &message)?;
    load_indexed_latest_commit_message(root_dir, message.world_id.as_str(), &candidate)?;
    newest_by_world.insert(message.world_id, candidate);

    for (world_id, candidate) in newest_by_world {
        match current_by_world.get(world_id.as_str()) {
            Some(current) if current.height > candidate.height => continue,
            Some(current)
                if current.height == candidate.height
                    && current.record_content_hash == candidate.record_content_hash
                    && current.message_hash == candidate.message_hash =>
            {
                continue;
            }
            Some(current) if current.height == candidate.height => {
                return Err(NodeError::Replication {
                    reason: format!(
                        "latest commit head conflict during startup reconciliation world={} height={}",
                        world_id, candidate.height
                    ),
                });
            }
            _ => {}
        }
        write_latest_commit_head_index_to_root(root_dir, &candidate)?;
        inventory.worlds.insert(world_id, candidate);
        inventory_changed = true;
    }
    if inventory_changed {
        write_latest_commit_world_inventory(root_dir, &inventory)?;
    }
    Ok(())
}

pub(super) fn record_latest_commit_world_inventory(
    root_dir: &Path,
    index: &LatestCommitHeadIndex,
) -> Result<(), NodeError> {
    let mut inventory = load_latest_commit_world_inventory(root_dir)?;
    match inventory.worlds.get(index.world_id.as_str()) {
        Some(existing) if existing.height > index.height => return Ok(()),
        Some(existing)
            if existing.height == index.height
                && existing.record_content_hash == index.record_content_hash
                && existing.message_hash == index.message_hash =>
        {
            return Ok(());
        }
        _ => {}
    }
    inventory
        .worlds
        .insert(index.world_id.clone(), index.clone());
    write_latest_commit_world_inventory(root_dir, &inventory)
}

fn load_latest_commit_world_inventory(
    root_dir: &Path,
) -> Result<LatestCommitWorldInventory, NodeError> {
    let path = root_dir.join(LATEST_COMMIT_WORLD_INVENTORY_FILE);
    if !path.exists() {
        return Ok(LatestCommitWorldInventory::default());
    }
    let inventory = super::load_json_or_default::<LatestCommitWorldInventory>(path.as_path())?;
    if inventory.schema != LATEST_COMMIT_WORLD_INVENTORY_SCHEMA {
        return Err(NodeError::Replication {
            reason: format!(
                "invalid latest commit world inventory schema: {}",
                path.display()
            ),
        });
    }
    for (world_id, index) in &inventory.worlds {
        if world_id.is_empty()
            || world_id != &index.world_id
            || index.schema != 1
            || index.height == 0
            || index.record_content_hash.is_empty()
            || index.message_hash.is_empty()
        {
            return Err(NodeError::Replication {
                reason: format!(
                    "invalid latest commit world inventory entry: {}",
                    path.display()
                ),
            });
        }
    }
    Ok(inventory)
}

fn write_latest_commit_world_inventory(
    root_dir: &Path,
    inventory: &LatestCommitWorldInventory,
) -> Result<(), NodeError> {
    super::write_json_compact(
        root_dir.join(LATEST_COMMIT_WORLD_INVENTORY_FILE).as_path(),
        inventory,
    )
}

fn latest_commit_head_index_path_for_world(root_dir: &Path, world_id: &str) -> PathBuf {
    latest_commit_head_index_directory_from_root(root_dir)
        .join(format!("{}.json", blake3_hex(world_id.as_bytes())))
}

fn load_index_from_path(path: &Path) -> Result<LatestCommitHeadIndex, NodeError> {
    let bytes = fs::read(path).map_err(|err| NodeError::Replication {
        reason: format!(
            "read latest commit head index {} failed: {err}",
            path.display()
        ),
    })?;
    let index = serde_json::from_slice::<LatestCommitHeadIndex>(&bytes).map_err(|err| {
        NodeError::Replication {
            reason: format!(
                "parse latest commit head index {} failed: {err}",
                path.display()
            ),
        }
    })?;
    if index.schema != 1
        || index.world_id.is_empty()
        || index.height == 0
        || index.record_content_hash.is_empty()
        || index.message_hash.is_empty()
    {
        return Err(NodeError::Replication {
            reason: format!("invalid latest commit head index: {}", path.display()),
        });
    }
    Ok(index)
}

fn load_indexed_latest_commit_message(
    root_dir: &Path,
    world_id: &str,
    index: &LatestCommitHeadIndex,
) -> Result<Option<GossipReplicationMessage>, NodeError> {
    let Some(message) = load_commit_message_from_root(root_dir, world_id, index.height)? else {
        return Err(NodeError::Replication {
            reason: format!(
                "latest commit head index points to missing commit world={} height={}",
                world_id, index.height
            ),
        });
    };
    let message_hash = serde_json::to_vec(&message)
        .map(|bytes| blake3_hex(bytes.as_slice()))
        .map_err(|err| NodeError::Replication {
            reason: format!("serialize indexed latest commit message failed: {err}"),
        })?;
    if message.world_id != world_id
        || message.record.content_hash != index.record_content_hash
        || message_hash != index.message_hash
        || blake3_hex(message.payload.as_slice()) != message.record.content_hash
        || message.record.path != format!("{COMMIT_FILE_PREFIX}/{:020}.json", index.height)
    {
        return Err(NodeError::Replication {
            reason: format!(
                "latest commit head index binding mismatch world={} height={}",
                world_id, index.height
            ),
        });
    }
    if message.signature_hex.is_some() || message.public_key_hex.is_some() {
        verify_replication_message_signature(&message)?;
    }
    if let Some(payload) = crate::replication_state_reconcile::parse_replication_commit_payload(
        message.payload.as_slice(),
    ) {
        if payload.world_id != world_id || payload.height != index.height {
            return Err(NodeError::Replication {
                reason: format!(
                    "latest commit payload/index height mismatch world={} index_height={}",
                    world_id, index.height
                ),
            });
        }
    }
    Ok(Some(message))
}

pub(super) fn latest_commit_head_index_for_message(
    height: u64,
    message: &GossipReplicationMessage,
) -> Result<LatestCommitHeadIndex, NodeError> {
    if height == 0 || message.world_id.is_empty() || message.record.content_hash.is_empty() {
        return Err(NodeError::Replication {
            reason: "cannot index invalid latest commit message".to_string(),
        });
    }
    let message_hash = serde_json::to_vec(message)
        .map(|bytes| blake3_hex(bytes.as_slice()))
        .map_err(|err| NodeError::Replication {
            reason: format!("serialize latest commit message for index failed: {err}"),
        })?;
    Ok(LatestCommitHeadIndex {
        schema: 1,
        world_id: message.world_id.clone(),
        height,
        record_content_hash: message.record.content_hash.clone(),
        message_hash,
    })
}

pub(super) fn prepare_latest_commit_head_persist(
    root_dir: &Path,
    height: u64,
    message: &GossipReplicationMessage,
    local_signer_public_key: Option<&str>,
) -> Result<(LatestCommitHeadIndex, Option<LatestCommitHeadIndex>), NodeError> {
    let next_index = latest_commit_head_index_for_message(height, message)?;
    let current_index =
        load_latest_commit_head_index_from_root(root_dir, message.world_id.as_str())?;
    let allow_same_height_replacement = local_signer_public_key
        .zip(message.public_key_hex.as_deref())
        .map(|(local, message_key)| local == message_key)
        .unwrap_or(false)
        && serde_json::from_slice::<super::ReplicatedCommitPayload>(message.payload.as_slice())
            .ok()
            .and_then(|payload| payload.lineage_envelope)
            .is_some();

    // The index is a recovery accelerator, not the authority for deciding
    // whether a height is already occupied. Legacy roots and interrupted
    // writes can have a durable hot/cold message with no sidecar; inspect and
    // fully validate that message before allowing any same-height write.
    if let Some(existing) = load_commit_message_record_from_root(root_dir, height)? {
        if existing.version != REPLICATION_VERSION
            || existing.world_id != message.world_id
            || existing.record.world_id != message.world_id
        {
            return Err(NodeError::Replication {
                reason: format!(
                    "latest commit head conflict at world={} height={}",
                    message.world_id, height
                ),
            });
        }
        let existing_index = latest_commit_head_index_for_message(height, &existing)?;
        load_indexed_latest_commit_message(root_dir, message.world_id.as_str(), &existing_index)?;
        if (existing_index.record_content_hash != next_index.record_content_hash
            || existing_index.message_hash != next_index.message_hash)
            && !allow_same_height_replacement
        {
            return Err(NodeError::Replication {
                reason: format!(
                    "latest commit head conflict at world={} height={}",
                    message.world_id, height
                ),
            });
        }
    }
    if let Some(current) = current_index.as_ref() {
        if height == current.height {
            load_indexed_latest_commit_message(root_dir, message.world_id.as_str(), current)?;
            if (current.record_content_hash != next_index.record_content_hash
                || current.message_hash != next_index.message_hash)
                && !allow_same_height_replacement
            {
                return Err(NodeError::Replication {
                    reason: format!(
                        "latest commit head conflict at world={} height={}",
                        message.world_id, height
                    ),
                });
            }
        }
    }
    Ok((next_index, current_index))
}

pub(super) fn finalize_latest_commit_head_persist(
    root_dir: &Path,
    next_index: &LatestCommitHeadIndex,
    current_index: Option<&LatestCommitHeadIndex>,
) -> Result<(), NodeError> {
    let should_replace = current_index
        .map(|current| {
            next_index.height > current.height
                || (next_index.height == current.height
                    && (current.record_content_hash != next_index.record_content_hash
                        || current.message_hash != next_index.message_hash))
        })
        .unwrap_or(true);
    if should_replace {
        write_latest_commit_head_index_to_root(root_dir, next_index)?;
    }
    Ok(())
}
