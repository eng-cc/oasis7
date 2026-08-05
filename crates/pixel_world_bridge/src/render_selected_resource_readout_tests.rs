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
