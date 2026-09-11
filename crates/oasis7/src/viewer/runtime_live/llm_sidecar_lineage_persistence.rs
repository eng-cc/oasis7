use super::*;
use crate::runtime::{World as RuntimeWorld, WorldCommitRecordV1};
#[cfg(not(target_arch = "wasm32"))]
use crate::simulator::AsyncAgentTurnOutcome;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const LEGACY_PROVIDER_LINEAGE_SCHEMA_VERSION: u16 = 1;
const PROVIDER_LINEAGE_SCHEMA_VERSION: u16 = 2;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct ProviderTerminalState {
    /// The map key is an optimization only; retain the subject in the
    /// terminal record so comparisons cannot accidentally cross Agent lanes.
    #[serde(default)]
    pub(super) agent_id: String,
    #[serde(default)]
    pub(super) agent_session_id: String,
    pub(super) agent_turn_id: String,
    pub(super) decision_request_id: String,
    #[serde(default)]
    pub(super) request_digest: String,
    pub(in crate::viewer::runtime_live) status: String,
    pub(super) reject_reason: Option<String>,
    pub(super) feedback_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct ProviderLateResponseDiagnostic {
    pub(super) agent_id: String,
    pub(super) turn_id: u64,
    pub(super) agent_turn_id: String,
    pub(super) decision_request_id: String,
    pub(super) response_digest: Option<String>,
}

/// Durable quarantine for an interrupted provider dispatch whose active
/// marker cannot be correlated to the saved request context.  Such a marker
/// must remain visible across restart while no new provider invocation is
/// admitted for the affected Agent.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct ProviderRecoveryPending {
    pub(super) active: cognition_context::ProviderContextState,
    pub(super) reason: String,
}

/// Durable identity retained after provider finalization until the Runtime
/// scheduler accepts the corresponding wake terminal disposition.  Runtime
/// owns the wake; this record only makes a failed handoff retryable without
/// allocating a new provider turn.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(in crate::viewer::runtime_live) struct ProviderWakeRecoveryPending {
    pub(in crate::viewer::runtime_live) active: cognition_context::ProviderContextState,
    pub(in crate::viewer::runtime_live) status: crate::runtime::ContinuationStatusV1,
    pub(in crate::viewer::runtime_live) reason: String,
}

/// Compare the complete provider request identity carried by two sidecar
/// contexts. Map keys are only routing hints: a restored context is safe to
/// reuse only when every Runtime correlation field agrees.
pub(super) fn provider_context_identity_matches(
    left: &cognition_context::ProviderContextState,
    right: &cognition_context::ProviderContextState,
) -> bool {
    left.request_context.agent_subject == right.request_context.agent_subject
        && left.request_context.agent_session_id == right.request_context.agent_session_id
        && left.request_context.agent_turn_id == right.request_context.agent_turn_id
        && left.request_context.decision_request_id == right.request_context.decision_request_id
        && left.request_context.request_digest == right.request_context.request_digest
        && payer_support::provider_payer_id(&left.request_context).ok()
            == payer_support::provider_payer_id(&right.request_context).ok()
}

pub(super) fn validate_provider_lease_identity(
    agent_id: &str,
    request: &crate::simulator::ContinuousAgentRequestContextV1,
    lease: &crate::runtime::CognitionLeaseV1,
) -> Result<(), String> {
    request
        .validate()
        .map_err(|error| format!("provider cognition request invalid: {error}"))?;
    lease
        .validate()
        .map_err(|error| format!("provider cognition lease invalid: {error}"))?;
    let expected_account = payer_support::provider_payer_id(request)?;
    if lease.status != crate::runtime::CognitionLeaseStatusV1::Reserved {
        return Err(format!(
            "provider cognition lease is not reserved for {agent_id}"
        ));
    }
    let expected_invocation_key = request.provider_invocation_key().to_string();
    if lease.agent_id != agent_id
        || request.agent_subject != agent_id
        || lease.account_id != expected_account
        || lease.idempotency_key != expected_invocation_key
        || lease.agent_session_id != request.agent_session_id
        || lease.agent_turn_id != request.agent_turn_id
        || lease.decision_request_id != request.decision_request_id
        || lease.request_digest != request.request_digest.to_string()
        || lease.quote.resource != "cognition_units"
        || lease.reserved_amount != 1
        || lease.quote.payer_id != lease.account_id
        || lease.quote.resource_version != crate::runtime::COGNITION_RESOURCE_VERSION_V1
        || lease.quote.purpose != "provider_cognition"
        || lease.quote.scope != "agent_turn"
        || lease.quote.policy_revision
            != crate::runtime::COGNITION_FIXED_UNIT_EXPERIMENTAL_POLICY_REVISION
        || lease.quote.authority_context != request.capability_invocation_context_digest.to_string()
        || lease.quote.world_binding != request.runtime_binding.base_world_hash.to_string()
    {
        return Err(format!(
            "provider cognition lease identity mismatch for {agent_id}"
        ));
    }
    Ok(())
}

pub(super) fn validate_provider_lease_binding(
    world: &RuntimeWorld,
    agent_id: &str,
    request: &crate::simulator::ContinuousAgentRequestContextV1,
    lease: &crate::runtime::CognitionLeaseV1,
    operation: &str,
) -> Result<(), String> {
    validate_provider_lease_identity(agent_id, request, lease)?;
    payer_support::runtime_authorized_provider_payer_id(world, request)?;
    let economy = world.cognition_economy().map_err(|error| {
        format!("provider cognition lease {operation} economy read failed: {error:?}")
    })?;
    let runtime_lease = economy.leases.get(lease.lease_id.as_str()).ok_or_else(|| {
        format!(
            "provider cognition lease {operation} missing from Runtime: {}",
            lease.lease_id
        )
    })?;
    if runtime_lease.idempotency_key != lease.idempotency_key
        || runtime_lease.account_id != lease.account_id
        || runtime_lease.agent_id != lease.agent_id
        || runtime_lease.agent_session_id != lease.agent_session_id
        || runtime_lease.agent_turn_id != lease.agent_turn_id
        || runtime_lease.decision_request_id != lease.decision_request_id
        || runtime_lease.request_digest != lease.request_digest
        || runtime_lease.quote != lease.quote
        || runtime_lease.reserved_amount != lease.reserved_amount
    {
        return Err(format!(
            "provider cognition lease {operation} Runtime identity mismatch for {agent_id}"
        ));
    }
    if operation == "dispatch"
        && runtime_lease.status != crate::runtime::CognitionLeaseStatusV1::Reserved
    {
        return Err(format!(
            "provider cognition lease dispatch is already closed for {agent_id}"
        ));
    }
    if operation == "dispatch"
        && (lease.reserved_at_tick > world.state().time
            || lease
                .quote
                .valid_until_tick
                .is_some_and(|expires| world.state().time > expires))
    {
        return Err(format!(
            "provider cognition lease dispatch is stale for {agent_id}"
        ));
    }
    Ok(())
}

pub(super) fn provider_context_matches_wake(
    context: &cognition_context::ProviderContextState,
    wake: &crate::runtime::SchedulerWakeV1,
) -> bool {
    context.request_context.agent_subject == wake.agent_id
        && context.request_context.agent_session_id == wake.agent_session_id
        && context.request_context.agent_turn_id == wake.agent_turn_id
        && context.request_context.decision_request_id == wake.decision_request_id
        && !wake.request_digest.is_empty()
        && context.request_context.request_digest.to_string() == wake.request_digest
}

fn wake_identity_without_digest_matches(
    sidecar: &crate::runtime::SchedulerWakeV1,
    runtime: &crate::runtime::SchedulerWakeV1,
) -> bool {
    sidecar.wake_id == runtime.wake_id
        && sidecar.continuation_id == runtime.continuation_id
        && sidecar.world_id == runtime.world_id
        && sidecar.branch_id == runtime.branch_id
        && sidecar.finality_epoch == runtime.finality_epoch
        && sidecar.finality_block_hash == runtime.finality_block_hash
        && sidecar.finality_status == runtime.finality_status
        && sidecar.reorg_epoch == runtime.reorg_epoch
        && sidecar.runtime_manifest_hash == runtime.runtime_manifest_hash
        && sidecar.agent_id == runtime.agent_id
        && sidecar.agent_session_id == runtime.agent_session_id
        && sidecar.agent_turn_id == runtime.agent_turn_id
        && sidecar.decision_request_id == runtime.decision_request_id
}

fn wake_matches_terminal_identity(
    wake: &crate::runtime::SchedulerWakeV1,
    terminal: &ProviderTerminalState,
) -> bool {
    !terminal.agent_id.is_empty()
        && !terminal.agent_session_id.is_empty()
        && !terminal.agent_turn_id.is_empty()
        && !terminal.decision_request_id.is_empty()
        && !terminal.request_digest.is_empty()
        && wake.agent_id == terminal.agent_id
        && wake.agent_session_id == terminal.agent_session_id
        && wake.agent_turn_id == terminal.agent_turn_id
        && wake.decision_request_id == terminal.decision_request_id
}

/// Normalize a legacy sidecar wake's missing request digest from a Runtime
/// in-flight wake or a complete terminal marker.  Both authorities are
/// compared on every persisted identity field before hydration; an explicit
/// digest disagreement is a checkpoint conflict and must fail closed.
pub(super) fn hydrate_pending_runtime_wake_identities(
    pending_runtime_wakes: &mut BTreeMap<String, crate::runtime::SchedulerWakeV1>,
    runtime_wakes: &[crate::runtime::SchedulerWakeV1],
    terminal_states: &BTreeMap<String, ProviderTerminalState>,
) -> Result<bool, String> {
    let mut migrated = false;
    for wake in pending_runtime_wakes.values_mut() {
        if let Some(runtime_wake) = runtime_wakes
            .iter()
            .find(|runtime_wake| runtime_wake.wake_id == wake.wake_id)
        {
            if !wake_identity_without_digest_matches(wake, runtime_wake) {
                return Err(format!(
                    "pending Runtime wake identity mismatch for {}",
                    wake.wake_id
                ));
            }
            if !runtime_wake.request_digest.is_empty() {
                if wake.request_digest.is_empty() {
                    *wake = runtime_wake.clone();
                    migrated = true;
                } else if wake.request_digest != runtime_wake.request_digest {
                    return Err(format!(
                        "pending Runtime wake request_digest conflict for {}",
                        wake.wake_id
                    ));
                }
                continue;
            }
        }

        let terminal = terminal_states
            .values()
            .find(|terminal| wake_matches_terminal_identity(wake, terminal));
        let Some(terminal) = terminal else {
            continue;
        };
        if wake.request_digest.is_empty() {
            wake.request_digest = terminal.request_digest.clone();
            migrated = true;
        } else if wake.request_digest != terminal.request_digest {
            return Err(format!(
                "pending Runtime wake terminal identity conflict for {}",
                wake.wake_id
            ));
        }
    }
    Ok(migrated)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct PersistedProviderLineageV1 {
    schema_version: u16,
    provider_session_ids: BTreeMap<String, String>,
    provider_agent_ids: BTreeSet<String>,
    provider_context_seq: BTreeMap<String, u64>,
    provider_contexts: BTreeMap<String, cognition_context::ProviderContextState>,
    provider_retry_contexts: BTreeMap<String, cognition_context::ProviderContextState>,
    provider_active_turns: BTreeMap<String, cognition_context::ProviderContextState>,
    #[serde(default)]
    provider_cognition_leases: BTreeMap<String, crate::runtime::CognitionLeaseV1>,
    /// The exact simulator proposal admitted into the Harness.  Runtime's
    /// durable continuation projection omits Harness chain inputs, so this
    /// mirror is required for restart hydration.
    #[serde(default)]
    provider_continuation_proposals: BTreeMap<String, SimulatorContinuationProposalV1>,
    #[serde(default)]
    provider_continuation_recovery_pending: BTreeMap<String, String>,
    #[serde(default)]
    provider_recovery_pending: BTreeMap<String, ProviderRecoveryPending>,
    #[serde(default)]
    provider_wake_recovery_pending: BTreeMap<String, ProviderWakeRecoveryPending>,
    provider_wait_until: BTreeMap<String, u64>,
    provider_feedback_seq: BTreeMap<String, u64>,
    #[serde(default)]
    provider_feedback_seq_by_session: BTreeMap<String, u64>,
    #[serde(default)]
    provider_memory_store: MemoryWriteStore,
    provider_completed_decisions: VecDeque<async_support::RuntimeLlmDecision>,
    provider_held_decisions: BTreeMap<String, async_support::RuntimeLlmDecision>,
    provider_stale_replans: BTreeMap<String, ProviderStaleReplanState>,
    provider_transport_exhausted: BTreeSet<String>,
    provider_terminal_states: BTreeMap<String, ProviderTerminalState>,
    provider_late_response_diagnostics: VecDeque<ProviderLateResponseDiagnostic>,
    pending_actions: BTreeMap<u64, RuntimePendingAction>,
    #[serde(default)]
    pending_provider_world_events: BTreeMap<String, RuntimePendingProviderWorldEvent>,
    #[serde(default)]
    provider_world_event_quarantine: BTreeMap<String, String>,
    /// A failed Runtime wake projection is a durable recovery fence.  Keep
    /// the reason in the sidecar checkpoint so a restart cannot turn an
    /// unreadable authoritative projection into a fresh provider dispatch.
    #[serde(default)]
    provider_lineage_recovery_pending: Option<String>,
    runtime_binding: Option<RuntimeBindingV1>,
    #[serde(default)]
    pending_runtime_wakes: BTreeMap<String, crate::runtime::SchedulerWakeV1>,
}

fn validate_persisted_provider_cognition_leases(
    world: &RuntimeWorld,
    checkpoint: &PersistedProviderLineageV1,
) -> Result<(), String> {
    for (agent_id, lease) in &checkpoint.provider_cognition_leases {
        if lease.agent_id != *agent_id {
            return Err(format!(
                "provider cognition lease key mismatch for {agent_id}"
            ));
        }
        let mut contexts = Vec::new();
        if let Some(context) = checkpoint.provider_contexts.get(agent_id) {
            contexts.push(&context.request_context);
        }
        if let Some(context) = checkpoint.provider_active_turns.get(agent_id) {
            contexts.push(&context.request_context);
        }
        if let Some(context) = checkpoint.provider_retry_contexts.get(agent_id) {
            contexts.push(&context.request_context);
        }
        if let Some(pending) = checkpoint.provider_recovery_pending.get(agent_id) {
            contexts.push(&pending.active.request_context);
        }
        if let Some(pending) = checkpoint.provider_wake_recovery_pending.get(agent_id) {
            contexts.push(&pending.active.request_context);
        }
        for pending in checkpoint.pending_actions.values() {
            if pending.agent_id == *agent_id
                && let Some(cognition) = pending.cognition.as_ref()
            {
                contexts.push(&cognition.request.request_context);
            }
        }
        for decision in checkpoint
            .provider_completed_decisions
            .iter()
            .chain(checkpoint.provider_held_decisions.values())
        {
            if decision.agent_id == *agent_id
                && let Some(cognition) = decision.cognition.as_ref()
            {
                contexts.push(&cognition.request.request_context);
            }
        }
        if contexts.is_empty() {
            return Err(format!(
                "provider cognition lease context missing for {agent_id}"
            ));
        }
        for request in contexts {
            // Validate the sidecar copy against its complete request identity
            // even when this process has not yet installed the matching
            // Runtime world. The server constructor restores the sidecar
            // before a caller can replace a bootstrap world in tests/tools;
            // dispatch and economic cleanup repeat the Runtime lookup below
            // before permitting any effect.
            validate_provider_lease_identity(agent_id, request, lease)?;
            if let Err(error) =
                validate_provider_lease_binding(world, agent_id, request, lease, "restore")
            {
                // Runtime removes a lease as part of an authoritative receipt
                // settlement. A crash can still leave the sidecar copy in the
                // checkpoint until receipt feedback finalization completes;
                // accept only the exact committed request identity, after
                // validating the lease fields above, and let restore's
                // commit-marker reconciliation discard the stale mirror. A
                // bootstrap world with no matching lease is also deferred to
                // the dispatch/economic validation gates described above.
                if !error.contains("missing from Runtime")
                    && committed_runtime_record_for_request(world, request)?.is_none()
                {
                    return Err(error);
                }
            }
        }
    }
    Ok(())
}

fn decode_provider_lineage_checkpoint(
    bytes: &[u8],
) -> Result<(PersistedProviderLineageV1, bool), String> {
    let mut value: Value = serde_json::from_slice(bytes)
        .map_err(|error| format!("provider lineage checkpoint decode failed: {error}"))?;
    let schema_version = value
        .get("schema_version")
        .and_then(Value::as_u64)
        .ok_or_else(|| {
            "provider lineage checkpoint decode failed: missing schema_version".to_string()
        })?;
    let migrated = match u16::try_from(schema_version).unwrap_or(u16::MAX) {
        PROVIDER_LINEAGE_SCHEMA_VERSION => false,
        LEGACY_PROVIDER_LINEAGE_SCHEMA_VERSION => {
            migrate_legacy_budget_contracts(&mut value)?;
            value["schema_version"] = json!(PROVIDER_LINEAGE_SCHEMA_VERSION);
            true
        }
        other => {
            return Err(format!(
                "unsupported provider lineage checkpoint schema {other}"
            ));
        }
    };
    let checkpoint = serde_json::from_value(value)
        .map_err(|error| format!("provider lineage checkpoint decode failed: {error}"))?;
    Ok((checkpoint, migrated))
}

pub(super) fn committed_runtime_record_for_request(
    world: &RuntimeWorld,
    request: &crate::simulator::ContinuousAgentRequestContextV1,
) -> Result<Option<WorldCommitRecordV1>, String> {
    let Some(values) = world
        .cognition()
        .get("commit_records")
        .and_then(Value::as_array)
    else {
        return Ok(None);
    };
    for value in values {
        let marker: WorldCommitRecordV1 =
            serde_json::from_value(value.clone()).map_err(|error| {
                format!(
                    "Runtime cognition commit record decode failed during provider restore: {error}"
                )
            })?;
        if marker.status == "committed"
            && marker.agent_id == request.agent_subject
            && marker.agent_session_id == request.agent_session_id
            && marker.agent_turn_id == request.agent_turn_id
            && marker.decision_request_id == request.decision_request_id
            && marker.request_digest == request.request_digest.to_string()
        {
            return Ok(Some(marker));
        }
    }
    Ok(None)
}

fn decision_matches_commit_record(
    decision: &async_support::RuntimeLlmDecision,
    marker: &WorldCommitRecordV1,
) -> bool {
    let Some(cognition) = decision.cognition.as_ref() else {
        return false;
    };
    let request = &cognition.request.request_context;
    marker.agent_id == decision.agent_id
        && marker.agent_id == request.agent_subject
        && marker.agent_session_id == request.agent_session_id
        && marker.agent_turn_id == request.agent_turn_id
        && marker.decision_request_id == request.decision_request_id
        && marker.request_digest == request.request_digest.to_string()
}

/// V1 checkpoints predate the explicit provider/tool budget limits.  A
/// missing limit is an explicit zero deny after migration; treating it as an
/// unlimited/default budget could spend credits that the checkpoint cannot
/// account for. Only nested budget objects are changed, preserving all saved
/// session, turn, request, and recovery identities.
fn migrate_legacy_budget_contracts(value: &mut Value) -> Result<(), String> {
    fn visit(value: &mut Value, migrated: &mut usize) {
        match value {
            Value::Object(fields) => {
                if let Some(Value::Object(budget)) = fields.get_mut("budget_contract") {
                    if !budget.contains_key("max_model_calls") {
                        budget.insert("max_model_calls".to_string(), json!(0));
                        *migrated = migrated.saturating_add(1);
                    }
                    if !budget.contains_key("max_tool_calls") {
                        budget.insert("max_tool_calls".to_string(), json!(0));
                        *migrated = migrated.saturating_add(1);
                    }
                }
                for child in fields.values_mut() {
                    visit(child, migrated);
                }
            }
            Value::Array(values) => {
                for child in values {
                    visit(child, migrated);
                }
            }
            _ => {}
        }
    }

    let mut migrated = 0;
    visit(value, &mut migrated);
    // An empty V1 checkpoint is valid and needs only a schema bump. Any
    // non-empty budget object still receives explicit zero-deny limits above;
    // malformed objects fail closed during the typed decode below.
    Ok(())
}

impl RuntimeLlmSidecar {
    pub(in crate::viewer::runtime_live) fn fence_provider_cognition_lease(
        &mut self,
        agent_id: &str,
        context: &cognition_context::ProviderContextState,
        reason: impl Into<String>,
    ) {
        self.provider_recovery_pending.insert(
            agent_id.to_string(),
            ProviderRecoveryPending {
                active: context.clone(),
                reason: reason.into(),
            },
        );
        self.provider_transport_exhausted
            .insert(agent_id.to_string());
        self.persist_provider_lineage_best_effort();
    }

    pub(in crate::viewer::runtime_live) fn validate_provider_cognition_lease_for_request(
        &self,
        world: &RuntimeWorld,
        agent_id: &str,
        request: &crate::simulator::ContinuousAgentRequestContextV1,
        lease: &crate::runtime::CognitionLeaseV1,
        operation: &str,
    ) -> Result<(), String> {
        validate_provider_lease_binding(world, agent_id, request, lease, operation)
    }

    pub(in crate::viewer::runtime_live) fn validate_provider_cognition_lease_for_agent(
        &self,
        world: &RuntimeWorld,
        agent_id: &str,
        lease: &crate::runtime::CognitionLeaseV1,
        operation: &str,
    ) -> Result<(), String> {
        let context = self.provider_recovery_context(agent_id).ok_or_else(|| {
            format!("provider cognition lease {operation} context missing for {agent_id}")
        })?;
        self.validate_provider_cognition_lease_for_request(
            world,
            agent_id,
            &context.request_context,
            lease,
            operation,
        )
    }

    /// Return true when a queued provider decision carries the exact identity
    /// already closed by Runtime. A legacy decision may omit its cognition
    /// envelope, so use the durable sidecar context for that compatibility
    /// shape while still requiring the complete terminal identity.
    pub(super) fn provider_decision_is_terminalized(
        &self,
        decision: &async_support::RuntimeLlmDecision,
    ) -> bool {
        let context = decision
            .cognition
            .as_ref()
            .map(|cognition| &cognition.request)
            .or_else(|| self.provider_contexts.get(&decision.agent_id));
        context.is_some_and(|context| {
            self.provider_terminal_matches_request(
                decision.agent_id.as_str(),
                &context.request_context,
            )
        })
    }

    /// Compare a persisted terminal marker with a full request identity. New
    /// fields deliberately fail closed when loading a legacy marker that did
    /// not contain them; an incomplete marker must not suppress a later turn.
    pub(super) fn provider_terminal_matches_request(
        &self,
        agent_id: &str,
        request: &crate::simulator::ContinuousAgentRequestContextV1,
    ) -> bool {
        let Some(terminal) = self.provider_terminal_states.get(agent_id) else {
            return false;
        };
        !terminal.agent_id.is_empty()
            && !terminal.agent_session_id.is_empty()
            && !terminal.agent_turn_id.is_empty()
            && !terminal.decision_request_id.is_empty()
            && !terminal.request_digest.is_empty()
            && terminal.agent_id == agent_id
            && request.agent_subject == agent_id
            && terminal.agent_session_id == request.agent_session_id
            && terminal.agent_turn_id == request.agent_turn_id
            && terminal.decision_request_id == request.decision_request_id
            && terminal.request_digest == request.request_digest.to_string()
    }

    /// Configure a Viewer-owned durable checkpoint for provider transport and
    /// response lineage. Runtime remains the authority for world state and
    /// binding validation; this file only retains work owned by the adapter.
    pub(in crate::viewer::runtime_live) fn configure_provider_lineage_store(
        &mut self,
        path: impl Into<std::path::PathBuf>,
    ) {
        self.provider_lineage_store = Some(path.into());
    }

    #[cfg(test)]
    pub(in crate::viewer::runtime_live) fn install_test_provider_lineage_checkpoint_blocker(
        &self,
    ) -> Result<(), String> {
        let Some(path) = self.provider_lineage_store.as_deref() else {
            return Err("provider lineage checkpoint path is not configured".to_string());
        };
        if path.is_dir() {
            return Ok(());
        }
        let backup_path = path.with_extension(format!("blocked-backup-{}", std::process::id()));
        fs::rename(path, &backup_path).map_err(|error| {
            format!(
                "provider lineage checkpoint blocker could not move {}: {error}",
                path.display()
            )
        })?;
        if let Err(error) = fs::create_dir(path) {
            let _ = fs::rename(&backup_path, path);
            return Err(format!(
                "provider lineage checkpoint blocker could not create {}: {error}",
                path.display()
            ));
        }
        Ok(())
    }

    pub(in crate::viewer::runtime_live) fn restore_provider_lineage(
        &mut self,
        world: &RuntimeWorld,
    ) -> Result<(), String> {
        let Some(path) = self.provider_lineage_store.as_deref() else {
            return Ok(());
        };
        let bytes = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => {
                return Err(format!(
                    "provider lineage checkpoint read failed ({}): {error}",
                    path.display()
                ));
            }
        };
        let (checkpoint, checkpoint_migrated) = match decode_provider_lineage_checkpoint(&bytes) {
            Ok(checkpoint) => checkpoint,
            Err(error) => {
                self.provider_lineage_recovery_pending = Some(error.clone());
                return Err(error);
            }
        };
        validate_persisted_provider_cognition_leases(world, &checkpoint)?;
        for (proposal_id, proposal) in &checkpoint.provider_continuation_proposals {
            if proposal_id != &proposal.continuation_proposal_id {
                return Err(format!(
                    "provider continuation checkpoint key mismatch for {proposal_id}"
                ));
            }
            proposal.validate().map_err(|error| {
                format!(
                    "provider continuation checkpoint proposal invalid ({proposal_id}): {error}"
                )
            })?;
        }
        let current_binding = checkpoint
            .runtime_binding
            .as_ref()
            .map(|_| {
                world
                    .current_cognition_runtime_binding()
                    .map_err(|error| format!("Runtime cognition binding unavailable: {error:?}"))
            })
            .transpose()?;
        let binding_changed = current_binding
            .as_ref()
            .zip(checkpoint.runtime_binding.as_ref())
            .is_some_and(|(current, saved)| current != saved);

        self.provider_session_ids = checkpoint.provider_session_ids;
        self.provider_agent_ids = checkpoint.provider_agent_ids;
        self.provider_context_seq = checkpoint.provider_context_seq;
        self.provider_contexts = checkpoint.provider_contexts;
        self.provider_retry_contexts = checkpoint.provider_retry_contexts;
        self.provider_active_turns = checkpoint.provider_active_turns;
        self.provider_cognition_leases = checkpoint.provider_cognition_leases;
        self.provider_continuation_proposals = checkpoint.provider_continuation_proposals;
        self.provider_continuation_recovery_pending =
            checkpoint.provider_continuation_recovery_pending;
        self.provider_recovery_pending = checkpoint.provider_recovery_pending;
        self.provider_wake_recovery_pending = checkpoint.provider_wake_recovery_pending;
        self.provider_wait_until = checkpoint.provider_wait_until;
        self.provider_feedback_seq = checkpoint.provider_feedback_seq;
        self.provider_feedback_seq_by_session = checkpoint.provider_feedback_seq_by_session;
        self.provider_memory_store = checkpoint.provider_memory_store;
        self.provider_completed_decisions = checkpoint.provider_completed_decisions;
        self.provider_transport_exhausted = checkpoint.provider_transport_exhausted;
        self.provider_terminal_states = checkpoint.provider_terminal_states;
        self.provider_completed_decisions = std::mem::take(&mut self.provider_completed_decisions)
            .into_iter()
            .filter(|decision| !self.provider_decision_is_terminalized(decision))
            .collect();
        // A retry context is an interrupted logical request whose actor-local
        // budget ledger is absent after restart.  Do not carry it into the
        // normal retry selector: fence the identity for terminal feedback and
        // remove the stale retry projection so clearing the fence cannot
        // redispatch it a second time.
        let persisted_retry_agents = self
            .provider_retry_contexts
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        let recovered_retry = !persisted_retry_agents.is_empty();
        for agent_id in persisted_retry_agents {
            self.provider_retry_contexts.remove(agent_id.as_str());
            self.provider_transport_exhausted.insert(agent_id);
        }
        for (agent_id, decision) in checkpoint.provider_held_decisions {
            if decision.cognition.is_none()
                && decision
                    .decision_trace
                    .as_ref()
                    .is_some_and(provider_trace_retryable)
            {
                // A held retryable error is a control-plane delivery detail,
                // not a durable provider response. Its in-memory async actor
                // cannot survive process restart, and the process-local budget
                // ledger cannot prove whether the provider already charged the
                // request. Terminalize the uncertain identity instead of
                // redispatching it.
                if self.provider_transport_exhausted.contains(&agent_id) {
                    continue;
                }
                self.provider_transport_exhausted.insert(agent_id);
                continue;
            }
            if self.provider_decision_is_terminalized(&decision) {
                continue;
            }
            if !self
                .provider_completed_decisions
                .iter()
                .any(|queued| queued.agent_id == decision.agent_id)
            {
                self.provider_completed_decisions
                    .push_back(decision.clone());
            }
            self.provider_held_decisions.insert(agent_id, decision);
        }
        self.provider_stale_replans = checkpoint.provider_stale_replans;
        self.provider_late_response_diagnostics = checkpoint.provider_late_response_diagnostics;
        self.pending_actions = checkpoint.pending_actions;
        self.pending_provider_world_events = checkpoint.pending_provider_world_events;
        self.provider_world_event_quarantine = checkpoint.provider_world_event_quarantine;
        self.provider_lineage_recovery_pending = checkpoint.provider_lineage_recovery_pending;
        let mut pending_runtime_wakes = checkpoint
            .pending_runtime_wakes
            .into_values()
            .map(|wake| (wake.wake_id.clone(), wake))
            .collect();
        let runtime_wakes = world.cognition_in_flight_wakes().map_err(|error| {
            format!("Runtime cognition wake read failed during provider lineage restore: {error:?}")
        })?;
        let pending_runtime_wakes_migrated = hydrate_pending_runtime_wake_identities(
            &mut pending_runtime_wakes,
            &runtime_wakes,
            &self.provider_terminal_states,
        )?;
        self.pending_runtime_wakes = pending_runtime_wakes;
        self.provider_lineage_binding = current_binding.or(checkpoint.runtime_binding);
        self.provider_lineage_restored = true;

        // Runtime's committed marker is authoritative over the sidecar's
        // checkpoint. A process can stop after Runtime commits the response
        // but before `track_action` or terminal feedback is persisted. In
        // that crash prefix, synthesize the exact terminal identity from the
        // Runtime marker and discard every stale sidecar mirror so the old
        // provider request cannot be admitted again.
        let mut committed_recovery = false;
        let candidate_agents = self
            .provider_active_turns
            .keys()
            .cloned()
            .chain(self.provider_contexts.keys().cloned())
            .collect::<BTreeSet<_>>();
        for agent_id in candidate_agents {
            let request = self
                .provider_active_turns
                .get(agent_id.as_str())
                .or_else(|| self.provider_contexts.get(agent_id.as_str()))
                .map(|context| context.request_context.clone());
            let Some(request) = request else { continue };
            let Some(marker) = committed_runtime_record_for_request(world, &request)? else {
                continue;
            };
            let recovery_context = self
                .provider_active_turns
                .get(agent_id.as_str())
                .or_else(|| self.provider_contexts.get(agent_id.as_str()))
                .cloned();
            // A resumed Runtime continuation may still own an in-flight wake
            // when the sidecar checkpoint failed after the action receipt was
            // committed. Keep that exact wake visible as a terminal recovery
            // handoff; otherwise restore would clear the provider identity
            // and accidentally admit the continuation as a fresh turn.
            let committed_wake = recovery_context.as_ref().and_then(|context| {
                self.pending_runtime_wakes
                    .values()
                    .chain(runtime_wakes.iter())
                    .find(|wake| provider_context_matches_wake(context, wake))
                    .cloned()
            });
            let has_committed_wake = committed_wake.is_some();
            let has_committed_lease = self
                .provider_cognition_leases
                .contains_key(agent_id.as_str());
            self.provider_terminal_states.insert(
                agent_id.clone(),
                ProviderTerminalState {
                    agent_id: marker.agent_id.clone(),
                    agent_session_id: marker.agent_session_id.clone(),
                    agent_turn_id: marker.agent_turn_id.clone(),
                    decision_request_id: marker.decision_request_id.clone(),
                    request_digest: marker.request_digest.clone(),
                    status: marker.status.clone(),
                    reject_reason: marker.abort_reason.clone(),
                    feedback_id: (!marker.feedback_id.is_empty())
                        .then_some(marker.feedback_id.clone()),
                },
            );
            if let (Some(context), Some(wake)) = (recovery_context, committed_wake) {
                self.pending_runtime_wakes
                    .insert(wake.wake_id.clone(), wake);
                self.provider_wake_recovery_pending.insert(
                    agent_id.clone(),
                    ProviderWakeRecoveryPending {
                        active: context,
                        status: crate::runtime::ContinuationStatusV1::Completed,
                        reason: "provider_action_committed".to_string(),
                    },
                );
            }
            self.provider_completed_decisions.retain(|decision| {
                !decision_matches_commit_record(decision, &marker)
                    && !(decision.cognition.is_none() && decision.agent_id == marker.agent_id)
            });
            self.provider_held_decisions.retain(|_, decision| {
                !decision_matches_commit_record(decision, &marker)
                    && !(decision.cognition.is_none() && decision.agent_id == marker.agent_id)
            });
            if !has_committed_lease {
                self.provider_active_turns.remove(agent_id.as_str());
                self.provider_contexts.remove(agent_id.as_str());
                self.provider_retry_contexts.remove(agent_id.as_str());
                self.provider_recovery_pending.remove(agent_id.as_str());
            }
            if !has_committed_wake {
                self.provider_wake_recovery_pending
                    .remove(agent_id.as_str());
            }
            self.provider_wait_until.remove(agent_id.as_str());
            if !has_committed_lease {
                self.pending_actions
                    .retain(|_, pending| pending.agent_id != agent_id);
            }
            self.provider_continuation_proposals
                .retain(|_, proposal| proposal.agent_id != agent_id);
            self.provider_continuation_recovery_pending
                .remove(agent_id.as_str());
            self.provider_transport_exhausted.remove(agent_id.as_str());
            committed_recovery = true;
        }

        // A response that was already accepted or is waiting for its
        // scheduled terminal feedback must stay occupied. An orphaned active
        // marker, however, represents an interrupted request; its complete
        // context remains in `provider_contexts` and may be re-dispatched with
        // the same identity rather than allocating a duplicate turn.
        let retained_agents = self
            .provider_held_decisions
            .keys()
            .cloned()
            .chain(
                self.provider_completed_decisions
                    .iter()
                    .map(|decision| decision.agent_id.clone()),
            )
            .chain(
                self.pending_actions
                    .values()
                    .map(|pending| pending.agent_id.clone()),
            )
            .chain(self.provider_wait_until.keys().cloned())
            .chain(self.provider_wake_recovery_pending.keys().cloned())
            .chain(runtime_wakes.iter().filter_map(|wake| {
                self.provider_contexts
                    .get(wake.agent_id.as_str())
                    .filter(|context| provider_context_matches_wake(context, wake))
                    .map(|_| wake.agent_id.clone())
            }))
            .collect::<BTreeSet<_>>();
        let orphaned_active_markers = self
            .provider_active_turns
            .iter()
            .map(|(agent_id, active)| {
                let context = self.provider_contexts.get(agent_id.as_str());
                let same_identity = context
                    .is_some_and(|context| provider_context_identity_matches(context, active));
                (
                    agent_id.clone(),
                    active.clone(),
                    context.cloned(),
                    same_identity,
                    retained_agents.contains(agent_id),
                )
            })
            .collect::<Vec<_>>();
        let mut recovered_orphan = false;
        for (agent_id, active, context, same_identity, retained) in orphaned_active_markers {
            if !same_identity {
                // Preserve the mismatched marker rather than silently
                // dropping evidence.  A subsequent prepare pass is fenced by
                // this record until an explicit recovery decision resolves
                // the identity conflict.
                self.provider_recovery_pending.insert(
                    agent_id.clone(),
                    ProviderRecoveryPending {
                        active,
                        reason: if context.is_some() {
                            "active_context_identity_mismatch".to_string()
                        } else {
                            "active_context_missing".to_string()
                        },
                    },
                );
                self.provider_active_turns.remove(agent_id.as_str());
                // Route the durable quarantine through the same Runtime
                // terminalization pass as an exhausted transport. Keeping the
                // marker only in `provider_recovery_pending` would silently
                // fence the agent forever with no actionable Runtime event.
                self.provider_transport_exhausted.insert(agent_id);
                recovered_orphan = true;
                continue;
            }
            if retained {
                continue;
            }
            self.provider_active_turns.remove(agent_id.as_str());
            // The active marker proves that the logical request was in flight,
            // but does not carry the actor's process-local budget counters.
            // Keep the correlated context for terminal feedback and fence the
            // Agent so a restart cannot spend the same credits twice.
            let _ = context;
            self.provider_retry_contexts.remove(agent_id.as_str());
            self.provider_transport_exhausted.insert(agent_id);
            recovered_orphan = true;
        }
        if binding_changed {
            let mut stale_contexts = self
                .provider_contexts
                .keys()
                .filter(|agent_id| !retained_agents.contains(*agent_id))
                .cloned()
                .collect::<Vec<_>>();
            let retry_agents = self
                .provider_retry_contexts
                .keys()
                .filter(|agent_id| !retained_agents.contains(*agent_id))
                .cloned()
                .collect::<Vec<_>>();
            for agent_id in retry_agents {
                if !stale_contexts.contains(&agent_id) {
                    stale_contexts.push(agent_id);
                }
            }
            for agent_id in stale_contexts {
                let context = self
                    .provider_contexts
                    .remove(agent_id.as_str())
                    .or_else(|| self.provider_retry_contexts.remove(agent_id.as_str()));
                let Some(context) = context else { continue };
                self.provider_retry_contexts.remove(agent_id.as_str());
                self.provider_active_turns.remove(agent_id.as_str());
                self.schedule_provider_stale_replan(
                    agent_id.as_str(),
                    context.request_context.agent_turn_id.as_str(),
                    context.request_context.decision_request_id.as_str(),
                );
            }
        }
        // A terminal marker is written before feedback delivery/release. If
        // the process stopped in that small window, do not replay the same
        // response or redispatch its action after restart. Compare identities
        // so a later request for the same agent is not mistaken for the old
        // terminal turn.
        let terminal_agents = self
            .provider_terminal_states
            .iter()
            .filter_map(|(agent_id, _terminal)| {
                if self.provider_wake_recovery_pending.contains_key(agent_id) {
                    return None;
                }
                if self.provider_cognition_leases.contains_key(agent_id) {
                    // Keep the request/lease mirror for mutable settlement.
                    return None;
                }
                let context = self.provider_contexts.get(agent_id)?;
                self.provider_terminal_matches_request(agent_id, &context.request_context)
                    .then_some(agent_id.clone())
            })
            .collect::<Vec<_>>();
        for agent_id in terminal_agents {
            self.provider_contexts.remove(agent_id.as_str());
            self.provider_retry_contexts.remove(agent_id.as_str());
            self.provider_active_turns.remove(agent_id.as_str());
            self.provider_cognition_leases.remove(agent_id.as_str());
            self.provider_wait_until.remove(agent_id.as_str());
            self.provider_held_decisions.remove(agent_id.as_str());
            self.pending_actions
                .retain(|_, pending| pending.agent_id != agent_id);
            self.provider_continuation_proposals
                .retain(|_, proposal| proposal.agent_id != agent_id);
            self.provider_continuation_recovery_pending
                .remove(agent_id.as_str());
        }
        if committed_recovery
            || recovered_orphan
            || recovered_retry
            || checkpoint_migrated
            || pending_runtime_wakes_migrated
        {
            // Persist the active-marker removal and retry/exhaustion decision
            // before the next Runtime tick, so a restart cannot strand the
            // same identity again. Persist an explicit schema upgrade too,
            // so a legacy zero-deny migration is durable before the next
            // process restart.
            self.persist_provider_lineage_best_effort();
        }
        Ok(())
    }

    pub(in crate::viewer::runtime_live) fn persist_provider_lineage(&self) -> Result<(), String> {
        let Some(path) = self.provider_lineage_store.as_deref() else {
            return Ok(());
        };
        let checkpoint = PersistedProviderLineageV1 {
            schema_version: PROVIDER_LINEAGE_SCHEMA_VERSION,
            provider_session_ids: self.provider_session_ids.clone(),
            provider_agent_ids: self.provider_agent_ids.clone(),
            provider_context_seq: self.provider_context_seq.clone(),
            provider_contexts: self.provider_contexts.clone(),
            provider_retry_contexts: self.provider_retry_contexts.clone(),
            provider_active_turns: self.provider_active_turns.clone(),
            provider_cognition_leases: self.provider_cognition_leases.clone(),
            provider_continuation_proposals: self.provider_continuation_proposals.clone(),
            provider_continuation_recovery_pending: self
                .provider_continuation_recovery_pending
                .clone(),
            provider_recovery_pending: self.provider_recovery_pending.clone(),
            provider_wake_recovery_pending: self.provider_wake_recovery_pending.clone(),
            provider_wait_until: self.provider_wait_until.clone(),
            provider_feedback_seq: self.provider_feedback_seq.clone(),
            provider_feedback_seq_by_session: self.provider_feedback_seq_by_session.clone(),
            provider_memory_store: self.provider_memory_store.clone(),
            provider_completed_decisions: self.provider_completed_decisions.clone(),
            provider_held_decisions: self.provider_held_decisions.clone(),
            provider_stale_replans: self.provider_stale_replans.clone(),
            provider_transport_exhausted: self.provider_transport_exhausted.clone(),
            provider_terminal_states: self.provider_terminal_states.clone(),
            provider_late_response_diagnostics: self.provider_late_response_diagnostics.clone(),
            pending_actions: self.pending_actions.clone(),
            pending_provider_world_events: self.pending_provider_world_events.clone(),
            provider_world_event_quarantine: self.provider_world_event_quarantine.clone(),
            provider_lineage_recovery_pending: self.provider_lineage_recovery_pending.clone(),
            runtime_binding: self.provider_lineage_binding.clone(),
            pending_runtime_wakes: self.pending_runtime_wakes.clone(),
        };
        let encoded = serde_json::to_vec_pretty(&checkpoint)
            .map_err(|error| format!("provider lineage checkpoint encode failed: {error}"))?;
        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "provider lineage checkpoint directory creation failed ({}): {error}",
                parent.display()
            )
        })?;
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        let temp_path = path.with_extension(format!("tmp-{}-{nonce}", std::process::id()));
        fs::write(&temp_path, encoded).map_err(|error| {
            format!(
                "provider lineage checkpoint temporary write failed ({}): {error}",
                temp_path.display()
            )
        })?;
        if let Err(error) = fs::rename(&temp_path, path) {
            let _ = fs::remove_file(&temp_path);
            return Err(format!(
                "provider lineage checkpoint commit failed ({}): {error}",
                path.display()
            ));
        }
        Ok(())
    }

    pub(super) fn persist_provider_lineage_best_effort(&self) {
        if let Err(error) = self.persist_provider_lineage() {
            tracing::warn!(error, "provider lineage checkpoint persistence failed");
        }
    }

    pub(super) fn record_provider_terminal_state(
        &mut self,
        agent_id: &str,
        context: &cognition_context::ProviderContextState,
        status: &str,
        reject_reason: Option<String>,
        feedback_id: Option<String>,
    ) {
        self.record_provider_terminal_state_for_request(
            agent_id,
            &context.request_context,
            status,
            reject_reason,
            feedback_id,
        );
    }

    /// Persist a terminal marker from the exact request used by Runtime. This
    /// path is needed when restoration quarantines an active marker while the
    /// sidecar's provider context is absent or belongs to another identity.
    pub(super) fn record_provider_terminal_state_for_request(
        &mut self,
        agent_id: &str,
        request: &crate::simulator::ContinuousAgentRequestContextV1,
        status: &str,
        reject_reason: Option<String>,
        feedback_id: Option<String>,
    ) {
        self.provider_terminal_states.insert(
            agent_id.to_string(),
            ProviderTerminalState {
                agent_id: agent_id.to_string(),
                agent_session_id: request.agent_session_id.clone(),
                agent_turn_id: request.agent_turn_id.clone(),
                decision_request_id: request.decision_request_id.clone(),
                request_digest: request.request_digest.to_string(),
                status: status.to_string(),
                reject_reason,
                feedback_id,
            },
        );
        self.persist_provider_lineage_best_effort();
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn record_late_provider_response(&mut self, outcome: &AsyncAgentTurnOutcome) {
        let Some(request) = outcome.prepared_request_context.as_ref() else {
            return;
        };
        self.provider_late_response_diagnostics
            .push_back(ProviderLateResponseDiagnostic {
                agent_id: outcome.agent_id.clone(),
                turn_id: outcome.turn_id.get(),
                agent_turn_id: request.agent_turn_id.clone(),
                decision_request_id: request.decision_request_id.clone(),
                response_digest: outcome
                    .prepared_response_context
                    .as_ref()
                    .map(|response| response.response_digest.to_string()),
            });
        while self.provider_late_response_diagnostics.len() > 32 {
            self.provider_late_response_diagnostics.pop_front();
        }
        self.persist_provider_lineage_best_effort();
    }
}
