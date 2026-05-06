use super::tests_ui_text::{
    build_agent_activity_text, build_events_text, build_status_text, build_summary_text,
    default_locale,
};
use super::*;
use crate::app_bootstrap::decide_offline;
use crate::button_feedback::StepControlLoadingState;
use crate::timeline_controls::{
    normalized_x_to_tick, TimelineAdjustButton, TimelineBar, TimelineBarFill,
    TimelineSeekSubmitButton, TimelineStatusText,
};
use crate::ui_text::events_summary;
use crate::viewer_3d_config::{
    Viewer3dConfig, ViewerExternalMaterialSlotConfig, ViewerExternalTextureSlotConfig,
    ViewerTonemappingMode,
};
use bevy::core_pipeline::tonemapping::{DebandDither, Tonemapping};
use oasis7::simulator::{MaterialKind, ResourceKind, WorldEventKind};

#[path = "tests_selection_details.rs"]
mod tests_selection_details;

#[path = "tests_scene_grid.rs"]
mod tests_scene_grid;

#[test]
fn update_ui_sets_status_and_events() {
    let event = WorldEvent {
        id: 1,
        time: 7,
        kind: oasis7::simulator::WorldEventKind::ActionRejected {
            reason: oasis7::simulator::RejectReason::InvalidAmount { amount: 1 },
        },
        runtime_event: None,
    };

    let state = ViewerState {
        status: ConnectionStatus::Error("oops".to_string()),
        snapshot: None,
        events: vec![event.clone()],
        decision_traces: Vec::new(),
        metrics: None,
    };
    let locale = default_locale();
    let status_text = build_status_text(&state, locale);
    assert_eq!(status_text, "Status: error: oops");

    let events_text = build_events_text(&state, None, locale);
    assert_eq!(events_text, events_summary(&[event], None));
}

#[test]
fn update_ui_populates_world_summary_and_metrics() {
    let mut model = oasis7::simulator::WorldModel::default();
    model.locations.insert(
        "loc-1".to_string(),
        oasis7::simulator::Location::new(
            "loc-1",
            "Alpha",
            oasis7::geometry::GeoPos {
                x_cm: 0,
                y_cm: 0,
                z_cm: 0,
            },
        ),
    );
    model.locations.insert(
        "loc-2".to_string(),
        oasis7::simulator::Location::new(
            "loc-2",
            "Beta",
            oasis7::geometry::GeoPos {
                x_cm: 1,
                y_cm: 1,
                z_cm: 0,
            },
        ),
    );
    model.agents.insert(
        "agent-1".to_string(),
        oasis7::simulator::Agent::new(
            "agent-1",
            "loc-1",
            oasis7::geometry::GeoPos {
                x_cm: 0,
                y_cm: 0,
                z_cm: 0,
            },
        ),
    );

    let snapshot = oasis7::simulator::WorldSnapshot {
        version: oasis7::simulator::SNAPSHOT_VERSION,
        chunk_generation_schema_version: oasis7::simulator::CHUNK_GENERATION_SCHEMA_VERSION,
        time: 42,
        config: oasis7::simulator::WorldConfig::default(),
        model,
        chunk_runtime: oasis7::simulator::ChunkRuntimeConfig::default(),
        next_event_id: 1,
        next_action_id: 1,
        pending_actions: Vec::new(),
        journal_len: 0,
        runtime_snapshot: None,
        player_gameplay: None,
    };

    let metrics = RunnerMetrics {
        total_ticks: 42,
        total_actions: 7,
        total_decisions: 4,
        ..RunnerMetrics::default()
    };

    let state = ViewerState {
        status: ConnectionStatus::Connected,
        snapshot: Some(snapshot),
        events: Vec::new(),
        decision_traces: Vec::new(),
        metrics: Some(metrics),
    };
    let locale = default_locale();
    let viewer_config = Viewer3dConfig::default();
    let summary_text = build_summary_text(&state, Some(&viewer_config), locale);

    assert!(summary_text.contains("Time: 42"));
    assert!(summary_text.contains("Locations: 2"));
    assert!(summary_text.contains("Agents: 1"));
    assert!(summary_text.contains("Ticks: 42"));
    assert!(summary_text.contains("Actions: 7"));
    assert!(summary_text.contains("Decisions: 4"));
    assert!(summary_text.contains("Render Physical: off"));
}

#[test]
fn update_ui_reflects_filtered_events() {
    let event = WorldEvent {
        id: 9,
        time: 5,
        kind: oasis7::simulator::WorldEventKind::Power(
            oasis7::simulator::PowerEvent::PowerConsumed {
                agent_id: "agent-1".to_string(),
                amount: 3,
                reason: oasis7::simulator::ConsumeReason::Decision,
                remaining: 7,
            },
        ),
        runtime_event: None,
    };

    let state = ViewerState {
        status: ConnectionStatus::Connected,
        snapshot: None,
        events: vec![event.clone()],
        decision_traces: Vec::new(),
        metrics: None,
    };
    let locale = default_locale();
    let events_text = build_events_text(&state, None, locale);
    assert!(events_text.contains("Power"));
}

#[test]
fn update_ui_populates_agent_activity_panel() {
    let mut model = oasis7::simulator::WorldModel::default();
    model.locations.insert(
        "loc-1".to_string(),
        oasis7::simulator::Location::new("loc-1", "Alpha", oasis7::geometry::GeoPos::new(0, 0, 0)),
    );
    model.locations.insert(
        "loc-2".to_string(),
        oasis7::simulator::Location::new("loc-2", "Beta", oasis7::geometry::GeoPos::new(1, 1, 0)),
    );

    let mut agent = oasis7::simulator::Agent::new("agent-1", "loc-2", GeoPos::new(1, 1, 0));
    agent
        .resources
        .set(ResourceKind::Electricity, 42)
        .expect("set electricity");
    model.agents.insert("agent-1".to_string(), agent);

    let snapshot = oasis7::simulator::WorldSnapshot {
        version: oasis7::simulator::SNAPSHOT_VERSION,
        chunk_generation_schema_version: oasis7::simulator::CHUNK_GENERATION_SCHEMA_VERSION,
        time: 9,
        config: oasis7::simulator::WorldConfig::default(),
        model,
        chunk_runtime: oasis7::simulator::ChunkRuntimeConfig::default(),
        next_event_id: 1,
        next_action_id: 1,
        pending_actions: Vec::new(),
        journal_len: 0,
        runtime_snapshot: None,
        player_gameplay: None,
    };

    let events = vec![WorldEvent {
        id: 7,
        time: 8,
        kind: WorldEventKind::RadiationHarvested {
            agent_id: "agent-1".to_string(),
            location_id: "loc-2".to_string(),
            amount: 6,
            available: 12,
        },
        runtime_event: None,
    }];

    let state = ViewerState {
        status: ConnectionStatus::Connected,
        snapshot: Some(snapshot),
        events,
        decision_traces: Vec::new(),
        metrics: None,
    };
    let locale = default_locale();
    let activity_text = build_agent_activity_text(&state, locale);
    assert!(activity_text.contains("agent-1 @ loc-2"));
    assert!(activity_text.contains("E=42"));
    assert!(activity_text.contains("harvest +6"));
}

#[test]
fn handle_control_buttons_sends_request() {
    let mut app = App::new();
    app.add_systems(Update, handle_control_buttons);

    let (tx, rx) = mpsc::channel::<ViewerRequest>();
    app.world_mut().insert_resource(ViewerClient {
        tx,
        rx: Mutex::new(mpsc::channel::<ViewerResponse>().1),
    });
    app.world_mut().insert_resource(ViewerState::default());
    app.world_mut()
        .insert_resource(StepControlLoadingState::default());

    app.world_mut().spawn((
        Button,
        Interaction::Pressed,
        ControlButton {
            control: ViewerControl::Step { count: 2 },
        },
    ));

    app.update();

    let request = rx.try_recv().expect("request sent");
    match request {
        ViewerRequest::Control {
            mode: ViewerControl::Step { count: 2 },
            request_id,
        } => assert!(
            request_id.is_some(),
            "step requests should carry a request_id"
        ),
        other => panic!("unexpected request: {other:?}"),
    }
}

#[test]
fn control_buttons_send_expected_requests() {
    let mut app = App::new();
    app.add_systems(Update, handle_control_buttons);

    let (tx, rx) = mpsc::channel::<ViewerRequest>();
    app.world_mut().insert_resource(ViewerClient {
        tx,
        rx: Mutex::new(mpsc::channel::<ViewerResponse>().1),
    });
    app.world_mut().insert_resource(ViewerState::default());
    app.world_mut()
        .insert_resource(StepControlLoadingState::default());

    for control in [
        ViewerControl::Play,
        ViewerControl::Pause,
        ViewerControl::Step { count: 1 },
        ViewerControl::Seek { tick: 0 },
    ] {
        app.world_mut().spawn((
            Button,
            Interaction::Pressed,
            ControlButton {
                control: control.clone(),
            },
        ));
    }

    app.update();

    let mut seen = Vec::new();
    while let Ok(request) = rx.try_recv() {
        seen.push(request);
    }

    assert!(seen.iter().any(|request| matches!(
        request,
        ViewerRequest::Control {
            mode: ViewerControl::Play,
            request_id: Some(_),
        }
    )));
    assert!(seen.contains(&ViewerRequest::Control {
        mode: ViewerControl::Pause,
        request_id: None,
    }));
    assert!(seen.iter().any(|request| matches!(
        request,
        ViewerRequest::Control {
            mode: ViewerControl::Step { count: 1 },
            request_id: Some(_),
        }
    )));
    assert!(seen.contains(&ViewerRequest::Control {
        mode: ViewerControl::Seek { tick: 0 },
        request_id: None,
    }));
}

#[test]
fn timeline_adjust_and_submit_sends_seek_request() {
    let mut app = App::new();
    app.add_systems(
        Update,
        (handle_timeline_adjust_buttons, handle_timeline_seek_submit).chain(),
    );

    let (tx, rx) = mpsc::channel::<ViewerRequest>();
    app.world_mut().insert_resource(ViewerClient {
        tx,
        rx: Mutex::new(mpsc::channel::<ViewerResponse>().1),
    });
    app.world_mut().insert_resource(TimelineUiState {
        target_tick: 10,
        max_tick_seen: 100,
        manual_override: false,
        drag_active: false,
    });

    app.world_mut().spawn((
        Button,
        Interaction::Pressed,
        TimelineAdjustButton { delta: 15 },
    ));

    app.update();

    app.world_mut()
        .spawn((Button, Interaction::Pressed, TimelineSeekSubmitButton));

    app.update();

    let request = rx.try_recv().expect("seek request");
    assert_eq!(
        request,
        ViewerRequest::Control {
            mode: ViewerControl::Seek { tick: 25 },
            request_id: None,
        }
    );
}

#[test]
fn timeline_drag_updates_target_tick() {
    let mut app = App::new();
    app.add_systems(Update, handle_timeline_bar_drag);

    app.world_mut().insert_resource(ViewerState::default());
    app.world_mut().insert_resource(TimelineUiState {
        target_tick: 0,
        max_tick_seen: 100,
        manual_override: false,
        drag_active: false,
    });

    app.world_mut().spawn((
        Button,
        Interaction::Pressed,
        bevy::ui::RelativeCursorPosition {
            cursor_over: true,
            normalized: Some(Vec2::new(0.25, 0.0)),
        },
        TimelineBar,
    ));

    app.update();

    let timeline = app.world().resource::<TimelineUiState>();
    assert_eq!(timeline.target_tick, 75);
    assert!(timeline.manual_override);
    assert!(timeline.drag_active);
}

#[test]
fn update_timeline_ui_renders_text_and_fill() {
    let mut app = App::new();
    app.add_systems(Update, update_timeline_ui);

    app.world_mut().spawn((Text::new(""), TimelineStatusText));
    app.world_mut().spawn((
        Node {
            width: Val::Px(0.0),
            height: Val::Px(8.0),
            ..default()
        },
        TimelineBarFill,
    ));

    let mut state = ViewerState::default();
    state.metrics = Some(RunnerMetrics {
        total_ticks: 40,
        ..RunnerMetrics::default()
    });
    app.world_mut().insert_resource(state);
    app.world_mut().insert_resource(TimelineUiState {
        target_tick: 50,
        max_tick_seen: 100,
        manual_override: true,
        drag_active: false,
    });

    app.update();

    let world = app.world_mut();
    let timeline_text = {
        let mut query = world.query::<(&Text, &TimelineStatusText)>();
        query.single(world).expect("timeline text").0.clone()
    };
    assert!(timeline_text.0.contains("now=40"));
    assert!(timeline_text.0.contains("target=50"));
    assert!(timeline_text.0.contains("max=100"));
    assert!(timeline_text.0.contains("mode=manual"));

    let fill_width = {
        let mut query = world.query::<(&Node, &TimelineBarFill)>();
        query.single(world).expect("timeline fill").0.width
    };
    assert_eq!(fill_width, Val::Percent(50.0));
}

#[test]
fn normalized_x_to_tick_maps_centered_range() {
    assert_eq!(normalized_x_to_tick(-0.5, 100), 0);
    assert_eq!(normalized_x_to_tick(0.0, 100), 50);
    assert_eq!(normalized_x_to_tick(0.5, 100), 100);
}

#[test]
fn poll_viewer_messages_collects_decision_traces() {
    let mut app = App::new();
    app.add_systems(Update, poll_viewer_messages);

    app.world_mut().insert_resource(ViewerConfig {
        addr: "127.0.0.1:0".to_string(),
        max_events: 2,
        event_window: EventWindowPolicy::new(2, 2, 1),
    });

    let (tx, rx) = mpsc::channel::<ViewerResponse>();
    app.world_mut().insert_resource(ViewerClient {
        tx: mpsc::channel::<ViewerRequest>().0,
        rx: Mutex::new(rx),
    });
    app.world_mut().insert_resource(ViewerState::default());

    tx.send(ViewerResponse::DecisionTrace {
        trace: oasis7::simulator::AgentDecisionTrace {
            agent_id: "agent-1".to_string(),
            time: 1,
            decision: oasis7::simulator::AgentDecision::Wait,
            llm_input: Some("p1".to_string()),
            llm_output: Some("o1".to_string()),
            llm_error: None,
            parse_error: None,
            llm_diagnostics: None,
            llm_effect_intents: Vec::new(),
            llm_effect_receipts: Vec::new(),
            llm_step_trace: Vec::new(),
            llm_prompt_section_trace: Vec::new(),
            llm_chat_messages: Vec::new(),
        },
    })
    .expect("send trace1");
    tx.send(ViewerResponse::DecisionTrace {
        trace: oasis7::simulator::AgentDecisionTrace {
            agent_id: "agent-1".to_string(),
            time: 2,
            decision: oasis7::simulator::AgentDecision::Wait,
            llm_input: Some("p2".to_string()),
            llm_output: Some("o2".to_string()),
            llm_error: None,
            parse_error: None,
            llm_diagnostics: None,
            llm_effect_intents: Vec::new(),
            llm_effect_receipts: Vec::new(),
            llm_step_trace: Vec::new(),
            llm_prompt_section_trace: Vec::new(),
            llm_chat_messages: Vec::new(),
        },
    })
    .expect("send trace2");
    tx.send(ViewerResponse::DecisionTrace {
        trace: oasis7::simulator::AgentDecisionTrace {
            agent_id: "agent-1".to_string(),
            time: 3,
            decision: oasis7::simulator::AgentDecision::Wait,
            llm_input: Some("p3".to_string()),
            llm_output: Some("o3".to_string()),
            llm_error: None,
            parse_error: None,
            llm_diagnostics: None,
            llm_effect_intents: Vec::new(),
            llm_effect_receipts: Vec::new(),
            llm_step_trace: Vec::new(),
            llm_prompt_section_trace: Vec::new(),
            llm_chat_messages: Vec::new(),
        },
    })
    .expect("send trace3");

    app.update();

    let state = app.world_mut().resource::<ViewerState>();
    assert_eq!(state.decision_traces.len(), 2);
    assert_eq!(state.decision_traces[0].time, 2);
    assert_eq!(state.decision_traces[1].time, 3);
}

#[test]
fn headless_report_tracks_status_and_event_count() {
    let mut app = App::new();
    app.add_systems(Update, headless_report);
    app.world_mut().insert_resource(HeadlessStatus::default());

    app.world_mut().insert_resource(ViewerState {
        status: ConnectionStatus::Connecting,
        snapshot: None,
        events: Vec::new(),
        decision_traces: Vec::new(),
        metrics: None,
    });

    app.update();

    let status = app.world_mut().resource::<HeadlessStatus>();
    assert_eq!(status.last_status, Some(ConnectionStatus::Connecting));
    assert_eq!(status.last_events, 0);

    app.world_mut().insert_resource(ViewerState {
        status: ConnectionStatus::Connected,
        snapshot: None,
        events: vec![WorldEvent {
            id: 1,
            time: 1,
            kind: oasis7::simulator::WorldEventKind::ActionRejected {
                reason: oasis7::simulator::RejectReason::InvalidAmount { amount: 1 },
            },
            runtime_event: None,
        }],
        decision_traces: Vec::new(),
        metrics: None,
    });

    app.update();

    let status = app.world_mut().resource::<HeadlessStatus>();
    assert_eq!(status.last_status, Some(ConnectionStatus::Connected));
    assert_eq!(status.last_events, 1);
}

#[test]
fn headless_auto_play_sends_play_once_after_connected() {
    struct AutoPlayEnvGuard;
    impl Drop for AutoPlayEnvGuard {
        fn drop(&mut self) {
            unsafe {
                std::env::remove_var("OASIS7_VIEWER_AUTO_PLAY");
            }
        }
    }
    unsafe {
        std::env::set_var("OASIS7_VIEWER_AUTO_PLAY", "1");
    }
    let _guard = AutoPlayEnvGuard;

    let mut app = App::new();
    app.add_systems(Update, headless_auto_play_once);

    let (tx_request, rx_request) = mpsc::channel::<ViewerRequest>();
    app.world_mut().insert_resource(ViewerClient {
        tx: tx_request,
        rx: Mutex::new(mpsc::channel::<ViewerResponse>().1),
    });
    app.world_mut().insert_resource(ViewerState {
        status: ConnectionStatus::Connected,
        ..ViewerState::default()
    });

    app.update();
    let first = rx_request.try_recv().expect("first control request");
    match first {
        ViewerRequest::Control {
            mode: ViewerControl::Play,
            request_id,
        } => assert!(request_id.is_some(), "auto-play should carry a request_id"),
        other => panic!("unexpected request: {other:?}"),
    }

    app.update();
    assert!(rx_request.try_recv().is_err());
}

#[test]
fn poll_viewer_messages_applies_event_window_sampling_policy() {
    let mut app = App::new();
    app.add_systems(Update, poll_viewer_messages);

    app.world_mut().insert_resource(ViewerConfig {
        addr: "127.0.0.1:0".to_string(),
        max_events: 16,
        event_window: EventWindowPolicy::new(6, 3, 2),
    });

    let (tx, rx) = mpsc::channel::<ViewerResponse>();
    app.world_mut().insert_resource(ViewerClient {
        tx: mpsc::channel::<ViewerRequest>().0,
        rx: Mutex::new(rx),
    });
    app.world_mut().insert_resource(ViewerState::default());

    for id in 1..=8_u64 {
        tx.send(ViewerResponse::Event {
            event: WorldEvent {
                id,
                time: id,
                kind: oasis7::simulator::WorldEventKind::ActionRejected {
                    reason: oasis7::simulator::RejectReason::InvalidAmount { amount: id as i64 },
                },
                runtime_event: None,
            },
        })
        .expect("send event");
    }

    app.update();

    let state = app.world().resource::<ViewerState>();
    let ids: Vec<u64> = state.events.iter().map(|event| event.id).collect();
    assert_eq!(ids, vec![1, 3, 5, 6, 7, 8]);
}

#[test]
fn poll_viewer_messages_keeps_transport_connected_on_agent_chat_error() {
    let mut app = App::new();
    app.add_systems(Update, poll_viewer_messages);

    app.world_mut().insert_resource(ViewerConfig {
        addr: "127.0.0.1:0".to_string(),
        max_events: 16,
        event_window: EventWindowPolicy::new(8, 4, 2),
    });

    let (tx, rx) = mpsc::channel::<ViewerResponse>();
    app.world_mut().insert_resource(ViewerClient {
        tx: mpsc::channel::<ViewerRequest>().0,
        rx: Mutex::new(rx),
    });
    app.world_mut().insert_resource(ViewerState {
        status: ConnectionStatus::Connected,
        ..ViewerState::default()
    });
    app.world_mut().insert_resource(ViewerControlProfileState {
        profile: Some(ViewerControlProfile::Live),
    });

    tx.send(ViewerResponse::AgentChatError {
        error: oasis7::viewer::AgentChatError {
            code: "invalid_auth".to_string(),
            message: "signature mismatch".to_string(),
            agent_id: Some("agent-a".to_string()),
        },
    })
    .expect("send agent chat error");

    app.update();

    let state = app.world().resource::<ViewerState>();
    assert_eq!(state.status, ConnectionStatus::Connected);

    let profile = app.world().resource::<ViewerControlProfileState>();
    assert_eq!(profile.profile, Some(ViewerControlProfile::Live));
}

#[test]
fn friendly_connection_error_maps_transport_messages() {
    assert_eq!(
        friendly_connection_error("websocket closed: code=1006 reason="),
        "connection closed (code 1006), retrying..."
    );
    assert_eq!(
        friendly_connection_error("Connection refused (os error 61)"),
        "viewer server unreachable, retrying..."
    );
    assert_eq!(
        friendly_connection_error("disconnected"),
        "viewer disconnected, retrying..."
    );
    assert_eq!(friendly_connection_error("offline mode"), "offline mode");
}

#[test]
fn reconnectable_error_signature_skips_non_transport_messages() {
    assert_eq!(
        reconnectable_error_signature("websocket closed: code=1006 reason=").as_deref(),
        Some("websocket")
    );
    assert_eq!(
        reconnectable_error_signature("Connection refused (os error 61)").as_deref(),
        Some("connection_refused")
    );
    assert!(reconnectable_error_signature("offline mode").is_none());
    assert!(reconnectable_error_signature("agent chat error: invalid auth (401)").is_none());
}

#[test]
fn attempt_viewer_reconnect_transitions_error_status_back_to_connecting() {
    let mut app = App::new();
    app.add_systems(Update, attempt_viewer_reconnect);
    app.world_mut().insert_resource(ViewerConfig {
        addr: "127.0.0.1:0".to_string(),
        max_events: 8,
        event_window: EventWindowPolicy::new(8, 4, 2),
    });
    app.world_mut().insert_resource(ViewerState {
        status: ConnectionStatus::Error("disconnected".to_string()),
        ..ViewerState::default()
    });

    app.update();

    let state = app.world().resource::<ViewerState>();
    assert_eq!(state.status, ConnectionStatus::Connecting);
    assert!(app.world().contains_resource::<ViewerClient>());
}

#[test]
fn attempt_viewer_reconnect_ignores_non_transport_errors() {
    let mut app = App::new();
    app.add_systems(Update, attempt_viewer_reconnect);
    app.world_mut().insert_resource(ViewerConfig {
        addr: "127.0.0.1:0".to_string(),
        max_events: 8,
        event_window: EventWindowPolicy::new(8, 4, 2),
    });
    app.world_mut().insert_resource(ViewerState {
        status: ConnectionStatus::Error("agent chat error: invalid auth (401)".to_string()),
        ..ViewerState::default()
    });

    app.update();

    let state = app.world().resource::<ViewerState>();
    assert_eq!(
        state.status,
        ConnectionStatus::Error("agent chat error: invalid auth (401)".to_string())
    );
    assert!(!app.world().contains_resource::<ViewerClient>());
}

#[test]
fn decide_offline_defaults_headless_and_respects_overrides() {
    assert!(decide_offline(true, false, false));
    assert!(!decide_offline(false, false, false));
    assert!(decide_offline(false, true, false));
    assert!(!decide_offline(true, true, true));
    assert!(!decide_offline(true, false, true));
}

#[test]
fn space_origin_is_center_of_bounds() {
    let space = SpaceConfig {
        width_cm: 100,
        depth_cm: 200,
        height_cm: 300,
    };
    let origin = space_origin(&space);
    assert_eq!(origin.x_cm, 50);
    assert_eq!(origin.y_cm, 100);
    assert_eq!(origin.z_cm, 150);
}

#[test]
fn geo_to_vec3_scales_and_swaps_axes() {
    let origin = GeoPos::new(100, 200, 300);
    let pos = GeoPos::new(110, 220, 330);
    let vec = geo_to_vec3(pos, origin, 0.01);
    assert!((vec.x - 0.1).abs() < 1e-6);
    assert!((vec.y - 0.3).abs() < 1e-6);
    assert!((vec.z - 0.2).abs() < 1e-6);
}

#[test]
fn ray_point_distance_returns_expected_distance() {
    let ray = Ray3d {
        origin: Vec3::ZERO,
        direction: Dir3::new(Vec3::X).expect("direction"),
    };
    let point = Vec3::new(2.0, 1.0, 0.0);
    let distance = ray_point_distance(ray, point).expect("distance");
    assert!((distance - 1.0).abs() < 1e-6);
    assert!(ray_point_distance(ray, Vec3::new(-1.0, 0.0, 0.0)).is_none());
}

#[test]
fn lighting_illuminance_triplet_tracks_key_fill_rim_ratios() {
    let mut config = Viewer3dConfig::default();
    config.physical.enabled = true;
    config.physical.stellar_distance_au = 2.5;
    config.physical.luminous_efficacy_lm_per_w = 120.0;
    config.physical.exposure_ev100 = 13.5;
    config.lighting.fill_light_ratio = 0.30;
    config.lighting.rim_light_ratio = 0.10;

    let (key, fill, rim) = lighting_illuminance_triplet(&config);
    assert!(key > fill);
    assert!(fill > rim);
    assert!(((fill / key) - 0.30).abs() < 0.01);
    assert!(((rim / key) - 0.10).abs() < 0.01);
}

#[test]
fn lighting_illuminance_triplet_clamps_low_fill_and_rim() {
    let mut config = Viewer3dConfig::default();
    config.physical.enabled = true;
    config.physical.stellar_distance_au = 2.5;
    config.physical.luminous_efficacy_lm_per_w = 120.0;
    config.physical.exposure_ev100 = 13.5;
    config.lighting.fill_light_ratio = 0.0;
    config.lighting.rim_light_ratio = 0.0;

    let (_key, fill, rim) = lighting_illuminance_triplet(&config);
    assert!((fill - 800.0).abs() < f32::EPSILON);
    assert!((rim - 450.0).abs() < f32::EPSILON);
}

#[test]
fn camera_post_process_components_map_config_values() {
    let mut config = Viewer3dConfig::default();
    config.post_process.tonemapping = ViewerTonemappingMode::AcesFitted;
    config.post_process.deband_dither_enabled = true;
    config.post_process.bloom_enabled = true;
    config.post_process.bloom_intensity = 0.33;
    config.post_process.color_grading_exposure = 0.6;
    config.post_process.color_grading_post_saturation = 1.14;

    let (tonemapping, deband, grading, bloom) = camera_post_process_components(&config);
    assert_eq!(tonemapping, Tonemapping::AcesFitted);
    assert_eq!(deband, DebandDither::Enabled);
    assert!((grading.global.exposure - 0.6).abs() < f32::EPSILON);
    assert!((grading.global.post_saturation - 1.14).abs() < f32::EPSILON);
    assert!((bloom.expect("bloom").intensity - 0.33).abs() < f32::EPSILON);
}

#[test]
fn camera_post_process_components_disable_bloom_and_deband() {
    let mut config = Viewer3dConfig::default();
    config.post_process.tonemapping = ViewerTonemappingMode::None;
    config.post_process.deband_dither_enabled = false;
    config.post_process.bloom_enabled = false;
    config.post_process.color_grading_exposure = -0.35;
    config.post_process.color_grading_post_saturation = 0.82;

    let (tonemapping, deband, grading, bloom) = camera_post_process_components(&config);
    assert_eq!(tonemapping, Tonemapping::None);
    assert_eq!(deband, DebandDither::Disabled);
    assert!((grading.global.exposure + 0.35).abs() < f32::EPSILON);
    assert!((grading.global.post_saturation - 0.82).abs() < f32::EPSILON);
    assert!(bloom.is_none());
}

#[test]
fn resolve_srgb_slot_color_prefers_override() {
    let resolved = resolve_srgb_slot_color([0.11, 0.22, 0.33], Some([0.44, 0.55, 0.66]));
    assert!((resolved[0] - 0.44).abs() < f32::EPSILON);
    assert!((resolved[1] - 0.55).abs() < f32::EPSILON);
    assert!((resolved[2] - 0.66).abs() < f32::EPSILON);
}

#[test]
fn emissive_from_srgb_with_boost_clamps_components() {
    let clamped = emissive_from_srgb_with_boost([1.0, 1.0, 1.0], 6.0);
    let clamped_more = emissive_from_srgb_with_boost([1.0, 1.0, 1.0], 100.0);
    assert!((clamped.red - clamped_more.red).abs() < f32::EPSILON);
    assert!((clamped.green - clamped_more.green).abs() < f32::EPSILON);
    assert!((clamped.blue - clamped_more.blue).abs() < f32::EPSILON);
}

#[test]
fn location_material_override_enabled_detects_any_slot_override() {
    let empty = ViewerExternalMaterialSlotConfig::default();
    assert!(!location_material_override_enabled(empty));

    let base_only = ViewerExternalMaterialSlotConfig {
        base_color_srgb: Some([0.2, 0.3, 0.4]),
        emissive_color_srgb: None,
    };
    assert!(location_material_override_enabled(base_only));

    let emissive_only = ViewerExternalMaterialSlotConfig {
        base_color_srgb: None,
        emissive_color_srgb: Some([0.6, 0.7, 0.8]),
    };
    assert!(location_material_override_enabled(emissive_only));
}

#[test]
fn texture_slot_override_enabled_detects_any_texture_channel_override() {
    let empty = ViewerExternalTextureSlotConfig::default();
    assert!(!texture_slot_override_enabled(&empty));

    let base_override = ViewerExternalTextureSlotConfig {
        base_texture_asset: Some("textures/world/location_albedo.png".to_string()),
        ..ViewerExternalTextureSlotConfig::default()
    };
    assert!(texture_slot_override_enabled(&base_override));

    let normal_override = ViewerExternalTextureSlotConfig {
        normal_texture_asset: Some("textures/world/location_normal.png".to_string()),
        ..ViewerExternalTextureSlotConfig::default()
    };
    assert!(texture_slot_override_enabled(&normal_override));

    let metal_rough_override = ViewerExternalTextureSlotConfig {
        metallic_roughness_texture_asset: Some(
            "textures/world/location_metal_rough.png".to_string(),
        ),
        ..ViewerExternalTextureSlotConfig::default()
    };
    assert!(texture_slot_override_enabled(&metal_rough_override));

    let emissive_override = ViewerExternalTextureSlotConfig {
        emissive_texture_asset: Some("textures/world/location_emissive.png".to_string()),
        ..ViewerExternalTextureSlotConfig::default()
    };
    assert!(texture_slot_override_enabled(&emissive_override));
}

#[test]
fn location_style_override_enabled_detects_material_or_texture_override() {
    let material_empty = ViewerExternalMaterialSlotConfig::default();
    let texture_empty = ViewerExternalTextureSlotConfig::default();
    assert!(!location_style_override_enabled(
        material_empty,
        &texture_empty
    ));

    let material_only = ViewerExternalMaterialSlotConfig {
        base_color_srgb: Some([0.3, 0.5, 0.7]),
        emissive_color_srgb: None,
    };
    assert!(location_style_override_enabled(
        material_only,
        &texture_empty
    ));

    let texture_only = ViewerExternalTextureSlotConfig {
        normal_texture_asset: Some("textures/world/location_normal.png".to_string()),
        ..ViewerExternalTextureSlotConfig::default()
    };
    assert!(location_style_override_enabled(
        material_empty,
        &texture_only
    ));
}

#[test]
fn parse_material_variant_preset_supports_aliases() {
    assert_eq!(
        parse_material_variant_preset("default"),
        Some(ViewerMaterialVariantPreset::Default)
    );
    assert_eq!(
        parse_material_variant_preset("MATTE"),
        Some(ViewerMaterialVariantPreset::Matte)
    );
    assert_eq!(
        parse_material_variant_preset("shine"),
        Some(ViewerMaterialVariantPreset::Glossy)
    );
    assert_eq!(parse_material_variant_preset("unknown"), None);
}

#[test]
fn resolve_material_variant_preview_state_defaults_when_missing_or_invalid() {
    let missing = resolve_material_variant_preview_state_from(|_| None);
    assert_eq!(missing.active, ViewerMaterialVariantPreset::Default);

    let invalid = resolve_material_variant_preview_state_from(|_| Some("bad".to_string()));
    assert_eq!(invalid.active, ViewerMaterialVariantPreset::Default);

    let matte = resolve_material_variant_preview_state_from(|_| Some("matte".to_string()));
    assert_eq!(matte.active, ViewerMaterialVariantPreset::Matte);
}

#[test]
fn material_variant_preset_next_cycles_in_order() {
    assert_eq!(
        ViewerMaterialVariantPreset::Default.next(),
        ViewerMaterialVariantPreset::Matte
    );
    assert_eq!(
        ViewerMaterialVariantPreset::Matte.next(),
        ViewerMaterialVariantPreset::Glossy
    );
    assert_eq!(
        ViewerMaterialVariantPreset::Glossy.next(),
        ViewerMaterialVariantPreset::Default
    );
}

#[test]
fn material_variant_scalars_follow_expected_direction() {
    let matte = material_variant_scalars(ViewerMaterialVariantPreset::Matte);
    assert!(matte.roughness_scale > 1.0);
    assert!(matte.metallic_scale < 1.0);

    let glossy = material_variant_scalars(ViewerMaterialVariantPreset::Glossy);
    assert!(glossy.roughness_scale < 1.0);
    assert!(glossy.metallic_scale > 1.0);
}

#[test]
fn apply_material_variant_scalar_clamps_to_unit_range() {
    assert!((apply_material_variant_scalar(0.5, 2.0) - 1.0).abs() < f32::EPSILON);
    assert!((apply_material_variant_scalar(0.5, 0.5) - 0.25).abs() < f32::EPSILON);
}

#[test]
fn apply_material_variant_to_material_updates_roughness_and_metallic() {
    let mut materials = Assets::<StandardMaterial>::default();
    let handle = materials.add(StandardMaterial::default());

    apply_material_variant_to_material(
        &mut materials,
        &handle,
        0.4,
        0.6,
        ViewerMaterialVariantPreset::Glossy,
    );

    let material = materials.get(&handle).expect("material exists");
    assert!(material.perceptual_roughness < 0.4);
    assert!(material.metallic > 0.6);
}

#[path = "tests_scene_entities.rs"]
mod tests_scene_entities;

#[path = "tests_selection_panels.rs"]
mod tests_selection_panels;

#[path = "tests_scene_helpers.rs"]
mod tests_scene_helpers;

use tests_scene_helpers::*;
