use super::*;
use crate::render::agent_labels::truncate_agent_label;

#[test]
fn agent_label_truncation_keeps_a_bounded_readable_identity() {
    assert_eq!(
        truncate_agent_label("01234567890123456789012345"),
        "012345678901234567890123…"
    );
}

fn agent_with_label(id: &str, label: &str, pos: Position) -> Agent {
    Agent {
        id: id.to_string(),
        label: label.to_string(),
        pos: Some(pos),
        location_id: None,
        resource_summary: "-".to_string(),
        status_badges: Vec::new(),
        position_source: AgentPositionSource::Snapshot,
        size_hint_px: None,
    }
}

fn rendered_texts(app: &mut App) -> Vec<String> {
    let world = app.world_mut();
    let mut labels = world.query::<(&Text2d, &TextFont)>();
    let mut texts = labels
        .iter(world)
        .filter(|(_, font)| font.font_size == FontSize::Px(10.0))
        .map(|(text, _)| text.0.clone())
        .collect::<Vec<_>>();
    texts.sort();
    texts
}

#[test]
fn agent_labels_are_zoom_gated_stably_suppressed_and_reconciled_without_hit_regions() {
    let anchor = sample_position(1_530_000.0, 1_010_000.0);
    let mut state = sample_render_state(12_000.0);
    state.selection = Some(Selection {
        kind: "agent".to_string(),
        id: "agent-z".to_string(),
    });
    state.agents = vec![
        agent_with_label(
            "agent-a",
            "A very long agent identity that must truncate",
            anchor.clone(),
        ),
        agent_with_label("agent-z", "Selected survey agent", anchor),
    ];
    let mut app = render_test_app(state);
    assert_eq!(
        hit_regions(&mut app).len(),
        3,
        "only the existing location and two Agents own hit-test regions"
    );

    assert_eq!(
        rendered_texts(&mut app),
        vec!["Selected survey agent".to_string()],
        "at high zoom the selected Agent wins a co-anchor collision without status semantics"
    );

    app.world_mut()
        .resource_mut::<BevyRuntimeState>()
        .camera
        .zoom = 1.0;
    app.update();
    assert!(
        rendered_texts(&mut app).is_empty(),
        "overview zoom must keep Agent identity labels hidden"
    );

    {
        let mut runtime = app.world_mut().resource_mut::<BevyRuntimeState>();
        runtime.camera.zoom = 3.0;
        let agents = &mut runtime
            .render_state
            .as_mut()
            .expect("test render state")
            .agents;
        agents.retain(|agent| agent.id == "agent-z");
        agents[0].label = "Renamed survey agent".to_string();
    }
    app.update();
    assert_eq!(
        rendered_texts(&mut app),
        vec!["Renamed survey agent".to_string()],
        "renames update the reused noninteractive label entity"
    );
    let regions = hit_regions(&mut app);
    assert_eq!(
        regions.len(),
        2,
        "removed Agent hit regions shrink normally"
    );
    assert!(
        regions
            .iter()
            .all(|region| region.kind == "location" || region.id == "agent-z"),
        "Agent labels must not introduce their own hit-test region"
    );

    app.world_mut()
        .resource_mut::<BevyRuntimeState>()
        .render_state
        .as_mut()
        .expect("test render state")
        .agents
        .clear();
    app.update();
    assert!(
        rendered_texts(&mut app).is_empty(),
        "deleted Agents leave no stale identity label"
    );
}

#[test]
fn agent_labels_clear_when_a_mounted_scene_loses_its_render_state() {
    let mut state = sample_render_state(12_000.0);
    state.agents = vec![agent_with_label(
        "agent-a",
        "Survey agent",
        sample_position(1_530_000.0, 1_010_000.0),
    )];
    let mut app = render_test_app(state);

    assert_eq!(rendered_texts(&mut app), vec!["Survey agent".to_string()]);

    app.world_mut()
        .resource_mut::<BevyRuntimeState>()
        .render_state = None;
    app.update();

    assert!(
        rendered_texts(&mut app).is_empty(),
        "a mounted scene without a render snapshot must not retain stale Agent labels"
    );
}
