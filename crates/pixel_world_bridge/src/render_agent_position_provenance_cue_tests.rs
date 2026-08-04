use super::fixtures::sample_render_state_with_receipt_target;
use super::*;

const DERIVED_POSITION_CUE_COLOR: Color = Color::srgba_u8(125, 153, 178, 150);

fn derived_position_cue_entities(
    app: &mut App,
) -> std::collections::BTreeMap<(String, DerivedPositionCueSegment), Entity> {
    let world = app.world_mut();
    let mut cues = world.query::<(Entity, &PixelWorldDerivedPositionCue)>();
    cues.iter(world)
        .map(|(entity, cue)| ((cue.agent_id.clone(), cue.segment), entity))
        .collect()
}

fn normalized_hit_regions(app: &mut App) -> Vec<(String, String, i64, i64, i64, i64)> {
    let mut regions = hit_regions(app)
        .into_iter()
        .map(|region| {
            (
                region.kind.to_string(),
                region.id,
                (region.left * 1_000.0).round() as i64,
                (region.top * 1_000.0).round() as i64,
                (region.right * 1_000.0).round() as i64,
                (region.bottom * 1_000.0).round() as i64,
            )
        })
        .collect::<Vec<_>>();
    regions.sort();
    regions
}

#[test]
fn derived_position_cue_is_provenance_gated_noninteractive_and_reused() {
    let mut state = sample_render_state_with_beacon_candidates("agent", "agent-0");
    state.agents[0].position_source = AgentPositionSource::LocationDerived;
    state.agents[1].position_source = AgentPositionSource::Snapshot;
    let mut missing = state.agents[1].clone();
    missing.id = "agent-missing".to_string();
    missing.position_source = AgentPositionSource::Missing;
    state.agents.push(missing);

    let mut app = render_test_app(state);
    let baseline_regions = normalized_hit_regions(&mut app);
    let initial = derived_position_cue_entities(&mut app);
    assert_eq!(
        initial
            .keys()
            .map(|(id, _)| id.as_str())
            .collect::<Vec<_>>(),
        ["agent-0", "agent-0", "agent-0", "agent-0"],
        "only location-derived Agent positions receive the four short locator marks"
    );
    assert_eq!(
        normalized_hit_regions(&mut app),
        baseline_regions,
        "provenance cues must not add or move hit regions"
    );

    let world = app.world_mut();
    let mut agents = world.query::<(&PixelWorldAgentVisual, &Transform)>();
    let agent_z = agents
        .iter(world)
        .find(|(agent, _)| agent.id == "agent-0")
        .map(|(_, transform)| transform.translation.z)
        .expect("derived-position Agent body");
    let mut sprites = world.query::<(&PixelWorldDerivedPositionCue, &Sprite, &Transform)>();
    for (_, sprite, transform) in sprites.iter(world) {
        assert_eq!(sprite.color, DERIVED_POSITION_CUE_COLOR);
        assert!(
            transform.translation.z < agent_z,
            "the derived-position cue must stay below the Agent body and higher-priority cues"
        );
    }

    app.update();
    assert_eq!(
        derived_position_cue_entities(&mut app),
        initial,
        "an unchanged reconcile must reuse every derived-position cue entity"
    );
}

#[test]
fn derived_position_cue_cleans_up_when_provenance_changes_or_state_disappears() {
    let mut state = sample_render_state(12_000.0);
    state.agents[0].position_source = AgentPositionSource::LocationDerived;
    let mut app = render_test_app(state);
    assert_eq!(derived_position_cue_entities(&mut app).len(), 4);

    {
        let mut runtime = app.world_mut().resource_mut::<BevyRuntimeState>();
        runtime.render_state.as_mut().expect("render state").agents[0].position_source =
            AgentPositionSource::Snapshot;
        runtime.render_version += 1;
    }
    app.update();
    assert!(
        derived_position_cue_entities(&mut app).is_empty(),
        "snapshot positions must remove stale derived-position cues"
    );

    {
        let mut runtime = app.world_mut().resource_mut::<BevyRuntimeState>();
        runtime.render_state = None;
        runtime.render_version += 1;
    }
    app.update();
    assert!(
        derived_position_cue_entities(&mut app).is_empty(),
        "removing render state must leave no provenance cue entities"
    );
}

#[test]
fn derived_position_ticks_differ_from_selected_corners_and_stay_below_narrow_receipts() {
    let mut state = sample_render_state_with_receipt_target(Some("blocked"), Some("agent-0"));
    state.selection = Some(Selection {
        kind: "agent".to_string(),
        id: "agent-0".to_string(),
    });
    state.agents[0].position_source = AgentPositionSource::LocationDerived;
    let mut app = render_test_app(state);
    {
        let world = app.world_mut();
        let mut windows = world.query_filtered::<&mut Window, With<PrimaryWindow>>();
        windows
            .single_mut(world)
            .expect("primary window")
            .resolution
            .set(318.0, 179.0);
    }
    app.update();
    let world = app.world_mut();
    let mut agents = world.query::<(&PixelWorldAgentVisual, &Transform)>();
    let agent = agents
        .iter(world)
        .find(|(agent, _)| agent.id == "agent-0")
        .map(|(_, transform)| transform.translation)
        .expect("selected Agent");
    let mut derived = world.query::<(&PixelWorldDerivedPositionCue, &Transform)>();
    let derived = derived
        .iter(world)
        .map(|(_, transform)| transform.translation)
        .collect::<Vec<_>>();
    assert_eq!(
        derived.len(),
        4,
        "narrow selected receipt fixture keeps four midpoint ticks"
    );
    assert!(
        derived
            .iter()
            .all(|tick| (tick.x - agent.x).abs() < f32::EPSILON
                || (tick.y - agent.y).abs() < f32::EPSILON),
        "midpoint ticks must not reuse corner geometry"
    );
    let mut selected = world.query::<(&PixelWorldSelectedAgentCue, &Transform)>();
    assert!(
        selected
            .iter(world)
            .all(
                |(_, corner)| (corner.translation.x - agent.x).abs() > f32::EPSILON
                    && (corner.translation.y - agent.y).abs() > f32::EPSILON
            ),
        "selected cue retains corner-only geometry"
    );
    let mut receipts = world.query::<(&PixelWorldReceiptTargetCue, &Transform)>();
    let receipt_z = receipts
        .iter(world)
        .map(|(_, transform)| transform.translation.z)
        .reduce(f32::min)
        .expect("blocked receipt cue");
    assert!(
        derived.iter().all(|tick| tick.z < receipt_z),
        "derived locator remains below the blocked receipt cue"
    );
}
