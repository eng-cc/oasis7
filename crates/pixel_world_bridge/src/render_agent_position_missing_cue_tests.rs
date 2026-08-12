use super::fixtures::sample_render_state_with_receipt_target;
use super::*;
use crate::render::agent_position_missing_cue::MissingPositionCueSegment;

const MISSING_POSITION_CUE_COLOR: Color = Color::srgba_u8(100, 116, 139, 180);

fn missing_position_cue_entities(
    app: &mut App,
) -> std::collections::BTreeMap<(String, MissingPositionCueSegment), Entity> {
    let world = app.world_mut();
    let mut cues = world.query::<(Entity, &PixelWorldMissingPositionCue)>();
    cues.iter(world)
        .map(|(entity, cue)| ((cue.agent_id.clone(), cue.segment), entity))
        .collect()
}

fn normalized_agent_hit_regions(app: &mut App) -> Vec<(String, i64, i64, i64, i64)> {
    let mut regions = hit_regions(app)
        .into_iter()
        .filter(|region| region.kind == "agent")
        .map(|region| {
            (
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

fn missing_agent_state() -> RenderState {
    let mut state = sample_render_state_with_receipt_target(Some("blocked"), Some("agent-0"));
    state.selection = None;
    state.agents[0].position_source = AgentPositionSource::Missing;
    state.agents[0].pos = None;
    state
}

#[test]
fn missing_position_cue_is_four_hollow_corners_below_body_and_reused() {
    let mut app = render_test_app(missing_agent_state());
    let baseline_hits = normalized_agent_hit_regions(&mut app);
    let initial = missing_position_cue_entities(&mut app);
    assert_eq!(initial.len(), 8, "four Missing brackets have two arms each");
    assert_eq!(
        initial.keys().filter(|(id, _)| id == "agent-0").count(),
        8,
        "only the Missing agent receives uncertainty corners"
    );

    let world = app.world_mut();
    let mut agents = world.query::<(&PixelWorldAgentVisual, &Transform)>();
    let body_z = agents
        .iter(world)
        .find(|(agent, _)| agent.id == "agent-0")
        .map(|(_, transform)| transform.translation.z)
        .expect("missing agent body");
    let mut sprites = world.query::<(&PixelWorldMissingPositionCue, &Sprite, &Transform)>();
    let mut horizontal = 0;
    let mut vertical = 0;
    for (_, sprite, transform) in sprites.iter(world) {
        assert_eq!(sprite.color, MISSING_POSITION_CUE_COLOR);
        assert!(transform.translation.z < body_z);
        let size = sprite.custom_size.expect("missing cue sprite size");
        if size.x > size.y {
            horizontal += 1;
            assert_eq!(size, Vec2::new(3.0, 1.0));
        } else {
            vertical += 1;
            assert_eq!(size, Vec2::new(1.0, 3.0));
        }
    }
    assert_eq!((horizontal, vertical), (4, 4));
    assert_eq!(normalized_agent_hit_regions(&mut app), baseline_hits);

    app.update();
    assert_eq!(
        missing_position_cue_entities(&mut app),
        initial,
        "unchanged reconcile reuses every Missing cue entity"
    );
}

#[test]
fn missing_cue_is_provenance_gated_and_cleans_up_on_transition_or_disappearance() {
    let mut app = render_test_app(missing_agent_state());
    assert_eq!(missing_position_cue_entities(&mut app).len(), 8);

    {
        let mut runtime = app.world_mut().resource_mut::<BevyRuntimeState>();
        runtime.render_state.as_mut().expect("render state").agents[0].position_source =
            AgentPositionSource::Snapshot;
        runtime.render_version += 1;
    }
    app.update();
    assert!(missing_position_cue_entities(&mut app).is_empty());

    {
        let mut runtime = app.world_mut().resource_mut::<BevyRuntimeState>();
        runtime.render_state = None;
        runtime.render_version += 1;
    }
    app.update();
    assert!(missing_position_cue_entities(&mut app).is_empty());

    let mut mounted_false = render_test_app(missing_agent_state());
    assert_eq!(missing_position_cue_entities(&mut mounted_false).len(), 8);
    mounted_false
        .world_mut()
        .resource_mut::<BevyRuntimeState>()
        .mounted = false;
    mounted_false.update();
    assert!(missing_position_cue_entities(&mut mounted_false).is_empty());

    let mut absent_state = render_test_app(missing_agent_state());
    assert_eq!(missing_position_cue_entities(&mut absent_state).len(), 8);
    absent_state
        .world_mut()
        .resource_mut::<BevyRuntimeState>()
        .render_state = None;
    absent_state.update();
    assert!(missing_position_cue_entities(&mut absent_state).is_empty());
}

#[test]
fn missing_cue_geometry_and_color_are_distinct_from_derived_ticks() {
    let mut missing = render_test_app(missing_agent_state());
    let missing_body = {
        let world = missing.world_mut();
        let mut agents = world.query::<(&PixelWorldAgentVisual, &Transform)>();
        agents
            .iter(world)
            .find(|(agent, _)| agent.id == "agent-0")
            .map(|(_, transform)| transform.translation)
            .expect("missing agent body")
    };
    let missing_segments = {
        let world = missing.world_mut();
        let mut cues = world.query::<(&PixelWorldMissingPositionCue, &Sprite, &Transform)>();
        cues.iter(world)
            .map(|(_, sprite, transform)| {
                (
                    sprite.custom_size.expect("missing cue size"),
                    transform.translation - missing_body,
                    sprite.color,
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(missing_segments.len(), 8);
    assert!(missing_segments.iter().all(|(size, offset, color)| {
        color == &MISSING_POSITION_CUE_COLOR
            && ((size.x == 3.0 && size.y == 1.0) || (size.x == 1.0 && size.y == 3.0))
            && offset.x.abs() > 0.0
            && offset.y.abs() > 0.0
    }));
    assert_eq!(
        missing_segments
            .iter()
            .filter(|(size, _, _)| size.x > size.y)
            .count(),
        4,
        "Missing cue has one horizontal arm at each corner"
    );
    assert_eq!(
        missing_segments
            .iter()
            .filter(|(size, _, _)| size.y > size.x)
            .count(),
        4,
        "Missing cue has one vertical arm at each corner"
    );

    let mut derived_state = missing_agent_state();
    derived_state.agents[0].position_source = AgentPositionSource::LocationDerived;
    derived_state.agents[0].pos = Some(sample_position(1_500_000.0, 1_000_000.0));
    let mut derived = render_test_app(derived_state);
    let derived_body = {
        let world = derived.world_mut();
        let mut agents = world.query::<(&PixelWorldAgentVisual, &Transform)>();
        agents
            .iter(world)
            .find(|(agent, _)| agent.id == "agent-0")
            .map(|(_, transform)| transform.translation)
            .expect("derived agent body")
    };
    let derived_segments = {
        let world = derived.world_mut();
        let mut cues = world.query::<(&PixelWorldDerivedPositionCue, &Sprite, &Transform)>();
        cues.iter(world)
            .map(|(_, sprite, transform)| {
                (
                    sprite.custom_size.expect("derived cue size"),
                    transform.translation - derived_body,
                    sprite.color,
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(derived_segments.len(), 4);
    assert!(derived_segments.iter().all(|(size, offset, color)| {
        color == &Color::srgba_u8(125, 153, 178, 150)
            && ((size.x == 4.0 && size.y == 1.0) || (size.x == 1.0 && size.y == 4.0))
            && (offset.x.abs() == 0.0 || offset.y.abs() == 0.0)
    }));
    assert!(
        missing_segments
            .iter()
            .all(|(size, _, _)| !derived_segments.iter().any(|(other, _, _)| size == other)),
        "Missing 3px arms must stay distinct from Derived 4px midpoint ticks"
    );
}

#[test]
fn missing_cue_raster_readback_is_visible_and_below_agent_body() {
    let mut state = missing_agent_state();
    state.agents[0].pos = Some(sample_position(1_500_000.0, 1_000_000.0));
    let mut app = render_test_app(state);
    let (_, summary) = rasterize_pixel_regression(&mut app);
    assert!(
        summary.missing_position_cue_pixels > 0,
        "Missing cue must contribute stable visible raster pixels"
    );
    assert!(
        summary.missing_position_cue_pixels < summary.agent_pixels,
        "Missing cue remains a low-density decoration below the Agent body"
    );
}
