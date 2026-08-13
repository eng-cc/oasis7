use super::fixtures::sample_render_state_with_receipt_target;
use super::*;

fn power_state(state: &str) -> RenderState {
    let mut render_state = sample_render_state(12_000.0);
    render_state.agents[0].power_state = Some(state.to_string());
    render_state
}

fn power_cues(app: &mut App) -> Vec<(&'static str, Vec2, Vec2, Color, f32)> {
    let world = app.world_mut();
    let mut query = world.query::<(&PixelWorldAgentPowerCue, &Sprite, &Transform)>();
    query
        .iter(world)
        .filter_map(|(_, sprite, transform)| {
            Some((
                "power",
                transform.translation.truncate(),
                sprite.custom_size?,
                sprite.color,
                transform.translation.z,
            ))
        })
        .collect()
}

fn max_z_for<T: Component>(app: &mut App) -> Option<f32> {
    let world = app.world_mut();
    let mut query = world.query_filtered::<&Transform, With<T>>();
    query
        .iter(world)
        .map(|transform| transform.translation.z)
        .max_by(f32::total_cmp)
}

fn max_z_for_agent_core(app: &mut App, agent_id: &str) -> Option<f32> {
    let world = app.world_mut();
    let mut query = world.query::<(&PixelWorldAgentCoreVisual, &Transform)>();
    query
        .iter(world)
        .filter(|(core, _)| core.id == agent_id)
        .map(|(_, transform)| transform.translation.z)
        .max_by(f32::total_cmp)
}

fn max_z_for_power_agent(app: &mut App, agent_id: &str) -> Option<f32> {
    let world = app.world_mut();
    let mut query = world.query::<(&PixelWorldAgentPowerCue, &Transform)>();
    query
        .iter(world)
        .filter(|(cue, _)| cue.agent_id == agent_id)
        .map(|(_, transform)| transform.translation.z)
        .max_by(f32::total_cmp)
}

#[test]
fn power_cues_are_state_gated_shape_distinct_and_noninteractive() {
    let mut normal = render_test_app(power_state("normal"));
    let normal_hits = hit_regions(&mut normal);
    assert!(power_cues(&mut normal).is_empty());

    let mut low = render_test_app(power_state("low_power"));
    let low_cues = power_cues(&mut low);
    assert_eq!(low_cues.len(), 1);
    assert_ne!(low_cues[0].3, Color::srgb_u8(99, 179, 255));

    let mut critical = render_test_app(power_state("critical"));
    assert_eq!(power_cues(&mut critical).len(), 4);
    let mut shutdown = render_test_app(power_state("shutdown"));
    assert_eq!(power_cues(&mut shutdown).len(), 6);
    assert_eq!(hit_regions(&mut shutdown), normal_hits);

    let mut invalid = render_test_app(power_state("depleted"));
    assert!(power_cues(&mut invalid).is_empty());
}

#[test]
fn power_cue_raster_is_visible_deterministic_and_orders_before_priority_cues() {
    let mut low = render_test_app(power_state("low_power"));
    let (_, low_summary) = rasterize_pixel_regression(&mut low);
    assert!(low_summary.agent_power_cue_pixels > 0);

    let mut repeated = render_test_app(power_state("low_power"));
    let (_, repeated_summary) = rasterize_pixel_regression(&mut repeated);
    assert_eq!(
        low_summary.raw_rgba_fnv1a64,
        repeated_summary.raw_rgba_fnv1a64
    );
    println!(
        "agent_power_cue_raster pixels={} hash={}",
        low_summary.agent_power_cue_pixels, low_summary.raw_rgba_fnv1a64
    );

    let mut selected = power_state("critical");
    selected.selection = Some(Selection {
        kind: "agent".to_string(),
        id: "agent-0".to_string(),
    });
    let mut selected_app = render_test_app(selected);
    let power_z = power_cues(&mut selected_app)
        .iter()
        .map(|cue| cue.4)
        .max_by(f32::total_cmp)
        .expect("critical power cue");
    let world = selected_app.world_mut();
    let mut selected_query = world.query::<(&PixelWorldSelectedAgentCue, &Transform)>();
    let selected_z = selected_query
        .iter(world)
        .map(|(_, transform)| transform.translation.z)
        .max_by(f32::total_cmp)
        .expect("selected cue");
    assert!(power_z < selected_z);
}

#[test]
fn power_cue_layer_is_above_core_but_below_recommended_selected_and_receipt_layers() {
    let mut render_state =
        sample_render_state_with_receipt_target(Some("blocked"), Some("agent-0"));
    render_state.agents.push(Agent {
        id: "agent-1".to_string(),
        label: "Recommended powered Agent".to_string(),
        pos: Some(sample_position(1_640_000.0, 1_115_000.0)),
        location_id: Some("loc-0".to_string()),
        resource_summary: "-".to_string(),
        status_badges: vec![],
        position_source: AgentPositionSource::Snapshot,
        size_hint_px: Some(16.0),
        power_state: Some("critical".to_string()),
    });
    render_state.selection = Some(Selection {
        kind: "agent".to_string(),
        id: "agent-0".to_string(),
    });
    render_state.recommended_target = Some(RecommendedTarget {
        agent_id: "agent-1".to_string(),
    });
    let mut app = render_test_app(render_state);
    let power_z = max_z_for_power_agent(&mut app, "agent-1").expect("critical power cue");
    assert!(power_z > max_z_for_agent_core(&mut app, "agent-1").expect("agent core"));
    assert!(
        power_z < max_z_for::<PixelWorldRecommendedTargetCue>(&mut app).expect("recommended cue")
    );
    assert!(power_z < max_z_for::<PixelWorldSelectedAgentCue>(&mut app).expect("selected cue"));
    assert!(power_z < max_z_for::<PixelWorldReceiptTargetCue>(&mut app).expect("receipt cue"));
}

#[test]
fn power_cue_reconcile_removes_stale_entities_for_normal_and_absent_states() {
    let mut app = render_test_app(power_state("shutdown"));
    assert_eq!(power_cues(&mut app).len(), 6);

    {
        let mut runtime = app.world_mut().resource_mut::<BevyRuntimeState>();
        runtime
            .render_state
            .as_mut()
            .expect("shutdown render state")
            .agents[0]
            .power_state = Some("normal".to_string());
        runtime.render_version += 1;
    }
    app.update();
    assert!(
        power_cues(&mut app).is_empty(),
        "normal must remove stale cues"
    );

    {
        let mut runtime = app.world_mut().resource_mut::<BevyRuntimeState>();
        runtime
            .render_state
            .as_mut()
            .expect("normal render state")
            .agents[0]
            .power_state = None;
        runtime.render_version += 1;
    }
    app.update();
    assert!(
        power_cues(&mut app).is_empty(),
        "absent power must remain cue-free"
    );
}
