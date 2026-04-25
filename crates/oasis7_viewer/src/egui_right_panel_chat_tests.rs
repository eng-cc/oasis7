use super::*;
use ed25519_dalek::SigningKey;
use oasis7::simulator::{
    initialize_kernel, AgentDecision, AgentDecisionTrace, PromptUpdateOperation, WorldConfig,
    WorldEvent, WorldInitConfig, WorldScenario,
};
#[cfg(not(target_arch = "wasm32"))]
use std::fs;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;
#[cfg(not(target_arch = "wasm32"))]
use std::time::{SystemTime, UNIX_EPOCH};

fn message(agent_id: &str, time: u64, role: LlmChatRole, content: &str) -> LlmChatMessageTrace {
    LlmChatMessageTrace {
        time,
        agent_id: agent_id.to_string(),
        role,
        content: content.to_string(),
    }
}

fn trace(agent_id: &str, time: u64, messages: Vec<LlmChatMessageTrace>) -> AgentDecisionTrace {
    AgentDecisionTrace {
        agent_id: agent_id.to_string(),
        time,
        decision: AgentDecision::Wait,
        llm_input: None,
        llm_output: None,
        llm_error: None,
        parse_error: None,
        llm_diagnostics: None,
        llm_effect_intents: Vec::new(),
        llm_effect_receipts: Vec::new(),
        llm_step_trace: Vec::new(),
        llm_prompt_section_trace: Vec::new(),
        llm_chat_messages: messages,
    }
}

fn viewer_state_with_traces(traces: Vec<AgentDecisionTrace>) -> ViewerState {
    ViewerState {
        status: crate::ConnectionStatus::Connected,
        snapshot: None,
        events: Vec::new(),
        decision_traces: traces,
        metrics: None,
    }
}

fn prompt_event(
    tick: u64,
    agent_id: &str,
    version: u64,
    system_prompt: Option<&str>,
    short_goal: Option<&str>,
    long_goal: Option<&str>,
) -> WorldEvent {
    WorldEvent {
        id: tick,
        time: tick,
        kind: WorldEventKind::AgentPromptUpdated {
            profile: AgentPromptProfile {
                agent_id: agent_id.to_string(),
                version,
                updated_at_tick: tick,
                updated_by: "tester".to_string(),
                system_prompt_override: system_prompt.map(str::to_string),
                short_term_goal_override: short_goal.map(str::to_string),
                long_term_goal_override: long_goal.map(str::to_string),
            },
            operation: PromptUpdateOperation::Apply,
            applied_fields: Vec::new(),
            digest: "digest".to_string(),
            rolled_back_to_version: None,
        },
        runtime_event: None,
    }
}

fn test_signer(seed: u8) -> ViewerAuthSigner {
    let private_key = [seed; 32];
    let signing_key = SigningKey::from_bytes(&private_key);
    ViewerAuthSigner {
        player_id: "player-a".to_string(),
        public_key: hex::encode(signing_key.verifying_key().to_bytes()),
        private_key: hex::encode(private_key),
    }
}

#[test]
fn collect_chat_threads_splits_by_player_message() {
    let state = viewer_state_with_traces(vec![trace(
        "agent-a",
        4,
        vec![
            message("agent-a", 1, LlmChatRole::Player, "hello"),
            message("agent-a", 2, LlmChatRole::Agent, "ack"),
            message("agent-a", 3, LlmChatRole::Player, "next topic"),
            message("agent-a", 4, LlmChatRole::Agent, "done"),
        ],
    )]);

    let threads = collect_chat_threads(&state, 16, 16);
    assert_eq!(threads.len(), 2);
    assert_eq!(threads[0].messages.len(), 2);
    assert_eq!(threads[1].messages.len(), 2);
    assert_eq!(threads[0].messages[0].content, "next topic");
}

#[test]
fn collect_chat_threads_orders_latest_first_across_agents() {
    let state = viewer_state_with_traces(vec![
        trace(
            "agent-a",
            4,
            vec![
                message("agent-a", 1, LlmChatRole::Player, "a1"),
                message("agent-a", 2, LlmChatRole::Agent, "a2"),
            ],
        ),
        trace(
            "agent-b",
            8,
            vec![
                message("agent-b", 5, LlmChatRole::Player, "b1"),
                message("agent-b", 8, LlmChatRole::Agent, "b2"),
            ],
        ),
    ]);

    let threads = collect_chat_threads(&state, 16, 16);
    assert_eq!(threads.len(), 2);
    assert_eq!(threads[0].agent_id, "agent-b");
    assert_eq!(threads[1].agent_id, "agent-a");
}

#[test]
fn truncate_text_marks_ellipsis_when_exceeding_limit() {
    assert_eq!(truncate_text("abcdef", 3), "abc…");
    assert_eq!(truncate_text("abc", 3), "abc");
}

#[test]
fn should_submit_chat_on_enter_requires_focus_and_no_modifiers() {
    assert!(should_submit_chat_on_enter(
        true,
        true,
        egui::Modifiers::default()
    ));
    assert!(!should_submit_chat_on_enter(
        false,
        true,
        egui::Modifiers::default()
    ));
    assert!(!should_submit_chat_on_enter(
        true,
        false,
        egui::Modifiers::default()
    ));

    let mut shift_mod = egui::Modifiers::default();
    shift_mod.shift = true;
    assert!(!should_submit_chat_on_enter(true, true, shift_mod));
}

#[test]
fn parse_tool_call_view_reads_structured_payload() {
    let tool_message = message(
        "agent-a",
        10,
        LlmChatRole::Tool,
        r#"{"type":"module_call_result","module":"environment.current_observation","status":"ok","args":{"limit":3},"result":{"ok":true,"module":"environment.current_observation"}}"#,
    );

    let parsed = parse_tool_call_view(&tool_message);
    assert_eq!(parsed.module, "environment.current_observation");
    assert_eq!(parsed.status, "ok");
    assert!(parsed.args_preview.contains("\"limit\":3"));
    assert!(parsed.result_preview.contains("\"ok\":true"));
}

#[test]
fn parse_tool_call_view_reads_removed_old_brand_text_format() {
    let tool_message = message(
        "agent-a",
        10,
        LlmChatRole::Tool,
        "module=agent.modules.list status=ok result={\"ok\":true}",
    );

    let parsed = parse_tool_call_view(&tool_message);
    assert_eq!(parsed.module, "agent.modules.list");
    assert_eq!(parsed.status, "ok");
    assert_eq!(parsed.args_preview, "-");
    assert!(parsed.result_preview.contains("\"ok\":true"));
}

#[test]
fn default_prompt_presets_are_non_empty() {
    let draft = AgentChatDraftState::default();
    assert!(!draft.prompt_presets.is_empty());
    assert_eq!(draft.selected_preset_index, 0);
}

#[test]
fn sync_prompt_presets_clamps_out_of_bounds_index() {
    let mut draft = AgentChatDraftState::default();
    draft.selected_preset_index = 999;
    sync_prompt_presets(&mut draft);
    assert_eq!(draft.selected_preset_index, draft.prompt_presets.len() - 1);
}

#[test]
fn apply_selected_preset_to_input_copies_content() {
    let mut draft = AgentChatDraftState::default();
    draft.prompt_presets = vec![PromptPresetDraft {
        name: "n".to_string(),
        content: "hello preset".to_string(),
    }];
    draft.selected_preset_index = 0;
    assert!(apply_selected_preset_to_input(&mut draft));
    assert_eq!(draft.input_message, "hello preset");
}

#[test]
fn prompt_preset_scroll_max_height_clamps_by_available_height() {
    assert_eq!(
        prompt_preset_scroll_max_height(PROMPT_PRESET_SCROLL_MAX_HEIGHT + 120.0),
        PROMPT_PRESET_SCROLL_MAX_HEIGHT
    );
    assert_eq!(prompt_preset_scroll_max_height(180.0), 180.0);
    assert_eq!(prompt_preset_scroll_max_height(-10.0), 0.0);
}

#[test]
fn prompt_preset_scroll_max_height_handles_non_finite_input() {
    assert_eq!(
        prompt_preset_scroll_max_height(f32::INFINITY),
        PROMPT_PRESET_SCROLL_MAX_HEIGHT
    );
    assert_eq!(
        prompt_preset_scroll_max_height(f32::NAN),
        PROMPT_PRESET_SCROLL_MAX_HEIGHT
    );
}

#[test]
fn current_prompt_profile_for_agent_prefers_latest_event_profile() {
    let mut state = viewer_state_with_traces(Vec::new());
    state.events = vec![
        prompt_event(1, "agent-a", 1, Some("s1"), None, None),
        prompt_event(2, "agent-a", 2, Some("s2"), Some("g2"), None),
    ];

    let profile = current_prompt_profile_for_agent(&state, "agent-a");
    assert_eq!(profile.version, 2);
    assert_eq!(profile.system_prompt_override.as_deref(), Some("s2"));
    assert_eq!(profile.short_term_goal_override.as_deref(), Some("g2"));
}

#[test]
fn build_prompt_profile_apply_request_only_patches_changed_fields() {
    let current = AgentPromptProfile {
        agent_id: "agent-a".to_string(),
        version: 3,
        updated_at_tick: 10,
        updated_by: "tester".to_string(),
        system_prompt_override: Some("system-a".to_string()),
        short_term_goal_override: Some("short-a".to_string()),
        long_term_goal_override: None,
    };
    let draft = AgentChatDraftState {
        profile_system_prompt: "system-a".to_string(),
        profile_short_term_goal: "short-updated".to_string(),
        profile_long_term_goal: "long-new".to_string(),
        ..AgentChatDraftState::default()
    };

    let request = build_prompt_profile_apply_request("agent-a", &current, &draft);
    assert_eq!(request.expected_version, Some(3));
    assert!(request.system_prompt_override.is_none());
    assert_eq!(
        request.short_term_goal_override,
        Some(Some("short-updated".to_string()))
    );
    assert_eq!(
        request.long_term_goal_override,
        Some(Some("long-new".to_string()))
    );
    assert!(prompt_apply_request_has_patch(&request));
}

#[test]
fn load_profile_draft_from_profile_prefills_defaults_when_override_missing() {
    let profile = AgentPromptProfile::for_agent("agent-a");
    let mut draft = AgentChatDraftState::default();

    load_profile_draft_from_profile(&mut draft, "agent-a", &profile);

    assert_eq!(draft.profile_system_prompt, DEFAULT_LLM_SYSTEM_PROMPT);
    assert_eq!(draft.profile_short_term_goal, DEFAULT_LLM_SHORT_TERM_GOAL);
    assert_eq!(draft.profile_long_term_goal, DEFAULT_LLM_LONG_TERM_GOAL);
}

#[test]
fn build_prompt_profile_apply_request_ignores_unmodified_defaults() {
    let current = AgentPromptProfile::for_agent("agent-a");
    let draft = AgentChatDraftState {
        profile_system_prompt: DEFAULT_LLM_SYSTEM_PROMPT.to_string(),
        profile_short_term_goal: DEFAULT_LLM_SHORT_TERM_GOAL.to_string(),
        profile_long_term_goal: DEFAULT_LLM_LONG_TERM_GOAL.to_string(),
        ..AgentChatDraftState::default()
    };

    let request = build_prompt_profile_apply_request("agent-a", &current, &draft);
    assert!(!prompt_apply_request_has_patch(&request));
}

#[test]
fn build_prompt_profile_apply_request_reverts_override_when_input_is_default() {
    let current = AgentPromptProfile {
        agent_id: "agent-a".to_string(),
        version: 5,
        updated_at_tick: 42,
        updated_by: "tester".to_string(),
        system_prompt_override: Some("custom-system".to_string()),
        short_term_goal_override: Some("custom-short".to_string()),
        long_term_goal_override: Some("custom-long".to_string()),
    };
    let draft = AgentChatDraftState {
        profile_system_prompt: DEFAULT_LLM_SYSTEM_PROMPT.to_string(),
        profile_short_term_goal: DEFAULT_LLM_SHORT_TERM_GOAL.to_string(),
        profile_long_term_goal: DEFAULT_LLM_LONG_TERM_GOAL.to_string(),
        ..AgentChatDraftState::default()
    };

    let request = build_prompt_profile_apply_request("agent-a", &current, &draft);
    assert_eq!(request.system_prompt_override, Some(None));
    assert_eq!(request.short_term_goal_override, Some(None));
    assert_eq!(request.long_term_goal_override, Some(None));
}

#[test]
fn resolve_viewer_auth_signer_from_uses_default_player_and_required_keys() {
    let signer = test_signer(31);
    let mut env = std::collections::BTreeMap::<String, String>::new();
    env.insert(
        VIEWER_AUTH_PUBLIC_KEY_ENV.to_string(),
        signer.public_key.clone(),
    );
    env.insert(
        VIEWER_AUTH_PRIVATE_KEY_ENV.to_string(),
        signer.private_key.clone(),
    );

    let resolved =
        resolve_viewer_auth_signer_from(|key| env.get(key).cloned()).expect("resolve signer");
    assert_eq!(resolved.player_id, VIEWER_PLAYER_ID);
    assert_eq!(resolved.public_key, signer.public_key);
    assert_eq!(resolved.private_key, signer.private_key);
}

#[test]
fn resolve_viewer_auth_signer_from_rejects_missing_private_key() {
    let signer = test_signer(33);
    let mut env = std::collections::BTreeMap::<String, String>::new();
    env.insert(
        VIEWER_AUTH_PUBLIC_KEY_ENV.to_string(),
        signer.public_key.clone(),
    );

    let err = resolve_viewer_auth_signer_from(|key| env.get(key).cloned())
        .expect_err("missing private key should fail");
    assert!(err.contains(VIEWER_AUTH_PRIVATE_KEY_ENV));
}

#[test]
fn resolve_viewer_auth_signer_from_rejects_removed_old_brand_key_names() {
    let signer = test_signer(35);
    let mut env = std::collections::BTreeMap::<String, String>::new();
    env.insert(
        removed_old_brand_viewer_auth_env("AUTH_PUBLIC_KEY"),
        signer.public_key.clone(),
    );
    env.insert(
        removed_old_brand_viewer_auth_env("AUTH_PRIVATE_KEY"),
        signer.private_key.clone(),
    );
    env.insert(
        removed_old_brand_viewer_auth_env("PLAYER_ID"),
        "removed-old-brand-viewer-player".to_string(),
    );

    let err = resolve_viewer_auth_signer_from(|key| env.get(key).cloned())
        .expect_err("removed old-brand keys should fail");
    assert!(err.contains(VIEWER_AUTH_PUBLIC_KEY_ENV));
}

fn removed_old_brand_viewer_auth_env(suffix: &str) -> String {
    ["AGENT", "WORLD", "VIEWER", suffix].join("_")
}

#[test]
fn attach_agent_chat_auth_sets_claims_and_verifiable_proof() {
    let signer = test_signer(34);
    let mut request = oasis7::viewer::AgentChatRequest {
        agent_id: "agent-a".to_string(),
        message: "hello".to_string(),
        player_id: None,
        public_key: None,
        auth: None,
        intent_tick: Some(9),
        intent_seq: None,
    };

    attach_agent_chat_auth(&mut request, &signer, 41).expect("attach auth");
    assert_eq!(
        request.player_id.as_deref(),
        Some(signer.player_id.as_str())
    );
    assert_eq!(
        request.public_key.as_deref(),
        Some(signer.public_key.as_str())
    );
    let proof = request.auth.as_ref().expect("proof attached").clone();
    let verified =
        oasis7::viewer::verify_agent_chat_auth_proof(&request, &proof).expect("proof verify");
    assert_eq!(verified.player_id, signer.player_id);
    assert_eq!(verified.public_key, signer.public_key);
    assert_eq!(verified.nonce, 41);
    assert_eq!(request.intent_tick, Some(9));
    assert_eq!(request.intent_seq, Some(41));
}

#[test]
fn attach_prompt_control_apply_auth_sets_updated_by_and_verifiable_proof() {
    let signer = test_signer(34);
    let mut request = oasis7::viewer::PromptControlApplyRequest {
        agent_id: "agent-a".to_string(),
        player_id: "removed-old-brand-player".to_string(),
        public_key: None,
        auth: None,
        expected_version: Some(3),
        updated_by: None,
        strong_auth_grant: None,
        system_prompt_override: Some(Some("system".to_string())),
        short_term_goal_override: None,
        long_term_goal_override: None,
    };

    attach_prompt_control_apply_auth(
        &mut request,
        &signer,
        42,
        oasis7::viewer::PromptControlAuthIntent::Apply,
    )
    .expect("attach auth");
    assert_eq!(request.player_id, signer.player_id);
    assert_eq!(
        request.updated_by.as_deref(),
        Some(signer.player_id.as_str())
    );
    assert_eq!(
        request.public_key.as_deref(),
        Some(signer.public_key.as_str())
    );
    let proof = request.auth.as_ref().expect("proof attached").clone();
    let verified = oasis7::viewer::verify_prompt_control_apply_auth_proof(
        oasis7::viewer::PromptControlAuthIntent::Apply,
        &request,
        &proof,
    )
    .expect("proof verify");
    assert_eq!(verified.player_id, signer.player_id);
    assert_eq!(verified.public_key, signer.public_key);
    assert_eq!(verified.nonce, 42);
}

#[test]
fn sync_viewer_auth_nonce_from_state_tracks_persisted_nonce() {
    let config = WorldConfig::default();
    let init = WorldInitConfig::from_scenario(WorldScenario::Minimal, &config);
    let (mut kernel, _) = initialize_kernel(config, init).expect("init world");
    kernel
        .consume_player_auth_nonce(VIEWER_PLAYER_ID, 1_000_000)
        .expect("seed nonce");

    let mut state = ViewerState::default();
    state.snapshot = Some(kernel.snapshot());

    let baseline = viewer_auth_nonce_for_tests();
    sync_viewer_auth_nonce_from_state(&state);
    let current = viewer_auth_nonce_for_tests();
    assert!(current >= baseline.max(1_000_001));
}

#[test]
fn prompt_apply_request_has_patch_returns_false_for_noop_request() {
    let request = oasis7::viewer::PromptControlApplyRequest {
        agent_id: "agent-a".to_string(),
        player_id: VIEWER_PLAYER_ID.to_string(),
        public_key: None,
        auth: None,
        expected_version: Some(1),
        updated_by: Some(VIEWER_PLAYER_ID.to_string()),
        strong_auth_grant: None,
        system_prompt_override: None,
        short_term_goal_override: None,
        long_term_goal_override: None,
    };
    assert!(!prompt_apply_request_has_patch(&request));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn resolve_viewer_auth_signer_from_node_config_reads_node_keypair() {
    let signer = test_signer(42);
    let temp_dir = make_temp_dir("viewer_auth_from_node_config");
    let config_path = temp_dir.join("config.toml");
    fs::write(
        config_path.as_path(),
        format!(
            "[node]\nprivate_key = \"{}\"\npublic_key = \"{}\"\n",
            signer.private_key, signer.public_key
        ),
    )
    .expect("write config");

    let resolved =
        resolve_viewer_auth_signer_from_node_config(config_path.as_path()).expect("resolve signer");
    assert_eq!(resolved.public_key, signer.public_key);
    assert_eq!(resolved.private_key, signer.private_key);
    assert!(!resolved.player_id.trim().is_empty());

    let _ = fs::remove_dir_all(temp_dir);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn resolve_viewer_auth_signer_from_node_config_rejects_missing_node_table() {
    let temp_dir = make_temp_dir("viewer_auth_missing_node_table");
    let config_path = temp_dir.join("config.toml");
    fs::write(config_path.as_path(), "[not_node]\nfoo = \"bar\"\n").expect("write config");

    let err = resolve_viewer_auth_signer_from_node_config(config_path.as_path())
        .expect_err("missing node table should fail");
    assert!(err.contains("node table"));

    let _ = fs::remove_dir_all(temp_dir);
}

#[cfg(not(target_arch = "wasm32"))]
fn make_temp_dir(label: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    path.push(format!(
        "oasis7_viewer_chat_auth_{label}_{}_{}",
        std::process::id(),
        stamp
    ));
    fs::create_dir_all(path.as_path()).expect("create temp dir");
    path
}
