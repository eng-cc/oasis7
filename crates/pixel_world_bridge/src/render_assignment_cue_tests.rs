use super::*;

fn assignment_cue_entities(
    app: &mut App,
) -> std::collections::BTreeMap<(String, AssignmentCuePart), (Entity, Transform)> {
    let world = app.world_mut();
    let mut cues = world.query::<(Entity, &PixelWorldAssignmentCueVisual, &Transform)>();
    cues.iter(world)
        .map(|(entity, cue, transform)| {
            ((cue.link_id.clone(), cue.part), (entity, transform.clone()))
        })
        .collect()
}

fn assignment_link_state() -> RenderState {
    let mut state = sample_render_state(12_000.0);
    state.selection = None;
    state.links = vec![
        Link {
            id: "assignment:agent-0:loc-0".to_string(),
            kind: "agent_assignment".to_string(),
            from: sample_position(1_250_000.0, 750_000.0),
            to: sample_position(1_750_000.0, 1_250_000.0),
            emphasis: Some(0.72),
        },
        Link {
            id: "unknown:agent-0".to_string(),
            kind: "resource_transfer".to_string(),
            from: sample_position(1_250_000.0, 1_250_000.0),
            to: sample_position(1_750_000.0, 750_000.0),
            emphasis: Some(0.72),
        },
    ];
    state
}

fn canvas_point_for_link_endpoint(app: &mut App, link: &Link, to: bool) -> Vec2 {
    let runtime = app.world_mut().resource::<BevyRuntimeState>();
    let bounds = runtime
        .render_state
        .as_ref()
        .and_then(|state| state.world_bounds.as_ref())
        .expect("assignment fixture bounds");
    let point = if to { &link.to } else { &link.from };
    let (x, y) = to_canvas_point(
        point,
        bounds,
        VIEWPORT_WIDTH as f64,
        VIEWPORT_HEIGHT as f64,
        &runtime.camera,
    )
    .expect("assignment fixture endpoint");
    Vec2::new(
        x as f32 - (VIEWPORT_WIDTH as f32 / 2.0),
        (VIEWPORT_HEIGHT as f32 / 2.0) - y as f32,
    )
}

#[test]
fn agent_assignment_chevrons_point_to_current_anchor_without_hit_regions_or_unknown_link_cues() {
    let state = assignment_link_state();
    let assignment = state.links[0].clone();
    let mut app = render_test_app(state);
    let baseline_regions = hit_regions(&mut app);
    let cues = assignment_cue_entities(&mut app);

    assert_eq!(
        cues.len(),
        2,
        "only agent assignments receive the two-stroke chevron"
    );
    assert!(cues.keys().all(|(id, _)| id == &assignment.id));
    assert_eq!(
        hit_regions(&mut app),
        baseline_regions,
        "display-only assignment cues must not add hit regions"
    );

    let from = canvas_point_for_link_endpoint(&mut app, &assignment, false);
    let to = canvas_point_for_link_endpoint(&mut app, &assignment, true);
    for (_, transform) in cues.values() {
        assert!(
            transform.translation.truncate().distance(to)
                < transform.translation.truncate().distance(from),
            "each chevron stroke must stay nearer the current assignment anchor than the Agent origin"
        );
    }

    app.update();
    assert_eq!(
        assignment_cue_entities(&mut app).keys().collect::<Vec<_>>(),
        cues.keys().collect::<Vec<_>>(),
        "unchanged reconciliation must retain the assignment cue identity"
    );
}

#[test]
fn assignment_chevrons_reverse_and_clean_up_for_short_or_non_assignment_links() {
    let mut app = render_test_app(assignment_link_state());
    let initial = assignment_cue_entities(&mut app);
    let initial_entities = initial
        .iter()
        .map(|(key, (entity, _))| (key.clone(), *entity))
        .collect::<std::collections::BTreeMap<_, _>>();

    let reversed = {
        let runtime = app.world_mut().resource::<BevyRuntimeState>();
        let mut state = runtime.render_state.clone().expect("render state");
        let link = &mut state.links[0];
        std::mem::swap(&mut link.from, &mut link.to);
        state
    };
    {
        let mut runtime = app.world_mut().resource_mut::<BevyRuntimeState>();
        runtime.render_state = Some(reversed.clone());
        runtime.render_version += 1;
    }
    app.update();
    let reversed_cues = assignment_cue_entities(&mut app);
    assert_eq!(
        reversed_cues
            .iter()
            .map(|(key, (entity, _))| (key.clone(), *entity))
            .collect::<std::collections::BTreeMap<_, _>>(),
        initial_entities,
        "reversing a stable assignment link must reuse its chevron entities"
    );
    let reversed_to = canvas_point_for_link_endpoint(&mut app, &reversed.links[0], true);
    let reversed_from = canvas_point_for_link_endpoint(&mut app, &reversed.links[0], false);
    assert!(reversed_cues.values().all(|(_, transform)| {
        transform.translation.truncate().distance(reversed_to)
            < transform.translation.truncate().distance(reversed_from)
    }));

    {
        let mut runtime = app.world_mut().resource_mut::<BevyRuntimeState>();
        let state = runtime.render_state.as_mut().expect("render state");
        state.links[0].to = Position {
            x_cm: 1_750_100.0,
            y_cm: 1_250_100.0,
            z_cm: 0.0,
        };
        runtime.render_version += 1;
    }
    app.update();
    assert!(
        assignment_cue_entities(&mut app).is_empty(),
        "short links must suppress the cue"
    );

    {
        let mut runtime = app.world_mut().resource_mut::<BevyRuntimeState>();
        let state = runtime.render_state.as_mut().expect("render state");
        state.links[0].kind = "unknown_link".to_string();
        state.links[0].to = sample_position(1_750_000.0, 1_250_000.0);
        runtime.render_version += 1;
    }
    app.update();
    assert!(
        assignment_cue_entities(&mut app).is_empty(),
        "unknown link kinds remain bare baseline lines"
    );

    {
        let mut runtime = app.world_mut().resource_mut::<BevyRuntimeState>();
        runtime.render_state = None;
        runtime.render_version += 1;
    }
    app.update();
    assert!(
        assignment_cue_entities(&mut app).is_empty(),
        "missing bounds/state leaves no stale assignment cue"
    );
}

#[test]
fn assignment_chevron_has_deterministic_raster_pixels() {
    let mut visible = render_test_app(assignment_link_state());
    let (_, visible_summary) = rasterize_pixel_regression(&mut visible);
    let mut bare_state = assignment_link_state();
    bare_state.links[0].kind = "unknown_link".to_string();
    let mut bare = render_test_app(bare_state);
    let (_, bare_summary) = rasterize_pixel_regression(&mut bare);

    assert!(
        visible_summary.non_background_pixels > bare_summary.non_background_pixels,
        "the assignment cue must contribute deterministic raster pixels"
    );
    assert_ne!(
        visible_summary.raw_rgba_fnv1a64,
        bare_summary.raw_rgba_fnv1a64
    );
}
