use oasis7::simulator::{Action, ActionCatalogEntry, DecisionRequest, ProviderDecision};
use serde::Deserialize;
use serde_json::{Value, json};

use super::support::{estimated_current_location_id, nearest_reachable_non_current_location_id};
use super::{AgentInvocation, DEFAULT_PROVIDER_AGENT_PROFILE};

pub(super) fn deterministic_mock_decision(request: &DecisionRequest) -> ProviderDecision {
    let agent_id = request.observation.agent_id.clone();
    if action_catalog_contains(&request.observation.action_catalog, "move_agent") {
        if let Some(to) =
            nearest_reachable_non_current_location_id(&request.observation.observation)
        {
            return ProviderDecision::Act {
                action_ref: "move_agent".to_string(),
                action: Action::MoveAgent { agent_id, to },
            };
        }
    }
    if action_catalog_contains(&request.observation.action_catalog, "inspect_target") {
        if let Some(target) = request.observation.observation.interaction_targets.first() {
            return ProviderDecision::Act {
                action_ref: "inspect_target".to_string(),
                action: Action::InspectTarget {
                    agent_id,
                    target_kind: target.target_kind.clone(),
                    target_id: target.target_ref.clone(),
                },
            };
        }
        if let Some(entity) = request.observation.observation.nearby_entities.first() {
            return ProviderDecision::Act {
                action_ref: "inspect_target".to_string(),
                action: Action::InspectTarget {
                    agent_id,
                    target_kind: entity.kind.clone(),
                    target_id: entity.entity_ref.clone(),
                },
            };
        }
    }
    ProviderDecision::Wait
}

fn action_catalog_contains(action_catalog: &[ActionCatalogEntry], action_ref: &str) -> bool {
    action_catalog
        .iter()
        .any(|entry| entry.action_ref.trim() == action_ref)
}

pub(super) fn provider_decision_label(decision: &ProviderDecision) -> &'static str {
    match decision {
        ProviderDecision::Wait => "wait",
        ProviderDecision::WaitTicks { .. } => "wait_ticks",
        ProviderDecision::Act { action_ref, .. } => match action_ref.as_str() {
            "move_agent" => "move_agent",
            "speak_to_nearby" => "speak_to_nearby",
            "inspect_target" => "inspect_target",
            "simple_interact" => "simple_interact",
            _ => "act",
        },
    }
}

pub(super) fn build_decision_prompt(
    request: &DecisionRequest,
    recent_feedback: &[String],
) -> String {
    let observation = &request.observation;
    let nearby_agents = observation
        .observation
        .nearby_entities
        .iter()
        .filter(|entity| entity.kind == "agent")
        .map(|entity| {
            json!({
                "entity_ref": entity.entity_ref,
                "relation": entity.relation,
                "relative_hint": entity.relative_hint,
                "interaction_hint": entity.interaction_hint,
            })
        })
        .collect::<Vec<_>>();
    let nearby_locations = observation
        .observation
        .nearby_entities
        .iter()
        .filter(|entity| entity.kind == "location")
        .map(|entity| {
            json!({
                "entity_ref": entity.entity_ref,
                "relation": entity.relation,
                "relative_hint": entity.relative_hint,
                "interaction_hint": entity.interaction_hint,
            })
        })
        .collect::<Vec<_>>();
    let resources = observation.observation.self_state.resource_summary.clone();
    let action_catalog = observation
        .action_catalog
        .iter()
        .map(|entry| json!({"action_ref": entry.action_ref, "summary": entry.summary}))
        .collect::<Vec<_>>();
    let current_location_id =
        estimated_current_location_id(&observation.observation).map(str::to_string);
    let nearest_non_current_location_id =
        nearest_reachable_non_current_location_id(&observation.observation);
    let can_legally_move_to_visible_non_current_location = observation
        .action_catalog
        .iter()
        .any(|entry| entry.action_ref == "move_agent")
        && nearest_non_current_location_id.is_some();
    let profile_guidance = profile_guidance(
        request,
        current_location_id.as_deref(),
        nearest_non_current_location_id.as_deref(),
    );
    let context = json!({
        "agent_profile": request.agent_profile,
        "mode": observation.mode.as_str(),
        "agent_id": observation.agent_id,
        "world_time": observation.world_time,
        "timeout_budget_ms": request.timeout_budget_ms,
        "self_state": observation.observation.self_state,
        "mission_context": observation.observation.mission_context,
        "resources": resources,
        "nearby_agents": nearby_agents,
        "nearby_locations": nearby_locations,
        "recent_events": observation.observation.recent_events,
        "local_navigation_graph": observation.observation.local_navigation_graph,
        "interaction_targets": observation.observation.interaction_targets,
        "current_location_id": current_location_id,
        "nearest_non_current_location_id": nearest_non_current_location_id,
        "can_legally_move_to_visible_non_current_location": can_legally_move_to_visible_non_current_location,
        "recent_event_summary": observation.recent_event_summary,
        "memory_summary": observation.memory_summary,
        "recent_feedback": recent_feedback,
        "action_catalog": action_catalog,
    });
    format!(
        concat!(
            "You are controlling a low-frequency NPC in oasis7. ",
            "Return ONLY one minified JSON object with no markdown, no prose, and no code fences. ",
            "Respect the profile and only use actions from action_catalog. ",
            "For profile oasis7_p0_low_freq_npc, prefer legal forward progress over idle waiting. ",
            "If move_agent is legal and a visible non-current location exists, prefer move_agent to the nearest such location. ",
            "Never choose move_agent.to equal to the current location. ",
            "Choose wait or wait_ticks only when no legal progress action exists or a recoverable error just happened. ",
            "Output schema:\n",
            "{{\"decision\":\"wait\"}}\n",
            "{{\"decision\":\"wait_ticks\",\"ticks\":<u64>}}\n",
            "{{\"decision\":\"act\",\"action_ref\":\"move_agent\",\"args\":{{\"to\":\"<location_id>\"}}}}\n",
            "{{\"decision\":\"act\",\"action_ref\":\"speak_to_nearby\",\"args\":{{\"message\":\"<short text>\",\"target_agent_id\":\"<optional agent id>\"}}}}\n",
            "{{\"decision\":\"act\",\"action_ref\":\"inspect_target\",\"args\":{{\"target_kind\":\"agent|location|object\",\"target_id\":\"<id>\"}}}}\n",
            "{{\"decision\":\"act\",\"action_ref\":\"simple_interact\",\"args\":{{\"target_kind\":\"agent|location|object\",\"target_id\":\"<id>\",\"interaction\":\"<verb>\"}}}}\n",
            "Profile guidance:\n{}\n",
            "Do not invent ids. Keep messages short. Context JSON follows:\n{}"
        ),
        profile_guidance, context
    )
}

fn profile_guidance(
    request: &DecisionRequest,
    current_location_id: Option<&str>,
    nearest_non_current_location_id: Option<&str>,
) -> String {
    match request.agent_profile.as_deref() {
        Some(DEFAULT_PROVIDER_AGENT_PROFILE) => {
            let current_location = current_location_id.unwrap_or("unknown");
            let next_location = nearest_non_current_location_id.unwrap_or("none");
            format!(
                concat!(
                    "- Goal priority: keep making low-risk forward progress and avoid repeated idle wait.
",
                    "- If a visible non-current location exists, moving there counts as progress for patrol parity.
",
                    "- Current visible location: {current_location}.
",
                    "- Preferred next visible non-current location: {next_location}.
",
                    "- Do not output wait if move_agent to that preferred location is legal and no recoverable failure blocks it.
",
                    "- Use inspect_target before wait when the target is ambiguous.
"
                ),
                current_location = current_location,
                next_location = next_location,
            )
        }
        Some(profile) => format!("- Active profile: {profile}"),
        None => "- No explicit profile provided; stay conservative and legal.".to_string(),
    }
}

const DEFAULT_MIN_PROVIDER_TIMEOUT_MS: u64 = 15_000;

fn minimum_provider_timeout_ms() -> u64 {
    std::env::var("OASIS7_PROVIDER_LOCAL_BRIDGE_MIN_TIMEOUT_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_MIN_PROVIDER_TIMEOUT_MS)
}

pub(super) fn timeout_seconds_from_budget(timeout_budget_ms: u64) -> u64 {
    let timeout_budget_ms = timeout_budget_ms
        .max(1000)
        .max(minimum_provider_timeout_ms());
    ((timeout_budget_ms + 999) / 1000).max(1)
}

pub(super) fn build_gateway_agent_params(
    invocation: &AgentInvocation,
) -> Result<String, serde_json::Error> {
    serde_json::to_string(&json!({
        "message": invocation.prompt,
        "agentId": invocation.agent_id,
        "sessionKey": invocation.session_key,
        "deliver": false,
        "channel": "webchat",
        "lane": "nested",
        "thinking": invocation.thinking,
        "timeout": invocation.timeout_seconds,
        "idempotencyKey": invocation.idempotency_key,
    }))
}

pub(super) fn build_session_key(request: &DecisionRequest, provider_agent_id: &str) -> String {
    let raw = format!(
        "agent:{provider_agent_id}:subagent:world-simulator:{}:{}:{}",
        request
            .provider_config_ref
            .as_deref()
            .unwrap_or("default-config"),
        request
            .agent_profile
            .as_deref()
            .unwrap_or("default-profile"),
        request.observation.agent_id,
    );
    raw.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == ':' {
                ch
            } else {
                '-'
            }
        })
        .collect()
}

#[derive(Debug, Deserialize)]
struct ModelDecisionEnvelope {
    decision: String,
    #[serde(default)]
    ticks: Option<u64>,
    #[serde(default)]
    action_ref: Option<String>,
    #[serde(default)]
    args: Option<Value>,
}

pub(super) fn parse_model_decision(
    agent_id: &str,
    request: &DecisionRequest,
    raw_text: &str,
) -> Result<(ProviderDecision, u32), String> {
    let mut schema_repair_count = 0;
    let mut candidate = raw_text.trim().to_string();
    if candidate.starts_with("```") {
        candidate = strip_code_fence(candidate.as_str());
        schema_repair_count += 1;
    }
    let envelope: ModelDecisionEnvelope = match serde_json::from_str(candidate.as_str()) {
        Ok(value) => value,
        Err(_) => {
            let repaired = extract_json_object(candidate.as_str()).ok_or_else(|| {
                format!(
                    "no JSON object found in model output: {}",
                    summarize_text(raw_text, 240)
                )
            })?;
            schema_repair_count += 1;
            serde_json::from_str(repaired.as_str()).map_err(|err| {
                format!(
                    "parse repaired model output failed: {err}; raw={}",
                    summarize_text(raw_text, 240)
                )
            })?
        }
    };
    let decision = match envelope.decision.as_str() {
        "wait" => ProviderDecision::Wait,
        "wait_ticks" => ProviderDecision::WaitTicks {
            ticks: envelope
                .ticks
                .filter(|ticks| *ticks > 0)
                .ok_or_else(|| "wait_ticks requires ticks > 0".to_string())?,
        },
        "act" => {
            let action_ref = envelope
                .action_ref
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| "act requires non-empty action_ref".to_string())?;
            if !request
                .observation
                .action_catalog
                .iter()
                .any(|entry| entry.action_ref == action_ref)
            {
                return Err(format!(
                    "action_ref `{action_ref}` not present in action_catalog"
                ));
            }
            let args = envelope.args.unwrap_or(Value::Null);
            ProviderDecision::Act {
                action_ref: action_ref.to_string(),
                action: build_action_from_args(agent_id, action_ref, &args)?,
            }
        }
        other => return Err(format!("unsupported decision `{other}`")),
    };
    Ok((decision, schema_repair_count))
}

fn build_action_from_args(
    agent_id: &str,
    action_ref: &str,
    args: &Value,
) -> Result<Action, String> {
    match action_ref {
        "move_agent" => Ok(Action::MoveAgent {
            agent_id: agent_id.to_string(),
            to: required_string(args, "to")?,
        }),
        "speak_to_nearby" => Ok(Action::SpeakToNearby {
            agent_id: agent_id.to_string(),
            message: required_string(args, "message")?,
            target_agent_id: optional_string(args, "target_agent_id"),
        }),
        "inspect_target" => Ok(Action::InspectTarget {
            agent_id: agent_id.to_string(),
            target_kind: required_string(args, "target_kind")?,
            target_id: required_string(args, "target_id")?,
        }),
        "simple_interact" => Ok(Action::SimpleInteract {
            agent_id: agent_id.to_string(),
            target_kind: required_string(args, "target_kind")?,
            target_id: required_string(args, "target_id")?,
            interaction: required_string(args, "interaction")?,
        }),
        other => Err(format!("unsupported action_ref `{other}`")),
    }
}

pub(super) fn apply_profile_guardrails(
    request: &DecisionRequest,
    decision: ProviderDecision,
) -> (ProviderDecision, Option<String>) {
    if !request
        .observation
        .memory_summary
        .as_deref()
        .unwrap_or_default()
        .contains("goal=巡游移动")
    {
        return (decision, None);
    }
    let current_location_id = estimated_current_location_id(&request.observation.observation);
    let preferred_location =
        nearest_reachable_non_current_location_id(&request.observation.observation);
    let Some(preferred_location) = preferred_location else {
        return (decision, None);
    };
    let move_available = request
        .observation
        .action_catalog
        .iter()
        .any(|entry| entry.action_ref == "move_agent");
    if !move_available {
        return (decision, None);
    }
    match decision {
        ProviderDecision::Wait | ProviderDecision::WaitTicks { .. } => (
            ProviderDecision::Act {
                action_ref: "move_agent".to_string(),
                action: Action::MoveAgent {
                    agent_id: request.observation.agent_id.clone(),
                    to: preferred_location.clone(),
                },
            },
            Some(format!(
                "profile_guardrail_reroute: patrol goal converted passive decision into move_agent(to={})",
                preferred_location
            )),
        ),
        ProviderDecision::Act { action_ref, action } => match action {
            Action::MoveAgent { agent_id, to }
                if to == current_location_id.unwrap_or_default()
                    || !request
                        .observation
                        .observation
                        .nearby_entities
                        .iter()
                        .any(|entity| entity.kind == "location" && entity.entity_ref == to) =>
            {
                (
                    ProviderDecision::Act {
                        action_ref: "move_agent".to_string(),
                        action: Action::MoveAgent {
                            agent_id,
                            to: preferred_location.clone(),
                        },
                    },
                    Some(format!(
                        "profile_guardrail_reroute: invalid move target replaced with move_agent(to={})",
                        preferred_location
                    )),
                )
            }
            _ => (ProviderDecision::Act { action_ref, action }, None),
        },
    }
}

fn required_string(value: &Value, key: &str) -> Result<String, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| format!("missing non-empty string field `{key}`"))
}

fn optional_string(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn strip_code_fence(text: &str) -> String {
    let stripped = text.trim().trim_start_matches("```");
    let stripped = stripped.strip_prefix("json").unwrap_or(stripped);
    stripped.trim().trim_end_matches("```").trim().to_string()
}

fn extract_json_object(text: &str) -> Option<String> {
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    if end <= start {
        return None;
    }
    Some(text[start..=end].trim().to_string())
}

pub(super) fn summarize_text(text: &str, max_chars: usize) -> String {
    let trimmed = text.trim();
    if trimmed.chars().count() <= max_chars {
        trimmed.to_string()
    } else {
        let prefix = trimmed.chars().take(max_chars).collect::<String>();
        format!("{}…", prefix)
    }
}
