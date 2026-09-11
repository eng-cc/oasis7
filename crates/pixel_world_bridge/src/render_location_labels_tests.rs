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
    let mut location_labels = world.query::<(
        &Text2d,
        &TextFont,
        &crate::render::location_labels::PixelWorldLocationLabel,
    )>();
    let mut texts = location_labels
        .iter(world)
        .filter(|(_, font, _)| font.font_size == FontSize::Px(10.0))
        .map(|(text, _, _)| text.0.clone())
        .collect::<Vec<_>>();
    let mut map_labels = world.query::<(&Text2d, &crate::render::map_labels::PixelWorldMapLabel)>();
    texts.extend(map_labels.iter(world).map(|(text, _)| text.0.clone()));
    texts
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

#[test]
fn objective_and_blocker_hotspots_keep_their_labels_when_ambient_locations_are_dense() {
    let anchor = sample_position(1_500_000.0, 1_000_000.0);
    let mut state = sample_render_state(12_000.0);
    state.agents.clear();
    state.fragment_terrain.clear();
    state.selection = None;
    state.locations = vec![
        location_with("loc-a", "Ambient Alpha", anchor.clone()),
        location_with("loc-b", "Ambient Beta", anchor.clone()),
    ];
    state.visual_hotspots = vec![
        VisualHotspot {
            id: "goal-highlight".to_string(),
            label: "Current objective".to_string(),
            kind: "goal".to_string(),
            pos: anchor.clone(),
            emphasis: Some(1.0),
            size_hint_px: Some(14.0),
        },
        VisualHotspot {
            id: "blocker-highlight".to_string(),
            label: "Blocked route".to_string(),
            kind: "blocker".to_string(),
            pos: anchor,
            emphasis: Some(1.0),
            size_hint_px: Some(16.0),
        },
    ];
    state.links = vec![Link {
        id: "route:ore-line".to_string(),
        kind: "route".to_string(),
        label: None,
        from: sample_position(100_000.0, 100_000.0),
        to: sample_position(600_000.0, 100_000.0),
        emphasis: Some(0.72),
        status: Some("active".to_string()),
        source_class: Some("runtime_projection".to_string()),
        freshness: Some("current".to_string()),
    }];

    let mut app = render_test_app(state);

    let labels = rendered_location_texts(&mut app);
    assert!(
        labels.contains(&"Current objective".to_string()),
        "the current objective must remain a visible map label"
    );
    assert!(
        labels.contains(&"Blocked route".to_string()),
        "the current blocker must remain a visible map label"
    );
    assert!(
        labels.contains(&"Route".to_string()),
        "an authoritative route must remain a visible map label"
    );
    assert!(
        !labels.contains(&"Ambient Alpha".to_string())
            && !labels.contains(&"Ambient Beta".to_string()),
        "dense ambient identity labels must yield to objective and blocker labels"
    );
}

fn route_label_state(locale: &str, kind: &str, label: Option<&str>) -> RenderState {
    let mut state = sample_render_state(12_000.0);
    state.locale = locale.to_string();
    state.locations.clear();
    state.agents.clear();
    state.fragment_terrain.clear();
    state.visual_hotspots.clear();
    state.selection = None;
    state.links = vec![Link {
        id: "route:locale-line".to_string(),
        kind: kind.to_string(),
        label: label.map(ToString::to_string),
        from: sample_position(100_000.0, 100_000.0),
        to: sample_position(600_000.0, 100_000.0),
        emphasis: Some(0.72),
        status: Some("active".to_string()),
        source_class: Some("runtime_projection".to_string()),
        freshness: Some("current".to_string()),
    }];
    state
}

#[test]
fn route_labels_use_locale_fallback_and_preserve_published_names() {
    let mut zh = render_test_app(route_label_state("zh-CN", "logistics_route", None));
    assert!(
        rendered_location_texts(&mut zh).contains(&"物流路线".to_string()),
        "Chinese locale must use the localized logistics route type fallback"
    );

    let mut en = render_test_app(route_label_state("en-US", "logistics_route", None));
    assert!(
        rendered_location_texts(&mut en).contains(&"Logistics route".to_string()),
        "English locale must use the English logistics route type fallback"
    );

    let mut published = render_test_app(route_label_state(
        "zh-CN",
        "logistics_route",
        Some("北线物资"),
    ));
    assert!(
        rendered_location_texts(&mut published).contains(&"北线物资".to_string()),
        "a published route name must take precedence over the localized type fallback"
    );
}
