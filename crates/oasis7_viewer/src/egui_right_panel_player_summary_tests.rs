use super::egui_right_panel_player_experience::{
    build_player_first_session_summary_snapshot, dismiss_player_first_session_summary,
    player_first_session_summary_visible, sync_player_first_session_summary_state,
    PlayerOnboardingState,
};
use super::egui_right_panel_player_guide::PlayerGuideProgressSnapshot;

#[test]
fn first_session_summary_becomes_visible_once_explore_step_is_ready() {
    let mut onboarding = PlayerOnboardingState::default();
    let state = super::sample_viewer_state(crate::ConnectionStatus::Connected, Vec::new());
    let before_complete = PlayerGuideProgressSnapshot {
        connect_world_done: true,
        open_panel_done: true,
        select_target_done: true,
        explore_ready: false,
    };
    sync_player_first_session_summary_state(&mut onboarding, &state, before_complete, 3.0);
    assert!(!player_first_session_summary_visible(&onboarding));

    let completed = PlayerGuideProgressSnapshot {
        connect_world_done: true,
        open_panel_done: true,
        select_target_done: true,
        explore_ready: true,
    };
    sync_player_first_session_summary_state(&mut onboarding, &state, completed, 5.0);
    assert!(player_first_session_summary_visible(&onboarding));

    dismiss_player_first_session_summary(&mut onboarding);
    assert!(!player_first_session_summary_visible(&onboarding));
}

#[test]
fn first_session_summary_snapshot_reports_duration_and_gains() {
    let mut onboarding = PlayerOnboardingState::default();
    let mut state = super::sample_viewer_state(crate::ConnectionStatus::Connected, Vec::new());
    state.metrics = Some(oasis7::simulator::RunnerMetrics {
        total_ticks: 10,
        ..oasis7::simulator::RunnerMetrics::default()
    });
    sync_player_first_session_summary_state(
        &mut onboarding,
        &state,
        PlayerGuideProgressSnapshot {
            connect_world_done: true,
            open_panel_done: true,
            select_target_done: true,
            explore_ready: false,
        },
        0.0,
    );

    state.metrics = Some(oasis7::simulator::RunnerMetrics {
        total_ticks: 18,
        ..oasis7::simulator::RunnerMetrics::default()
    });
    state.events = vec![
        super::sample_agent_moved_event(1, 12),
        super::sample_agent_moved_event(2, 13),
        super::sample_agent_moved_event(3, 14),
    ];
    sync_player_first_session_summary_state(
        &mut onboarding,
        &state,
        PlayerGuideProgressSnapshot {
            connect_world_done: true,
            open_panel_done: true,
            select_target_done: true,
            explore_ready: true,
        },
        9.0,
    );

    let snapshot = build_player_first_session_summary_snapshot(
        &onboarding,
        &state,
        crate::i18n::UiLocale::EnUs,
        9.0,
    )
    .expect("summary snapshot should exist");
    assert_eq!(snapshot.duration_secs, 9);
    assert_eq!(snapshot.tick_gain, 8);
    assert_eq!(snapshot.event_gain, 3);
    assert_eq!(
        snapshot.title,
        "First Session Recap: PostOnboarding unlocked"
    );
    assert!(snapshot.detail.contains("first industrial line"));
    assert!(snapshot.next_tip.contains("output reward"));
}
