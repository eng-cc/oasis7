//! Public historical cuts committed by a successor's business result root.
//! This payload is never a self-pinned read anchor. Consumers authenticate its
//! inclusion beneath their own canonical anchor and verify the old proof.
use super::business_index::{BusinessIndexLeafKindV1, BusinessIndexLeafV1};
use super::*;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormalHistoricalCutV1 {
    pub schema_version: u32,
    pub parent_record: CanonicalRecordV1,
    pub parent_qualified: FormalQualifiedCommitProofV1,
}
pub fn formal_historical_cut_key(
    world: &FormalWorldIdentityV1,
    parent_position: u64,
) -> Result<String, FormalError> {
    world.validate()?;
    if parent_position == 0 {
        return Err(crypto::invalid("historical cut position"));
    }
    crypto::hash(
        "oasis7.formal.business-index.historical-cut-key.v1",
        &(
            crypto::hash("oasis7.formal.world_identity.v1", world)?,
            parent_position,
        ),
    )
}
impl FormalHistoricalCutV1 {
    pub fn from_proof(proof: &FormalQualifiedCommitProofV1) -> Result<Self, FormalError> {
        let cut = Self {
            schema_version: 1,
            parent_record: proof.decision.record.clone(),
            parent_qualified: proof.clone(),
        };
        cut.validate()?;
        Ok(cut)
    }
    fn validate(&self) -> Result<(), FormalError> {
        self.parent_record.validate()?;
        if self.schema_version != 1 || self.parent_record != self.parent_qualified.decision.record {
            return Err(crypto::invalid(
                "historical cut schema/qualified canonical record",
            ));
        }
        Ok(())
    }
    pub fn leaf(&self) -> Result<BusinessIndexLeafV1, FormalError> {
        self.validate()?;
        let canonical_value =
            serde_cbor::to_vec(self).map_err(|error| crypto::invalid(error.to_string()))?;
        if canonical_value.len() > business_index::BUSINESS_INDEX_MAX_LEAF_BYTES {
            return Err(crypto::invalid("historical cut public proof bound"));
        }
        Ok(BusinessIndexLeafV1 {
            key_digest: formal_historical_cut_key(
                &self.parent_record.world,
                self.parent_record.position,
            )?,
            kind: BusinessIndexLeafKindV1::HistoricalCut,
            canonical_value,
        })
    }
    pub fn decode_leaf(
        leaf: &BusinessIndexLeafV1,
        world: &FormalWorldIdentityV1,
        position: u64,
    ) -> Result<Self, FormalError> {
        if leaf.kind != BusinessIndexLeafKindV1::HistoricalCut
            || leaf.key_digest != formal_historical_cut_key(world, position)?
            || leaf.canonical_value.len() > business_index::BUSINESS_INDEX_MAX_LEAF_BYTES
        {
            return Err(crypto::invalid(
                "historical cut exact kind/key/world/position",
            ));
        }
        let cut: Self = serde_cbor::from_slice(&leaf.canonical_value)
            .map_err(|error| crypto::invalid(error.to_string()))?;
        cut.validate()?;
        if cut.parent_record.world != *world
            || cut.parent_record.position != position
            || cut.leaf()? != *leaf
        {
            return Err(crypto::invalid("historical cut canonical leaf payload"));
        }
        Ok(cut)
    }
}

#[path = "business_history_committed.rs"]
mod committed;
pub use committed::VerifiedCommittedHistoricalCutV1;
pub(crate) use committed::VerifiedHistoricalCutInclusionV1;
pub use committed::VerifiedHistoricalCutInclusionV2;
