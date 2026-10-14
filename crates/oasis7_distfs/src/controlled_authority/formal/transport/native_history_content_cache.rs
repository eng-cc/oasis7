//! One restoration's immutable bytes only. No authorization or negative cache.
use super::*;
use std::collections::{HashMap, VecDeque};
const MAX_OBJECTS: usize = 512;
const MAX_BYTES: usize = 16 * 1024 * 1024;
#[derive(Default)]
pub(super) struct RestoreContentCache {
    objects: HashMap<String, Vec<u8>>,
    order: VecDeque<String>,
    bytes: usize,
}
impl RestoreContentCache {
    pub(super) fn get_checked(
        &self,
        id: &str,
        mut check_deadline: impl FnMut() -> Result<(), FormalError>,
    ) -> Result<Option<Vec<u8>>, FormalError> {
        check_deadline()?;
        let result = self.get(id)?;
        check_deadline()?;
        Ok(result)
    }
    fn get(&self, id: &str) -> Result<Option<Vec<u8>>, FormalError> {
        hash_hex(id)?;
        if let Some(bytes) = self.objects.get(id) {
            if blake3::hash(bytes).to_hex().as_str() != id {
                return Err(invalid("historical operation cache checksum"));
            }
            return Ok(Some(bytes.clone()));
        }
        Ok(None)
    }
    pub(super) fn insert(&mut self, id: &str, bytes: &[u8]) -> Result<(), FormalError> {
        hash_hex(id)?;
        if blake3::hash(bytes).to_hex().as_str() != id {
            return Err(invalid("historical operation cache checksum"));
        }
        if self.objects.contains_key(id) || bytes.len() > MAX_BYTES {
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
        }
        self.bytes += bytes.len();
        self.order.push_back(id.to_owned());
        self.objects.insert(id.to_owned(), bytes.to_vec());
        Ok(())
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
