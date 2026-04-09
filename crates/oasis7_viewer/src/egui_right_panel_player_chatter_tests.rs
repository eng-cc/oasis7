use super::egui_right_panel_player_experience::{
    player_agent_chatter_cap, player_agent_chatter_ids, player_agent_chatter_last_seen_event_id,
    player_agent_chatter_len, player_agent_chatter_snapshot, sync_agent_chatter_bubbles,
};
use super::*;

#[test]
fn sync_agent_chatter_bubbles_skips_history_then_tracks_new_events_only() {
    let mut achievements = PlayerAchievementState::default();
    let mut state = sample_viewer_state(
        crate::ConnectionStatus::Connected,
        vec![sample_agent_moved_event(1, 1), sample_rejected_event(2, 2)],
    );
    let locale = crate::i18n::UiLocale::ZhCn;

    sync_agent_chatter_bubbles(&mut achievements, &state, 10.0, locale);
    assert_eq!(player_agent_chatter_len(&achievements), 0);
    assert_eq!(
        player_agent_chatter_last_seen_event_id(&achievements),
        Some(2)
    );

    state.events.push(sample_agent_moved_event(3, 3));
    sync_agent_chatter_bubbles(&mut achievements, &state, 11.0, locale);

    assert_eq!(
        player_agent_chatter_last_seen_event_id(&achievements),
        Some(3)
    );
    assert_eq!(player_agent_chatter_len(&achievements), 1);

    let snapshot = player_agent_chatter_snapshot(&achievements, 0)
        .expect("expected one chatter bubble after new agent event");
    assert_eq!(snapshot.0, 3);
    assert_eq!(snapshot.1, FeedbackTone::Positive);
    assert!(snapshot.2.contains("agent-3"));
    assert!(snapshot.3.contains("移动"));
}

#[test]
fn sync_agent_chatter_bubbles_formats_runtime_industry_feedback() {
    let mut achievements = PlayerAchievementState::default();
    let mut state = sample_viewer_state(
        crate::ConnectionStatus::Connected,
        vec![sample_runtime_event(
            1,
            1,
            "runtime.economy.recipe_started",
            "factory=factory.alpha recipe=recipe.motor requester=agent.alpha outputs=motor_mk1x2",
        )],
    );
    let locale = crate::i18n::UiLocale::ZhCn;

    sync_agent_chatter_bubbles(&mut achievements, &state, 10.0, locale);
    assert_eq!(player_agent_chatter_len(&achievements), 0);

    state.events.push(sample_runtime_event(2, 2, "runtime.economy.factory_production_blocked", "factory=factory.alpha recipe=recipe.motor requester=agent.alpha reason=material_shortage detail=material_shortage:iron_ingot"));
    sync_agent_chatter_bubbles(&mut achievements, &state, 11.0, locale);

    let snapshot = player_agent_chatter_snapshot(&achievements, 0)
        .expect("expected one chatter bubble after runtime industry event");
    assert_eq!(snapshot.0, 2);
    assert_eq!(snapshot.1, FeedbackTone::Warning);
    assert!(snapshot.2.contains("factory.alpha"));
    assert!(snapshot.3.contains("代价已显现"));
}

#[test]
fn sync_agent_chatter_bubbles_surfaces_recovery_confirmation() {
    let mut achievements = PlayerAchievementState::default();
    let mut state = sample_viewer_state(
        crate::ConnectionStatus::Connected,
        vec![sample_agent_moved_event(1, 1)],
    );
    let locale = crate::i18n::UiLocale::EnUs;

    sync_agent_chatter_bubbles(&mut achievements, &state, 10.0, locale);
    state.events.push(sample_runtime_event(
        2,
        2,
        "runtime.economy.factory_production_resumed",
        "factory=factory.alpha recipe=recipe.motor requester=agent.alpha previous_reason=material_shortage",
    ));
    sync_agent_chatter_bubbles(&mut achievements, &state, 11.0, locale);

    let snapshot = player_agent_chatter_snapshot(&achievements, 0)
        .expect("expected one chatter bubble after resume event");
    assert_eq!(snapshot.1, FeedbackTone::Positive);
    assert!(snapshot.3.contains("Recovery confirmed"));
    assert!(snapshot.3.contains("resumed"));
}

#[test]
fn sync_agent_chatter_bubbles_filters_agent_not_found_focus_noise() {
    let mut achievements = PlayerAchievementState::default();
    let mut state = sample_viewer_state(
        crate::ConnectionStatus::Connected,
        vec![sample_agent_moved_event(1, 1)],
    );
    let locale = crate::i18n::UiLocale::ZhCn;

    sync_agent_chatter_bubbles(&mut achievements, &state, 10.0, locale);
    assert_eq!(player_agent_chatter_len(&achievements), 0);
    assert_eq!(
        player_agent_chatter_last_seen_event_id(&achievements),
        Some(1)
    );

    state
        .events
        .push(sample_agent_not_found_rejected_event(2, 2));
    sync_agent_chatter_bubbles(&mut achievements, &state, 11.0, locale);

    assert_eq!(
        player_agent_chatter_last_seen_event_id(&achievements),
        Some(2)
    );
    assert_eq!(player_agent_chatter_len(&achievements), 0);
}

#[test]
fn sync_agent_chatter_bubbles_clamps_queue_and_expires() {
    let mut achievements = PlayerAchievementState::default();
    let mut state = sample_viewer_state(
        crate::ConnectionStatus::Connected,
        vec![sample_agent_moved_event(1, 1)],
    );
    let locale = crate::i18n::UiLocale::EnUs;

    sync_agent_chatter_bubbles(&mut achievements, &state, 20.0, locale);
    assert_eq!(player_agent_chatter_len(&achievements), 0);
    assert_eq!(
        player_agent_chatter_last_seen_event_id(&achievements),
        Some(1)
    );

    let newest_id = player_agent_chatter_cap() as u64 + 3;
    for id in 2..=newest_id {
        state.events.push(sample_agent_moved_event(id, id));
        sync_agent_chatter_bubbles(&mut achievements, &state, 20.0 + id as f64, locale);
    }

    let ids = player_agent_chatter_ids(&achievements);
    let oldest_id = newest_id + 1 - player_agent_chatter_cap() as u64;
    let expected_ids: Vec<u64> = (oldest_id..=newest_id).collect();
    assert_eq!(ids, expected_ids);
    assert_eq!(
        player_agent_chatter_len(&achievements),
        player_agent_chatter_cap()
    );

    sync_agent_chatter_bubbles(&mut achievements, &state, 120.0, locale);
    assert_eq!(player_agent_chatter_len(&achievements), 0);
}
