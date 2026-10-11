//! Successful detached restores only. This is never a record, CAS or authority cache.
use oasis7::runtime::{Journal, Snapshot, World};
use std::collections::VecDeque;
use std::sync::{Mutex, OnceLock};

const MAX_ENTRIES: usize = 2;
const MAX_ENTRY_INPUT_BYTES: usize = 8 * 1024 * 1024;
const MAX_RETAINED_INPUT_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone, PartialEq, Eq)]
struct Key {
    snapshot_digest: [u8; 32],
    journal_digest: [u8; 32],
    snapshot_len: usize,
    journal_len: usize,
}
impl Key {
    fn from_bytes(snapshot: &[u8], journal: &[u8]) -> Self {
        Self {
            snapshot_digest: blake3::derive_key("oasis7.pinned-world.snapshot.v1", snapshot),
            journal_digest: blake3::derive_key("oasis7.pinned-world.journal.v1", journal),
            snapshot_len: snapshot.len(),
            journal_len: journal.len(),
        }
    }
    fn input_bytes(&self) -> Option<usize> {
        self.snapshot_len.checked_add(self.journal_len)
    }
}
struct Entry {
    key: Key,
    world: World,
}
#[derive(Default)]
pub(super) struct RestoreCache {
    entries: VecDeque<Entry>,
    // Serialized source lengths are a retention proxy, not the World heap size.
    retained_input_bytes: usize,
}
static CACHE: OnceLock<Mutex<RestoreCache>> = OnceLock::new();
pub(super) fn shared() -> &'static Mutex<RestoreCache> {
    CACHE.get_or_init(|| Mutex::new(RestoreCache::default()))
}

pub(super) fn restore(
    cache: &Mutex<RestoreCache>,
    snapshot_bytes: &[u8],
    journal_bytes: &[u8],
    snapshot: Snapshot,
    journal: Journal,
) -> Result<World, oasis7::runtime::WorldError> {
    restore_with(
        cache,
        Key::from_bytes(snapshot_bytes, journal_bytes),
        || World::from_snapshot(snapshot, journal),
    )
}

fn restore_with(
    cache: &Mutex<RestoreCache>,
    key: Key,
    restore: impl FnOnce() -> Result<World, oasis7::runtime::WorldError>,
) -> Result<World, oasis7::runtime::WorldError> {
    let eligible = key
        .input_bytes()
        .filter(|bytes| *bytes <= MAX_ENTRY_INPUT_BYTES);
    if eligible.is_some()
        && let Ok(cache) = cache.lock()
        && let Some(entry) = cache.entries.iter().find(|entry| entry.key == key)
    {
        // The retained original is never handed to a consumer or mutated.
        return Ok(entry.world.clone());
    }
    // Expensive validation never holds the process-wide cache mutex.
    let world = restore()?;
    if let Some(input_bytes) = eligible
        && let Ok(mut cache) = cache.lock()
        && !cache.entries.iter().any(|entry| entry.key == key)
    {
        while cache.entries.len() >= MAX_ENTRIES
            || cache.retained_input_bytes.saturating_add(input_bytes) > MAX_RETAINED_INPUT_BYTES
        {
            let Some(evicted) = cache.entries.pop_front() else {
                break;
            };
            cache.retained_input_bytes -= evicted.key.input_bytes().unwrap_or(0);
        }
        cache.retained_input_bytes += input_bytes;
        cache.entries.push_back(Entry {
            key,
            world: world.clone(),
        });
    }
    Ok(world)
}

#[cfg(test)]
#[path = "tests/world_service_restore_cache.rs"]
mod tests;
