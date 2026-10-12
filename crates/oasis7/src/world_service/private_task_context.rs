//! Full private task-lane input, not a complete collaboration TaskSnapshot.
use super::private_goal_prepare::verify_owner_signature_strict;
use super::private_goal_store::PreparedGoalReceiptV1;
use super::projection::WorldServiceAgentContext;
use super::*;
use crate::runtime::RuntimeCognitionBaseBindingV1;
use oasis7_agent_api::game::{
    DecimalU64, GameTaskControlV1, GameTaskFenceV1, TaskContextIdentityV1,
};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;
pub const TASK_CONTEXT_PREPARE_PATH: &str = "/v1/world/private-task/context-prepare";
pub const TASK_CONTEXT_READ_PATH: &str = "/v1/world/private-task/context-read";
pub const TASK_SNAPSHOT_PATH: &str = "/v1/world/private-task/goal-snapshot";
pub const MAX_PRIVATE_CONTEXT_BYTES: usize = 2 * 1024 * 1024;
/// Maximum lifetime of one captured decision context, within the actual grant expiry.
pub const TASK_CONTEXT_LEASE_TICKS: u64 = 16;
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskContextPrepareV1 {
    pub schema_revision: u32,
    pub fence: GameTaskFenceV1,
    pub client_operation_id: String,
    pub client_nonce: String,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskContextGetV1 {
    pub schema_revision: u32,
    pub context: TaskContextIdentityV1,
    pub prepare_operation_id: String,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskGoalSnapshotRequestV1 {
    pub schema_revision: u32,
    pub fence: GameTaskFenceV1,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GoalExecutionSnapshotV1 {
    pub schema_revision: u32,
    pub control: GameTaskControlV1,
    pub read_commit: CommitRef,
    pub activation_commit: CommitRef,
    pub active_goal: PreparedGoalReceiptV1,
    pub actor_observation_schema: String,
    pub actor_observation: serde_json::Value,
    pub provider_authority: WorldServiceAgentContext,
    pub runtime_binding: oasis7_agent_api::RuntimeBindingV1,
    pub captured_base_binding: RuntimeCognitionBaseBindingV1,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedTaskContextV1 {
    pub schema_revision: u32,
    pub prepare_proof: SignedReadRequest<TaskContextPrepareV1>,
    pub snapshot: GoalExecutionSnapshotV1,
    pub identity: TaskContextIdentityV1,
    pub reference_salt: String,
    pub valid_until_tick: DecimalU64,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapturedTaskContextV1 {
    pub schema_revision: u32,
    pub prepared: PreparedTaskContextV1,
    pub capture_commit: CommitRef,
    pub capture_proof: CanonicalIntentResultV1,
    pub current_public_context: TaskPublicContextV1,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskPublicContextV1 {
    pub read_commit: CommitRef,
    pub captured_base_binding: RuntimeCognitionBaseBindingV1,
    pub provider_authority: WorldServiceAgentContext,
    pub actor_observation: serde_json::Value,
}
impl TaskPublicContextV1 {
    pub fn validate_for(&self, identity: &TaskContextIdentityV1) -> Result<(), String> {
        self.read_commit.validate().map_err(|e| e.to_string())?;
        self.captured_base_binding
            .validate()
            .map_err(|e| e.to_string())?;
        let f = &identity.fence;
        let b = &self.captured_base_binding;
        let authority = &self.provider_authority;
        let catalog = &authority.capability_catalog;
        let invocation = &authority.capability_invocation_context;
        catalog
            .validate()
            .map_err(|_| "invalid current task catalog")?;
        if self.read_commit.world.world_id != f.world_id
            || self.read_commit.world.genesis_digest != f.genesis_digest
            || self.read_commit.binding.branch_id != f.branch_id
            || self.read_commit.binding.reorg_generation != f.reorg_epoch.0
            || b.world_id != f.world_id
            || b.branch_id != f.branch_id
            || b.reorg_epoch != f.reorg_epoch.0
            || b.base_tick > identity.valid_until_tick.0
            || authority.agent_id != f.agent_id
            || !self.actor_observation.is_object()
            || catalog.world_id != f.world_id
            || catalog.branch_id != f.branch_id
            || catalog.logical_tick != b.base_tick
            || !matches!(&catalog.subject,oasis7_wasm_abi::CapabilitySubject::Agent{agent_id,owner_binding,generation} if agent_id==&f.agent_id&&owner_binding==&f.owner_binding&&*generation==f.owner_generation.0)
            || invocation.catalog_snapshot_id != catalog.snapshot_id
            || invocation.subject != catalog.subject
            || invocation.presenter != catalog.presenter
            || invocation.audience != catalog.audience
        {
            return Err("current task public context differs from task fence".into());
        }
        Ok(())
    }
}
impl CapturedTaskContextV1 {
    pub fn validate_against(
        &self,
        request: &SignedReadRequest<TaskContextGetV1>,
    ) -> Result<(), String> {
        verify_get(request)?;
        self.prepared.validate()?;
        self.capture_commit.validate().map_err(|e| e.to_string())?;
        let key = correlation::key_digest(&self.capture_proof.request.correlation.key)?;
        correlation::validate_result(
            &key,
            &serde_json::to_value(&self.capture_proof)
                .map_err(|_| "invalid context capture proof")?,
        )?;
        let WorldServicePayloadV1::Scheduler(prefix) = &self.capture_proof.request.signed_payload
        else {
            return Err("context capture proof is not a scheduler prefix".into());
        };
        let SchedulerOperationV1::TaskProviderPrefix { context } = &prefix.request.operation else {
            return Err("context capture proof is not task lane".into());
        };
        if self.schema_revision != 1
            || self.prepared.identity != request.request.context
            || self.prepared.prepare_proof.request.client_operation_id
                != request.request.prepare_operation_id
            || self.prepared.prepare_proof.subject_public_key != request.subject_public_key
            || context != &self.prepared.identity
            || self.capture_proof.rejected.is_some()
            || prefix.subject_public_key != request.subject_public_key
            || self.capture_commit.world != self.prepared.snapshot.read_commit.world
            || self.capture_commit.position != self.capture_proof.committed_height
            || self.capture_commit.binding.branch_id != context.fence.branch_id
            || self.capture_commit.binding.reorg_generation != context.fence.reorg_epoch.0
        {
            return Err("private context capture mismatch".into());
        }
        verify_owner_signature_strict(
            "scheduler",
            &prefix.request,
            &prefix.subject_public_key,
            &prefix.signature_hex,
        )?;
        if prefix.request != public_prefix(&self.prepared)? {
            return Err("context capture differs from prepared prefix".into());
        }
        let current = &self.current_public_context;
        current.validate_for(&self.prepared.identity)?;
        current.read_commit.validate().map_err(|e| e.to_string())?;
        if current.read_commit.world != self.capture_commit.world
            || current.read_commit.position < self.capture_commit.position
            || current.read_commit.binding.branch_id != context.fence.branch_id
            || current.read_commit.binding.reorg_generation != context.fence.reorg_epoch.0
            || current.provider_authority.agent_id != context.fence.agent_id
            || !current.actor_observation.is_object()
        {
            return Err("current task public context mismatch".into());
        }
        current
            .provider_authority
            .capability_catalog
            .validate()
            .map_err(|_| "invalid current capability catalog")?;
        Ok(())
    }
}
fn identity(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 256
}
fn salt(value: &str) -> Result<[u8; 32], String> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err("invalid private context salt".into());
    }
    hex::decode(value)
        .map_err(|_| "invalid private context salt".to_owned())?
        .try_into()
        .map_err(|_| "invalid private context salt".into())
}
pub fn verify_prepare(request: &SignedReadRequest<TaskContextPrepareV1>) -> Result<(), String> {
    let r = &request.request;
    r.fence.validate()?;
    if r.schema_revision != 1 || !identity(&r.client_operation_id) || salt(&r.client_nonce).is_err()
    {
        return Err("invalid task context preparation".into());
    }
    let binding = r
        .fence
        .executor_binding
        .as_ref()
        .ok_or("task executor missing")?;
    if binding.executor_public_key != request.subject_public_key {
        return Err("task context signer differs from executor".into());
    }
    verify_owner_signature_strict(
        TASK_CONTEXT_PREPARE_PATH,
        r,
        &request.subject_public_key,
        &request.signature_hex,
    )
}
pub fn verify_get(request: &SignedReadRequest<TaskContextGetV1>) -> Result<(), String> {
    let r = &request.request;
    r.context.validate()?;
    if r.schema_revision != 1
        || !identity(&r.prepare_operation_id)
        || r.context
            .fence
            .executor_binding
            .as_ref()
            .is_none_or(|b| b.executor_public_key != request.subject_public_key)
    {
        return Err("invalid task context read".into());
    }
    verify_owner_signature_strict(
        TASK_CONTEXT_READ_PATH,
        r,
        &request.subject_public_key,
        &request.signature_hex,
    )
}
pub fn verify_snapshot_request(
    request: &SignedReadRequest<TaskGoalSnapshotRequestV1>,
) -> Result<(), String> {
    request.request.fence.validate()?;
    if request.request.schema_revision != 1 {
        return Err("invalid goal execution snapshot request".into());
    }
    verify_owner_signature_strict(
        TASK_SNAPSHOT_PATH,
        &request.request,
        &request.subject_public_key,
        &request.signature_hex,
    )
}
impl GoalExecutionSnapshotV1 {
    pub fn validate(&self) -> Result<(), String> {
        self.runtime_binding.validate().map_err(|e| e.to_string())?;
        self.captured_base_binding
            .validate()
            .map_err(|e| e.to_string())?;
        let c = &self.control;
        let f = GameTaskFenceV1::from_control(c);
        f.validate()?;
        self.read_commit.validate().map_err(|e| e.to_string())?;
        self.activation_commit
            .validate()
            .map_err(|e| e.to_string())?;
        self.active_goal.validate().map_err(|e| e.to_string())?;
        let reference = c
            .active_goal_reference
            .as_ref()
            .ok_or("active task goal unavailable")?;
        let publish = super::private_goal_read::publication(
            &self.active_goal.owner_proof,
            &self.active_goal,
            reference.publish_operation_id.clone(),
        )?;
        let proof = &self.active_goal.owner_proof.request;
        let binding = &self.runtime_binding;
        if self.schema_revision != 1
            || self.actor_observation_schema != "world-agent-state/v1"
            || !self.actor_observation.is_object()
            || self.read_commit.world.world_id != c.world_id
            || self.read_commit.world.genesis_digest != c.genesis_digest
            || self.read_commit.binding.branch_id != c.branch_id
            || self.read_commit.binding.reorg_generation != c.reorg_epoch.0
            || self.activation_commit.world != self.read_commit.world
            || self.activation_commit.binding.branch_id != c.branch_id
            || self.activation_commit.binding.reorg_generation != c.reorg_epoch.0
            || self.activation_commit.position != reference.activation_committed_height.0
            || self.activation_commit.position > self.read_commit.position
            || reference.goal_ref != publish.goal_ref
            || reference.prepare_operation_id != publish.prepare_operation_id
            || reference.goal_revision != proof.goal_revision
            || c.active_goal_revision != proof.goal_revision
            || proof.task_id != c.task_id
            || proof.agent_id != c.agent_id
            || proof.world_id != c.world_id
            || proof.genesis_digest != c.genesis_digest
            || proof.owner_public_key != c.owner_public_key
            || proof.owner_binding != c.owner_binding
            || proof.owner_generation != c.owner_generation
            || proof.branch_id != c.branch_id
            || proof.reorg_epoch != c.reorg_epoch
            || proof
                .expected_task_revision
                .0
                .checked_add(1)
                .is_none_or(|v| v > c.task_revision.0)
            || binding.world_id != c.world_id
            || binding.branch_id != c.branch_id
            || binding.reorg_epoch != c.reorg_epoch.0
            || self.captured_base_binding.world_id != c.world_id
            || self.captured_base_binding.branch_id != c.branch_id
            || self.captured_base_binding.reorg_epoch != c.reorg_epoch.0
            || self.captured_base_binding.base_tick != binding.base_tick
            || self.captured_base_binding.base_world_hash != binding.base_world_hash.0
            || self.provider_authority.agent_id != c.agent_id
        {
            return Err("goal execution snapshot binding mismatch".into());
        }
        self.active_goal
            .validate_publish(&publish)
            .map_err(|e| e.to_string())?;
        self.provider_authority
            .capability_catalog
            .validate()
            .map_err(|_| "invalid task capability catalog")?;
        let catalog = &self.provider_authority.capability_catalog;
        if catalog.world_id != c.world_id
            || catalog.branch_id != c.branch_id
            || catalog.logical_tick != binding.base_tick
            || !matches!(&catalog.subject,oasis7_wasm_abi::CapabilitySubject::Agent{agent_id,owner_binding,generation} if agent_id==&c.agent_id&&owner_binding==&c.owner_binding&&*generation==c.owner_generation.0)
            || self
                .provider_authority
                .capability_invocation_context
                .catalog_snapshot_id
                != catalog.snapshot_id
            || self
                .provider_authority
                .capability_invocation_context
                .subject
                != catalog.subject
        {
            return Err("task provider authority differs from snapshot".into());
        }
        Ok(())
    }
}
#[derive(Serialize)]
struct ContextCore<'a> {
    prepare_proof: &'a SignedReadRequest<TaskContextPrepareV1>,
    snapshot: &'a GoalExecutionSnapshotV1,
    session: &'a str,
    turn: &'a str,
    request: &'a str,
    valid_until_tick: DecimalU64,
}
impl PreparedTaskContextV1 {
    fn commitment(&self, domain: &str) -> Result<String, String> {
        let salt = Zeroizing::new(salt(&self.reference_salt)?);
        let core = ContextCore {
            prepare_proof: &self.prepare_proof,
            snapshot: &self.snapshot,
            session: &self.identity.agent_session_id,
            turn: &self.identity.agent_turn_id,
            request: &self.identity.decision_request_id,
            valid_until_tick: self.valid_until_tick,
        };
        let bytes = Zeroizing::new(
            oasis7_wasm_abi::encode_canonical_cbor(&(domain, hex::encode(salt.as_slice()), core))
                .map_err(|_| "private task context encoding failed")?,
        );
        Ok(format!("blake3:{}", blake3::hash(&bytes)))
    }
    pub fn validate(&self) -> Result<(), String> {
        verify_prepare(&self.prepare_proof)?;
        self.snapshot.validate()?;
        self.identity.validate()?;
        if self.schema_revision != 1
            || self.identity.fence != self.prepare_proof.request.fence
            || self.identity.fence != GameTaskFenceV1::from_control(&self.snapshot.control)
            || self.identity.valid_until_tick != self.valid_until_tick
            || self.identity.retry_seq != 1
            || self.identity.transport_attempt != 1
            || self.valid_until_tick.0 < self.snapshot.runtime_binding.base_tick
            || self
                .snapshot
                .captured_base_binding
                .base_tick
                .checked_add(TASK_CONTEXT_LEASE_TICKS)
                .is_none_or(|limit| self.valid_until_tick.0 > limit)
            || self.valid_until_tick.0
                > self
                    .snapshot
                    .provider_authority
                    .capability_catalog
                    .valid_until_tick
            || self.identity.request_commitment
                != self.commitment("oasis7/private-task-request/v1")?
            || self.identity.context_commitment
                != self.commitment("oasis7/private-task-context/v1")?
            || self.identity.provider_invocation_commitment
                != self.commitment("oasis7/private-task-invocation/v1")?
        {
            return Err("private task context commitment mismatch".into());
        }
        Ok(())
    }
    pub fn validate_against(
        &self,
        original: &SignedReadRequest<TaskContextPrepareV1>,
    ) -> Result<(), String> {
        self.validate()?;
        if &self.prepare_proof != original {
            return Err("private task operation conflict".into());
        }
        Ok(())
    }
    #[cfg(unix)]
    pub(crate) fn create(
        original: SignedReadRequest<TaskContextPrepareV1>,
        snapshot: GoalExecutionSnapshotV1,
    ) -> Result<Self, String> {
        verify_prepare(&original)?;
        snapshot.validate()?;
        let random_id = |prefix: &str| -> Result<String, String> {
            let mut bytes = [0; 32];
            getrandom::fill(&mut bytes).map_err(|_| "private task randomness unavailable")?;
            Ok(format!("{prefix}-{}", hex::encode(bytes)))
        };
        let mut bytes = Zeroizing::new([0; 32]);
        getrandom::fill(&mut *bytes).map_err(|_| "private task randomness unavailable")?;
        let valid_until_tick = context_deadline(
            snapshot.captured_base_binding.base_tick,
            snapshot
                .provider_authority
                .capability_catalog
                .valid_until_tick,
        )?;
        let mut record = Self {
            schema_revision: 1,
            identity: TaskContextIdentityV1 {
                schema_revision: 1,
                fence: original.request.fence.clone(),
                valid_until_tick,
                agent_session_id: random_id("task-session")?,
                agent_turn_id: random_id("task-turn")?,
                decision_request_id: random_id("task-request")?,
                retry_seq: 1,
                transport_attempt: 1,
                request_commitment: format!("blake3:{}", "0".repeat(64)),
                context_commitment: format!("blake3:{}", "0".repeat(64)),
                provider_invocation_commitment: format!("blake3:{}", "0".repeat(64)),
            },
            prepare_proof: original,
            snapshot,
            reference_salt: hex::encode(bytes.as_slice()),
            valid_until_tick,
        };
        record.identity.request_commitment = record.commitment("oasis7/private-task-request/v1")?;
        record.identity.context_commitment = record.commitment("oasis7/private-task-context/v1")?;
        record.identity.provider_invocation_commitment =
            record.commitment("oasis7/private-task-invocation/v1")?;
        record.validate()?;
        Ok(record)
    }
}
pub fn public_prefix(record: &PreparedTaskContextV1) -> Result<SchedulerIntentV1, String> {
    record.validate()?;
    let binding = record
        .identity
        .fence
        .executor_binding
        .as_ref()
        .ok_or("executor binding missing")?;
    Ok(SchedulerIntentV1 {
        agent_id: record.identity.fence.agent_id.clone(),
        request_id: record.identity.decision_request_id.clone(),
        delegation_generation: binding.delegation_generation.0,
        captured_base_binding: record.snapshot.captured_base_binding.clone(),
        operation: SchedulerOperationV1::TaskProviderPrefix {
            context: record.identity.clone(),
        },
    })
}
pub fn verify_private_reply<Q: Serialize, T: Serialize>(
    path: &str,
    request: &Q,
    response: &SignedServiceResponse<T>,
    key: &str,
) -> Result<(), String> {
    if response.request_digest != request_digest(path, request)? {
        return Err("private task response request mismatch".into());
    }
    verify_owner_signature_strict(
        path,
        &(&response.request_digest, &response.payload),
        key,
        &response.signature_hex,
    )
}

pub fn cognition_intent(
    captured: &CapturedTaskContextV1,
    action: crate::runtime::Action,
) -> Result<CognitionIntentV1, String> {
    captured.prepared.validate()?;
    let current = &captured.current_public_context;
    current.validate_for(&captured.prepared.identity)?;
    super::game_task::task_cognition_intent_from_public_context(
        &captured.prepared.identity,
        current.captured_base_binding.clone(),
        &current.provider_authority,
        &current.actor_observation,
        action,
    )
}

/// Reports a terminal outcome using a distinct public scheduler correlation identity.
pub fn outcome_intent(
    captured: &CapturedTaskContextV1,
    reason: &str,
) -> Result<SchedulerIntentV1, String> {
    captured.prepared.validate()?;
    captured
        .current_public_context
        .validate_for(&captured.prepared.identity)?;
    public_outcome_intent(
        &captured.prepared.identity,
        captured
            .current_public_context
            .captured_base_binding
            .clone(),
        reason,
    )
}
/// Identity-only terminal failure; expiry cleanup needs no private body retrieval.
pub fn public_outcome_intent(
    identity: &TaskContextIdentityV1,
    current_base: RuntimeCognitionBaseBindingV1,
    reason: &str,
) -> Result<SchedulerIntentV1, String> {
    identity.validate()?;
    current_base.validate().map_err(|e| e.to_string())?;
    if !matches!(
        reason,
        "provider_failed" | "transport_exhausted" | "context_expired" | "executor_aborted"
    ) || current_base.world_id != identity.fence.world_id
        || current_base.branch_id != identity.fence.branch_id
        || current_base.reorg_epoch != identity.fence.reorg_epoch.0
        || (reason == "context_expired") != (current_base.base_tick > identity.valid_until_tick.0)
    {
        return Err("task failure reason or public binding mismatch".into());
    }
    let binding = identity
        .fence
        .executor_binding
        .as_ref()
        .ok_or("executor binding missing")?;
    Ok(SchedulerIntentV1 {
        agent_id: identity.fence.agent_id.clone(),
        request_id: format!("{}:outcome", identity.decision_request_id),
        delegation_generation: binding.delegation_generation.0,
        captured_base_binding: current_base,
        operation: SchedulerOperationV1::TaskProviderFailure {
            context: identity.clone(),
            reason: reason.into(),
        },
    })
}

fn context_deadline(base_tick: u64, grant_until: u64) -> Result<DecimalU64, String> {
    let lease_until = base_tick
        .checked_add(TASK_CONTEXT_LEASE_TICKS)
        .ok_or("task context deadline overflow")?;
    if grant_until < base_tick {
        return Err("task grant expired".into());
    }
    Ok(DecimalU64(grant_until.min(lease_until)))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn private_task_context_deadline_never_extends_grant_or_overflows() {
        assert_eq!(context_deadline(10, 100).unwrap(), DecimalU64(26));
        assert_eq!(context_deadline(10, 12).unwrap(), DecimalU64(12));
        assert!(context_deadline(u64::MAX, u64::MAX).is_err());
        assert!(context_deadline(10, 9).is_err());
    }
}
