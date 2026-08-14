use super::*;

fn location_with(id: &str, label: &str, pos: Position) -> Location {
    Location {
        id: id.to_string(),
        label: label.to_string(),
        pos,
        radius_cm: 30_000.0,
        resource_summary: "-".to_string(),
        size_hint_px: Some(10.0),
        marker_role: Some("logic_anchor".to_string()),
        marker_alpha: Some(0.32),
    }
}

fn rendered_location_texts(app: &mut App) -> Vec<String> {
    let world = app.world_mut();
    let mut labels = world.query::<(&Text2d, &TextFont)>();
    labels
        .iter(world)
        .filter(|(_, font)| font.font_size == FontSize::Px(10.0))
        .map(|(text, _)| text.0.clone())
        .collect()
}

#[test]
fn location_labels_are_selected_first_and_suppress_collisions_deterministically() {
    let anchor = sample_position(1_500_000.0, 1_000_000.0);
    let mut state = sample_render_state(12_000.0);
    state.agents.clear();
    state.fragment_terrain.clear();
    state.selection = Some(Selection {
        kind: "location".to_string(),
        id: "loc-z".to_string(),
    });
    state.locations = vec![
        location_with(
            "loc-b",
            "Beta location",
            sample_position(2_300_000.0, 1_500_000.0),
        ),
        location_with("loc-a", "Alpha location", anchor.clone()),
        location_with("loc-z", "Selected location", anchor),
    ];

    let mut app = render_test_app(state);

    assert_eq!(
        rendered_location_texts(&mut app),
        vec!["Selected location", "Beta location"],
        "selected location must win a collision before stable id ordering"
    );
    assert_eq!(
        hit_regions(&mut app).len(),
        3,
        "location labels remain display-only and add no hit regions"
    );

    let mut unselected = sample_render_state(12_000.0);
    unselected.agents.clear();
    unselected.fragment_terrain.clear();
    unselected.selection = None;
    unselected.locations = vec![
        location_with(
            "loc-b",
            "Beta location",
            sample_position(1_500_000.0, 1_000_000.0),
        ),
        location_with(
            "loc-a",
            "Alpha location",
            sample_position(1_500_000.0, 1_000_000.0),
        ),
    ];
    {
        let mut runtime = app.world_mut().resource_mut::<BevyRuntimeState>();
        runtime.render_state = Some(unselected);
        runtime.render_version += 1;
        runtime.hit_regions_dirty = true;
    }
    app.update();

    assert_eq!(
        rendered_location_texts(&mut app),
        vec!["Alpha location"],
        "without selection, the lexicographically smallest id wins a collision"
    );
    assert_eq!(hit_regions(&mut app).len(), 2);
}

#[test]
fn location_labels_fallback_and_truncate_identity_at_high_zoom() {
    let mut state = sample_render_state(12_000.0);
    state.agents.clear();
    state.fragment_terrain.clear();
    state.selection = None;
    state.locations = vec![
        location_with(
            "loc-long",
            "01234567890123456789012345",
            sample_position(1_500_000.0, 1_000_000.0),
        ),
        location_with(
            "location-fallback-0123456789",
            "   ",
            sample_position(2_300_000.0, 1_500_000.0),
        ),
    ];

    let mut app = render_test_app(state);

    assert_eq!(
        rendered_location_texts(&mut app),
        vec!["012345678901234567890123…", "location-fallback-012345…"],
        "labels use the trimmed display label and id fallback"
    );
}

#[test]
fn location_labels_gate_on_zoom_and_clean_up_stale_entities() {
    let mut state = sample_render_state(12_000.0);
    state.agents.clear();
    state.fragment_terrain.clear();
    state.locations[0].label = "Survey Anchor".to_string();
    let mut app = render_test_app(state);

    assert_eq!(rendered_location_texts(&mut app), vec!["Survey Anchor"]);
    assert_eq!(hit_regions(&mut app).len(), 1);

    app.world_mut()
        .resource_mut::<BevyRuntimeState>()
        .camera
        .zoom = 1.0;
    app.update();
    assert!(rendered_location_texts(&mut app).is_empty());
    assert_eq!(hit_regions(&mut app).len(), 1);

    {
        let mut runtime = app.world_mut().resource_mut::<BevyRuntimeState>();
        runtime.camera.zoom = 3.0;
        let location = runtime
            .render_state
            .as_mut()
            .expect("test render state")
            .locations
            .first_mut()
            .expect("sample location");
        location.label = "Renamed Anchor".to_string();
        runtime.render_version += 1;
    }
    app.update();
    assert_eq!(rendered_location_texts(&mut app), vec!["Renamed Anchor"]);

    app.world_mut()
        .resource_mut::<BevyRuntimeState>()
        .render_state
        .as_mut()
        .expect("test render state")
        .locations
        .clear();
    app.world_mut()
        .resource_mut::<BevyRuntimeState>()
        .hit_regions_dirty = true;
    app.update();
    assert!(rendered_location_texts(&mut app).is_empty());
    assert_eq!(hit_regions(&mut app).len(), 0);

    app.world_mut()
        .resource_mut::<BevyRuntimeState>()
        .render_state = None;
    app.update();
    assert!(rendered_location_texts(&mut app).is_empty());
}
