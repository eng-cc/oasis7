use super::*;
use oasis7::simulator::{
    AgentDecision, AgentDecisionTrace, ConsumeReason, PowerEvent, RejectReason,
};
use oasis7::viewer::ViewerControlProfile;

#[test]
fn timeline_seek_button_is_disabled_for_live_profile_only() {
    assert!(viewer_seek_supported(None));
    assert!(viewer_seek_supported(Some(&ViewerControlProfileState {
        profile: Some(ViewerControlProfile::Playback),
    })));
    assert!(!viewer_seek_supported(Some(&ViewerControlProfileState {
        profile: Some(ViewerControlProfile::Live),
    })));
}

#[test]
fn key_insights_collects_error_llm_and_peaks() {
    let events = vec![
        WorldEvent {
            id: 1,
            time: 2,
            kind: WorldEventKind::ActionRejected {
                reason: RejectReason::InvalidAmount { amount: 1 },
            },
            runtime_event: None,
        },
        WorldEvent {
            id: 2,
            time: 6,
            kind: WorldEventKind::Power(PowerEvent::PowerGenerated {
                plant_id: "plant-1".to_string(),
                location_id: "loc-1".to_string(),
                amount: 120,
            }),
            runtime_event: None,
        },
        WorldEvent {
            id: 3,
            time: 8,
            kind: WorldEventKind::ResourceTransferred {
                from: oasis7::simulator::ResourceOwner::Location {
                    location_id: "loc-1".to_string(),
                },
                to: oasis7::simulator::ResourceOwner::Agent {
                    agent_id: "agent-1".to_string(),
                },
                kind: oasis7::simulator::ResourceKind::Electricity,
                amount: 300,
            },
            runtime_event: None,
        },
        WorldEvent {
            id: 4,
            time: 1,
            kind: WorldEventKind::Power(PowerEvent::PowerConsumed {
                agent_id: "agent-1".to_string(),
                amount: 7,
                reason: ConsumeReason::Decision,
                remaining: 10,
            }),
            runtime_event: None,
        },
    ];

    let traces = vec![
        AgentDecisionTrace {
            agent_id: "agent-1".to_string(),
            time: 4,
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
            llm_chat_messages: Vec::new(),
        },
        AgentDecisionTrace {
            agent_id: "agent-2".to_string(),
            time: 9,
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
            llm_chat_messages: Vec::new(),
        },
    ];

    let summary = build_timeline_key_insights(&events, &traces, 10);
    assert_eq!(summary.error_ticks, vec![2]);
    assert_eq!(summary.llm_ticks, vec![4, 9]);
    assert_eq!(summary.resource_peak_ticks, vec![1, 6, 8]);
    assert_eq!(summary.density_sparkline.chars().count(), DENSITY_BINS);
}

#[test]
fn density_sparkline_reflects_event_distribution() {
    let events = vec![
        WorldEvent {
            id: 1,
            time: 0,
            kind: WorldEventKind::ActionRejected {
                reason: RejectReason::InvalidAmount { amount: 1 },
            },
            runtime_event: None,
        },
        WorldEvent {
            id: 2,
            time: 5,
            kind: WorldEventKind::ActionRejected {
                reason: RejectReason::InvalidAmount { amount: 1 },
            },
            runtime_event: None,
        },
        WorldEvent {
            id: 3,
            time: 5,
            kind: WorldEventKind::ActionRejected {
                reason: RejectReason::InvalidAmount { amount: 1 },
            },
            runtime_event: None,
        },
        WorldEvent {
            id: 4,
            time: 10,
            kind: WorldEventKind::ActionRejected {
                reason: RejectReason::InvalidAmount { amount: 1 },
            },
            runtime_event: None,
        },
    ];

    let sparkline = event_density_sparkline(&events, 10, 5);
    assert_eq!(sparkline.chars().count(), 5);
    assert!(sparkline.contains('█'));
    assert!(sparkline.contains('·'));
}

#[test]
fn format_tick_list_compacts_output() {
    assert_eq!(format_tick_list(&[], 4), "-");
    assert_eq!(format_tick_list(&[1, 2, 3], 4), "1,2,3");
    assert_eq!(format_tick_list(&[1, 2, 3, 4, 5], 3), "1,2,3+2");
}

#[test]
fn select_next_mark_tick_cycles_forward_then_wraps() {
    let ticks = vec![3, 8, 12];
    assert_eq!(select_next_mark_tick(&ticks, 0), Some(3));
    assert_eq!(select_next_mark_tick(&ticks, 3), Some(8));
    assert_eq!(select_next_mark_tick(&ticks, 11), Some(12));
    assert_eq!(select_next_mark_tick(&ticks, 99), Some(3));
    assert_eq!(select_next_mark_tick(&[], 10), None);
}

#[test]
fn timeline_feedback_summary_and_recovery_visibility() {
    let feedback = crate::web_test_api::WebTestApiControlFeedbackSnapshot {
        action: "step".to_string(),
        stage: "completed_no_progress".to_string(),
        reason: Some("Cause: completion ack timeout_no_progress".to_string()),
        hint: Some("Next: click Recover: play, then retry step".to_string()),
        effect: "completion ack: timeout without observed progress".to_string(),
        delta_logical_time: 0,
        delta_event_seq: 0,
        delta_trace_count: 0,
    };
    let summary = timeline_control_feedback_summary(Some(&feedback), UiLocale::EnUs);
    assert!(summary.contains("Completed (no progress)"));
    assert!(summary.contains("Recover: play"));
    assert!(timeline_should_show_recovery_actions(Some(&feedback)));
}

#[test]
fn timeline_feedback_summary_defaults_without_feedback() {
    let summary = timeline_control_feedback_summary(None, UiLocale::EnUs);
    assert!(summary.contains("Control feedback: none"));
    assert!(!timeline_should_show_recovery_actions(None));
}

#[test]
fn mark_filter_button_toggles_state() {
    let mut app = App::new();
    app.add_systems(Update, handle_timeline_mark_filter_buttons);

    app.world_mut()
        .insert_resource(TimelineMarkFilterState::default());
    app.world_mut().spawn((
        Button,
        Interaction::Pressed,
        TimelineMarkFilterButton {
            kind: TimelineMarkKind::Error,
        },
    ));

    app.update();

    let filters = app.world().resource::<TimelineMarkFilterState>();
    assert!(!filters.show_error);
    assert!(filters.show_llm);
    assert!(filters.show_peak);
}

#[test]
fn mark_jump_respects_disabled_filter() {
    let mut app = App::new();
    app.add_systems(Update, handle_timeline_mark_jump_buttons);

    let events = vec![WorldEvent {
        id: 1,
        time: 5,
        kind: WorldEventKind::ActionRejected {
            reason: RejectReason::InvalidAmount { amount: 1 },
        },
        runtime_event: None,
    }];

    app.world_mut().insert_resource(ViewerState {
        status: crate::ConnectionStatus::Connected,
        snapshot: None,
        events,
        decision_traces: Vec::new(),
        metrics: None,
    });
    app.world_mut().insert_resource(TimelineUiState {
        target_tick: 0,
        max_tick_seen: 10,
        manual_override: false,
        drag_active: false,
    });
    app.world_mut().insert_resource(TimelineMarkFilterState {
        show_error: false,
        show_llm: true,
        show_peak: true,
    });

    app.world_mut().spawn((
        Button,
        Interaction::Pressed,
        TimelineMarkJumpButton {
            kind: TimelineMarkKind::Error,
        },
    ));

    app.update();

    let timeline = app.world().resource::<TimelineUiState>();
    assert_eq!(timeline.target_tick, 0);
    assert!(!timeline.manual_override);
}

#[test]
fn mark_jump_button_updates_timeline_target() {
    let mut app = App::new();
    app.add_systems(Update, handle_timeline_mark_jump_buttons);

    let events = vec![WorldEvent {
        id: 1,
        time: 5,
        kind: WorldEventKind::ActionRejected {
            reason: RejectReason::InvalidAmount { amount: 1 },
        },
        runtime_event: None,
    }];

    app.world_mut().insert_resource(ViewerState {
        status: crate::ConnectionStatus::Connected,
        snapshot: None,
        events,
        decision_traces: Vec::new(),
        metrics: None,
    });
    app.world_mut().insert_resource(TimelineUiState {
        target_tick: 0,
        max_tick_seen: 10,
        manual_override: false,
        drag_active: false,
    });

    app.world_mut().spawn((
        Button,
        Interaction::Pressed,
        TimelineMarkJumpButton {
            kind: TimelineMarkKind::Error,
        },
    ));

    app.update();

    let timeline = app.world().resource::<TimelineUiState>();
    assert_eq!(timeline.target_tick, 5);
    assert!(timeline.manual_override);
}
