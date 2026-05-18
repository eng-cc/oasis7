use super::*;
use oasis7::simulator::{
    ActionCatalogEntry, ObservationEnvelope, ProviderExecutionMode, ProviderInteractionTarget,
    ProviderMissionContext, ProviderNavigationNode, ProviderNearbyEntity, ProviderObservation,
    ProviderRecentEvent, ProviderSelfState, DEFAULT_PROVIDER_ACTION_SCHEMA_VERSION,
    DEFAULT_PROVIDER_OBSERVATION_SCHEMA_VERSION,
};
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
struct FakeInvoker {
    response: Result<AgentInvocationOutput, String>,
}

impl AgentInvoker for FakeInvoker {
    fn invoke(&self, _invocation: AgentInvocation) -> Result<AgentInvocationOutput, String> {
        self.response.clone()
    }
}

#[test]
fn parse_model_decision_accepts_code_fence_and_maps_move_agent() {
    let request = sample_request();
    let raw = "```json\n{\"decision\":\"act\",\"action_ref\":\"move_agent\",\"args\":{\"to\":\"loc-2\"}}\n```";
    let (decision, repairs) =
        parse_model_decision("agent-1", &request, raw).expect("parse decision");
    assert_eq!(repairs, 1);
    assert_eq!(
        decision,
        ProviderDecision::Act {
            action_ref: "move_agent".to_string(),
            action: Action::MoveAgent {
                agent_id: "agent-1".to_string(),
                to: "loc-2".to_string(),
            },
        }
    );
}

#[test]
fn handle_decision_returns_wait_without_provider_error_on_invalid_json() {
    let state = ProviderState::new(CliOptions::default()).expect("provider state");
    let invoker = FakeInvoker {
        response: Ok(AgentInvocationOutput {
            prompt: "prompt".to_string(),
            text: "not-json".to_string(),
            provider_version: Some("provider/model".to_string()),
            duration_ms: Some(42),
            prompt_tokens: Some(11),
            completion_tokens: Some(7),
            total_tokens: Some(18),
            route_note: None,
        }),
    };
    let response = state.handle_decision(sample_request(), None, &invoker);
    assert_eq!(response.decision, ProviderDecision::Wait);
    assert!(response.provider_error.is_none());
    assert_eq!(response.trace_payload.schema_repair_count, 1);
    assert!(response
        .trace_payload
        .output_summary
        .as_deref()
        .unwrap_or_default()
        .contains("invalid_model_output"));
}

#[test]
fn handle_decision_surfaces_gateway_failure_as_provider_error() {
    let state = ProviderState::new(CliOptions::default()).expect("provider state");
    let invoker = FakeInvoker {
        response: Err("gateway down".to_string()),
    };
    let response = state.handle_decision(sample_request(), None, &invoker);
    assert_eq!(response.decision, ProviderDecision::Wait);
    assert_eq!(
        response
            .provider_error
            .as_ref()
            .expect("provider_error")
            .code,
        "provider_gateway_unreachable"
    );
}

#[test]
fn build_decision_prompt_embeds_preferred_visible_move_hint() {
    let prompt = build_decision_prompt(&sample_request(), &[]);
    assert!(prompt.contains("Preferred next visible non-current location: loc-2"));
    assert!(prompt.contains("Do not output wait if move_agent to that preferred location is legal"));
}

#[test]
fn build_session_key_uses_provider_config_ref_to_avoid_cross_talk() {
    let mut request = sample_request();
    request.provider_config_ref = Some("provider://loopback-http/parity/run-a/agent-0".to_string());
    let session_a = build_session_key(&request, "main");
    request.provider_config_ref = Some("provider://loopback-http/parity/run-b/agent-0".to_string());
    let session_b = build_session_key(&request, "main");
    assert_ne!(session_a, session_b);
    assert!(session_a.contains("run-a"));
    assert!(session_b.contains("run-b"));
}

#[test]
fn parse_provider_agent_command_output_accepts_local_shape() {
    let parsed = agent_output_from_json(
        "prompt".to_string(),
        r#"{"payloads":[{"text":"{\"decision\":\"wait\"}"}],"meta":{"durationMs":2484,"agentMeta":{"provider":"custom-right-codes","model":"gpt-5.4","promptTokens":9885,"usage":{"output":9,"total":9894}}}}"#,
        None,
    )
    .expect("parse local output");
    assert_eq!(parsed.text, "{\"decision\":\"wait\"}");
    assert_eq!(parsed.duration_ms, Some(2484));
    assert_eq!(
        parsed.provider_version.as_deref(),
        Some("custom-right-codes/gpt-5.4")
    );
    assert_eq!(parsed.prompt_tokens, Some(9885));
    assert_eq!(parsed.completion_tokens, Some(9));
    assert_eq!(parsed.total_tokens, Some(9894));
}

#[test]
fn local_session_id_from_session_key_hashes_invalid_chars() {
    let session_id = local_session_id_from_session_key(
        "agent:oasis7_provider_agent:subagent:world-simulator:manual:agent-1",
    );
    assert!(session_id.starts_with("ws-"));
    assert!(session_id
        .chars()
        .all(|ch| ch.is_ascii_hexdigit() || ch == '-' || ch == 'w' || ch == 's'));
    assert_eq!(session_id.len(), 67);
}

#[test]
fn should_fallback_to_local_agent_matches_gateway_timeout() {
    assert!(should_fallback_to_local_agent(
        "provider-cli gateway call agent exited with status 1: stderr=Gateway call failed: Error: gateway timeout after 17000ms stdout="
    ));
    assert!(!should_fallback_to_local_agent("provider agent not found"));
}

#[test]
fn build_gateway_agent_params_uses_session_key_and_timeout() {
    let invocation = AgentInvocation {
        provider_cli_bin: "provider-cli".to_string(),
        agent_id: "main".to_string(),
        thinking: "off".to_string(),
        session_key: "agent:main:subagent:world-simulator:test".to_string(),
        timeout_seconds: 15,
        prompt: "{\"action\":\"wait\"}".to_string(),
        idempotency_key: "idem-1".to_string(),
        route_label: None,
    };
    let params = build_gateway_agent_params(&invocation).expect("params");
    let value: Value = serde_json::from_str(params.as_str()).expect("json");
    assert_eq!(
        value.get("sessionKey").and_then(Value::as_str),
        Some("agent:main:subagent:world-simulator:test")
    );
    assert_eq!(value.get("agentId").and_then(Value::as_str), Some("main"));
    assert_eq!(
        value.get("channel").and_then(Value::as_str),
        Some("webchat")
    );
    assert_eq!(value.get("lane").and_then(Value::as_str), Some("nested"));
    assert_eq!(value.get("thinking").and_then(Value::as_str), Some("off"));
    assert_eq!(value.get("timeout").and_then(Value::as_u64), Some(15));
    assert_eq!(
        value.get("idempotencyKey").and_then(Value::as_str),
        Some("idem-1")
    );
}

#[test]
fn apply_profile_guardrails_reroutes_patrol_wait_to_move() {
    let mut request = sample_request();
    request.observation.memory_summary = Some(
        "goal=巡游移动; prefer move_agent to nearest visible non-current location".to_string(),
    );
    let (decision, note) = apply_profile_guardrails(&request, ProviderDecision::Wait);
    assert_eq!(
        decision,
        ProviderDecision::Act {
            action_ref: "move_agent".to_string(),
            action: Action::MoveAgent {
                agent_id: "agent-1".to_string(),
                to: "loc-2".to_string(),
            },
        }
    );
    assert!(note
        .unwrap_or_default()
        .contains("profile_guardrail_reroute"));
}

#[test]
fn validate_profile_accepts_oasis7_and_rejects_removed_profile_alias() {
    assert_eq!(validate_profile(Some(DEFAULT_PROVIDER_AGENT_PROFILE)), None);
    assert_eq!(
        validate_profile(Some("legacy_p0_low_freq_npc")),
        Some(format!(
            "unsupported agent_profile `legacy_p0_low_freq_npc`; expected {DEFAULT_PROVIDER_AGENT_PROFILE}"
        ))
    );
}

#[test]
fn handle_decision_rejects_unsupported_schema_version() {
    let state = ProviderState::new(CliOptions::default()).expect("provider state");
    let invoker = FakeInvoker {
        response: Err("should not invoke".to_string()),
    };
    let mut request = sample_request();
    request.observation.observation_schema_version = "oc_dual_obs_v0".to_string();
    let response = state.handle_decision(request, None, &invoker);
    assert_eq!(response.decision, ProviderDecision::Wait);
    assert_eq!(
        response
            .provider_error
            .as_ref()
            .expect("provider_error")
            .code,
        "unsupported_schema_version"
    );
}

#[test]
fn handle_decision_rejects_unsupported_action_schema_version() {
    let state = ProviderState::new(CliOptions::default()).expect("provider state");
    let invoker = FakeInvoker {
        response: Err("should not invoke".to_string()),
    };
    let mut request = sample_request();
    request.observation.action_schema_version = "oc_dual_act_v0".to_string();
    let response = state.handle_decision(request, None, &invoker);
    assert_eq!(response.decision, ProviderDecision::Wait);
    assert_eq!(
        response
            .provider_error
            .as_ref()
            .expect("provider_error")
            .code,
        "unsupported_schema_version"
    );
}

#[test]
fn handle_decision_rejects_player_parity_request_with_headless_fields() {
    let state = ProviderState::new(CliOptions::default()).expect("provider state");
    let invoker = FakeInvoker {
        response: Err("should not invoke".to_string()),
    };
    let mut request = sample_request();
    request.observation.mode = ProviderExecutionMode::PlayerParity;
    let response = state.handle_decision(request, None, &invoker);
    assert_eq!(response.decision, ProviderDecision::Wait);
    assert_eq!(
        response
            .provider_error
            .as_ref()
            .expect("provider_error")
            .code,
        "mode_observation_mismatch"
    );
}

#[test]
fn parse_options_accepts_auth_route_from_bearer() {
    let options = parse_options(["--auth-route-from-bearer"].into_iter()).expect("parse options");
    assert!(options.auth_route_from_bearer);
    assert!(options.auth_route_map.is_empty());
    assert!(options.auth_token.is_none());
}

fn sample_request() -> DecisionRequest {
    DecisionRequest {
        observation: ObservationEnvelope {
            agent_id: "agent-1".to_string(),
            world_time: 7,
            mode: ProviderExecutionMode::HeadlessAgent,
            observation_schema_version: DEFAULT_PROVIDER_OBSERVATION_SCHEMA_VERSION.to_string(),
            action_schema_version: DEFAULT_PROVIDER_ACTION_SCHEMA_VERSION.to_string(),
            environment_class: Some("provider_local_bridge".to_string()),
            fallback_reason: None,
            observation: ProviderObservation {
                self_state: ProviderSelfState {
                    location_ref: "loc-1".to_string(),
                    pose_hint: "grid_pose=(0, 0, 0) visibility_range_cm=1000".to_string(),
                    status_flags: Vec::new(),
                    resource_summary: BTreeMap::from([(String::from("Electricity"), 24)]),
                },
                mission_context: ProviderMissionContext {
                    goal_summary: "prefer safe low-frequency actions".to_string(),
                    blocked_reason: None,
                },
                nearby_entities: vec![
                    ProviderNearbyEntity {
                        entity_ref: "loc-1".to_string(),
                        kind: "location".to_string(),
                        relation: "current_location".to_string(),
                        relative_hint: "current visible location".to_string(),
                        interaction_hint: None,
                    },
                    ProviderNearbyEntity {
                        entity_ref: "loc-2".to_string(),
                        kind: "location".to_string(),
                        relation: "reachable_location".to_string(),
                        relative_hint: "reachable location distance_cm=100".to_string(),
                        interaction_hint: Some("move_agent".to_string()),
                    },
                    ProviderNearbyEntity {
                        entity_ref: "agent-2".to_string(),
                        kind: "agent".to_string(),
                        relation: "nearby_agent".to_string(),
                        relative_hint: "nearby agent distance_cm=100".to_string(),
                        interaction_hint: Some("inspect_target".to_string()),
                    },
                ],
                recent_events: vec![ProviderRecentEvent {
                    event_ref: "recent_event_0".to_string(),
                    kind: "event_summary".to_string(),
                    summary: "event: AgentRegistered".to_string(),
                    age_ticks: 0,
                }],
                local_navigation_graph: vec![
                    ProviderNavigationNode {
                        node_ref: "loc-1".to_string(),
                        relation: "current_location".to_string(),
                        relative_hint: "distance_cm=0 visible_name=current".to_string(),
                        traversable: true,
                    },
                    ProviderNavigationNode {
                        node_ref: "loc-2".to_string(),
                        relation: "reachable_location".to_string(),
                        relative_hint: "distance_cm=100 visible_name=neighbor".to_string(),
                        traversable: true,
                    },
                ],
                hazard_summary: Vec::new(),
                interaction_targets: vec![ProviderInteractionTarget {
                    target_ref: "loc-2".to_string(),
                    target_kind: "location".to_string(),
                    interaction_hint: "move_agent".to_string(),
                }],
            },
            recent_event_summary: vec!["event: AgentRegistered".to_string()],
            memory_summary: Some("prefer safe low-frequency actions".to_string()),
            action_catalog: vec![
                ActionCatalogEntry::new("move_agent", "move to a visible location"),
                ActionCatalogEntry::new("wait", "do nothing this tick"),
            ],
            timeout_budget_ms: 7000,
        },
        provider_config_ref: Some("provider://local-bridge".to_string()),
        agent_profile: Some(DEFAULT_PROVIDER_AGENT_PROFILE.to_string()),
        fixture_id: None,
        replay_id: None,
        timeout_budget_ms: 7000,
    }
}
