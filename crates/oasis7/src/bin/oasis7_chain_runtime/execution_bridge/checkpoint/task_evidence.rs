//! Derived live-task read dependencies. This scope is a GC cache, never authority.
use super::*;
use oasis7::world_service::{CanonicalIntentResultV1, correlation, game_task};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::io::Read;
const SCOPE: &str = "execution_bridge_live_tasks_v1";
#[cfg(test)]
std::thread_local! {
    static PROTECTED_HEIGHT_VALIDATIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(in super::super) fn reset_protected_height_validation_count() {
    PROTECTED_HEIGHT_VALIDATIONS.with(|count| count.set(0));
}

#[cfg(test)]
pub(in super::super) fn protected_height_validation_count() -> usize {
    PROTECTED_HEIGHT_VALIDATIONS.with(std::cell::Cell::get)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Shard {
    pins: BTreeSet<String>,
}
fn required_refs(record: &ExecutionBridgeRecord) -> Result<BTreeSet<String>, String> {
    let snapshot = record
        .snapshot_ref
        .as_ref()
        .ok_or("live task recovery required: snapshot record compacted")?;
    let journal = record
        .journal_ref
        .as_ref()
        .ok_or("live task recovery required: journal record compacted")?;
    let proof = record
        .world_head_proof_ref
        .as_ref()
        .ok_or("live task recovery required: head proof unavailable")?;
    Ok([snapshot.clone(), journal.clone(), proof.clone()]
        .into_iter()
        .collect())
}
fn checked_record(
    records: &Path,
    store: &LocalCasStore,
    height: u64,
) -> Result<ExecutionBridgeRecord, String> {
    let record = load_execution_bridge_record(&execution_bridge_record_path(records, height))?;
    if record.height != height || height == 0 {
        return Err("invalid live task record height".into());
    }
    for reference in required_refs(&record)? {
        store
            .get_verified(&reference)
            .map_err(|_| "live task recovery required: historical CAS missing or corrupt")?;
    }
    let proof_bytes = store
        .get_verified(record.world_head_proof_ref.as_ref().unwrap())
        .map_err(|_| "live task recovery required: historical proof missing or corrupt")?;
    let proof: super::super::WorldHeadProofV1 =
        serde_cbor::from_slice(&proof_bytes).map_err(|_| "invalid live task historical proof")?;
    proof.validate_contract()?;
    if record.world_head_proof_hash.as_deref() != Some(proof.proof_hash()?.as_str())
        || proof.world_id != record.world_id
        || proof.height != height
        || proof.timestamp_ms != record.timestamp_ms
        || proof.execution.execution_state_root != record.execution_state_root
        || proof.execution.execution_block_hash != record.execution_block_hash
        || proof.execution.node_block_hash != record.node_block_hash.as_deref().unwrap_or("")
        || proof.execution.action_root != record.action_root.as_deref().unwrap_or("")
        || Some(&proof.snapshot_manifest_ref.content_hash) != record.snapshot_ref.as_ref()
        || Some(&proof.journal_segments_ref.content_hash) != record.journal_ref.as_ref()
        || record.snapshot_ref.as_deref() != Some(record.execution_state_root.as_str())
        || record.latest_state_ref != record.snapshot_ref
    {
        return Err("live task historical proof binding mismatch".into());
    }
    Ok(record)
}
/// Reject malformed, unknown or substituted cache shards before any GC/compaction.
pub(super) fn protected_heights(
    records: &Path,
    store: &LocalCasStore,
) -> Result<BTreeSet<u64>, String> {
    #[cfg(test)]
    PROTECTED_HEIGHT_VALIDATIONS.with(|count| count.set(count.get().saturating_add(1)));

    let directory = store.root().join("pin_scopes").join(SCOPE);
    match fs::symlink_metadata(&directory) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(BTreeSet::new()),
        Err(_) => return Err("live task pin scope unavailable".into()),
        Ok(metadata) if !metadata.is_dir() || metadata.file_type().is_symlink() => {
            return Err("invalid live task pin scope".into());
        }
        Ok(_) => {}
    }
    if fs::symlink_metadata(store.root().join("pin_scopes"))
        .map_err(|_| "live task pin scope parent unavailable")?
        .file_type()
        .is_symlink()
    {
        return Err("invalid live task pin scope parent".into());
    }
    let mut result = BTreeSet::new();
    for entry in fs::read_dir(directory).map_err(|_| "live task pin scope unreadable")? {
        let entry = entry.map_err(|_| "live task pin scope unreadable")?;
        if !entry
            .file_type()
            .map_err(|_| "live task pin shard unavailable")?
            .is_file()
        {
            return Err("invalid live task pin shard type".into());
        }
        let name = entry.file_name();
        let name = name.to_str().ok_or("invalid live task pin shard name")?;
        let height = name
            .strip_prefix("record-")
            .and_then(|s| s.strip_suffix(".json"))
            .filter(|s| s.len() == 20 && s.bytes().all(|b| b.is_ascii_digit()))
            .and_then(|s| s.parse::<u64>().ok())
            .ok_or("invalid live task pin shard name")?;
        if name != format!("record-{height:020}.json") || height == 0 {
            return Err("invalid live task pin shard height".into());
        }
        let mut options = fs::OpenOptions::new();
        options.read(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.custom_flags(libc::O_NOFOLLOW);
        }
        let mut file = options
            .open(entry.path())
            .map_err(|_| "live task pin shard unavailable")?;
        if file
            .metadata()
            .map_err(|_| "live task pin shard metadata unavailable")?
            .len()
            > 65536
        {
            return Err("live task pin shard exceeds bound".into());
        }
        let mut bytes = Vec::new();
        file.by_ref()
            .take(65537)
            .read_to_end(&mut bytes)
            .map_err(|_| "live task pin shard unreadable")?;
        let shard: Shard =
            serde_json::from_slice(&bytes).map_err(|_| "invalid live task pin shard")?;
        let record = checked_record(records, store, height)?;
        if shard.pins != required_refs(&record)? {
            return Err("live task pin shard does not bind historical record".into());
        }
        result.insert(height);
    }
    Ok(result)
}
fn live_results(
    world: &oasis7::runtime::World,
) -> Result<BTreeMap<u64, Vec<CanonicalIntentResultV1>>, String> {
    let history = game_task::validate_retention_history(world)?;
    let mut selected: BTreeMap<u64, Vec<CanonicalIntentResultV1>> = BTreeMap::new();
    for agent in world.state().agents.keys() {
        if let Some(publication) = history.active_publish(agent)? {
            selected
                .entry(publication.committed_height)
                .or_default()
                .push(publication);
        }
    }
    for (result, context, subject) in history.retained_prefixes() {
        if history.validate_prefix_capture(context, subject).is_ok() {
            selected
                .entry(result.committed_height)
                .or_default()
                .push(result.clone());
        }
    }
    Ok(selected)
}
/// Called only with the driver's already validated current world, after durable record publish.
/// New dependencies are pinned first; release is recoverable under the caller's retention transaction.
pub(super) fn refresh(
    records: &Path,
    store: &LocalCasStore,
    world: &oasis7::runtime::World,
    hot: u64,
) -> Result<super::ValidatedTaskRetention, String> {
    let previous = protected_heights(records, store)?;
    let wanted = live_results(world)?;
    let latest = list_execution_bridge_record_heights(records)?
        .last()
        .copied()
        .ok_or("live task latest record unavailable")?;
    for (height, results) in &wanted {
        if *height > latest {
            return Err("live task result exceeds durable head".into());
        }
        // `protected_heights` has just validated every retained shard against
        // its current durable record, CAS objects, and historical proof. Do
        // not repeat that same record/CAS/proof read for an already-protected
        // height on every commit; newly protected heights still receive the
        // full validation below before their exact receipt is pinned.
        if previous.contains(height) {
            continue;
        }
        let record = checked_record(records, store, *height)?;
        // A newly retained height must prove each exact public receipt at the real historical cut.
        let identity = &results
            .first()
            .ok_or("empty live task evidence")?
            .request
            .correlation
            .key
            .world;
        let pinned = super::super::world_service_read::pin_at_height(
            records,
            store.root(),
            identity,
            *height,
        )?;
        for result in results {
            let key = correlation::key_digest(&result.request.correlation.key)?;
            if pinned
                .world
                .capability_revocation_state()
                .world_service_results
                .get(&key)
                != Some(&serde_json::to_value(result).map_err(|_| "invalid live task evidence")?)
            {
                return Err("live task historical receipt mismatch".into());
            }
        }
        // Validate the historical receipt before atomically publishing pins.
        store
            .replace_pin_scope_shard(
                SCOPE,
                &format!("record-{height:020}"),
                &required_refs(&record)?,
            )
            .map_err(|_| "live task dependency pin failed")?;
    }
    let wanted = wanted.keys().copied().collect::<BTreeSet<_>>();
    let hot_start = latest.saturating_sub(hot.max(1).saturating_sub(1));
    let archive_start = list_execution_checkpoint_heights(records)?.first().copied();
    for height in previous.difference(&wanted) {
        store
            .remove_pin_scope_shard(SCOPE, &format!("record-{height:020}"))
            .map_err(|_| "live task dependency release failed")?;
        if *height >= hot_start {
            continue;
        }
        let Some(mut record) = compact_execution_bridge_record_at_height(records, *height)? else {
            return Err("retired live task record unavailable".into());
        };
        let archive = record.schema_version < EXECUTION_BRIDGE_RECORD_SCHEMA_V2
            || archive_start.is_none_or(|start| *height >= start);
        if !archive {
            prune_execution_bridge_record_archive_at_height(records, *height)?;
            record = load_execution_bridge_record(&execution_bridge_record_path(records, *height))?;
        }
        replace_execution_bridge_record_pin_shard(&record, archive, false, false, store)?;
    }
    let durable_record = super::load_execution_bridge_record(
        super::execution_bridge_record_path(records, latest).as_path(),
    )?;
    Ok(super::ValidatedTaskRetention::new(
        records,
        store,
        durable_record,
        wanted,
    ))
}
