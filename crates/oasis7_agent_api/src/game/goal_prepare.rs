//! Private goal preparation proposal only. Never a public intent payload.
use super::{API_REVISION, DecimalU64, GoalContentV1, game_goal_version_digest};
use crate::Digest32;
use serde::{Deserialize, Serialize};

pub const GAME_GOAL_PREPARE_DOMAIN: &str = "game_goal_prepare_private/v1";

/// Complete owner proposal. Neither content, digest nor its eventual signature
/// belongs in public world streams. Codec validation grants no authority.
/// Debug is deliberately absent to prevent accidental private-body logging.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameGoalPrepareV1 {
    pub api_rev: u32,
    pub world_id: String,
    pub genesis_digest: String,
    pub task_id: String,
    pub agent_id: String,
    pub owner_public_key: String,
    pub owner_binding: String,
    pub owner_generation: DecimalU64,
    pub branch_id: String,
    pub reorg_epoch: DecimalU64,
    pub expected_task_revision: DecimalU64,
    pub expected_latest_goal_revision: DecimalU64,
    pub goal_revision: DecimalU64,
    pub client_operation_id: String,
    /// Client-generated 32 random bytes, encoded as lowercase hex. The codec
    /// checks representation, not entropy. Retries retain the same nonce.
    pub client_nonce: String,
    /// Recomputed private fingerprint, never canonical activation authority.
    pub goal_digest: Digest32,
    pub content: GoalContentV1,
}

impl GameGoalPrepareV1 {
    pub fn validate(&self) -> Result<(), &'static str> {
        let valid_identity = |value: &str| !value.trim().is_empty() && value.len() <= 256;
        let hex32 = |value: &str| {
            value.len() == 64
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        };
        if self.api_rev != API_REVISION
            || self.owner_generation.0 == 0
            || self.expected_task_revision.0 == 0
            || self.expected_latest_goal_revision.0.checked_add(1) != Some(self.goal_revision.0)
            || [
                &self.world_id,
                &self.genesis_digest,
                &self.task_id,
                &self.agent_id,
                &self.owner_binding,
                &self.branch_id,
                &self.client_operation_id,
            ]
            .iter()
            .any(|value| !valid_identity(value))
            || !hex32(&self.owner_public_key)
            || !hex32(&self.client_nonce)
        {
            return Err("invalid private goal prepare identity or revision");
        }
        if self.goal_digest
            != game_goal_version_digest(&self.task_id, self.goal_revision, &self.content)?
        {
            return Err("private goal prepare digest differs from complete content");
        }
        Ok(())
    }
}
