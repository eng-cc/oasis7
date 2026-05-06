use super::*;
use oasis7::geometry::GeoPos;
use oasis7::simulator::RejectReason;
use oasis7::simulator::{
    Agent, ChunkRuntimeConfig, PowerEvent, WorldConfig, WorldModel, WorldSnapshot,
    CHUNK_GENERATION_SCHEMA_VERSION, SNAPSHOT_VERSION,
};
use std::{hint::black_box, time::Instant};

#[test]
fn events_summary_without_focus_keeps_compact_view() {
    let events = vec![WorldEvent {
        id: 1,
        time: 7,
        kind: WorldEventKind::ActionRejected {
            reason: RejectReason::InvalidAmount { amount: 1 },
        },
        runtime_event: None,
    }];

    let text = events_summary(&events, None);
    assert!(text.starts_with("Events:"));
    assert!(text.contains("#1 t7"));
    assert!(!text.contains("Events (focused):"));
}

#[test]
fn events_summary_with_focus_marks_nearest_context() {
    let events = vec![
        WorldEvent {
            id: 1,
            time: 3,
            kind: WorldEventKind::ActionRejected {
                reason: RejectReason::InvalidAmount { amount: 1 },
            },
            runtime_event: None,
        },
        WorldEvent {
            id: 2,
            time: 8,
            kind: WorldEventKind::ActionRejected {
                reason: RejectReason::InvalidAmount { amount: 2 },
            },
            runtime_event: None,
        },
        WorldEvent {
            id: 3,
            time: 11,
            kind: WorldEventKind::ActionRejected {
                reason: RejectReason::InvalidAmount { amount: 3 },
            },
            runtime_event: None,
        },
    ];

    let text = events_summary(&events, Some(9));
    assert!(text.starts_with("Events (focused):"));
    assert!(text.contains("Focus: requested t9 -> nearest t8 (#2), Δt=1"));
    assert!(text.contains(">> #2 t8"));
}

#[test]
fn thermal_ratio_color_follows_design_thresholds() {
    assert_eq!(thermal_ratio_color(0.0), "heat_low");
    assert_eq!(thermal_ratio_color(0.6), "heat_low");
    assert_eq!(thermal_ratio_color(0.61), "heat_mid");
    assert_eq!(thermal_ratio_color(1.0), "heat_mid");
    assert_eq!(thermal_ratio_color(1.01), "heat_high");
}

#[test]
fn radiation_visual_metrics_convert_to_power_and_flux() {
    let (power, flux, area) = radiation_visual_metrics(12, 1_000, 10, 2.0);
    assert!((power - 1_200.0).abs() < f64::EPSILON);
    assert!((flux - 600.0).abs() < f64::EPSILON);
    assert!((area - 2.0).abs() < f64::EPSILON);

    let (_, fallback_flux, fallback_area) = radiation_visual_metrics(3, 500, 0, 0.0);
    assert!((fallback_area - 1.0).abs() < f64::EPSILON);
    assert!((fallback_flux - 1_500.0).abs() < f64::EPSILON);
}

#[test]
fn world_summary_includes_physical_render_block_when_enabled() {
    let mut physical = ViewerPhysicalRenderConfig::default();
    physical.enabled = true;
    physical.meters_per_unit = 1.0;
    physical.stellar_distance_au = 2.5;
    physical.exposure_ev100 = 13.5;
    physical.reference_radiation_area_m2 = 2.0;

    let summary = world_summary(None, None, Some(&physical));
    assert!(summary.contains("World: (no snapshot)"));
    assert!(summary.contains("Render Physical: on"));
    assert!(summary.contains("Unit: 1u=1.00m"));
    assert!(summary.contains("Camera Clip(m): near=0.10 far=25000"));
    assert!(summary.contains("Stellar Distance(AU): 2.50"));
    assert!(summary.contains("Irradiance(W/m²): 217.8"));
    assert!(summary.contains("Exposed Illuminance(lux): 26131"));
    assert!(summary.contains("Exposure(EV100): 13.50"));
    assert!(summary.contains("Radiation Ref Area(m²): 2.00"));
}

#[test]
fn world_summary_displays_physical_flag_when_disabled() {
    let physical = ViewerPhysicalRenderConfig::default();
    let summary = world_summary(None, None, Some(&physical));
    assert!(summary.contains("Render Physical: off"));
    assert!(!summary.contains("Unit: 1u="));
}

#[test]
fn agent_activity_summary_uses_latest_matching_event_per_agent() {
    let snapshot = sample_agent_activity_snapshot(2);
    let events = vec![
        WorldEvent {
            id: 1,
            time: 1,
            kind: WorldEventKind::AgentMoved {
                agent_id: "agent-000".to_string(),
                from: "loc-000".to_string(),
                to: "loc-001".to_string(),
                distance_cm: 120,
                electricity_cost: 2,
            },
            runtime_event: None,
        },
        WorldEvent {
            id: 2,
            time: 2,
            kind: WorldEventKind::Power(PowerEvent::PowerCharged {
                agent_id: "agent-001".to_string(),
                amount: 5,
                new_level: 5,
            }),
            runtime_event: None,
        },
        WorldEvent {
            id: 3,
            time: 3,
            kind: WorldEventKind::RadiationHarvested {
                agent_id: "agent-000".to_string(),
                location_id: "loc-001".to_string(),
                amount: 7,
                available: 20,
            },
            runtime_event: None,
        },
    ];

    let summary = agent_activity_summary(Some(&snapshot), &events);
    assert!(summary.contains("agent-000 @ loc-000 | E=0 | t3 harvest +7 at loc-001"));
    assert!(summary.contains("agent-001 @ loc-001 | E=0 | t2 power +5"));
}

fn sample_agent_activity_snapshot(agent_count: usize) -> WorldSnapshot {
    let mut model = WorldModel::default();
    for idx in 0..agent_count {
        let agent_id = format!("agent-{idx:03}");
        let location_id = format!("loc-{:03}", idx % 8);
        model.agents.insert(
            agent_id.clone(),
            Agent::new(
                agent_id.as_str(),
                location_id.as_str(),
                GeoPos::new(idx as i64 * 10, 0, 0),
            ),
        );
    }
    WorldSnapshot {
        version: SNAPSHOT_VERSION,
        chunk_generation_schema_version: CHUNK_GENERATION_SCHEMA_VERSION,
        time: 256,
        config: WorldConfig::default(),
        model,
        chunk_runtime: ChunkRuntimeConfig::default(),
        next_event_id: 20_000,
        next_action_id: 1_000,
        pending_actions: Vec::new(),
        journal_len: 20_000,
        runtime_snapshot: None,
        player_gameplay: None,
    }
}

fn sample_agent_activity_events(agent_count: usize, rounds: usize) -> Vec<WorldEvent> {
    let mut events = Vec::with_capacity(agent_count * rounds);
    let mut next_id = 1_u64;
    for round in 0..rounds {
        for agent_idx in 0..agent_count {
            events.push(WorldEvent {
                id: next_id,
                time: next_id,
                kind: WorldEventKind::AgentMoved {
                    agent_id: format!("agent-{agent_idx:03}"),
                    from: format!("loc-{:03}", (agent_idx + round) % 8),
                    to: format!("loc-{:03}", (agent_idx + round + 1) % 8),
                    distance_cm: 100 + agent_idx as i64,
                    electricity_cost: 1 + (agent_idx as i64 % 3),
                },
                runtime_event: None,
            });
            next_id += 1;
        }
    }
    events
}

#[test]
#[ignore = "perf harness"]
fn perf_agent_activity_summary_many_agents_and_events() {
    let snapshot = sample_agent_activity_snapshot(256);
    let events = sample_agent_activity_events(256, 96);
    let iterations = 30;

    let started_at = Instant::now();
    let mut checksum = 0usize;
    for _ in 0..iterations {
        let summary = agent_activity_summary(Some(&snapshot), &events);
        checksum = checksum.saturating_add(black_box(summary.len()));
    }
    let elapsed = started_at.elapsed();

    println!(
        "perf agent_activity total_ms={:.2} avg_ms={:.3} checksum={} agents={} events={}",
        elapsed.as_secs_f64() * 1000.0,
        elapsed.as_secs_f64() * 1000.0 / iterations as f64,
        checksum,
        snapshot.model.agents.len(),
        events.len(),
    );

    assert!(checksum > 0);
}
