use super::*;
use crate::render::links::PixelWorldLinkVisual;

fn generic_link_state(
    kind: &str,
    status: Option<&str>,
    source_class: Option<&str>,
    freshness: Option<&str>,
) -> RenderState {
    let mut state = sample_render_state(12_000.0);
    state.locations.clear();
    state.agents.clear();
    state.visual_hotspots.clear();
    state.selection = None;
    state.links = vec![Link {
        id: "route:ore-line".to_string(),
        kind: kind.to_string(),
        label: None,
        from: sample_position(1_050_000.0, 750_000.0),
        to: sample_position(2_100_000.0, 1_250_000.0),
        emphasis: Some(0.72),
        status: status.map(ToString::to_string),
        source_class: source_class.map(ToString::to_string),
        freshness: freshness.map(ToString::to_string),
    }];
    state
}

fn direction_cues(app: &mut App) -> Vec<(String, Transform)> {
    let world = app.world_mut();
    let mut query = world.query::<(&PixelWorldLinkVisual, &Transform)>();
    query
        .iter(world)
        .filter(|(visual, _)| visual.id.contains(":direction:"))
        .map(|(visual, transform)| (visual.id.clone(), *transform))
        .collect()
}

fn canvas_point_for_link_endpoint(app: &mut App, link: &Link, to: bool) -> Vec2 {
    let runtime = app.world_mut().resource::<BevyRuntimeState>();
    let bounds = runtime
        .render_state
        .as_ref()
        .and_then(|state| state.world_bounds.as_ref())
        .expect("route fixture bounds");
    let point = if to { &link.to } else { &link.from };
    let (x, y) = to_canvas_point(
        point,
        bounds,
        VIEWPORT_WIDTH as f64,
        VIEWPORT_HEIGHT as f64,
        &runtime.camera,
    )
    .expect("route fixture endpoint");
    Vec2::new(
        x as f32 - (VIEWPORT_WIDTH as f32 / 2.0),
        (VIEWPORT_HEIGHT as f32 / 2.0) - y as f32,
    )
}

fn direction_sprite_geometry(app: &mut App) -> Vec<(String, Vec2, Vec2)> {
    let world = app.world_mut();
    let mut query = world.query::<(&PixelWorldLinkVisual, &Sprite, &Transform)>();
    query
        .iter(world)
        .filter(|(visual, _, _)| visual.id.contains(":direction:"))
        .map(|(visual, sprite, transform)| {
            (
                visual.id.clone(),
                sprite.custom_size.unwrap_or(Vec2::ZERO),
                transform.translation.truncate(),
            )
        })
        .collect()
}

#[test]
fn authoritative_route_links_get_one_destination_direction_cue() {
    let mut app = render_test_app(generic_link_state(
        "route",
        Some("active"),
        Some("runtime_projection"),
        Some("current"),
    ));

    let cues = direction_cues(&mut app);
    assert_eq!(
        cues.len(),
        2,
        "the authoritative route keeps one two-stroke destination cue"
    );
    assert!(
        cues.iter()
            .all(|(id, _)| id.starts_with("route:ore-line:direction:"))
    );
    assert_eq!(
        cues.iter()
            .map(|(id, _)| id.as_str())
            .collect::<HashSet<_>>(),
        HashSet::from([
            "route:ore-line:direction:upper",
            "route:ore-line:direction:lower",
        ]),
        "the cue is a single arrowhead rather than a repeated flow texture"
    );
    let link = generic_link_state(
        "route",
        Some("active"),
        Some("runtime_projection"),
        Some("current"),
    )
    .links[0]
        .clone();
    let from = canvas_point_for_link_endpoint(&mut app, &link, false);
    let to = canvas_point_for_link_endpoint(&mut app, &link, true);
    assert!(
        cues.iter().all(|(_, transform)| {
            transform.translation.truncate().distance(to)
                < transform.translation.truncate().distance(from)
        }),
        "generic route direction strokes must point toward the authoritative destination"
    );
}

#[test]
fn authoritative_route_direction_strokes_have_visible_destination_geometry() {
    let mut visible = render_test_app(generic_link_state(
        "route",
        Some("active"),
        Some("runtime_projection"),
        Some("current"),
    ));
    let geometry = direction_sprite_geometry(&mut visible);
    assert_eq!(geometry.len(), 2);
    assert!(
        geometry
            .iter()
            .all(|(_, size, _)| size.x > 0.0 && size.y > 0.0),
        "each direction cue must have non-zero Sprite geometry"
    );

    let mut neutral = render_test_app(generic_link_state(
        "route",
        Some("active"),
        Some("local_pending"),
        Some("current"),
    ));
    assert!(
        direction_sprite_geometry(&mut neutral).is_empty(),
        "a route without current runtime authority must keep a neutral line"
    );
}

#[test]
fn link_id_never_grants_or_revokes_direction_authority() {
    let mut authoritative = generic_link_state(
        "route",
        Some("active"),
        Some("runtime_projection"),
        Some("current"),
    );
    authoritative.links[0].id = "forged:opaque-link-id".to_string();
    let mut authoritative_app = render_test_app(authoritative);
    assert_eq!(
        direction_cues(&mut authoritative_app).len(),
        2,
        "published route semantics and current authority permit cues even with an opaque id"
    );

    let mut forged = generic_link_state(
        "unknown",
        Some("active"),
        Some("runtime_projection"),
        Some("current"),
    );
    forged.links[0].id = "route:forged-arrow".to_string();
    let mut forged_app = render_test_app(forged);
    assert!(
        direction_cues(&mut forged_app).is_empty(),
        "a route-looking id cannot grant direction to an unknown semantic kind"
    );
}

#[test]
fn unknown_undirected_unfresh_or_zero_length_links_remain_neutral() {
    for (kind, status, source_class, freshness) in [
        (
            "unknown",
            Some("active"),
            Some("runtime_projection"),
            Some("current"),
        ),
        ("route", None, Some("runtime_projection"), Some("current")),
        (
            "logistics",
            Some("active"),
            Some("local_pending"),
            Some("current"),
        ),
        (
            "logistics",
            Some("active"),
            Some("runtime_projection"),
            Some("stale"),
        ),
    ] {
        let mut app = render_test_app(generic_link_state(kind, status, source_class, freshness));
        assert!(
            direction_cues(&mut app).is_empty(),
            "non-authoritative or unknown link {kind} must retain a neutral line"
        );
    }

    let mut zero_length = generic_link_state(
        "route",
        Some("active"),
        Some("runtime_projection"),
        Some("current"),
    );
    zero_length.links[0].to = zero_length.links[0].from.clone();
    let mut app = render_test_app(zero_length);
    assert!(
        direction_cues(&mut app).is_empty(),
        "a zero-length route cannot claim a direction"
    );
}
