use super::*;

#[test]
fn first_check_opens_startup_guide_once_for_game_issues() {
    let mut app = ClientLauncherApp::default();
    let game_issues = [ConfigIssue::LiveBindInvalid];
    app.maybe_open_startup_guide_on_first_check(&game_issues, &[]);
    assert!(app.startup_guide_state.open);
    assert_eq!(app.startup_guide_state.target, StartupGuideTarget::Game);
    assert!(app.startup_guide_state.first_check_done);

    app.startup_guide_state.open = false;
    app.maybe_open_startup_guide_on_first_check(&game_issues, &[]);
    assert!(!app.startup_guide_state.open);
}

#[test]
fn handle_start_game_click_opens_startup_guide_when_invalid() {
    let mut app = ClientLauncherApp::default();
    let game_issues = [ConfigIssue::LiveBindInvalid];
    app.handle_start_game_click(&game_issues);
    assert_eq!(app.status, LauncherStatus::InvalidArgs);
    assert!(app.startup_guide_state.open);
    assert_eq!(app.startup_guide_state.target, StartupGuideTarget::Game);
}

#[test]
fn handle_start_chain_click_opens_startup_guide_when_invalid() {
    let mut app = ClientLauncherApp::default();
    let chain_issues = [ConfigIssue::ChainNodeIdRequired];
    app.handle_start_chain_click(&chain_issues);
    assert!(matches!(
        app.chain_runtime_status,
        ChainRuntimeStatus::ConfigError(_)
    ));
    assert!(app.startup_guide_state.open);
    assert_eq!(app.startup_guide_state.target, StartupGuideTarget::Chain);
}

#[test]
fn apply_safe_defaults_for_game_target_recovers_required_fields() {
    let mut app = ClientLauncherApp::default();
    app.config.scenario.clear();
    app.config.live_bind = "127.0.0.1".to_string();
    app.config.web_bind = "127.0.0.1".to_string();
    app.config.viewer_host.clear();
    app.config.viewer_port = "0".to_string();
    app.config.viewer_static_dir.clear();

    app.apply_safe_defaults_for_startup_target(StartupGuideTarget::Game);

    let game_issues = collect_required_config_issues(&app.config);
    assert!(!game_issues.contains(&ConfigIssue::LiveBindInvalid));
    assert!(!game_issues.contains(&ConfigIssue::WebBindInvalid));
    assert!(!game_issues.contains(&ConfigIssue::ViewerHostRequired));
    assert!(!game_issues.contains(&ConfigIssue::ViewerPortInvalid));
    assert!(!game_issues.contains(&ConfigIssue::ViewerStaticDirRequired));
}

#[test]
fn apply_safe_defaults_for_chain_target_recovers_required_fields() {
    let mut app = ClientLauncherApp::default();
    app.config.chain_enabled = false;
    app.config.chain_runtime_bin.clear();
    app.config.chain_status_bind = "127.0.0.1".to_string();
    app.config.chain_node_id.clear();
    app.config.chain_node_role = "invalid".to_string();
    app.config.chain_node_tick_ms = "0".to_string();
    app.config.chain_pos_slot_duration_ms = "0".to_string();
    app.config.chain_pos_ticks_per_slot = "0".to_string();
    app.config.chain_pos_proposal_tick_phase = "99".to_string();
    app.config.chain_pos_max_past_slot_lag = "-1".to_string();

    app.apply_safe_defaults_for_startup_target(StartupGuideTarget::Chain);

    let chain_issues = collect_chain_required_config_issues(&app.config);
    assert!(app.config.chain_enabled);
    assert!(chain_issues.is_empty());
    assert_eq!(app.chain_runtime_status, ChainRuntimeStatus::NotStarted);
}

#[test]
fn apply_safe_defaults_for_chain_target_keeps_hosted_public_join_chain_disabled() {
    let mut app = ClientLauncherApp::default();
    app.config.deployment_mode = "hosted_public_join".to_string();
    app.config.chain_enabled = false;

    app.apply_safe_defaults_for_startup_target(StartupGuideTarget::Chain);

    assert!(!app.config.chain_enabled);
    assert_eq!(app.chain_runtime_status, ChainRuntimeStatus::Disabled);
}

#[test]
fn onboarding_auto_open_targets_fix_config_step_when_required_fields_missing() {
    let mut app = ClientLauncherApp::default();
    app.onboarding_state.auto_open_checked = false;
    app.onboarding_state.completed = false;
    app.onboarding_state.open = false;
    let game_issues = [ConfigIssue::LiveBindInvalid];
    app.maybe_open_onboarding_on_first_visit(&game_issues, &[], false, false);
    assert!(app.onboarding_state.open);
    assert_eq!(app.onboarding_state.step, OnboardingStep::FixConfig);
}

#[test]
fn onboarding_auto_open_happens_only_once_per_session() {
    let mut app = ClientLauncherApp::default();
    app.onboarding_state.auto_open_checked = false;
    app.onboarding_state.completed = false;
    app.onboarding_state.open = false;
    app.maybe_open_onboarding_on_first_visit(&[], &[], false, false);
    assert!(app.onboarding_state.open);
    assert_eq!(app.onboarding_state.step, OnboardingStep::Understand);

    app.onboarding_state.open = false;
    app.maybe_open_onboarding_on_first_visit(&[], &[], false, false);
    assert!(!app.onboarding_state.open);
}

#[test]
fn onboarding_auto_open_respects_dismissed_state() {
    let mut app = ClientLauncherApp::default();
    app.onboarding_state.auto_open_checked = false;
    app.onboarding_state.completed = false;
    app.onboarding_state.dismissed = true;
    app.onboarding_state.open = false;

    app.maybe_open_onboarding_on_first_visit(&[], &[], false, false);
    assert!(!app.onboarding_state.open);
}

#[test]
fn dismiss_onboarding_with_reminder_keeps_reminder_visible() {
    let mut app = ClientLauncherApp::default();
    app.onboarding_state.open = true;

    app.dismiss_onboarding_with_reminder();

    assert!(!app.onboarding_state.completed);
    assert!(app.onboarding_state.dismissed);
    assert!(app.should_show_onboarding_reminder());
    assert_eq!(app.ux_state.onboarding_skipped_count, 1);
}

#[test]
fn should_show_onboarding_reminder_hides_when_completed_or_open() {
    let mut app = ClientLauncherApp::default();
    app.onboarding_state.completed = false;
    app.onboarding_state.open = false;
    assert!(app.should_show_onboarding_reminder());

    app.onboarding_state.open = true;
    assert!(!app.should_show_onboarding_reminder());

    app.onboarding_state.open = false;
    app.onboarding_state.completed = true;
    assert!(!app.should_show_onboarding_reminder());
}

#[test]
fn resolve_next_task_hint_prioritizes_config_fix_then_start_order() {
    assert_eq!(
        resolve_next_task_hint(true, &[], &[ConfigIssue::ChainNodeIdRequired], false, false),
        NextTaskHint::FixChainConfig
    );
    assert_eq!(
        resolve_next_task_hint(true, &[ConfigIssue::LiveBindInvalid], &[], false, false),
        NextTaskHint::FixGameConfig
    );
    assert_eq!(
        resolve_next_task_hint(true, &[], &[], false, false),
        NextTaskHint::StartChain
    );
    assert_eq!(
        resolve_next_task_hint(true, &[], &[], false, true),
        NextTaskHint::StartGame
    );
    assert_eq!(
        resolve_next_task_hint(true, &[], &[], true, true),
        NextTaskHint::OpenGamePage
    );
}

#[test]
fn resolve_config_guide_target_follows_blocking_priority() {
    assert_eq!(
        resolve_config_guide_target(
            true,
            &[ConfigIssue::LiveBindInvalid],
            &[ConfigIssue::ChainNodeIdRequired],
        ),
        Some(ConfigGuideTargetHint::Chain)
    );
    assert_eq!(
        resolve_config_guide_target(true, &[ConfigIssue::LiveBindInvalid], &[]),
        Some(ConfigGuideTargetHint::Game)
    );
    assert_eq!(
        resolve_config_guide_target(false, &[ConfigIssue::LiveBindInvalid], &[]),
        Some(ConfigGuideTargetHint::Game)
    );
    assert_eq!(resolve_config_guide_target(true, &[], &[]), None);
}

#[test]
fn resolve_primary_disabled_cta_prefers_first_blocking_action() {
    assert_eq!(
        resolve_primary_disabled_cta(false, &[], &[], false),
        Some(DisabledActionCta::EnableChain)
    );
    assert_eq!(
        resolve_primary_disabled_cta(true, &[], &[ConfigIssue::ChainNodeIdRequired], false),
        Some(DisabledActionCta::FixChainConfig)
    );
    assert_eq!(
        resolve_primary_disabled_cta(true, &[], &[], false),
        Some(DisabledActionCta::StartChain)
    );
    assert_eq!(
        resolve_primary_disabled_cta(true, &[ConfigIssue::LiveBindInvalid], &[], true),
        Some(DisabledActionCta::FixGameConfig)
    );
    assert_eq!(resolve_primary_disabled_cta(true, &[], &[], true), None);
}

#[test]
fn resolve_disabled_cta_plan_prefers_retry_when_chain_is_starting() {
    let (primary, secondary) =
        resolve_disabled_cta_plan(&ChainRuntimeStatus::Starting, true, &[], &[]);
    assert_eq!(primary, Some(DisabledActionCta::RetryChainStatus));
    assert_eq!(secondary, Some(DisabledActionCta::StartChain));
}

#[test]
fn resolve_disabled_cta_plan_prioritizes_chain_fix_before_game_fix() {
    let (primary, secondary) = resolve_disabled_cta_plan(
        &ChainRuntimeStatus::ConfigError("bad bind".to_string()),
        true,
        &[ConfigIssue::LiveBindInvalid],
        &[ConfigIssue::ChainNodeIdRequired],
    );
    assert_eq!(primary, Some(DisabledActionCta::FixChainConfig));
    assert_eq!(secondary, Some(DisabledActionCta::RetryChainStatus));
}

#[test]
fn resolve_chain_runtime_preflight_state_requires_ready_chain() {
    assert_eq!(
        resolve_chain_runtime_preflight_state(true, &ChainRuntimeStatus::Ready),
        PreflightCheckState::Pass
    );
    assert_eq!(
        resolve_chain_runtime_preflight_state(true, &ChainRuntimeStatus::Starting),
        PreflightCheckState::Blocked
    );
    assert_eq!(
        resolve_chain_runtime_preflight_state(false, &ChainRuntimeStatus::Disabled),
        PreflightCheckState::Blocked
    );
}

#[test]
fn expert_mode_toggle_updates_runtime_state() {
    let mut app = ClientLauncherApp::default();
    app.set_expert_mode(true);
    assert!(app.is_expert_mode());
    app.set_expert_mode(false);
    assert!(!app.is_expert_mode());
}

#[test]
fn successful_config_profile_is_saved_on_running_state() {
    let mut app = ClientLauncherApp::default();
    app.config.scenario = "profile-save".to_string();

    app.maybe_save_last_successful_config_profile(true);

    let saved = app
        .ux_state
        .last_successful_config
        .as_ref()
        .expect("saved profile");
    assert_eq!(saved.scenario, "profile-save");
    assert!(app.ux_state.last_successful_saved_at_unix_ms.is_some());
}

#[test]
fn restore_last_successful_config_profile_replaces_runtime_config() {
    let mut app = ClientLauncherApp::default();
    let mut saved = app.config.clone();
    saved.scenario = "restored-scenario".to_string();
    saved.chain_enabled = false;
    app.ux_state.last_successful_config = Some(saved);

    app.restore_last_successful_config_profile();

    assert_eq!(app.config.scenario, "restored-scenario");
    assert!(!app.config.chain_enabled);
    assert_eq!(app.chain_runtime_status, ChainRuntimeStatus::Disabled);
}

#[test]
fn restore_last_successful_config_profile_normalizes_hosted_public_join() {
    let mut app = ClientLauncherApp::default();
    let mut saved = app.config.clone();
    saved.deployment_mode = "hosted_public_join".to_string();
    saved.chain_enabled = true;
    app.ux_state.last_successful_config = Some(saved);

    app.restore_last_successful_config_profile();

    assert!(!app.config.chain_enabled);
    assert_eq!(app.chain_runtime_status, ChainRuntimeStatus::Disabled);
}

#[test]
fn clear_last_successful_config_profile_clears_saved_snapshot() {
    let mut app = ClientLauncherApp::default();
    app.ux_state.last_successful_config = Some(app.config.clone());
    app.ux_state.last_successful_saved_at_unix_ms = Some(7);

    app.clear_last_successful_config_profile();

    assert!(app.ux_state.last_successful_config.is_none());
    assert!(app.ux_state.last_successful_saved_at_unix_ms.is_none());
}

#[test]
fn start_demo_mode_one_click_applies_safe_defaults() {
    let mut app = ClientLauncherApp::default();
    app.config.chain_enabled = false;
    app.config.scenario = "custom".to_string();

    app.start_demo_mode_one_click();

    assert_eq!(app.demo_mode_phase, DemoModePhase::StartChainRequested);
    assert!(app.config.chain_enabled);
    assert_eq!(app.config.scenario, "llm_bootstrap");
}

#[test]
fn advance_demo_mode_reaches_done_when_chain_and_game_are_ready() {
    let mut app = ClientLauncherApp::default();
    app.start_demo_mode_one_click();
    app.advance_demo_mode(&[], &[], false, true);
    assert_eq!(app.demo_mode_phase, DemoModePhase::StartGameRequested);

    app.advance_demo_mode(&[], &[], true, true);
    assert_eq!(app.demo_mode_phase, DemoModePhase::Done);
}

#[test]
fn advance_demo_mode_fails_when_chain_config_is_blocked() {
    let mut app = ClientLauncherApp::default();
    app.start_demo_mode_one_click();
    app.advance_demo_mode(&[], &[ConfigIssue::ChainNodeIdRequired], false, false);
    assert_eq!(app.demo_mode_phase, DemoModePhase::Failed);
}

#[test]
fn guidance_counters_increase_for_open_demo_and_quick_actions() {
    let mut app = ClientLauncherApp::default();
    app.open_onboarding_manual();
    assert_eq!(app.ux_state.onboarding_opened_count, 1);

    app.start_demo_mode_one_click();
    assert_eq!(app.ux_state.demo_mode_runs_count, 1);

    app.apply_explorer_quick_shortcut(ExplorerQuickShortcut::RecentTxs);
    assert_eq!(app.ux_state.quick_action_click_count, 1);
}
