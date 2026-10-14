//! Formal business-result and scoped change-page wire contracts.
//!
//! These types are data only. They do not authenticate service signatures,
//! formal history, Merkle paths, or caller authorization.
use super::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const FORMAL_BUSINESS_RESULT_SCHEMA_V1: u32 = 1;
/// Sparse Merkle inclusion paths have one sibling for every 256-bit key.
pub const MAX_FORMAL_BUSINESS_RESULT_SIBLINGS: usize = 256;
pub const MAX_FORMAL_BUSINESS_RESULT_BYTES: usize = 4 * 1024 * 1024;

fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Exact outcome committed in the World intent ledger. It intentionally has
/// no canonical record ID or CommitRef, so its hash can be included by that
/// record's `result_root` without a hash cycle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormalBusinessOutcomeLeafV1 {
    pub schema_version: u32,
    pub correlation: RequestCorrelation,
    /// `public` for a public action, or one exact `agent:<id>` audience.
    pub visibility_scope: String,
    /// Hash of the complete original SubmitIntentRequest, including the
    /// signed payload bytes, deadline, request key, and payload digest.
    pub original_request_digest: String,
    pub action_id: u64,
    pub committed_height: u64,
    /// Position of the first qualified formal record that committed this
    /// immutable result. This numeric locator is not a record ID or CommitRef.
    pub first_formal_position: u64,
    pub outcome: FormalBusinessOutcomeV1,
    /// Profile 2 retains the exact original result for authenticated cold
    /// replay after the World hot ledger has been compacted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canonical_result_cbor_hex: Option<String>,
}
impl FormalBusinessOutcomeLeafV1 {
    pub fn validate(&self) -> Result<(), ContractError> {
        self.correlation.validate()?;
        if self.schema_version != FORMAL_BUSINESS_RESULT_SCHEMA_V1
            || !digest(&self.original_request_digest)
            || !(self.visibility_scope == "public"
                || self
                    .visibility_scope
                    .strip_prefix("agent:")
                    .is_some_and(|agent| !agent.trim().is_empty() && agent.len() <= 256))
            || self.action_id == 0
            || self.committed_height == 0
            || self.first_formal_position == 0
        {
            return Err(ContractError("formal business result identity"));
        }
        if let FormalBusinessOutcomeV1::Rejected { reason } = &self.outcome
            && (reason.trim().is_empty() || reason.len() > MAX_FORMAL_BUSINESS_RESULT_BYTES)
        {
            return Err(ContractError("formal business rejection bound"));
        }
        if let Some(bytes) = &self.canonical_result_cbor_hex
            && (bytes.is_empty()
                || bytes.len() % 2 != 0
                || bytes.len() > MAX_FORMAL_BUSINESS_RESULT_BYTES
                || !bytes
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)))
        {
            return Err(ContractError("formal original result byte bounds"));
        }
        let bytes = serde_json::to_vec(self).map_err(|_| ContractError("result encoding"))?;
        if bytes.len() > MAX_FORMAL_BUSINESS_RESULT_BYTES {
            return Err(ContractError("formal business result byte bound"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum FormalBusinessOutcomeV1 {
    Committed { receipt: Value },
    Rejected { reason: String },
}

/// The aliases needed to construct a CommitRef are committed as a separate
/// leaf in the same result tree. The formal record supplies position, record
/// ID, after-state root, governing-manifest digest, and projection root; this
/// leaf supplies the actual World binding and retention counters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormalBusinessCommitMetadataV1 {
    pub schema_version: u32,
    pub formal_position: u64,
    pub world_id: String,
    pub chain_id: String,
    pub genesis_digest: String,
    pub after_state_root: String,
    pub provider_world_id: String,
    pub branch_id: String,
    pub finality_epoch: u64,
    pub finality_status: String,
    pub finality_block_hash: Option<String>,
    pub reorg_generation: u64,
    pub governing_manifest_digest: String,
    pub authority_generation: u64,
    pub permission_generation: u64,
    /// Global journal event-ID bounds, not audience-filtered counts.
    pub event_id_era: u64,
    pub journal_retention_floor: u64,
    pub journal_high_watermark: u64,
}
impl FormalBusinessCommitMetadataV1 {
    pub fn validate(&self) -> Result<(), ContractError> {
        if self.schema_version != FORMAL_BUSINESS_RESULT_SCHEMA_V1
            || self.formal_position == 0
            || self.world_id.trim().is_empty()
            || self.chain_id.trim().is_empty()
            || self.provider_world_id.trim().is_empty()
            || self.branch_id.trim().is_empty()
            || !digest(&self.genesis_digest)
            || !digest(&self.after_state_root)
            || !digest(&self.governing_manifest_digest)
            || !matches!(
                self.finality_status.as_str(),
                "pending" | "verified" | "reorged" | "suspended"
            )
            || self
                .finality_block_hash
                .as_ref()
                .is_some_and(|value| !digest(value))
            || (self.finality_status == "verified" && self.finality_block_hash.is_none())
            || self.journal_retention_floor == 0
            || self.journal_retention_floor > self.journal_high_watermark.saturating_add(1)
        {
            return Err(ContractError("formal business commit metadata"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormalBusinessResultInclusionV1 {
    /// Domain-separated digest of the result-key or the fixed metadata key.
    pub key_digest: String,
    pub leaf_count: u64,
    pub siblings: Vec<String>,
}
impl FormalBusinessResultInclusionV1 {
    pub fn validate(&self) -> Result<(), ContractError> {
        if !digest(&self.key_digest)
            || self.leaf_count == 0
            || self.siblings.len() != MAX_FORMAL_BUSINESS_RESULT_SIBLINGS
            || self.siblings.iter().any(|value| !digest(value))
        {
            return Err(ContractError("formal business result inclusion bounds"));
        }
        Ok(())
    }
}

/// Authenticated event cursor for the formal Changes profile. `event_id_era`
/// is the committed journal allocator era, not finality or reorg generation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormalBusinessChangesCursorV1 {
    pub stream_id: String,
    pub scope_id: String,
    pub event_id_era: u64,
    pub sequence: u64,
    pub commit: CommitRef,
}
impl FormalBusinessChangesCursorV1 {
    pub fn validate(&self) -> Result<(), ContractError> {
        if self.stream_id.trim().is_empty() || self.scope_id.trim().is_empty() {
            return Err(ContractError("formal Changes cursor identity"));
        }
        self.commit.validate()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormalBusinessChangeV1<T> {
    pub cursor: FormalBusinessChangesCursorV1,
    pub change: T,
}

/// One result plus the metadata needed to reconstruct its exact qualified
/// CommitRef. Both inclusion paths must use the same committed result root.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormalBusinessResultProofV1 {
    pub schema_version: u32,
    pub outcome: FormalBusinessOutcomeLeafV1,
    pub outcome_inclusion: FormalBusinessResultInclusionV1,
    pub commit_metadata: FormalBusinessCommitMetadataV1,
    pub metadata_inclusion: FormalBusinessResultInclusionV1,
}
impl FormalBusinessResultProofV1 {
    pub fn validate(&self) -> Result<(), ContractError> {
        if self.schema_version != FORMAL_BUSINESS_RESULT_SCHEMA_V1 {
            return Err(ContractError("formal business result proof version"));
        }
        self.outcome.validate()?;
        self.commit_metadata.validate()?;
        self.outcome_inclusion.validate()?;
        self.metadata_inclusion.validate()?;
        if self.outcome_inclusion.leaf_count != self.metadata_inclusion.leaf_count
            || self.outcome_inclusion.key_digest == self.metadata_inclusion.key_digest
        {
            return Err(ContractError("formal result paths do not share a tree"));
        }
        Ok(())
    }
}

/// A page advances formal history by at most the existing bounded read-proof
/// window. Intermediate pages carry only a verified projection; the business
/// outcome is present only on the page whose qualified head is its original
/// formal position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormalBusinessOutcomePageV1<T> {
    pub schema_version: u32,
    pub requested_correlation: RequestCorrelation,
    pub from_anchor: FormalReadAnchorRefV1,
    pub target_request: ReadWorldViewRequest,
    pub target_view: ReadWorldViewResponse<T>,
    pub result: Option<FormalBusinessResultProofV1>,
    /// Current-root delegation and the separately proved original publication.
    /// Consumers must verify both roots and the exact old cut.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub historical_cbor_hex: Option<String>,
    pub continuation_anchor: Option<FormalReadAnchorRefV1>,
}
impl<T> FormalBusinessOutcomePageV1<T> {
    pub fn validate(&self) -> Result<(), ContractError> {
        self.requested_correlation.validate()?;
        self.from_anchor.validate()?;
        self.target_request.validate_formal_shape()?;
        self.target_view.validate(&self.target_request)?;
        let proof = self
            .target_view
            .formal_proof
            .as_ref()
            .ok_or(ContractError("formal result page has no target proof"))?;
        if self.schema_version != FORMAL_BUSINESS_RESULT_SCHEMA_V1
            || proof.anchor != self.from_anchor
            || self.target_request.formal_anchor.as_ref() != Some(&self.from_anchor)
            || self.target_view.formal_profile.as_ref() != Some(&FormalReadProfileV1::controlled())
            || self.target_view.execution_evidence_scope.is_some()
            || self.requested_correlation.key.world != self.target_request.world
            || self.target_request.fixed_commit.is_some()
            || self.target_request.min_commit.is_some()
        {
            return Err(ContractError("formal result page request/head binding"));
        }
        match (&self.result, &self.continuation_anchor) {
            (Some(result), None) => {
                result.validate()?;
                if result.outcome.correlation != self.requested_correlation
                    || (self.historical_cbor_hex.is_none()
                        && result.commit_metadata.formal_position
                            != self.target_view.version.commit.position)
                    || result.outcome.first_formal_position
                        != result.commit_metadata.formal_position
                    || result.commit_metadata.formal_position
                        > self.target_view.version.commit.position
                    || result.outcome_inclusion.leaf_count != result.metadata_inclusion.leaf_count
                {
                    return Err(ContractError("formal result page outcome target"));
                }
            }
            (None, Some(next)) => {
                if self.historical_cbor_hex.is_some() {
                    return Err(ContractError("historical result cannot be a continuation"));
                }
                next.validate()?;
                if next.position != self.target_view.version.commit.position
                    || next.record_id != self.target_view.version.commit.execution_block_hash
                    || next == &self.from_anchor
                {
                    return Err(ContractError("formal result page continuation"));
                }
            }
            _ => {
                return Err(ContractError(
                    "formal result page must complete or continue",
                ));
            }
        }
        Ok(())
    }
}

/// Explicit range covered by one formal Changes page. The projection witness
/// commits every event visible to this audience inside the retained global
/// journal window; numeric event IDs may be sparse for a filtered audience.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormalBusinessChangesPageV1<T> {
    pub schema_version: u32,
    /// Exact local verified head from which the caller requested this page.
    pub from_anchor: FormalReadAnchorRefV1,
    pub from_exclusive: FormalBusinessChangesCursorV1,
    pub to_inclusive: FormalBusinessChangesCursorV1,
    pub next_cursor: FormalBusinessChangesCursorV1,
    pub target_request: ReadWorldViewRequest,
    pub target_view: ReadWorldViewResponse<T>,
    pub changes: Vec<FormalBusinessChangeV1<serde_json::Value>>,
}
impl<T> FormalBusinessChangesPageV1<T> {
    pub fn validate(&self) -> Result<(), ContractError> {
        self.from_exclusive.validate()?;
        self.from_anchor.validate()?;
        self.to_inclusive.validate()?;
        self.next_cursor.validate()?;
        self.target_request.validate_formal_shape()?;
        self.target_view.validate(&self.target_request)?;
        if self.schema_version != FORMAL_BUSINESS_RESULT_SCHEMA_V1
            || self.target_view.formal_profile.as_ref() != Some(&FormalReadProfileV1::controlled())
            || self.target_view.execution_evidence_scope.is_some()
            || self.target_request.world != self.from_exclusive.commit.world
            || self.target_request.scope_id != self.from_exclusive.scope_id
            || self.target_request.formal_anchor.as_ref() != Some(&self.from_anchor)
            || self.target_view.version.commit != self.to_inclusive.commit
            || self.target_view.version.commit != self.next_cursor.commit
            || self.from_exclusive.stream_id != self.to_inclusive.stream_id
            || self.from_exclusive.scope_id != self.to_inclusive.scope_id
            || self.from_exclusive.event_id_era != self.to_inclusive.event_id_era
            || self.from_exclusive.event_id_era != self.next_cursor.event_id_era
            || self.to_inclusive.sequence < self.from_exclusive.sequence
            || self.next_cursor != self.to_inclusive
            || self
                .changes
                .windows(2)
                .any(|pair| pair[0].cursor.sequence >= pair[1].cursor.sequence)
        {
            return Err(ContractError("formal Changes range binding"));
        }
        let sidecar = self
            .target_view
            .formal_proof
            .as_ref()
            .ok_or(ContractError("formal Changes page has no projection proof"))?;
        if sidecar.anchor != self.from_anchor
            || self.from_anchor.position != self.from_exclusive.commit.position
            || self.from_anchor.record_id != self.from_exclusive.commit.execution_block_hash
            || self.from_exclusive.stream_id != self.target_view.continuation.stream_id
        {
            return Err(ContractError(
                "formal Changes source is not the prior verified head",
            ));
        }
        for change in &self.changes {
            change.cursor.validate()?;
            if change.cursor.stream_id != self.from_exclusive.stream_id
                || change.cursor.scope_id != self.from_exclusive.scope_id
                || change.cursor.event_id_era != self.from_exclusive.event_id_era
                || change.cursor.sequence <= self.from_exclusive.sequence
                || change.cursor.sequence > self.to_inclusive.sequence
                || change.cursor.commit != self.to_inclusive.commit
            {
                return Err(ContractError(
                    "formal Changes event outside requested range",
                ));
            }
        }
        Ok(())
    }
}
