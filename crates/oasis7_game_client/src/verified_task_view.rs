//! Validation of the explicitly normalized JSON projection profile. The exact
//! original JSON payload is authenticated before these read-only checks run.
use crate::WorldServiceClientError as Error;
use oasis7_agent_api::{FeedbackEnvelopeV1, RuntimeBindingV1, RuntimeReceiptLineageV1};
use oasis7_agent_api::{
    game::{GameTaskControlV1, GameTaskFenceV1},
    world_context::WorldServiceAgentContext,
};
use oasis7_client_api::world_service::*;
use serde::Deserialize;
use serde_json::Value;

/// Service-authenticated scoped observation; not a World, task admission token,
/// durable receipt proof, or permission to execute a local world action.
#[derive(Debug, Clone)]
pub struct VerifiedTaskWorldView {
    response: ReadWorldViewResponse<Value>,
    projection: TaskWorldProjection,
    task_fence: Option<GameTaskFenceV1>,
}
/// Typed client read model decoded only after authentication of the complete
/// normalized JSON response. This value cannot be submitted as a World.
#[derive(Debug, Clone)]
pub struct TaskWorldProjection {
    pub state: Value,
    pub native_delegation_authorizations:
        Vec<oasis7_agent_api::world_quote::AgentDelegationAuthorizationV1>,
    pub game_task_control: Option<GameTaskControlV1>,
    pub agent_context: Option<WorldServiceAgentContext>,
    pub runtime_binding: Option<RuntimeBindingV1>,
}
#[derive(Deserialize)]
struct ScopedAgent {
    agent_id: String,
}
#[derive(Deserialize)]
struct History {
    agent_id: String,
    records: Vec<ReplayRecord>,
}
#[derive(Deserialize)]
struct ReplayRecord {
    feedback: FeedbackEnvelopeV1,
    original_envelope_digest: String,
    delivery_state: String,
    receipt_lineage: Option<RuntimeReceiptLineageV1>,
}
#[derive(Deserialize)]
struct OwnerScope {
    agent_id: String,
    world_id: String,
    reorg_epoch: u64,
    canonical_authority: OwnerAuthority,
}
#[derive(Deserialize)]
struct OwnerAuthority {
    branch_id: String,
    agent_identity_generation: u64,
}
/// Only validation inputs are decoded here. The signed original retains every
/// legacy field and its JSON numeric representation without reserialization.
#[derive(Deserialize)]
struct ProjectionChecks {
    state: Value,
    runtime_binding: Option<RuntimeBindingV1>,
    agent_context: Option<WorldServiceAgentContext>,
    game_task_control: Option<GameTaskControlV1>,
    canonical_agent_owner: Option<OwnerScope>,
    canonical_agent_chat: Option<OwnerScope>,
    #[serde(default)]
    native_delegation_authorizations:
        Vec<oasis7_agent_api::world_quote::AgentDelegationAuthorizationV1>,
    feedback_history: Option<History>,
    scheduler_wakes: Vec<ScopedAgent>,
    continuations: Vec<ScopedAgent>,
    cognition_leases: Vec<ScopedAgent>,
    continuation_contexts: std::collections::BTreeMap<String, Value>,
    events: Vec<Value>,
}
fn invalid(message: impl ToString) -> Error {
    Error::Assurance(message.to_string())
}
impl VerifiedTaskWorldView {
    pub(crate) fn new(
        response: ReadWorldViewResponse<Value>,
        request: &ReadWorldViewRequest,
    ) -> Result<Self, Error> {
        if response.execution_evidence_scope.is_some() {
            return Err(invalid("task client requires ordinary service evidence"));
        }
        response.validate(request).map_err(invalid)?;
        let checks: ProjectionChecks =
            serde_json::from_value(response.view.clone()).map_err(invalid)?;
        if checks.state.get("time").and_then(Value::as_u64) != Some(response.logical_tick) {
            return Err(invalid(
                "projection tick differs from declared logical tick",
            ));
        }
        if let Some(binding) = &checks.runtime_binding {
            binding.validate().map_err(invalid)?;
            let expected = &response.version.commit.binding;
            if binding.world_id != expected.provider_world_id
                || binding.branch_id != expected.branch_id
                || binding.reorg_epoch != expected.reorg_generation
                || binding.base_tick != response.logical_tick
                || binding.runtime_manifest_hash.to_string() != expected.governing_manifest_ref
                || request_digest(
                    "finality",
                    &(
                        binding.finality_epoch,
                        &binding.finality_status,
                        &binding.finality_block_hash,
                    ),
                )
                .map_err(invalid)?
                    != expected.finality_ref
            {
                return Err(invalid(
                    "projection runtime binding differs from committed view",
                ));
            }
        }
        let agent = request.scope_id.strip_prefix("agent:");
        let scoped = |id: &str| Some(id) == agent;
        if checks
            .agent_context
            .as_ref()
            .is_some_and(|v| !scoped(&v.agent_id))
            || checks.scheduler_wakes.iter().any(|v| !scoped(&v.agent_id))
            || checks.continuations.iter().any(|v| !scoped(&v.agent_id))
            || checks.cognition_leases.iter().any(|v| !scoped(&v.agent_id))
            || checks
                .native_delegation_authorizations
                .iter()
                .any(|v| !scoped(&v.grant.agent_id))
        {
            return Err(invalid(
                "Agent observation differs from authorized visibility scope",
            ));
        }
        for owner in [
            checks.canonical_agent_owner.as_ref(),
            checks.canonical_agent_chat.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            if !scoped(&owner.agent_id)
                || owner.world_id != request.world.world_id
                || owner.reorg_epoch != response.version.commit.binding.reorg_generation
                || owner.canonical_authority.branch_id != response.version.commit.binding.branch_id
                || owner.canonical_authority.agent_identity_generation == 0
            {
                return Err(invalid(
                    "owner observation differs from authorized visibility scope",
                ));
            }
        }
        if let Some(history) = &checks.feedback_history {
            if !scoped(&history.agent_id) || history.records.len() > 16384 {
                return Err(invalid("scoped feedback history audience or bound invalid"));
            }
            for record in &history.records {
                record.feedback.validate().map_err(invalid)?;
                if !scoped(&record.feedback.agent_subject)
                    || record.original_envelope_digest.is_empty()
                    || record.delivery_state.is_empty()
                {
                    return Err(invalid("scoped feedback history metadata invalid"));
                }
                if let Some(lineage) = &record.receipt_lineage {
                    lineage.validate().map_err(invalid)?;
                    if lineage.agent_id != record.feedback.agent_subject
                        || lineage.feedback_id != record.feedback.feedback_id
                        || lineage.agent_session_id != record.feedback.agent_session_id
                        || lineage.agent_turn_id != record.feedback.agent_turn_id
                        || lineage.decision_request_id != record.feedback.decision_request_id
                        || lineage.request_digest != record.feedback.request_digest.to_string()
                        || Some(lineage.receipt_id.as_str())
                            != record.feedback.runtime_receipt_id.as_deref()
                        || record
                            .feedback
                            .candidate_action_id
                            .is_none_or(|id| lineage.action_id != format!("action:{id}"))
                        || record.feedback.status != "committed"
                    {
                        return Err(invalid("scoped feedback receipt lineage mismatch"));
                    }
                } else if record.feedback.status == "committed" {
                    return Err(invalid("committed feedback receipt missing"));
                }
            }
        }
        let task_fence = checks
            .game_task_control
            .as_ref()
            .map(GameTaskFenceV1::from_control);
        if let Some(fence) = &task_fence {
            fence.validate().map_err(invalid)?;
            if !scoped(&fence.agent_id)
                || fence.world_id != request.world.world_id
                || fence.genesis_digest != request.world.genesis_digest
                || fence.branch_id != response.version.commit.binding.branch_id
                || fence.reorg_epoch.0 != response.version.commit.binding.reorg_generation
            {
                return Err(invalid("task fence differs from authorized world or scope"));
            }
        }
        // Retained in the original signed projection, never interpreted as
        // capabilities or independent proof by this client.
        let _ = (&checks.events, &checks.continuation_contexts);
        Ok(Self {
            response,
            projection: TaskWorldProjection {
                state: checks.state,
                native_delegation_authorizations: checks.native_delegation_authorizations,
                game_task_control: checks.game_task_control,
                agent_context: checks.agent_context,
                runtime_binding: checks.runtime_binding,
            },
            task_fence,
        })
    }
    pub fn projection(&self) -> &TaskWorldProjection {
        &self.projection
    }
    pub fn raw_projection(&self) -> &Value {
        &self.response.view
    }
    pub fn version(&self) -> &ProjectionVersion {
        &self.response.version
    }
    pub fn continuation(&self) -> &EventCursor {
        &self.response.continuation
    }
    pub fn logical_tick(&self) -> u64 {
        self.response.logical_tick
    }
    pub fn agent_context(&self) -> Option<&WorldServiceAgentContext> {
        self.projection.agent_context.as_ref()
    }
    pub fn task_fence(&self) -> Option<&GameTaskFenceV1> {
        self.task_fence.as_ref()
    }
}

#[cfg(test)]
#[path = "verified_task_view_tests.rs"]
mod tests;
