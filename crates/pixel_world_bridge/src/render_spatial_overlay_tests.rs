use super::*;
use crate::render::spatial_overlays::SpatialOverlayVisual;

#[test]
fn aggregate_and_historical_event_have_independent_glyphs_counts_and_hit_identities() {
    let mut state = sample_render_state(12000.0);
    state.industry_aggregates = vec![crate::spatial_display::SpatialVisual {
        id: "agg".into(),
        label: "Industry".into(),
        pos: sample_position(1400000.0, 900000.0),
        historical: false,
        member_count: Some("42".into()),
    }];
    state.world_events = vec![crate::spatial_display::SpatialVisual {
        id: "event".into(),
        label: "Published".into(),
        pos: sample_position(1600000.0, 1100000.0),
        historical: true,
        member_count: None,
    }];
    let mut app = render_test_app(state);
    let markers = app
        .world_mut()
        .query::<&SpatialOverlayVisual>()
        .iter(app.world())
        .count();
    assert_eq!(markers, 6);
    assert!(
        hit_regions(&mut app)
            .iter()
            .any(|h| h.kind == "industry_aggregate" && h.id == "agg")
    );
    assert!(
        hit_regions(&mut app)
            .iter()
            .any(|h| h.kind == "world_event" && h.id == "event")
    );
    let shapes = app
        .world_mut()
        .query::<(&SpatialOverlayVisual, &Sprite, &Transform)>()
        .iter(app.world())
        .map(|(v, s, t)| (v.kind, v.part, s.custom_size, t.rotation))
        .collect::<Vec<_>>();
    assert!(
        shapes
            .iter()
            .any(|(kind, part, size, rotation)| *kind == "industry_aggregate"
                && *part == 0
                && *size == Some(Vec2::new(20.0, 14.0))
                && *rotation == Quat::IDENTITY)
    );
    assert!(
        shapes
            .iter()
            .any(|(kind, part, size, _)| *kind == "world_event"
                && *part == 1
                && *size == Some(Vec2::new(7.0, 2.0)))
    );
    let texts = app
        .world_mut()
        .query::<(&SpatialOverlayVisual, &Text2d)>()
        .iter(app.world())
        .map(|(v, t)| (v.kind, t.0.clone()))
        .collect::<Vec<_>>();
    assert!(
        texts
            .iter()
            .any(|(kind, text)| *kind == "industry_aggregate" && text.contains("42"))
    );
    assert!(texts.iter().any(|(kind, _)| *kind == "world_event"));
    {
        let mut runtime = app.world_mut().resource_mut::<BevyRuntimeState>();
        let state = runtime.render_state.as_mut().unwrap();
        state.industry_aggregates.clear();
        state.world_events.clear();
        runtime.render_version += 1;
    }
    app.update();
    assert_eq!(
        app.world_mut()
            .query::<&SpatialOverlayVisual>()
            .iter(app.world())
            .count(),
        0
    );
    assert!(
        !hit_regions(&mut app)
            .iter()
            .any(|h| matches!(h.kind, "industry_aggregate" | "world_event"))
    );
}
