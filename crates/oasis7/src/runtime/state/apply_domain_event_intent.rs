use super::*;

use crate::runtime::{AGENT_INTENT_V2_SCHEMA_VERSION, AgentIntentV2};

const STATUS_ACCEPTED: &str = "accepted";
const STATUS_SUPERSEDED: &str = "superseded";
const TERMINAL_STATUSES: &[&str] = &[
    "completed",
    "rejected",
    "expired",
    "cancelled",
    STATUS_SUPERSEDED,
];

fn invalid_intent(reason: impl Into<String>) -> WorldError {
    WorldError::ResourceBalanceInvalid {
        reason: format!("invalid AgentIntentV2: {}", reason.into()),
    }
}

fn validate_common_intent(intent: &AgentIntentV2) -> Result<(), WorldError> {
    if intent.schema_version != AGENT_INTENT_V2_SCHEMA_VERSION {
        return Err(invalid_intent(format!(
            "unsupported schema_version {}, expected {}",
            intent.schema_version, AGENT_INTENT_V2_SCHEMA_VERSION
        )));
    }
    if intent.agent_id.trim().is_empty() {
        return Err(invalid_intent("agent_id cannot be empty"));
    }
    if intent.intent_id.trim().is_empty() {
        return Err(invalid_intent("intent_id cannot be empty"));
    }
    if intent.kind.trim().is_empty() {
        return Err(invalid_intent("kind cannot be empty"));
    }
    if intent.summary.trim().is_empty() {
        return Err(invalid_intent("summary cannot be empty"));
    }
    if intent.source.trim().is_empty() {
        return Err(invalid_intent("source cannot be empty"));
    }
    if intent.replaced_by.as_deref() == Some(intent.intent_id.as_str()) {
        return Err(invalid_intent("replaced_by cannot equal intent_id"));
    }
    Ok(())
}

impl WorldState {
    pub(super) fn apply_domain_event_intent(
        &mut self,
        event: &DomainEvent,
        _now: WorldTime,
    ) -> Result<(), WorldError> {
        let intent = match event {
            DomainEvent::AgentIntentAccepted { intent }
            | DomainEvent::AgentIntentReplaced { intent } => intent,
            _ => unreachable!("apply_domain_event_intent received unsupported event"),
        };
        validate_common_intent(intent)?;

        if !self.agents.contains_key(&intent.agent_id) {
            return Err(WorldError::AgentNotFound {
                agent_id: intent.agent_id.clone(),
            });
        }

        let cell = self
            .agents
            .get_mut(&intent.agent_id)
            .expect("agent existence checked immediately above");
        let existing = cell.intent.clone();

        match event {
            DomainEvent::AgentIntentAccepted { .. } => {
                if intent.status != STATUS_ACCEPTED {
                    return Err(invalid_intent(format!(
                        "accepted event must carry status {STATUS_ACCEPTED}, got {}",
                        intent.status
                    )));
                }

                match existing {
                    None => cell.intent = Some(intent.clone()),
                    Some(current) if current == *intent => {
                        // Exact replay is intentionally a no-op: do not refresh
                        // timestamps or mutate Activity/Receipt state.
                    }
                    Some(current) if current.intent_id == intent.intent_id => {
                        return Err(invalid_intent(format!(
                            "intent_id {} conflicts with its recorded payload",
                            intent.intent_id
                        )));
                    }
                    Some(current) if TERMINAL_STATUSES.contains(&current.status.as_str()) => {
                        cell.intent = Some(intent.clone());
                    }
                    Some(current) => {
                        return Err(invalid_intent(format!(
                            "active intent {} must be explicitly replaced before accepting {}",
                            current.intent_id, intent.intent_id
                        )));
                    }
                }
            }
            DomainEvent::AgentIntentReplaced { .. } => {
                if intent.status != STATUS_SUPERSEDED {
                    return Err(invalid_intent(format!(
                        "replacement event must carry status {STATUS_SUPERSEDED}, got {}",
                        intent.status
                    )));
                }
                let Some(replaced_by) = intent.replaced_by.as_deref() else {
                    return Err(invalid_intent(
                        "replacement event requires a non-empty replaced_by",
                    ));
                };
                if replaced_by.trim().is_empty() {
                    return Err(invalid_intent(
                        "replacement event requires a non-empty replaced_by",
                    ));
                }

                match existing {
                    Some(current) if current == *intent => {
                        // Exact replay of the replacement is a no-op.
                    }
                    Some(current) if current.intent_id == intent.intent_id => {
                        if TERMINAL_STATUSES.contains(&current.status.as_str()) {
                            return Err(invalid_intent(format!(
                                "terminal intent {} cannot be replaced again",
                                current.intent_id
                            )));
                        }
                        cell.intent = Some(intent.clone());
                    }
                    Some(current) => {
                        return Err(invalid_intent(format!(
                            "replacement targets {}, but current intent is {}",
                            intent.intent_id, current.intent_id
                        )));
                    }
                    None => {
                        return Err(invalid_intent(format!(
                            "replacement target {} is not present",
                            intent.intent_id
                        )));
                    }
                }
            }
            _ => unreachable!("apply_domain_event_intent received unsupported event"),
        }

        Ok(())
    }
}
