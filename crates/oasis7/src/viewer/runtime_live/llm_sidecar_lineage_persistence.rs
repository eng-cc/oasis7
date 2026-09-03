use super::*;
use crate::runtime::World as RuntimeWorld;
#[cfg(not(target_arch = "wasm32"))]
use crate::simulator::AsyncAgentTurnOutcome;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const PROVIDER_LINEAGE_SCHEMA_VERSION: u16 = 1;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct ProviderTerminalState {
    pub(super) agent_turn_id: String,
    pub(super) decision_request_id: String,
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
    provider_recovery_pending: BTreeMap<String, ProviderRecoveryPending>,
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
    runtime_binding: Option<RuntimeBindingV1>,
    #[serde(default)]
    pending_runtime_wakes: BTreeMap<String, crate::runtime::SchedulerWakeV1>,
}

impl RuntimeLlmSidecar {
    /// Configure a Viewer-owned durable checkpoint for provider transport and
    /// response lineage. Runtime remains the authority for world state and
    /// binding validation; this file only retains work owned by the adapter.
    pub(in crate::viewer::runtime_live) fn configure_provider_lineage_store(
        &mut self,
        path: impl Into<std::path::PathBuf>,
    ) {
        self.provider_lineage_store = Some(path.into());
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
        let checkpoint: PersistedProviderLineageV1 = serde_json::from_slice(&bytes)
            .map_err(|error| format!("provider lineage checkpoint decode failed: {error}"))?;
        if checkpoint.schema_version != PROVIDER_LINEAGE_SCHEMA_VERSION {
            return Err(format!(
                "unsupported provider lineage checkpoint schema {}",
                checkpoint.schema_version
            ));
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
        self.provider_recovery_pending = checkpoint.provider_recovery_pending;
        self.provider_wait_until = checkpoint.provider_wait_until;
        self.provider_feedback_seq = checkpoint.provider_feedback_seq;
        self.provider_feedback_seq_by_session = checkpoint.provider_feedback_seq_by_session;
        self.provider_memory_store = checkpoint.provider_memory_store;
        self.provider_completed_decisions = checkpoint.provider_completed_decisions;
        for (agent_id, decision) in checkpoint.provider_held_decisions {
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
        self.provider_transport_exhausted = checkpoint.provider_transport_exhausted;
        self.provider_terminal_states = checkpoint.provider_terminal_states;
        self.provider_late_response_diagnostics = checkpoint.provider_late_response_diagnostics;
        self.pending_actions = checkpoint.pending_actions;
        self.pending_provider_world_events = checkpoint.pending_provider_world_events;
        self.provider_world_event_quarantine = checkpoint.provider_world_event_quarantine;
        self.pending_runtime_wakes = checkpoint
            .pending_runtime_wakes
            .into_values()
            .map(|wake| (wake.wake_id.clone(), wake))
            .collect();
        self.provider_lineage_binding = current_binding.or(checkpoint.runtime_binding);
        self.provider_lineage_restored = true;

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
                self.pending_actions
                    .values()
                    .map(|pending| pending.agent_id.clone()),
            )
            .chain(self.provider_wait_until.keys().cloned())
            .chain(
                world
                    .cognition_in_flight_wakes()
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|wake| {
                        self.provider_contexts
                            .get(wake.agent_id.as_str())
                            .filter(|context| {
                                context.request_context.agent_turn_id == wake.agent_turn_id
                                    && context.request_context.decision_request_id
                                        == wake.decision_request_id
                            })
                            .map(|_| wake.agent_id)
                    }),
            )
            .collect::<BTreeSet<_>>();
        let orphaned_active_markers = self
            .provider_active_turns
            .iter()
            .map(|(agent_id, active)| {
                let context = self.provider_contexts.get(agent_id.as_str());
                let same_identity = context.is_some_and(|context| {
                    context.request_context.agent_turn_id == active.request_context.agent_turn_id
                        && context.request_context.decision_request_id
                            == active.request_context.decision_request_id
                        && context.request_context.request_digest
                            == active.request_context.request_digest
                });
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
                recovered_orphan = true;
                continue;
            }
            if retained {
                continue;
            }
            self.provider_active_turns.remove(agent_id.as_str());
            if let Some(context) = context {
                if context.request_context.transport_attempt < MAX_PROVIDER_TRANSPORT_ATTEMPTS {
                    // The active marker is the interrupted dispatch record. Use
                    // its matching context rather than retaining an older retry
                    // entry that could belong to another identity.
                    self.provider_retry_contexts.insert(agent_id, context);
                } else {
                    self.provider_transport_exhausted.insert(agent_id);
                }
            }
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
            .filter_map(|(agent_id, terminal)| {
                let context = self.provider_contexts.get(agent_id)?;
                (context.request_context.agent_turn_id == terminal.agent_turn_id
                    && context.request_context.decision_request_id == terminal.decision_request_id)
                    .then_some(agent_id.clone())
            })
            .collect::<Vec<_>>();
        for agent_id in terminal_agents {
            self.provider_contexts.remove(agent_id.as_str());
            self.provider_retry_contexts.remove(agent_id.as_str());
            self.provider_active_turns.remove(agent_id.as_str());
            self.provider_wait_until.remove(agent_id.as_str());
            self.provider_held_decisions.remove(agent_id.as_str());
            self.pending_actions
                .retain(|_, pending| pending.agent_id != agent_id);
        }
        if recovered_orphan {
            // Persist the active-marker removal and retry/exhaustion decision
            // before the next Runtime tick, so a restart cannot strand the
            // same identity again.
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
            provider_recovery_pending: self.provider_recovery_pending.clone(),
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
        self.provider_terminal_states.insert(
            agent_id.to_string(),
            ProviderTerminalState {
                agent_turn_id: context.request_context.agent_turn_id.clone(),
                decision_request_id: context.request_context.decision_request_id.clone(),
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
