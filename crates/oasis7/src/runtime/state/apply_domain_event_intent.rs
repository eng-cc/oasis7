use super::*;

use crate::runtime::{AGENT_INTENT_V2_SCHEMA_VERSION, AgentIntentV2};

const STATUS_ACCEPTED: &str = "accepted";
const STATUS_BLOCKED: &str = "blocked";
const STATUS_SUPERSEDED: &str = "superseded";
const SOURCE_PROVIDER_ADVISORY: &str = "provider_advisory";
const MAX_AGENT_INTENT_SUMMARY_CHARS: usize = 512;
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
    if intent.summary.chars().count() > MAX_AGENT_INTENT_SUMMARY_CHARS {
        return Err(invalid_intent(format!(
            "summary exceeds {} characters",
            MAX_AGENT_INTENT_SUMMARY_CHARS
        )));
    }
    if intent
        .summary
        .chars()
        .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
    {
        return Err(invalid_intent("summary contains a control character"));
    }
    if intent.source.trim().is_empty() {
        return Err(invalid_intent("source cannot be empty"));
    }
    if intent
        .world_id
        .as_deref()
        .is_some_and(|world_id| world_id.trim().is_empty())
    {
        return Err(invalid_intent("world_id cannot be empty when supplied"));
    }
    if intent
        .authority_scope
        .as_deref()
        .is_some_and(|scope| scope.trim().is_empty())
    {
        return Err(invalid_intent(
            "authority_scope cannot be empty when supplied",
        ));
    }
    if intent.replaced_by.as_deref() == Some(intent.intent_id.as_str()) {
        return Err(invalid_intent("replaced_by cannot equal intent_id"));
    }
    Ok(())
}

fn validate_runtime_acceptance_source(intent: &AgentIntentV2) -> Result<(), WorldError> {
    if intent.status == STATUS_ACCEPTED && intent.source == SOURCE_PROVIDER_ADVISORY {
        return Err(invalid_intent(
            "provider_advisory cannot become a runtime-accepted intent",
        ));
    }
    Ok(())
}

fn validate_receipt_reference(
    intent: &AgentIntentV2,
    committed_receipt_event_id: Option<u64>,
) -> Result<(), WorldError> {
    if intent.status != "completed" {
        return Ok(());
    }
    let Some(receipt_ref) = intent.receipt_ref.as_deref() else {
        return Err(invalid_intent(
            "completed transition requires a committed receipt_ref",
        ));
    };
    if intent
        .effect_intent_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_none()
    {
        return Err(invalid_intent(
            "completed transition requires a bound effect_intent_id",
        ));
    }
    let Some(event_id) = receipt_ref
        .strip_prefix("world-event:")
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|event_id| *event_id > 0)
    else {
        return Err(invalid_intent(
            "completed transition requires a non-zero world-event receipt_ref",
        ));
    };
    let Some(committed_receipt_event_id) = committed_receipt_event_id else {
        return Err(invalid_intent(
            "completed transition requires a canonical committed receipt witness",
        ));
    };
    if committed_receipt_event_id != event_id {
        return Err(invalid_intent(format!(
            "receipt witness {} does not match receipt_ref {}",
            committed_receipt_event_id, event_id
        )));
    }
    Ok(())
}

fn immutable_payload_matches(current: &AgentIntentV2, incoming: &AgentIntentV2) -> bool {
    current.schema_version == incoming.schema_version
        && current.agent_id == incoming.agent_id
        && current.intent_id == incoming.intent_id
        && current.kind == incoming.kind
        && current.summary == incoming.summary
        && current.target_id == incoming.target_id
        && current.effect_intent_id == incoming.effect_intent_id
        && current.intent_tick == incoming.intent_tick
        && current.world_id == incoming.world_id
        && current.reorg_epoch == incoming.reorg_epoch
        && current.authority_scope == incoming.authority_scope
        && current.source == incoming.source
        && current.logical_time == incoming.logical_time
        && current.actor_id == incoming.actor_id
        && current.request_digest == incoming.request_digest
}

impl WorldState {
    pub(super) fn apply_domain_event_intent(
        &mut self,
        event: &DomainEvent,
        _now: WorldTime,
        envelope_event_seq: Option<u64>,
        committed_receipt_event_id: Option<u64>,
    ) -> Result<(), WorldError> {
        let intent = match event {
            DomainEvent::AgentIntentAccepted { intent }
            | DomainEvent::AgentIntentReplaced { intent }
            | DomainEvent::AgentIntentTransitioned { intent } => intent,
            _ => unreachable!("apply_domain_event_intent received unsupported event"),
        };
        validate_common_intent(intent)?;
        if let Some(envelope_event_seq) = envelope_event_seq {
            if intent.event_seq != envelope_event_seq {
                return Err(invalid_intent(format!(
                    "event_seq {} does not match world event envelope {}",
                    intent.event_seq, envelope_event_seq
                )));
            }
        }
        validate_receipt_reference(intent, committed_receipt_event_id)?;
        validate_runtime_acceptance_source(intent)?;

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

                if let Some(recorded) = self.agent_intent_ledger.get(&intent.intent_id) {
                    if recorded == intent {
                        return Ok(());
                    }
                    if recorded.status != STATUS_ACCEPTED
                        && immutable_payload_matches(recorded, intent)
                    {
                        // A replay of the original accepted event after a later
                        // disposition is a historical no-op.
                        return Ok(());
                    }
                    return Err(invalid_intent(format!(
                        "intent_id {} conflicts with its durable ledger payload",
                        intent.intent_id
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
                self.agent_intent_ledger
                    .insert(intent.intent_id.clone(), intent.clone());
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

                if let Some(recorded) = self.agent_intent_ledger.get(&intent.intent_id) {
                    if recorded == intent {
                        return Ok(());
                    }
                    if !immutable_payload_matches(recorded, intent) {
                        return Err(invalid_intent(format!(
                            "intent_id {} conflicts with its durable ledger payload",
                            intent.intent_id
                        )));
                    }
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
                        if !immutable_payload_matches(&current, intent) {
                            return Err(invalid_intent(
                                "replacement cannot mutate immutable intent payload",
                            ));
                        }
                        if intent.event_seq <= current.event_seq {
                            return Err(invalid_intent(
                                "replacement event_seq must advance the current intent",
                            ));
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
                self.agent_intent_ledger
                    .insert(intent.intent_id.clone(), intent.clone());
            }
            DomainEvent::AgentIntentTransitioned { .. } => {
                if let Some(recorded) = self.agent_intent_ledger.get(&intent.intent_id) {
                    if recorded == intent {
                        return Ok(());
                    }
                    if !immutable_payload_matches(recorded, intent) {
                        return Err(invalid_intent(
                            "transition cannot mutate immutable intent payload",
                        ));
                    }
                }
                let Some(current) = existing else {
                    return Err(invalid_intent(format!(
                        "transition target {} is not present",
                        intent.intent_id
                    )));
                };
                if current == *intent {
                    return Ok(());
                }
                if current.intent_id != intent.intent_id {
                    return Err(invalid_intent(format!(
                        "transition targets {}, but current intent is {}",
                        intent.intent_id, current.intent_id
                    )));
                }
                if !immutable_payload_matches(&current, intent) {
                    return Err(invalid_intent(
                        "transition cannot mutate immutable intent payload",
                    ));
                }
                if intent.event_seq <= current.event_seq {
                    return Err(invalid_intent(
                        "transition event_seq must advance the current intent",
                    ));
                }
                if TERMINAL_STATUSES.contains(&current.status.as_str()) {
                    return Err(invalid_intent(format!(
                        "terminal intent {} cannot transition again",
                        current.intent_id
                    )));
                }

                let allowed = matches!(
                    (current.status.as_str(), intent.status.as_str()),
                    (STATUS_ACCEPTED, STATUS_BLOCKED)
                        | (STATUS_BLOCKED, STATUS_ACCEPTED)
                        | (STATUS_ACCEPTED, "completed")
                        | (STATUS_BLOCKED, "completed")
                        | (STATUS_ACCEPTED, "rejected")
                        | (STATUS_BLOCKED, "rejected")
                        | (STATUS_ACCEPTED, "expired")
                        | (STATUS_BLOCKED, "expired")
                        | (STATUS_ACCEPTED, "cancelled")
                        | (STATUS_BLOCKED, "cancelled")
                );
                if !allowed {
                    return Err(invalid_intent(format!(
                        "unsupported transition {} -> {}",
                        current.status, intent.status
                    )));
                }
                if intent.status == STATUS_BLOCKED
                    && intent
                        .reason_code
                        .as_deref()
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .is_none()
                {
                    return Err(invalid_intent("blocked transition requires a reason_code"));
                }
                cell.intent = Some(intent.clone());
                self.agent_intent_ledger
                    .insert(intent.intent_id.clone(), intent.clone());
            }
            _ => unreachable!("apply_domain_event_intent received unsupported event"),
        }

        Ok(())
    }
}
