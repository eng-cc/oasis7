use super::*;
use sha2::{Digest, Sha256};

/// A provider world-event notification is durable until the actor accepts it.
/// The Runtime chain link may advance its watermark after this adapter call,
/// so dropping a full/unavailable mailbox would otherwise lose the event.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(super) struct RuntimePendingProviderWorldEvent {
    pub(super) agent_id: String,
    pub(super) event: WorldEvent,
}

fn provider_world_event_key(agent_id: &str, event: &WorldEvent) -> String {
    let mut digest = Sha256::new();
    digest.update(b"oasis7.viewer.provider-world-event.v1");
    digest.update(agent_id.as_bytes());
    digest.update([0]);
    digest.update(serde_json::to_vec(event).expect("WorldEvent must remain serializable"));
    format!("provider-world-event-{}", hex::encode(digest.finalize()))
}

impl RuntimeLlmSidecar {
    /// Deliver an authoritative production completion to its owning Agent.
    /// Runtime-live receives the full runtime receipt through `mapped_event`,
    /// while the simulator behavior owns coverage state and replay idempotence.
    pub(in crate::viewer::runtime_live) fn notify_recipe_completion_if_needed(
        &mut self,
        runtime_event: &RuntimeWorldEvent,
        mapped_event: WorldEvent,
    ) {
        let RuntimeWorldEventBody::Domain(RuntimeDomainEvent::RecipeCompleted {
            requester_agent_id,
            ..
        }) = &runtime_event.body
        else {
            return;
        };

        let provider_runner = self
            .runner
            .as_ref()
            .is_some_and(|runner| matches!(runner, RuntimeDecisionRunner::ProviderBacked(_)));
        let provider_expected =
            self.runner.is_none() && self.is_llm_mode() && env_requests_provider_backend();
        if provider_runner || provider_expected {
            self.enqueue_provider_world_event(requester_agent_id, mapped_event);
        } else if let Some(RuntimeDecisionRunner::Builtin(runner)) = self.runner.as_mut() {
            if let Some(agent) = runner.get_mut(requester_agent_id.as_str()) {
                agent.behavior.on_event(&mapped_event);
            }
        }
    }

    fn enqueue_provider_world_event(&mut self, agent_id: &str, event: WorldEvent) {
        let key = provider_world_event_key(agent_id, &event);
        self.pending_provider_world_events
            .entry(key)
            .or_insert_with(|| RuntimePendingProviderWorldEvent {
                agent_id: agent_id.to_string(),
                event,
            });
        // Persist before trying the actor. If the process stops after the
        // actor accepts the command but before this checkpoint is cleared,
        // the duplicate is intentional: this boundary is at-least-once.
        self.persist_provider_lineage_best_effort();
        self.flush_pending_provider_world_events();
    }

    fn dispatch_provider_world_event(
        &mut self,
        agent_id: &str,
        event: WorldEvent,
    ) -> Result<(), String> {
        match self.runner.as_mut() {
            #[cfg(not(target_arch = "wasm32"))]
            Some(RuntimeDecisionRunner::ProviderBacked(runner)) => runner
                .notify_world_event(agent_id, event)
                .map_err(|error| error.to_string()),
            #[cfg(target_arch = "wasm32")]
            Some(RuntimeDecisionRunner::ProviderBacked(runner)) => runner
                .get_mut(agent_id)
                .map(|agent| agent.behavior.on_event(&event))
                .ok_or_else(|| format!("provider actor unavailable: {agent_id}")),
            Some(RuntimeDecisionRunner::Builtin(_)) => {
                Err("provider world event requested from builtin runner".to_string())
            }
            None => Err("provider runner not initialized".to_string()),
        }
    }

    pub(super) fn flush_pending_provider_world_events(&mut self) {
        let pending = self
            .pending_provider_world_events
            .iter()
            .map(|(key, pending)| (key.clone(), pending.agent_id.clone(), pending.event.clone()))
            .collect::<Vec<_>>();
        let mut delivered = false;
        for (key, agent_id, event) in pending {
            match self.dispatch_provider_world_event(agent_id.as_str(), event) {
                Ok(()) => {
                    self.pending_provider_world_events.remove(key.as_str());
                    delivered = true;
                }
                Err(error) => {
                    tracing::warn!(
                        agent_id,
                        error,
                        "provider world event delivery deferred for retry"
                    );
                }
            }
        }
        if delivered {
            self.persist_provider_lineage_best_effort();
        }
    }
}
