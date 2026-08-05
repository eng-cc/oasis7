use super::*;

#[derive(Debug, PartialEq, Eq)]
struct ResourceReadoutProbe {
    target_kind: String,
    target_id: String,
    display: String,
}

fn selected_resource_readouts(app: &mut App) -> Vec<ResourceReadoutProbe> {
    let world = app.world_mut();
    let mut readouts = world.query::<&PixelWorldSelectedResourceReadout>();
    let mut probes = readouts
        .iter(world)
        .map(|readout| ResourceReadoutProbe {
            target_kind: readout.target_kind.clone(),
            target_id: readout.target_id.clone(),
            display: readout.display.clone(),
        })
        .collect::<Vec<_>>();
    probes.sort_by(|left, right| {
        left.target_kind
            .cmp(&right.target_kind)
            .then(left.target_id.cmp(&right.target_id))
    });
    probes
}

fn selected_resource_readout_y(app: &mut App) -> f32 {
    let world = app.world_mut();
    let mut readouts = world.query::<(&PixelWorldSelectedResourceReadout, &Transform)>();
    readouts
        .iter(world)
        .next()
        .expect("selected resource readout")
        .1
        .translation
        .y
}

fn render_resource_readout_test_app_at_viewport(
    render_state: RenderState,
    width: f32,
    height: f32,
) -> App {
    let mut app = render_test_app(render_state);
    let world = app.world_mut();
    let mut windows = world.query_filtered::<&mut Window, With<PrimaryWindow>>();
    windows
        .single_mut(world)
        .expect("resource readout test primary window")
        .resolution
        .set(width, height);
    app.update();
    app
}

#[test]
fn selected_resource_readout_tracks_agent_location_empty_state_and_deselection() {
    let mut selected_agent = sample_render_state(12_000.0);
    selected_agent.agents[0].resource_summary = "water=12, ore=5".to_string();
    selected_agent.agents[0].status_badges = vec!["position=location_derived".to_string()];
    let mut app = render_test_app(selected_agent);

    assert_eq!(
        selected_resource_readouts(&mut app),
        vec![ResourceReadoutProbe {
            target_kind: "agent".to_string(),
            target_id: "agent-0".to_string(),
            display: "water=12, ore=5".to_string(),
        }],
        "the selected Agent must expose its existing resource summary through one dedicated readout, not status badges"
    );

    {
        let mut runtime = app.world_mut().resource_mut::<BevyRuntimeState>();
        let state = runtime.render_state.as_mut().expect("render state");
        state.selection = Some(Selection {
            kind: "location".to_string(),
            id: "loc-0".to_string(),
        });
        state.locations[0].resource_summary = "-".to_string();
        runtime.render_version += 1;
    }
    app.update();

    let location_readout = selected_resource_readouts(&mut app);
    assert_eq!(
        location_readout.len(),
        1,
        "selection switches must replace, not duplicate, resource readouts"
    );
    assert_eq!(location_readout[0].target_kind, "location");
    assert_eq!(location_readout[0].target_id, "loc-0");
    assert_ne!(
        location_readout[0].display, "-",
        "empty resources need a player-friendly state"
    );
    assert!(
        !location_readout[0].display.trim().is_empty(),
        "the friendly empty-resource state must be visible rather than blank"
    );
    assert!(
        !location_readout[0].display.contains("position=")
            && !location_readout[0].display.contains("location_derived"),
        "internal status_badges tokens must never leak into the resource readout"
    );

    {
        let mut runtime = app.world_mut().resource_mut::<BevyRuntimeState>();
        runtime
            .render_state
            .as_mut()
            .expect("render state")
            .selection = None;
        runtime.render_version += 1;
    }
    app.update();
    assert!(
        selected_resource_readouts(&mut app).is_empty(),
        "deselection must remove the selected resource readout"
    );
}

#[test]
fn selected_resource_readout_yields_a_narrow_lane_to_blocker_and_goal_cues() {
    let ordinary_state = sample_render_state(12_000.0);
    let mut with_priority_cue = ordinary_state.clone();
    with_priority_cue.visual_hotspots.push(VisualHotspot {
        id: "blocker-highlight".to_string(),
        label: "Missing Material".to_string(),
        kind: "blocker".to_string(),
        pos: sample_position(1_450_000.0, 950_000.0),
        emphasis: Some(1.0),
        size_hint_px: Some(16.0),
    });
    with_priority_cue.visual_hotspots.push(VisualHotspot {
        id: "goal-highlight".to_string(),
        label: "Recover sustainable capability".to_string(),
        kind: "goal".to_string(),
        pos: sample_position(1_550_000.0, 1_050_000.0),
        emphasis: Some(1.0),
        size_hint_px: Some(14.0),
    });

    let mut ordinary = render_resource_readout_test_app_at_viewport(ordinary_state, 390.0, 844.0);
    let mut priority =
        render_resource_readout_test_app_at_viewport(with_priority_cue, 390.0, 844.0);

    assert!(
        selected_resource_readout_y(&mut priority)
            > selected_resource_readout_y(&mut ordinary) + 27.0,
        "a 390px blocker or goal cue must reserve a separate upward text lane"
    );
    let ordinary_agent_regions = hit_regions(&mut ordinary)
        .into_iter()
        .filter(|region| region.kind == "agent")
        .collect::<Vec<_>>();
    let priority_agent_regions = hit_regions(&mut priority)
        .into_iter()
        .filter(|region| region.kind == "agent")
        .collect::<Vec<_>>();
    assert_eq!(priority_agent_regions, ordinary_agent_regions);
}
