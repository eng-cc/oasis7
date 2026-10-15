//! Old material reads retain this client's current protected floor. Only an
//! independently checked canonical business-root inclusion licenses the cut.
use super::super::super::endpoint::VerifiedCommittedHistoricalMaterialV1;
use super::*;
struct HistoricalMaterialReader<'a> {
    endpoint: std::cell::RefCell<&'a mut RemoteFormalEndpoint>,
    cache: Option<
        &'a dyn crate::controlled_authority::formal::business_index::BusinessIndexNodeReader,
    >,
}
impl crate::controlled_authority::formal::business_index::BusinessIndexNodeReader
    for HistoricalMaterialReader<'_>
{
    fn read_node(&self, digest: &str) -> Result<Option<Vec<u8>>, String> {
        if let Some(cache) = self.cache
            && let Some(bytes) = cache.read_node(digest)?
        {
            return Ok(Some(bytes));
        }
        self.endpoint
            .borrow_mut()
            .fetch_hash_object(digest)
            .map_err(|error| error.to_string())
    }
}

impl RemoteFormalEndpoint {
    pub fn restore_committed_historical_cut(
        &mut self,
        cut: &VerifiedCommittedHistoricalCutV1,
    ) -> Result<VerifiedCommittedHistoricalMaterialV1, FormalError> {
        self.restore_committed_historical_cut_inner(cut, None)
    }

    /// Validate the same committed material while resolving its immutable CAS
    /// objects through a caller-provided content-addressed reader first. This
    /// is a byte cache only: the qualified cut, material proof, and each
    /// committed write's digest/bytes are still checked below.
    pub fn restore_committed_historical_cut_with_reader(
        &mut self,
        cut: &VerifiedCommittedHistoricalCutV1,
        reader: &impl crate::controlled_authority::formal::business_index::BusinessIndexNodeReader,
    ) -> Result<VerifiedCommittedHistoricalMaterialV1, FormalError> {
        self.restore_committed_historical_cut_inner(cut, Some(reader))
    }

    fn restore_committed_historical_cut_inner(
        &mut self,
        cut: &VerifiedCommittedHistoricalCutV1,
        reader: Option<
            &dyn crate::controlled_authority::formal::business_index::BusinessIndexNodeReader,
        >,
    ) -> Result<VerifiedCommittedHistoricalMaterialV1, FormalError> {
        if self.authority.definition() != cut.delegating_authority().definition()
            || self.authority.policy() != cut.delegating_authority().policy()
            || self.authority.activation_ref() != cut.delegating_authority().activation_ref()
        {
            return Err(invalid(
                "delegated historical transport exact same authority context",
            ));
        }
        // Query returns old data, while the signed response head/minimum and
        // provider tuple continue advancing monotonically through normal call().
        let state = match self
            .call(Command::Query(cut.commit().record().request_digest.clone()))?
            .0
        {
            Value::Query(outcome) => match *outcome {
                FormalQueryOutcomeV1::Recorded(state) => *state,
                FormalQueryOutcomeV1::NotRecorded => {
                    return Err(invalid(
                        "HistoryUnavailable: delegated material not recorded",
                    ));
                }
            },
            _ => return Err(invalid("delegated historical Query response type")),
        };
        let content = HistoricalMaterialReader {
            endpoint: std::cell::RefCell::new(self),
            cache: reader,
        };
        let restored = super::super::super::endpoint::verify_committed_historical_material(
            cut, state, &content,
        )?;
        Self::check_committed_material_content(&restored, &content, reader)?;
        Ok(restored)
    }

    /// Complete the original content-availability check for an already verified
    /// delegated material. An explicit caller cache owns immutable bytes only.
    pub(crate) fn check_committed_material_content(
        restored: &VerifiedCommittedHistoricalMaterialV1,
        content: &impl crate::controlled_authority::formal::business_index::BusinessIndexNodeReader,
        cache: Option<
            &dyn crate::controlled_authority::formal::business_index::BusinessIndexNodeReader,
        >,
    ) -> Result<(), FormalError> {
        let result = &restored.material().roots
            [&crate::controlled_authority::replicated_protocol::ArtifactRole::Result];
        let envelope = FormalBusinessResultEnvelopeV1::decode(
            &restored
                .material()
                .objects
                .iter()
                .find(|object| &object.content_hash == result)
                .ok_or_else(|| invalid("historical Result material"))?
                .bytes,
        )?;
        let writes: Vec<_> = envelope
            .delta
            .node_writes
            .iter()
            .chain(
                envelope
                    .retained_history_delta
                    .as_ref()
                    .into_iter()
                    .flat_map(|delta| delta.node_writes.iter()),
            )
            .collect();
        // Validate the whole batch before installing any writes. This does not
        // memoize qualification: the opaque restored material above has already
        // passed every exact proof, canonical, Frozen and delta/root check.
        for write in &writes {
            if hex::encode(blake3::hash(&write.bytes).as_bytes()) != write.digest {
                return Err(invalid(
                    "HistoryUnavailable: committed business/retained CAS write digest mismatch",
                ));
            }
        }
        if let Some(cache) = cache {
            for write in &writes {
                cache
                    .cache_content_node(&write.digest, &write.bytes)
                    .map_err(|error| invalid(format!("HistoryUnavailable: {error}")))?;
            }
        }
        for write in writes {
            let fetched = crate::controlled_authority::formal::business_index::BusinessIndexNodeReader::read_node(content, &write.digest)
                .map_err(|error| invalid(format!("HistoryUnavailable: {error}")))?;
            if fetched.as_deref() != Some(write.bytes.as_slice()) {
                return Err(invalid(
                    "HistoryUnavailable: delegated business/retained CAS nodes missing/corrupt",
                ));
            }
        }
        Ok(())
    }
}
