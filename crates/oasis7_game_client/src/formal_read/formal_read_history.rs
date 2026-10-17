use oasis7_client_api::world_service::FormalReadAnchorRefV1;
use oasis7_client_api::world_service::MAX_FORMAL_PROOF_BYTES;
use oasis7_formal_protocol::controlled_authority::{fencing::*, formal::*};
use serde::{Deserialize, Serialize};
pub const MAX_FORMAL_HISTORY_STEPS: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FormalReadFencingEvidenceV1 {
    HealthyPair(Box<[EpochSealReceiptV1; 2]>),
    NecessaryEndpoint(Box<EpochSealReceiptV1>),
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormalRecoveryReadStepV1 {
    pub certificate: SignedRecoveryCertificateV1,
    pub issuance: issuer::SignedIssuanceWitnessV1,
    pub fencing: FormalReadFencingEvidenceV1,
    pub original_tail: RecoveryTailWitnessV1,
}
/// Healthy replacement evidence, separate from disaster recovery. The
/// transition boundary must already be Qualified by the old configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormalNormalDurableSetReadStepV1 {
    pub transition: SignedNormalDurableSetTransitionV1,
    pub old_seals: [EpochSealReceiptV1; 2],
    pub new_installs: [DurableSetInstallReceiptV1; 2],
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FormalReadStepV1 {
    Commit(Box<FormalQualifiedCommitProofV1>),
    Recovery(Box<FormalRecoveryReadStepV1>),
    DurableSetActivation(Box<FormalNormalDurableSetReadStepV1>),
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormalReadHistoryV1 {
    pub schema_version: u32,
    #[serde(deserialize_with = "bounded_steps")]
    pub steps: Vec<FormalReadStepV1>,
}

/// Opaque replay boundary for one bounded history page. The anchor ref is
/// accompanied by a digest of the verifier-derived anchor and recovery state;
/// callers may persist this value, but cannot use it as authority without
/// replaying every page from the externally configured V1 origin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormalReadReplayCursorV1 {
    pub schema_version: u32,
    pub anchor: FormalReadAnchorRefV1,
    pub checkpoint_digest: String,
    pub last_recovery_digest: Option<String>,
    pub pending_recovery_digest: Option<String>,
    pub requires_v2_checkpoint: bool,
}

/// A content-addressable, independently replayable page. A chain of pages is
/// always replayed from the fixed external origin; a local page hash is never
/// accepted as a replacement trust anchor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormalReadHistoryPageV1 {
    pub schema_version: u32,
    pub start: FormalReadReplayCursorV1,
    pub history: FormalReadHistoryV1,
    pub end: FormalReadReplayCursorV1,
}

impl FormalReadHistoryPageV1 {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1
            || self.history.schema_version != 1
            || self.history.steps.is_empty()
            || self.history.steps.len() > MAX_FORMAL_HISTORY_STEPS
            || self.start == self.end
            || !valid_cursor(&self.start)
            || !valid_cursor(&self.end)
        {
            return Err("formal history page schema/cursor/step bound".into());
        }
        let bytes = serde_cbor::to_vec(&self.history).map_err(|error| error.to_string())?;
        if bytes.len() > MAX_FORMAL_PROOF_BYTES {
            return Err("formal history page byte bound".into());
        }
        Ok(())
    }

    pub fn encode_bytes(&self) -> Result<Vec<u8>, String> {
        self.validate()?;
        let bytes = serde_cbor::to_vec(self).map_err(|error| error.to_string())?;
        if bytes.len() > MAX_FORMAL_PROOF_BYTES {
            return Err("formal history page byte bound".into());
        }
        Ok(bytes)
    }

    pub fn decode_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.is_empty() || bytes.len() > MAX_FORMAL_PROOF_BYTES {
            return Err("formal history page byte bound".into());
        }
        let page: Self = serde_cbor::from_slice(bytes).map_err(|error| error.to_string())?;
        page.validate()?;
        if serde_cbor::to_vec(&page).map_err(|error| error.to_string())? != bytes {
            return Err("formal history page is not canonical CBOR".into());
        }
        Ok(page)
    }
}

fn valid_cursor(cursor: &FormalReadReplayCursorV1) -> bool {
    cursor.schema_version == 1
        && cursor.anchor.position > 0
        && valid_hex_digest(&cursor.anchor.record_id)
        && valid_hex_digest(&cursor.anchor.active_authority_id)
        && valid_hex_digest(&cursor.checkpoint_digest)
        && cursor
            .last_recovery_digest
            .as_deref()
            .is_none_or(valid_hex_digest)
        && cursor
            .pending_recovery_digest
            .as_deref()
            .is_none_or(valid_hex_digest)
}
fn bounded_steps<'de, D: serde::Deserializer<'de>>(
    decoder: D,
) -> Result<Vec<FormalReadStepV1>, D::Error> {
    struct Visitor;
    impl<'de> serde::de::Visitor<'de> for Visitor {
        type Value = Vec<FormalReadStepV1>;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("bounded formal history")
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(
            self,
            mut sequence: A,
        ) -> Result<Self::Value, A::Error> {
            if sequence
                .size_hint()
                .is_some_and(|count| count > MAX_FORMAL_HISTORY_STEPS)
            {
                return Err(serde::de::Error::custom("formal history step bound"));
            }
            let mut steps = Vec::new();
            while let Some(step) = sequence.next_element()? {
                if steps.len() == MAX_FORMAL_HISTORY_STEPS {
                    return Err(serde::de::Error::custom("formal history step bound"));
                }
                steps.push(step);
            }
            Ok(steps)
        }
    }
    decoder.deserialize_seq(Visitor)
}
impl FormalReadHistoryV1 {
    pub fn encode_hex(&self) -> Result<String, String> {
        if self.schema_version != 1 || self.steps.len() > MAX_FORMAL_HISTORY_STEPS {
            return Err("formal history version/step bound".into());
        }
        let bytes = serde_cbor::to_vec(self).map_err(|error| error.to_string())?;
        if bytes.len() > MAX_FORMAL_PROOF_BYTES {
            return Err("formal history byte bound".into());
        }
        Ok(hex::encode(bytes))
    }
    pub fn decode_hex(value: &str) -> Result<Self, String> {
        if value.len() > 2 * MAX_FORMAL_PROOF_BYTES
            || !value.len().is_multiple_of(2)
            || !value
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err("formal history byte/hex bound".into());
        }
        let bytes = hex::decode(value).map_err(|error| error.to_string())?;
        let history: Self = serde_cbor::from_slice(&bytes).map_err(|error| error.to_string())?;
        if history.schema_version != 1 {
            return Err("formal history schema".into());
        }
        Ok(history)
    }
}

/// A versioned, externally pinned replay checkpoint. The origin keeps the
/// strict V1 pin contract; the bounded signed history derives the active V2
/// state and the final checkpoint is only a consistency commitment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormalReadOriginQualificationV2 {
    pub base: FormalReadCheckpointV1,
    pub base_digest: String,
    pub history: FormalReadHistoryV1,
    pub origin_record: CanonicalRecordV1,
    pub finalizing_authority_id: String,
    pub qualified_proof: FormalQualifiedCommitProofV1,
    pub qualified_proof_digest: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormalReadCheckpointV2 {
    pub schema_version: u32,
    pub origin: FormalReadCheckpointV1,
    pub origin_digest: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin_qualification: Option<FormalReadOriginQualificationV2>,
    pub history: FormalReadHistoryV1,
    pub current: FormalReadCheckpointV1,
}
impl FormalReadCheckpointV2 {
    fn validate_shape(&self) -> Result<(), String> {
        if self.schema_version != 2
            || self.history.schema_version != 1
            || self.history.steps.len() > MAX_FORMAL_HISTORY_STEPS
        {
            return Err("formal V2 checkpoint schema/history bound".into());
        }
        if self.origin.digest().map_err(|error| error.to_string())? != self.origin_digest {
            return Err("formal V2 checkpoint origin digest mismatch".into());
        }
        if let Some(qualification) = &self.origin_qualification
            && (qualification.history.schema_version != 1
                || qualification.history.steps.len() > MAX_FORMAL_HISTORY_STEPS
                || qualification
                    .base
                    .digest()
                    .map_err(|error| error.to_string())?
                    != qualification.base_digest
                || qualification.origin_record != self.origin.head_record
                || qualification.finalizing_authority_id
                    != self.origin.head_finalizing_authority_id
                || qualification.origin_record.validate().is_err()
                || !valid_hex_digest(&qualification.qualified_proof_digest)
                || !valid_hex_digest(&qualification.finalizing_authority_id)
                || qualification.qualified_proof.decision.record != qualification.origin_record
                || formal_qualified_proof_digest(&qualification.qualified_proof)
                    .map_err(|error| error.to_string())?
                    != qualification.qualified_proof_digest)
        {
            return Err("formal V2 origin qualification binding/bound".into());
        }
        Ok(())
    }
    pub fn digest(&self) -> Result<String, String> {
        self.validate_shape()?;
        let mut bytes = b"oasis7.formal.read_checkpoint.v2\0".to_vec();
        bytes.extend(serde_cbor::to_vec(self).map_err(|error| error.to_string())?);
        Ok(blake3::hash(&bytes).to_hex().to_string())
    }
    pub fn encode_hex(&self) -> Result<String, String> {
        self.validate_shape()?;
        let bytes = serde_cbor::to_vec(self).map_err(|error| error.to_string())?;
        if bytes.len() > MAX_FORMAL_PROOF_BYTES {
            return Err("formal V2 checkpoint byte bound".into());
        }
        Ok(hex::encode(bytes))
    }
    pub fn decode_hex(value: &str) -> Result<Self, String> {
        if value.len() > 2 * MAX_FORMAL_PROOF_BYTES
            || !value.len().is_multiple_of(2)
            || !value
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err("formal V2 checkpoint byte/hex bound".into());
        }
        let bytes = hex::decode(value).map_err(|error| error.to_string())?;
        let checkpoint: Self = serde_cbor::from_slice(&bytes).map_err(|error| error.to_string())?;
        checkpoint.validate_shape()?;
        Ok(checkpoint)
    }
}
fn valid_hex_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub(super) struct VerifiedReadRecoveryStep {
    pub(super) recovery: VerifiedRecovery,
    pub(super) retained_context: Option<VerifiedActiveRecoveryContextV2>,
}

pub(super) fn verify_recovery_step(
    step: &FormalRecoveryReadStepV1,
    anchor: &VerifiedFormalReadAnchorV1,
    previous: Option<&VerifiedRecovery>,
) -> Result<VerifiedReadRecoveryStep, String> {
    let fencing_policy = TrustedFencingPolicyV1 {
        provider_public_key: anchor
            .authority()
            .policy()
            .fencing_provider_public_key
            .clone(),
        authorized_resource_bindings: anchor
            .authority()
            .policy()
            .authorized_fencing_resource_bindings
            .clone(),
    };
    let body = step.certificate.body.clone();
    let context = match body.schema_version {
        1 => {
            if previous.is_some() {
                return Err(
                    "schema-1 recovery cannot replace a retained same-cut permission".into(),
                );
            }
            None
        }
        2 => {
            let previous = previous.ok_or_else(|| {
                "HistoryUnavailable: retained recovery requires prior verified permission"
                    .to_owned()
            })?;
            let original = if previous.body().tail == RecoveryTailV1::Empty
                && !matches!(&step.original_tail, RecoveryTailWitnessV1::Empty)
            {
                previous.successor()
            } else {
                previous.original_authority()
            };
            let context = VerifiedActiveRecoveryContextV2::from_previous(
                previous,
                original,
                anchor.head(),
                &step.original_tail,
            )
            .map_err(|error| error.to_string())?;
            Some(context)
        }
        _ => return Err("formal recovery certificate schema".into()),
    };
    let authority = match context.as_ref() {
        Some(context) => context.active(),
        None => anchor.authority(),
    };
    let policy = authority.policy();
    let intent = match (&step.fencing, context.as_ref()) {
        (FormalReadFencingEvidenceV1::HealthyPair(seals), None) => {
            let primary =
                verify_epoch_seal(&seals[0], &fencing_policy).map_err(|error| error.to_string())?;
            let replica =
                verify_epoch_seal(&seals[1], &fencing_policy).map_err(|error| error.to_string())?;
            VerifiedRecoveryIntent::verify(
                body,
                authority,
                anchor.head(),
                [&primary, &replica],
                &step.original_tail,
            )
        }
        (FormalReadFencingEvidenceV1::NecessaryEndpoint(seal), None) => {
            let seal =
                verify_epoch_seal(seal, &fencing_policy).map_err(|error| error.to_string())?;
            let coverage = VerifiedAppendFencingCoverageV1::derive_from_necessary_endpoint_seal(
                authority,
                anchor.head(),
                &seal,
            )
            .map_err(|error| error.to_string())?;
            VerifiedRecoveryIntent::verify_single_survivor(
                body,
                authority,
                anchor.head(),
                &coverage,
                &step.original_tail,
            )
        }
        (FormalReadFencingEvidenceV1::HealthyPair(seals), Some(context)) => {
            let primary =
                verify_epoch_seal(&seals[0], &fencing_policy).map_err(|error| error.to_string())?;
            let replica =
                verify_epoch_seal(&seals[1], &fencing_policy).map_err(|error| error.to_string())?;
            VerifiedRecoveryIntent::verify_retained(body, context, [&primary, &replica])
        }
        (FormalReadFencingEvidenceV1::NecessaryEndpoint(seal), Some(context)) => {
            let seal =
                verify_epoch_seal(seal, &fencing_policy).map_err(|error| error.to_string())?;
            let coverage = VerifiedAppendFencingCoverageV1::derive_from_active_recovery_context(
                context, &seal,
            )
            .map_err(|error| error.to_string())?;
            VerifiedRecoveryIntent::verify_retained_single_survivor(body, context, &coverage)
        }
    }
    .map_err(|error| error.to_string())?;
    let issuance = issuer::verify_issuance_witness(&step.issuance, &step.certificate, policy)
        .map_err(|error| error.to_string())?;
    let recovery = VerifiedRecovery::verify(&step.certificate, &intent, authority, &issuance)
        .map_err(|error| error.to_string())?;
    Ok(VerifiedReadRecoveryStep {
        recovery,
        retained_context: context,
    })
}
