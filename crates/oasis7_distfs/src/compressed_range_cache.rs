use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

use oasis7_proto::world_error::WorldError;

pub(super) const ENTRY_MAX_BYTES: u64 = 32 * 1024 * 1024;
const MAX_ENTRIES: usize = 2;
type Cache = HashMap<PathBuf, Arc<Mutex<Option<Vec<u8>>>>>;
struct CacheState {
    entries: Cache,
    fifo: VecDeque<PathBuf>,
}
static CACHE: OnceLock<Mutex<CacheState>> = OnceLock::new();

pub(super) fn read_cached_range(
    file: fs::File,
    path: PathBuf,
    offset: u64,
    limit: usize,
    raw_len: u64,
) -> Result<(Vec<u8>, bool), WorldError> {
    let entry = {
        let mut cache = CACHE
            .get_or_init(|| {
                Mutex::new(CacheState {
                    entries: HashMap::new(),
                    fifo: VecDeque::new(),
                })
            })
            .lock()
            .expect("compressed range cache");
        if let Some(entry) = cache.entries.get(&path) {
            Arc::clone(entry)
        } else {
            if cache.entries.len() >= MAX_ENTRIES {
                if let Some(evicted) = cache.fifo.pop_front() {
                    cache.entries.remove(&evicted);
                }
            }
            let entry = Arc::new(Mutex::new(None));
            cache.entries.insert(path.clone(), Arc::clone(&entry));
            cache.fifo.push_back(path);
            entry
        }
    };
    let mut decoded = entry.lock().expect("compressed range cache entry");
    if decoded.is_none() {
        let mut decoder = zstd::stream::read::Decoder::new(file)?;
        let mut bytes = Vec::with_capacity(usize::try_from(raw_len).unwrap_or(0));
        decoder.read_to_end(&mut bytes)?;
        if bytes.len() as u64 != raw_len {
            return Err(WorldError::DistributedValidationFailed {
                reason: format!(
                    "compressed blob decoded length mismatch: expected={raw_len}, actual={}",
                    bytes.len()
                ),
            });
        }
        *decoded = Some(bytes);
    }
    let bytes = decoded.as_ref().expect("cache entry just populated");
    let offset = usize::try_from(offset).unwrap_or(usize::MAX);
    if offset >= bytes.len() {
        return Ok((Vec::new(), true));
    }
    let end = offset.saturating_add(limit).min(bytes.len());
    Ok((bytes[offset..end].to_vec(), end == bytes.len()))
}

#[cfg(test)]
pub(super) fn paths() -> Vec<PathBuf> {
    CACHE
        .get()
        .map(|cache| {
            cache
                .lock()
                .expect("compressed range cache")
                .fifo
                .iter()
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}
