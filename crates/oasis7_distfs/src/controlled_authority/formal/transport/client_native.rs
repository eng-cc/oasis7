//! Private checkpoint restore and bounded immutable CAS retrieval. This is a
//! child of the authenticated client; callers cannot inject a signed header.
use super::super::super::endpoint::{
    FORMAL_ARCHIVE_CHUNK_LIMIT, FORMAL_ARCHIVE_OBJECT_LIMIT, VerifiedNativeCheckpointRestoreV1,
};
use super::*;
const MAX_RESTORE_MEMO_ENTRIES: usize = 4096;
const MAX_RESTORE_MEMO_BYTES: usize = 16 * 1024 * 1024;

#[derive(Default)]
struct RestoreMemo {
    objects: std::collections::BTreeMap<String, Vec<u8>>,
    bytes: usize,
    fetches: usize,
    hits: usize,
}
impl RestoreMemo {
    fn get(&mut self, id: &str) -> Result<Option<Vec<u8>>, FormalError> {
        hash_hex(id)?;
        let Some(bytes) = self.objects.get(id) else {
            return Ok(None);
        };
        if blake3::hash(bytes).to_hex().as_str() != id {
            return Err(invalid("native restore memo CAS hash mismatch"));
        }
        self.hits += 1;
        Ok(Some(bytes.clone()))
    }
    fn insert(&mut self, id: &str, bytes: &[u8]) -> Result<(), FormalError> {
        hash_hex(id)?;
        if blake3::hash(bytes).to_hex().as_str() != id {
            return Err(invalid("native restore memo CAS hash mismatch"));
        }
        if self.objects.contains_key(id) {
            return Ok(());
        }
        let Some(next_bytes) = self.bytes.checked_add(bytes.len()) else {
            return Ok(());
        };
        if self.objects.len() >= MAX_RESTORE_MEMO_ENTRIES || next_bytes > MAX_RESTORE_MEMO_BYTES {
            return Ok(());
        }
        self.objects.insert(id.to_owned(), bytes.to_vec());
        self.bytes = next_bytes;
        Ok(())
    }
}
impl Drop for RestoreMemo {
    fn drop(&mut self) {
        if std::env::var_os("OASIS7_HEALTHY_NATIVE_TRACE").is_some() {
            eprintln!(
                "healthy-native-restore memo_fetches={} memo_hits={} memo_entries={} memo_bytes={}",
                self.fetches,
                self.hits,
                self.objects.len(),
                self.bytes
            );
        }
    }
}

impl RemoteFormalEndpoint {
    pub fn archive_chunk(
        &mut self,
        id: &str,
        offset: u64,
        limit: u32,
    ) -> Result<Option<FormalArchiveChunkV1>, FormalError> {
        hash_hex(id)?;
        match self
            .call(Command::ArchiveChunk {
                id: id.into(),
                offset,
                limit,
            })?
            .0
        {
            Value::ArchiveChunk(chunk) => {
                if let Some(chunk) = &chunk {
                    chunk.validate(id, offset, limit)?;
                }
                Ok(chunk)
            }
            _ => Err(invalid("formal RPC CAS chunk response type")),
        }
    }
    pub(super) fn fetch_hash_object(&mut self, id: &str) -> Result<Option<Vec<u8>>, FormalError> {
        let Some(first) = self.archive_chunk(id, 0, FORMAL_ARCHIVE_CHUNK_LIMIT)? else {
            return Ok(None);
        };
        let total = first.total_length;
        if total > FORMAL_ARCHIVE_OBJECT_LIMIT {
            return Err(invalid("formal native CAS total bound"));
        }
        let mut bytes = first.bytes;
        while (bytes.len() as u64) < total {
            let offset = bytes.len() as u64;
            let next = self
                .archive_chunk(id, offset, FORMAL_ARCHIVE_CHUNK_LIMIT)?
                .ok_or_else(|| invalid("formal native CAS disappeared during read"))?;
            if next.total_length != total || next.bytes.is_empty() {
                return Err(invalid("formal native CAS inconsistent length/progress"));
            }
            bytes.extend(next.bytes);
        }
        if bytes.len() as u64 != total || blake3::hash(&bytes).to_hex().as_str() != id {
            return Err(invalid("formal native CAS complete checksum"));
        }
        Ok(Some(bytes))
    }
    /// Restores only from this client's own freshly authenticated protected
    /// checkpoint. The returned token cannot be constructed from Query bytes.
    pub fn restore_native_current(
        &mut self,
        original: &VerifiedAuthorityV1,
        recovery: Option<&VerifiedRecovery>,
        imported_cut: Option<&VerifiedFormalCommitV1>,
    ) -> Result<VerifiedNativeCheckpointRestoreV1, FormalError> {
        self.restore_native_current_inner(original, recovery, imported_cut, None, None)
    }
    pub fn restore_native_current_cached(
        &mut self,
        original: &VerifiedAuthorityV1,
        recovery: Option<&VerifiedRecovery>,
        imported_cut: Option<&VerifiedFormalCommitV1>,
        cache: &dyn super::super::super::business_index::BusinessIndexNodeReader,
    ) -> Result<VerifiedNativeCheckpointRestoreV1, FormalError> {
        self.restore_native_current_inner(original, recovery, imported_cut, None, Some(cache))
    }
    pub fn restore_native_current_with_durable_set(
        &mut self,
        original: &VerifiedAuthorityV1,
        activation: &VerifiedDurableSetActivationV1,
        cache: Option<&dyn super::super::super::business_index::BusinessIndexNodeReader>,
    ) -> Result<VerifiedNativeCheckpointRestoreV1, FormalError> {
        self.restore_native_current_inner(original, None, None, Some(activation), cache)
    }
    fn restore_native_current_inner(
        &mut self,
        original: &VerifiedAuthorityV1,
        recovery: Option<&VerifiedRecovery>,
        imported_cut: Option<&VerifiedFormalCommitV1>,
        durable_set: Option<&VerifiedDurableSetActivationV1>,
        cache: Option<&dyn super::super::super::business_index::BusinessIndexNodeReader>,
    ) -> Result<VerifiedNativeCheckpointRestoreV1, FormalError> {
        let previous_deadline = self.operation_deadline;
        if previous_deadline.is_none() {
            self.set_operation_deadline(std::time::Instant::now() + self.timeout);
        }
        let mut memo = RestoreMemo::default();
        let result = (|| {
            self.complete(Command::Head)?;
            let context = self
                .last_native_context
                .clone()
                .ok_or_else(|| invalid("formal native missing authenticated context"))?;
            let checkpoint = self
                .last_checkpoint
                .clone()
                .ok_or_else(|| invalid("formal native missing protected floor"))?;
            let restored = super::super::super::endpoint::restore_native_with_durable_set(
                &context,
                original,
                recovery,
                imported_cut,
                durable_set,
                &mut |id: &str| {
                    self.remaining_timeout(
                        self.operation_deadline
                            .ok_or_else(|| invalid("native deadline absent"))?,
                    )?;
                    if let Some(bytes) = memo.get(id)? {
                        self.remaining_timeout(
                            self.operation_deadline
                                .ok_or_else(|| invalid("native deadline absent"))?,
                        )?;
                        return Ok(bytes);
                    }
                    if let Some(cache) = cache
                        && let Some(bytes) = cache.read_cached_node(id).map_err(invalid)?
                    {
                        hash_hex(id)?;
                        if blake3::hash(&bytes).to_hex().as_str() != id {
                            return Err(invalid("native cached CAS object hash mismatch"));
                        }
                        self.remaining_timeout(
                            self.operation_deadline
                                .ok_or_else(|| invalid("native deadline absent"))?,
                        )?;
                        memo.insert(id, &bytes)?;
                        return Ok(bytes);
                    }
                    memo.fetches += 1;
                    let bytes = self
                        .fetch_hash_object(id)?
                        .ok_or_else(|| invalid("formal native referenced object missing"))?;
                    if let Some(cache) = cache {
                        cache.cache_content_node(id, &bytes).map_err(invalid)?;
                    }
                    memo.insert(id, &bytes)?;
                    Ok(bytes)
                },
            )?;
            self.complete(Command::Head)?;
            if self.last_checkpoint.as_ref() != Some(&checkpoint) {
                return Err(FormalError::Rollback(
                    "formal native checkpoint changed during restore".into(),
                ));
            }
            self.remaining_timeout(
                self.operation_deadline
                    .ok_or_else(|| invalid("restore deadline absent"))?,
            )?;
            Ok(restored)
        })();
        self.operation_deadline = previous_deadline;
        result
    }
}

#[cfg(test)]
#[path = "client_native_memo_tests.rs"]
mod memo_tests;
