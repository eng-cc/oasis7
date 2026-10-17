//! Bytes only, owned by one authenticated copy walk. A hit never supplies an
//! edge verdict, authority, qualification, absence or independent durability.
use super::*;
use std::collections::{BTreeMap, VecDeque};

const MEMO_BYTES: usize = 16 * 1024 * 1024;
const MEMO_OBJECTS: usize = 4096;

pub(super) struct CopyReadMemo<'a> {
    source: &'a dyn FormalArchiveSourceV1,
    objects: BTreeMap<String, Vec<u8>>,
    order: VecDeque<String>,
    bytes: usize,
    source_reads: u64,
    source_bytes: u64,
    hits: u64,
    seeded: u64,
    evictions: u64,
    trace: bool,
}
impl<'a> CopyReadMemo<'a> {
    pub(super) fn new(source: &'a dyn FormalArchiveSourceV1, trace: bool) -> Self {
        Self {
            source,
            objects: BTreeMap::new(),
            order: VecDeque::new(),
            bytes: 0,
            source_reads: 0,
            source_bytes: 0,
            hits: 0,
            seeded: 0,
            evictions: 0,
            trace,
        }
    }
    fn validate(id: &str, bytes: &[u8]) -> Result<(), FormalError> {
        hash_hex(id)?;
        if bytes.len() > LIMIT || blake3::hash(bytes).to_hex().as_str() != id {
            return Err(invalid("healthy copy memo object checksum/bound"));
        }
        Ok(())
    }
    fn remember(&mut self, id: &str, bytes: &[u8]) -> Result<bool, FormalError> {
        Self::validate(id, bytes)?;
        // Bound retained payload weight and object count independently. The
        // caller's current bytes, clones and map/key allocations are separate.
        if bytes.len() > MEMO_BYTES || self.objects.contains_key(id) {
            return Ok(false);
        }
        while self.objects.len() >= MEMO_OBJECTS || bytes.len() > MEMO_BYTES - self.bytes {
            let oldest = self
                .order
                .pop_front()
                .ok_or_else(|| invalid("healthy copy memo FIFO invariant"))?;
            let removed = self
                .objects
                .remove(&oldest)
                .ok_or_else(|| invalid("healthy copy memo entry invariant"))?;
            self.bytes -= removed.len();
            self.evictions = self.evictions.saturating_add(1);
        }
        self.bytes += bytes.len();
        self.objects.insert(id.into(), bytes.to_vec());
        self.order.push_back(id.into());
        Ok(true)
    }
    pub(super) fn seed(&mut self, id: &str, bytes: &[u8]) -> Result<(), FormalError> {
        if self.remember(id, bytes)? {
            self.seeded = self.seeded.saturating_add(1);
        }
        Ok(())
    }
    pub(super) fn read_object(&mut self, id: &str) -> Result<Option<Vec<u8>>, FormalError> {
        hash_hex(id)?;
        if let Some(bytes) = self.objects.get(id) {
            Self::validate(id, bytes)?;
            self.hits = self.hits.saturating_add(1);
            return Ok(Some(bytes.clone()));
        }
        self.source_reads = self.source_reads.saturating_add(1);
        let Some(bytes) = self.source.read_object(id)? else {
            // Missing or failed reads are never retained as authenticated absence.
            return Ok(None);
        };
        self.remember(id, &bytes)?;
        self.source_bytes = self.source_bytes.saturating_add(bytes.len() as u64);
        Ok(Some(bytes))
    }
}
impl Drop for CopyReadMemo<'_> {
    fn drop(&mut self) {
        if self.trace {
            eprintln!(
                "healthy-native-copy-memo source_reads={} source_bytes={} hits={} seeded={} evictions={} retained_objects={} retained_bytes={}",
                self.source_reads,
                self.source_bytes,
                self.hits,
                self.seeded,
                self.evictions,
                self.objects.len(),
                self.bytes,
            );
        }
    }
}

#[cfg(test)]
#[path = "endpoint_store_durable_set_copy_memo_tests.rs"]
mod tests;
