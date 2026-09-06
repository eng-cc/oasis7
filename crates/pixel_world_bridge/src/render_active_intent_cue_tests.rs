use super::fixtures::sample_render_state_with_receipt_target;
use super::*;

fn active_intent_cues(app: &mut App) -> Vec<(String, f32)> {
    let world = app.world_mut();
    let mut query = world.query::<(&PixelWorldActiveIntentCue, &Transform)>();
    let mut cues = query
        .iter(world)
        .map(|(cue, transform)| (cue.agent_id.clone(), transform.translation.z))
        .collect::<Vec<_>>();
    cues.sort_by(|left, right| left.0.cmp(&right.0));
    cues
}

fn active_intent_size(app: &mut App) -> Vec2 {
    let world = app.world_mut();
    let mut query = world.query::<(&PixelWorldActiveIntentCue, &Sprite)>();
    query
        .iter(world)
        .next()
        .and_then(|(_, sprite)| sprite.custom_size)
        .expect("active Intent cue size")
}

fn selected_and_blocker_z(app: &mut App) -> (f32, f32) {
    let world = app.world_mut();
    let mut selected_query = world.query::<(&PixelWorldSelectedAgentCue, &Transform)>();
    let selected_z = selected_query
        .iter(world)
        .map(|(_, transform)| transform.translation.z)
        .fold(0.0, f32::max);
    let mut blocker_query = world.query::<(&PixelWorldHotspotVisual, &Transform)>();
    let blocker_z = blocker_query
        .iter(world)
        .find(|(hotspot, _)| hotspot.id == "blocker-attention")
        .map(|(_, transform)| transform.translation.z)
        .expect("blocker attention hotspot");
    (selected_z, blocker_z)
}

#[test]
fn authoritative_active_intent_is_display_only_reused_and_below_receipt() {
    let mut state = sample_render_state_with_receipt_target(None, None);
    state.agents[0].position_source = AgentPositionSource::Snapshot;
    state.active_intent_target = Some(ActiveIntentTarget {
        agent_id: "agent-0".to_string(),
        status: "accepted".to_string(),
    });
    state.visual_hotspots.push(VisualHotspot {
        id: "blocker-attention".to_string(),
        label: "Blocked".to_string(),
        kind: "blocker".to_string(),
        pos: state.agents[0].pos.clone().expect("positioned Agent"),
        emphasis: Some(1.0),
        size_hint_px: Some(10.0),
    });
    let mut app = render_test_app(state.clone());
    let hit_regions_before = hit_regions(&mut app);

    let cues = active_intent_cues(&mut app);
    assert!(!cues.is_empty(), "active Intent must have a visible cue");
    assert!(
        cues.iter()
            .all(|(_, z)| *z > AGENT_LAYER_Z + SELECTED_ENTITY_LAYER_Z_OFFSET),
        "active Intent must read above the selected Agent"
    );
    assert_eq!(hit_regions(&mut app), hit_regions_before);
    let (selected_z, blocker_z) = selected_and_blocker_z(&mut app);
    let active_z = cues[0].1;
    assert!(selected_z < active_z && active_z < blocker_z);

    app.update();
    assert_eq!(
        active_intent_cues(&mut app),
        cues,
        "cue reconciliation must be deterministic"
    );

    let accepted_size = active_intent_size(&mut app);
    state.active_intent_target.as_mut().unwrap().status = "blocked".to_string();
    app.world_mut()
        .resource_mut::<BevyRuntimeState>()
        .render_state = Some(state.clone());
    app.update();
    assert_ne!(
        active_intent_size(&mut app),
        accepted_size,
        "blocked and accepted Intent states need a non-color geometry difference"
    );

    state.receipt_target = Some(ReceiptTarget {
        agent_id: "agent-0".to_string(),
        state: "accepted".to_string(),
    });
    app.world_mut()
        .resource_mut::<BevyRuntimeState>()
        .render_state = Some(state);
    app.update();
    let receipt_z = {
        let world = app.world_mut();
        let mut query = world.query::<(&PixelWorldReceiptTargetCue, &Transform)>();
        query
            .iter(world)
            .map(|(_, transform)| transform.translation.z)
            .fold(0.0, f32::max)
    };
    assert!(
        active_intent_cues(&mut app)
            .iter()
            .all(|(_, z)| *z < receipt_z)
    );
    assert!(blocker_z < receipt_z);
    assert_eq!(hit_regions(&mut app), hit_regions_before);

    app.world_mut()
        .resource_mut::<BevyRuntimeState>()
        .render_state = Some(sample_render_state_with_receipt_target(None, None));
    app.update();
    assert!(
        active_intent_cues(&mut app).is_empty(),
        "removed authority must clean up the cue"
    );
}

#[test]
fn active_intent_cue_has_a_stable_nonzero_raster_delta() {
    let baseline_state = sample_render_state_with_receipt_target(None, None);
    let mut baseline = render_test_app(baseline_state.clone());
    let (baseline_image, baseline_summary) = rasterize_pixel_regression(&mut baseline);

    let mut visible_state = baseline_state;
    visible_state.agents[0].position_source = AgentPositionSource::Snapshot;
    visible_state.active_intent_target = Some(ActiveIntentTarget {
        agent_id: "agent-0".to_string(),
        status: "accepted".to_string(),
    });
    let mut visible = render_test_app(visible_state.clone());
    let (visible_image, visible_summary) = rasterize_pixel_regression(&mut visible);
    let mut repeated = render_test_app(visible_state);
    let (repeated_image, repeated_summary) = rasterize_pixel_regression(&mut repeated);

    assert!(
        baseline_image
            .pixels()
            .zip(visible_image.pixels())
            .any(|(baseline, visible)| baseline != visible),
        "the authoritative active Intent cue must add visible raster pixels"
    );
    assert_ne!(
        visible_summary.raw_rgba_fnv1a64, baseline_summary.raw_rgba_fnv1a64,
        "the active Intent cue must change the deterministic raster signature"
    );
    assert_eq!(
        visible_summary.raw_rgba_fnv1a64, repeated_summary.raw_rgba_fnv1a64,
        "the same active Intent cue must reproduce a stable raster hash"
    );
    assert_eq!(visible_image.as_raw(), repeated_image.as_raw());
}

#[test]
fn active_intent_cue_requires_snapshot_position_provenance() {
    let mut derived_state = sample_render_state_with_receipt_target(None, None);
    derived_state.active_intent_target = Some(ActiveIntentTarget {
        agent_id: "agent-0".to_string(),
        status: "accepted".to_string(),
    });
    let mut app = render_test_app(derived_state);
    assert!(
        active_intent_cues(&mut app).is_empty(),
        "location-derived position must not receive an active Intent cue"
    );

    app.world_mut()
        .resource_mut::<BevyRuntimeState>()
        .render_state
        .as_mut()
        .expect("render state")
        .agents[0]
        .position_source = AgentPositionSource::Snapshot;
    app.update();
    assert_eq!(active_intent_cues(&mut app).len(), 1);
}
