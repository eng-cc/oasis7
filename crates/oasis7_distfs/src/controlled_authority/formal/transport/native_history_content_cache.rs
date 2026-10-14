//! One restoration's immutable bytes only. No authorization or negative cache.
use super::*;
use std::collections::{HashMap, VecDeque};
const MAX_OBJECTS: usize = 512;
const MAX_BYTES: usize = 16 * 1024 * 1024;
pub(super) struct RestoreContentCache {
    objects: HashMap<String, Vec<u8>>,
    order: VecDeque<String>,
    bytes: usize,
    started: std::time::Instant,
    source_reads: usize,
    source_failures: usize,
    source_bytes: usize,
    source_us: u128,
    external_cache_hits: usize,
    hits: usize,
    misses: usize,
    inserts: usize,
    evictions: usize,
    oversized: usize,
    restore_us: u128,
    outcome: &'static str,
}

impl Default for RestoreContentCache {
    fn default() -> Self {
        Self {
            objects: HashMap::new(),
            order: VecDeque::new(),
            bytes: 0,
            started: std::time::Instant::now(),
            source_reads: 0,
            source_failures: 0,
            source_bytes: 0,
            source_us: 0,
            external_cache_hits: 0,
            hits: 0,
            misses: 0,
            inserts: 0,
            evictions: 0,
            oversized: 0,
            restore_us: 0,
            outcome: "in_progress",
        }
    }
}
impl RestoreContentCache {
    pub(super) fn get_checked(
        &mut self,
        id: &str,
        mut check_deadline: impl FnMut() -> Result<(), FormalError>,
    ) -> Result<Option<Vec<u8>>, FormalError> {
        check_deadline()?;
        let result = self.get(id)?;
        check_deadline()?;
        Ok(result)
    }
    fn get(&mut self, id: &str) -> Result<Option<Vec<u8>>, FormalError> {
        hash_hex(id)?;
        if let Some(bytes) = self.objects.get(id) {
            if blake3::hash(bytes).to_hex().as_str() != id {
                return Err(invalid("historical operation cache checksum"));
            }
            self.hits += 1;
            return Ok(Some(bytes.clone()));
        }
        self.misses += 1;
        Ok(None)
    }
    pub(super) fn insert(&mut self, id: &str, bytes: &[u8]) -> Result<(), FormalError> {
        hash_hex(id)?;
        if blake3::hash(bytes).to_hex().as_str() != id {
            return Err(invalid("historical operation cache checksum"));
        }
        if self.objects.contains_key(id) {
            return Ok(());
        }
        if bytes.len() > MAX_BYTES {
            self.oversized += 1;
            return Ok(());
        }
        while self.objects.len() >= MAX_OBJECTS || self.bytes > MAX_BYTES - bytes.len() {
            let oldest = self
                .order
                .pop_front()
                .ok_or_else(|| invalid("historical operation cache accounting"))?;
            let removed = self
                .objects
                .remove(&oldest)
                .ok_or_else(|| invalid("historical operation cache order"))?;
            self.bytes -= removed.len();
            self.evictions += 1;
        }
        self.bytes += bytes.len();
        self.order.push_back(id.to_owned());
        self.objects.insert(id.to_owned(), bytes.to_vec());
        self.inserts += 1;
        Ok(())
    }

    pub(super) fn record_external_cache_hit(&mut self) {
        self.external_cache_hits += 1;
    }

    pub(super) fn record_source_read(
        &mut self,
        elapsed: std::time::Duration,
        bytes: Option<&[u8]>,
        failed: bool,
    ) {
        self.source_reads += 1;
        self.source_us = self.source_us.saturating_add(elapsed.as_micros());
        self.source_bytes = self
            .source_bytes
            .saturating_add(bytes.map_or(0, <[u8]>::len));
        self.source_failures += usize::from(failed);
    }

    pub(super) fn record_restore_elapsed(&mut self, elapsed: std::time::Duration) {
        self.restore_us = elapsed.as_micros();
    }

    pub(super) fn record_outcome(&mut self, success: bool) {
        self.outcome = if success { "success" } else { "error" };
    }
}

impl Drop for RestoreContentCache {
    fn drop(&mut self) {
        if std::env::var_os("OASIS7_HEALTHY_NATIVE_TRACE").is_none() {
            return;
        }
        let restore_local_us = self.restore_us.saturating_sub(self.source_us);
        eprintln!(
            "native-history-restore result={} source_reads={} source_failures={} source_bytes={} source_us={} memo_hits={} memo_misses={} external_cache_hits={} memo_inserts={} memo_evictions={} memo_oversized={} memo_entries={} memo_bytes={} restore_us={} restore_nonfetch_us={} operation_us={}",
            self.outcome,
            self.source_reads,
            self.source_failures,
            self.source_bytes,
            self.source_us,
            self.hits,
            self.misses,
            self.external_cache_hits,
            self.inserts,
            self.evictions,
            self.oversized,
            self.objects.len(),
            self.bytes,
            self.restore_us,
            restore_local_us,
            self.started.elapsed().as_micros(),
        );
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn historical_operation_cache_hit_cannot_extend_absolute_deadline() {
        let mut cache = RestoreContentCache::default();
        let bytes = b"cached immutable object";
        let id = blake3::hash(bytes).to_hex().to_string();
        cache.insert(&id, bytes).unwrap();
        let deadline = std::time::Instant::now() - std::time::Duration::from_secs(1);
        assert!(
            cache
                .get_checked(&id, || {
                    if std::time::Instant::now() >= deadline {
                        Err(FormalError::Io("absolute deadline expired".into()))
                    } else {
                        Ok(())
                    }
                })
                .is_err()
        );
        let mut checks = 0;
        assert!(
            cache
                .get_checked(&id, || {
                    checks += 1;
                    if checks == 2 {
                        Err(FormalError::Io("expired during hash validation".into()))
                    } else {
                        Ok(())
                    }
                })
                .is_err()
        );
        assert_eq!(checks, 2);
    }
    #[test]
    fn historical_operation_cache_reuses_content_and_evicts_with_fixed_bounds() {
        let mut cache = RestoreContentCache::default();
        let bytes = b"one authenticated CAS object";
        let id = blake3::hash(bytes).to_hex().to_string();
        let mut reads = 0;
        for _ in 0..4 {
            if cache.get(&id).unwrap().is_none() {
                reads += 1;
                cache.insert(&id, bytes).unwrap();
            }
        }
        assert_eq!(reads, 1);
        assert!(cache.insert(&id, b"corrupt").is_err());
        for n in 0u64..600 {
            let bytes = n.to_le_bytes();
            let id = blake3::hash(&bytes).to_hex().to_string();
            cache.insert(&id, &bytes).unwrap();
        }
        assert!(cache.get(&id).unwrap().is_none());
        assert_eq!(cache.objects.len(), MAX_OBJECTS);
        assert!(cache.bytes <= MAX_BYTES);
        let bytes = vec![17; MAX_BYTES];
        let large = blake3::hash(&bytes).to_hex().to_string();
        cache.insert(&large, &bytes).unwrap();
        assert_eq!(cache.objects.len(), 1);
        assert_eq!(cache.bytes, MAX_BYTES);
    }
}
