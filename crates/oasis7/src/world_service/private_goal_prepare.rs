//! Private proof codec only: no HTTP, storage, public intent or activation.
use super::{sign_read_request, signing_bytes};
use ed25519_dalek::{Signature, VerifyingKey};
use oasis7_agent_api::game::{GAME_GOAL_PREPARE_DOMAIN, GameGoalPrepareV1, GameTaskControlV1};
use serde::{Deserialize, Serialize};

/// Kept strictly outside WorldServicePayloadV1. Debug is absent so body and
/// enumerable signature material cannot accidentally enter ordinary logs.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedGoalPrepareV1 {
    pub request: GameGoalPrepareV1,
    pub subject_public_key: String,
    pub signature_hex: String,
}

pub fn sign_goal_prepare(
    request: GameGoalPrepareV1,
    private_key_hex: &str,
) -> Result<SignedGoalPrepareV1, String> {
    request.validate().map_err(str::to_owned)?;
    let signed = sign_read_request(GAME_GOAL_PREPARE_DOMAIN, request, private_key_hex)?;
    if signed.subject_public_key != signed.request.owner_public_key {
        return Err("private goal signer differs from declared owner".into());
    }
    Ok(SignedGoalPrepareV1 {
        request: signed.request,
        subject_public_key: signed.subject_public_key,
        signature_hex: signed.signature_hex,
    })
}

impl SignedGoalPrepareV1 {
    /// Proves only the declared signer and exact private proposal. A valid
    /// signature alone never establishes current World owner authority.
    pub fn verify(&self) -> Result<(), String> {
        self.request.validate().map_err(str::to_owned)?;
        if self.subject_public_key != self.request.owner_public_key
            || self.signature_hex.len() != 128
            || !self
                .signature_hex
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err("invalid private goal signature envelope".into());
        }
        verify_owner_signature_strict(
            GAME_GOAL_PREPARE_DOMAIN,
            &self.request,
            &self.subject_public_key,
            &self.signature_hex,
        )
    }

    /// Caller must obtain this control from freshly authenticated canonical
    /// World authority and recheck it at admission/delivery. This pure proof
    /// comparison does not perform those checks or prepare/activate a goal.
    pub fn validate_against(&self, current: &GameTaskControlV1) -> Result<(), String> {
        self.verify()?;
        let request = &self.request;
        if current.schema_revision != 1
            || request.world_id != current.world_id
            || request.genesis_digest != current.genesis_digest
            || request.task_id != current.task_id
            || request.agent_id != current.agent_id
            || request.owner_binding != current.owner_binding
            || request.owner_public_key != current.owner_public_key
            || request.owner_generation != current.owner_generation
            || request.branch_id != current.branch_id
            || request.reorg_epoch != current.reorg_epoch
            || request.expected_task_revision != current.task_revision
            || request.expected_latest_goal_revision != current.latest_goal_revision
        {
            return Err("private goal differs from trusted current control".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;

/// Narrow shared cryptographic boundary for private goal proofs and public owner
/// goal publications. This does not change generic World Service verification.
pub(crate) fn verify_owner_signature_strict<T: Serialize>(
    domain: &str,
    request: &T,
    subject_public_key: &str,
    signature_hex: &str,
) -> Result<(), String> {
    let canonical = |value: &str, length: usize| {
        value.len() == length
            && value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    };
    if !canonical(subject_public_key, 64) || !canonical(signature_hex, 128) {
        return Err("invalid owner signature encoding".into());
    }
    let public_key: [u8; 32] = hex::decode(subject_public_key)
        .map_err(|_| "invalid private goal owner key")?
        .try_into()
        .map_err(|_| "invalid private goal owner key")?;
    let key =
        VerifyingKey::from_bytes(&public_key).map_err(|_| "invalid private goal owner key")?;
    if key.is_weak() {
        return Err("weak private goal owner key".into());
    }
    let signature: [u8; 64] = hex::decode(signature_hex)
        .map_err(|_| "invalid private goal signature")?
        .try_into()
        .map_err(|_| "invalid private goal signature")?;
    // This private owner boundary requires strict Ed25519 verification,
    // including canonical scalar/point encodings and non-small-order R.
    key.verify_strict(
        &signing_bytes(domain, request)?,
        &Signature::from_bytes(&signature),
    )
    .map_err(|_| "invalid private goal owner signature".into())
}
