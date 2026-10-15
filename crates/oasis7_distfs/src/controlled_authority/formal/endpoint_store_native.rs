//! Bounded native restore support; immutable index links are authorized only
//! by the independently protected signed checkpoint root supplied by the caller.
use super::*;
impl Store {
    pub fn persist_business_nodes(
        &self,
        writes: &[super::super::super::business_index::BusinessIndexNodeWriteV1],
    ) -> Result<(), FormalError> {
        super::super::super::business_pack::persist(&self.path.join("formal-objects-v1"), writes)
    }
    /// ClosedRecord bytes retain their logical artifact hashes. Larger artifacts
    /// use the existing raw CAS representation, not a larger business-node cap.
    pub fn persist_closed_artifacts(
        &self,
        writes: &[super::super::super::business_index::BusinessIndexNodeWriteV1],
    ) -> Result<(), FormalError> {
        let directory = self.path.join("formal-objects-v1");
        let mut small = Vec::new();
        for write in writes {
            hash_hex(&write.digest)?;
            if blake3::hash(&write.bytes).to_hex().as_str() != write.digest {
                return Err(invalid("closed artifact CAS hash mismatch"));
            }
            if write.bytes.len() as u64 <= super::super::super::business_pack::MAX_PACK_NODE
                && !write.bytes.is_empty()
            {
                small.push(write.clone());
                continue;
            }
            let path = directory.join(&write.digest);
            if let Some(existing) =
                super::super::super::business_pack::read_object(&path, &write.digest)?
            {
                if existing != write.bytes {
                    return Err(invalid("closed immutable artifact CAS changed"));
                }
                let mut options = OpenOptions::new();
                options.read(true);
                #[cfg(unix)]
                options.custom_flags(libc::O_NOFOLLOW);
                options.open(&path).map_err(io)?.sync_all().map_err(io)?;
                sync(&directory)?;
            } else {
                Self::atomic(&directory, &path, &write.bytes)?;
            }
        }
        self.persist_business_nodes(&small)
    }
    pub fn local_object_exists(&self, id: &str) -> bool {
        self.path.join("formal-objects-v1").join(id).is_file()
    }
    pub(crate) fn lookup_from_root(
        root: Option<&str>,
        request: &str,
        load: &mut impl FnMut(&str) -> Result<Vec<u8>, FormalError>,
    ) -> Result<Option<String>, FormalError> {
        hash_hex(request)?;
        let mut id = root.map(str::to_owned);
        let mut previous = None;
        while let Some(reference) = id {
            hash_hex(&reference)?;
            let bytes = load(&reference)?;
            if bytes.len() > 128 * 1024 || blake3::hash(&bytes).to_hex().as_str() != reference {
                return Err(invalid("formal native index checksum/metadata bound"));
            }
            match decode::<Index>(&bytes)? {
                Index::Leaf {
                    request: key,
                    entry,
                } => {
                    hash_hex(&key)?;
                    hash_hex(&entry)?;
                    return Ok((key == request).then_some(entry));
                }
                Index::Fork { bit, left, right } => {
                    hash_hex(&left)?;
                    hash_hex(&right)?;
                    if bit >= 256 || previous.is_some_and(|old| bit <= old) {
                        return Err(invalid("formal native index path order"));
                    }
                    previous = Some(bit);
                    id = Some(if Self::bit(request, bit)? {
                        right
                    } else {
                        left
                    });
                }
            }
        }
        Ok(None)
    }
    pub fn object_range(
        &self,
        id: &str,
        offset: u64,
        limit: u32,
    ) -> Result<Option<FormalArchiveChunkV1>, FormalError> {
        if let Some(chunk) = chunk::file_range(
            &self.path.join("formal-objects-v1").join(id),
            id,
            offset,
            limit,
        )? {
            return Ok(Some(chunk));
        }
        for source in &self.sources {
            if let Some(chunk) = source.read_range(id, offset, limit)? {
                chunk.validate(id, offset, limit)?;
                return Ok(Some(chunk));
            }
        }
        Ok(None)
    }
}
