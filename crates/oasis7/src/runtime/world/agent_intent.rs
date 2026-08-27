use super::super::util::sha256_hex;
use super::super::{
    AGENT_INTENT_V2_SCHEMA_VERSION, AgentIntentAuthorityContext, AgentIntentV2, DomainEvent,
    WorldError, WorldEventBody, WorldEventId,
};
use super::World;
use crate::simulator::canonical_agent_intent_summary;

const AGENT_CHAT_INTENT_KIND: &str = "agent_chat";
const AGENT_CHAT_INTENT_SOURCE: &str = "player";
const AGENT_INTENT_STATUS_ACCEPTED: &str = "accepted";
const AGENT_INTENT_STATUS_SUPERSEDED: &str = "superseded";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentIntentRecordOutcome {
    Accepted { event_seq: WorldEventId },
    Replayed { event_seq: WorldEventId },
}

impl AgentIntentRecordOutcome {
    pub fn event_seq(self) -> WorldEventId {
        match self {
            Self::Accepted { event_seq } | Self::Replayed { event_seq } => event_seq,
        }
    }

    pub fn is_replay(self) -> bool {
        matches!(self, Self::Replayed { .. })
    }
}

fn invalid_agent_intent(reason: impl Into<String>) -> WorldError {
    WorldError::ResourceBalanceInvalid {
        reason: format!("invalid AgentIntentV2 handoff: {}", reason.into()),
    }
}

fn derive_agent_chat_intent_id(player_id: &str, agent_id: &str, intent_seq: u64) -> String {
    let payload = format!(
        "agent-chat-intent-v2\0player={}\0agent={}\0intent_seq={}",
        player_id, agent_id, intent_seq
    );
    format!("agent-intent-v2:{}", sha256_hex(payload.as_bytes()))
}

fn derive_agent_chat_request_digest(
    player_id: &str,
    agent_id: &str,
    intent_seq: u64,
    message: &str,
    authority: &AgentIntentAuthorityContext,
) -> String {
    let payload = format!(
        "agent-chat-request-v2\0player={}\0agent={}\0intent_seq={}\0intent_tick={}\0world_id={}\0reorg_epoch={}\0authority_scope={}\0message={}",
        player_id,
        agent_id,
        intent_seq,
        authority
            .intent_tick
            .map_or_else(|| "<none>".to_string(), |value| value.to_string()),
        authority.world_id.as_deref().unwrap_or("<none>"),
        authority
            .reorg_epoch
            .map_or_else(|| "<none>".to_string(), |value| value.to_string()),
        authority.authority_scope.as_deref().unwrap_or("<none>"),
        message
    );
    sha256_hex(payload.as_bytes())
}

fn normalize_authority_context(
    mut authority: AgentIntentAuthorityContext,
) -> Result<AgentIntentAuthorityContext, WorldError> {
    if authority.intent_tick == Some(0) {
        return Err(invalid_agent_intent(
            "intent_tick must be greater than zero when supplied",
        ));
    }
    authority.world_id = authority
        .world_id
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    authority.authority_scope = authority
        .authority_scope
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    Ok(authority)
}

impl World {
    /// Append a player-authenticated agent-chat intent to the canonical runtime journal.
    ///
    /// Authentication, authorization, nonce consumption, and provider enqueueing remain
    /// outside this kernel method.  The caller supplies the already verified player identity
    /// and intent sequence; the runtime owns logical time and journal event sequencing.
    pub fn record_agent_chat_intent(
        &mut self,
        player_id: &str,
        agent_id: &str,
        intent_seq: u64,
        message: &str,
    ) -> Result<WorldEventId, WorldError> {
        Ok(self
            .record_agent_chat_intent_with_authority(
                player_id,
                agent_id,
                intent_seq,
                message,
                AgentIntentAuthorityContext::default(),
            )?
            .event_seq())
    }

    /// Append a player-authenticated agent-chat intent with the authority
    /// identity that must survive durable retry/replay.
    pub fn record_agent_chat_intent_with_authority(
        &mut self,
        player_id: &str,
        agent_id: &str,
        intent_seq: u64,
        message: &str,
        authority: AgentIntentAuthorityContext,
    ) -> Result<AgentIntentRecordOutcome, WorldError> {
        let player_id = player_id.trim();
        let agent_id = agent_id.trim();
        let message = message.trim();
        let authority = normalize_authority_context(authority)?;
        if player_id.is_empty() {
            return Err(invalid_agent_intent("player_id cannot be empty"));
        }
        if agent_id.is_empty() {
            return Err(invalid_agent_intent("agent_id cannot be empty"));
        }
        if intent_seq == 0 {
            return Err(invalid_agent_intent("intent_seq must be greater than zero"));
        }
        if message.is_empty() {
            return Err(invalid_agent_intent("summary cannot be empty"));
        }
        if message.chars().count() > 512 {
            return Err(invalid_agent_intent("summary exceeds 512 characters"));
        }
        if message
            .chars()
            .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
        {
            return Err(invalid_agent_intent("summary contains a control character"));
        }
        let player_summary = canonical_agent_intent_summary(
            AGENT_CHAT_INTENT_KIND,
            AGENT_CHAT_INTENT_SOURCE,
            AGENT_INTENT_STATUS_ACCEPTED,
        )
        .map_err(|error| invalid_agent_intent(error.to_string()))?
        .text;
        if !self.state.agents.contains_key(agent_id) {
            return Err(WorldError::AgentNotFound {
                agent_id: agent_id.to_string(),
            });
        }

        let intent_id = derive_agent_chat_intent_id(player_id, agent_id, intent_seq);
        let request_digest =
            derive_agent_chat_request_digest(player_id, agent_id, intent_seq, message, &authority);
        if let Some(recorded) = self.state.agent_intent_ledger.get(&intent_id) {
            if recorded.actor_id == player_id
                && recorded.request_digest == request_digest
                && recorded.agent_id == agent_id
                && recorded.kind == AGENT_CHAT_INTENT_KIND
                && recorded.summary == player_summary
                && recorded.source == AGENT_CHAT_INTENT_SOURCE
                && recorded.intent_tick == authority.intent_tick
                && recorded.world_id == authority.world_id
                && recorded.reorg_epoch == authority.reorg_epoch
                && recorded.authority_scope == authority.authority_scope
            {
                return Ok(AgentIntentRecordOutcome::Replayed {
                    event_seq: recorded.event_seq,
                });
            }
            return Err(invalid_agent_intent(format!(
                "intent_id {} conflicts with its durable request digest",
                intent_id
            )));
        }
        let now = self.state.time;
        let current = self
            .state
            .agents
            .get(agent_id)
            .and_then(|cell| cell.intent.clone());

        if let Some(existing) = current.as_ref() {
            if existing.intent_id == intent_id {
                if existing.schema_version == AGENT_INTENT_V2_SCHEMA_VERSION
                    && existing.agent_id == agent_id
                    && existing.kind == AGENT_CHAT_INTENT_KIND
                    && existing.summary == player_summary
                    && existing.target_id.is_none()
                    && existing.status == AGENT_INTENT_STATUS_ACCEPTED
                    && existing.source == AGENT_CHAT_INTENT_SOURCE
                    && existing.intent_tick == authority.intent_tick
                    && existing.world_id == authority.world_id
                    && existing.reorg_epoch == authority.reorg_epoch
                    && existing.authority_scope == authority.authority_scope
                    && existing.receipt_ref.is_none()
                    && existing.reason_code.is_none()
                    && existing.reason_summary.is_none()
                    && existing.replaced_by.is_none()
                    && (existing.actor_id.is_empty() || existing.actor_id == player_id)
                    && (existing.request_digest.is_empty()
                        || existing.request_digest == request_digest)
                {
                    // The canonical event is already present.  Returning its recorded journal
                    // position keeps a retry idempotent even when the sidecar ack was lost.
                    return Ok(AgentIntentRecordOutcome::Replayed {
                        event_seq: existing.event_seq,
                    });
                }
                return Err(invalid_agent_intent(format!(
                    "intent_id {} conflicts with its recorded payload",
                    intent_id
                )));
            }
        }

        // An active prior intent is explicitly superseded before the new accepted intent is
        // appended.  Terminal dispositions are already closed and can be replaced directly.
        if let Some(previous) = current {
            let terminal = matches!(
                previous.status.as_str(),
                "completed" | "rejected" | "expired" | "cancelled" | "superseded"
            );
            if !terminal {
                let replacement_event_seq = self.next_event_id.max(1);
                let replacement = AgentIntentV2 {
                    schema_version: AGENT_INTENT_V2_SCHEMA_VERSION,
                    agent_id: previous.agent_id,
                    intent_id: previous.intent_id,
                    kind: previous.kind,
                    summary: previous.summary,
                    target_id: previous.target_id,
                    effect_intent_id: previous.effect_intent_id,
                    intent_tick: previous.intent_tick,
                    world_id: previous.world_id,
                    reorg_epoch: previous.reorg_epoch,
                    authority_scope: previous.authority_scope,
                    status: AGENT_INTENT_STATUS_SUPERSEDED.to_string(),
                    source: previous.source,
                    logical_time: previous.logical_time,
                    event_seq: replacement_event_seq,
                    updated_at: now,
                    receipt_ref: previous.receipt_ref,
                    reason_code: previous.reason_code,
                    reason_summary: previous.reason_summary,
                    replaced_by: Some(intent_id.clone()),
                    actor_id: previous.actor_id,
                    request_digest: previous.request_digest,
                };
                let actual_event_seq = self.append_event(
                    WorldEventBody::Domain(DomainEvent::AgentIntentReplaced {
                        intent: replacement,
                    }),
                    None,
                )?;
                if actual_event_seq != replacement_event_seq {
                    return Err(invalid_agent_intent(format!(
                        "replacement event sequence drifted: expected {}, got {}",
                        replacement_event_seq, actual_event_seq
                    )));
                }
            }
        }

        let accepted_event_seq = self.next_event_id.max(1);
        let accepted = AgentIntentV2 {
            schema_version: AGENT_INTENT_V2_SCHEMA_VERSION,
            agent_id: agent_id.to_string(),
            intent_id,
            kind: AGENT_CHAT_INTENT_KIND.to_string(),
            summary: player_summary,
            target_id: None,
            effect_intent_id: None,
            intent_tick: authority.intent_tick,
            world_id: authority.world_id,
            reorg_epoch: authority.reorg_epoch,
            authority_scope: authority.authority_scope,
            status: AGENT_INTENT_STATUS_ACCEPTED.to_string(),
            source: AGENT_CHAT_INTENT_SOURCE.to_string(),
            logical_time: now,
            event_seq: accepted_event_seq,
            updated_at: now,
            receipt_ref: None,
            reason_code: None,
            reason_summary: None,
            replaced_by: None,
            actor_id: player_id.to_string(),
            request_digest,
        };
        let actual_event_seq = self.append_event(
            WorldEventBody::Domain(DomainEvent::AgentIntentAccepted { intent: accepted }),
            None,
        )?;
        if actual_event_seq != accepted_event_seq {
            return Err(invalid_agent_intent(format!(
                "accepted event sequence drifted: expected {}, got {}",
                accepted_event_seq, actual_event_seq
            )));
        }
        Ok(AgentIntentRecordOutcome::Accepted {
            event_seq: actual_event_seq,
        })
    }
}
