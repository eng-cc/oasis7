//! Versioned light proof transport. These DTOs never authenticate a proof or
//! authorize a trust key; verification belongs to the configured consumer.
use super::ContractError;
use serde::{Deserialize, Serialize};
pub const MAX_FORMAL_PROOF_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_FORMAL_CURSOR_FRAMES: usize = 32;
pub const MAX_FORMAL_CURSOR_BYTES: usize = 16 * 1024 * 1024;

/// Historical public proof frames, never a locally granted trust checkpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormalReadCursorFrameV1<T> {
    pub request: super::ReadWorldViewRequest,
    pub response: super::ReadWorldViewResponse<T>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, bound(deserialize = "T: Deserialize<'de>"))]
pub struct FormalReadCursorBundleV1<T> {
    pub schema_version: u32,
    /// Selector must match CURRENT externally authorized configuration. This
    /// field and a digest calculated from saved data supply no trust themselves.
    pub external_checkpoint_digest: String,
    pub commit: super::CommitRef,
    #[serde(deserialize_with = "bounded_cursor_frames")]
    pub frames: Vec<FormalReadCursorFrameV1<T>>,
}
fn bounded_cursor_frames<'de, D, T>(decoder: D) -> Result<Vec<FormalReadCursorFrameV1<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct Visitor<T>(std::marker::PhantomData<T>);
    impl<'de, T: Deserialize<'de>> serde::de::Visitor<'de> for Visitor<T> {
        type Value = Vec<FormalReadCursorFrameV1<T>>;
        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("bounded formal cursor frames")
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(
            self,
            mut sequence: A,
        ) -> Result<Self::Value, A::Error> {
            if sequence
                .size_hint()
                .is_some_and(|count| count > MAX_FORMAL_CURSOR_FRAMES)
            {
                return Err(serde::de::Error::custom("formal cursor frame bound"));
            }
            let mut frames = Vec::new();
            while let Some(frame) = sequence.next_element()? {
                if frames.len() == MAX_FORMAL_CURSOR_FRAMES {
                    return Err(serde::de::Error::custom("formal cursor frame bound"));
                }
                frames.push(frame);
            }
            Ok(frames)
        }
    }
    decoder.deserialize_seq(Visitor(std::marker::PhantomData))
}
impl<T: Serialize> FormalReadCursorBundleV1<T> {
    pub fn validate_structure(&self) -> Result<(), ContractError> {
        if self.schema_version != 1
            || !digest(&self.external_checkpoint_digest)
            || self.frames.is_empty()
            || self.frames.len() > MAX_FORMAL_CURSOR_FRAMES
        {
            return Err(ContractError(
                "formal cursor version, origin or frame bound",
            ));
        }
        self.commit.validate()?;
        for frame in &self.frames {
            frame.request.validate_formal_shape()?;
            if frame.request.scope_id != "public"
                || frame.request.fixed_commit.is_some()
                || frame.response.formal_profile.as_ref()
                    != Some(&FormalReadProfileV1::controlled())
            {
                return Err(ContractError(
                    "formal cursor requires ordinary public forward reads",
                ));
            }
            frame
                .response
                .formal_proof
                .as_ref()
                .ok_or(ContractError("formal cursor lacks proof"))?
                .validate()?;
        }
        if serde_json::to_vec(self)
            .map_err(|_| ContractError("formal cursor encoding"))?
            .len()
            > MAX_FORMAL_CURSOR_BYTES
        {
            return Err(ContractError("formal cursor byte bound"));
        }
        Ok(())
    }
}
/// Committed World capability counters, never physical writer/endpoint epochs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormalProjectionCommitHeaderV1 {
    pub authority_generation: u64,
    pub permission_generation: u64,
    pub event_id_era: u64,
    pub journal_retention_floor: u64,
    pub journal_high_watermark: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub governance_membership: Option<super::FormalGovernanceMembershipV1>,
}
fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormalReadProfileV1 {
    pub name: String,
    pub version: u32,
}
impl FormalReadProfileV1 {
    pub fn controlled() -> Self {
        Self {
            name: "controlled_single_authority".into(),
            version: 1,
        }
    }
    pub fn validate(&self) -> Result<(), ContractError> {
        if self != &Self::controlled() {
            return Err(ContractError("unsupported formal read profile"));
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormalReadAnchorRefV1 {
    pub position: u64,
    pub record_id: String,
    pub active_authority_id: String,
}
impl FormalReadAnchorRefV1 {
    pub fn validate(&self) -> Result<(), ContractError> {
        if self.position == 0 || !digest(&self.record_id) || !digest(&self.active_authority_id) {
            return Err(ContractError("formal read anchor"));
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormalProjectionAudienceV1 {
    pub scope_id: String,
    pub reader_public_key: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormalProjectionInclusionV1 {
    pub schema_version: u32,
    pub audience: FormalProjectionAudienceV1,
    pub leaf_index: u64,
    pub leaf_count: u64,
    pub siblings: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormalReadSidecarV1 {
    pub schema_version: u32,
    pub profile: FormalReadProfileV1,
    /// Exactly the caller's locally verified anchor; the service cannot replace
    /// it with an untrusted checkpoint or signing identity from the response.
    pub anchor: FormalReadAnchorRefV1,
    /// Canonical typed light history, max 128 steps. No ClosedRecord bytes.
    pub history_cbor_hex: String,
    pub projection: FormalProjectionInclusionV1,
}
impl FormalReadSidecarV1 {
    pub fn validate(&self) -> Result<(), ContractError> {
        self.profile.validate()?;
        self.anchor.validate()?;
        if self.schema_version != 1
            || self.history_cbor_hex.len() > 2 * MAX_FORMAL_PROOF_BYTES
            || !self.history_cbor_hex.len().is_multiple_of(2)
            || !self
                .history_cbor_hex
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            || self.projection.schema_version != 1
            || self.projection.leaf_count == 0
            || self.projection.leaf_count > 16384
            || self.projection.leaf_index >= self.projection.leaf_count
            || self.projection.siblings.len() > 14
            || self.projection.siblings.iter().any(|value| !digest(value))
        {
            return Err(ContractError("formal sidecar bounds/version"));
        }
        Ok(())
    }
}
