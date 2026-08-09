use super::*;

const LOCATION_CORNER_FRAME_COLOR: [u8; 3] = [167, 243, 208];

#[derive(Clone, Copy, Debug, PartialEq)]
struct CornerFrameSegment {
    entity: Entity,
    x: f32,
    y: f32,
    z: f32,
    width: f32,
    height: f32,
    alpha: u8,
}

fn normal_location_corner_frame_state() -> RenderState {
    let mut state = sample_render_state(12_000.0);
    state.selection = None;
    state.locations[0].id = "loc-corner-frame".to_string();
    state.locations[0].marker_role = None;
    state.locations[0].marker_alpha = Some(0.72);
    state
}

fn location_corner_frame_segments(app: &mut App) -> Vec<CornerFrameSegment> {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &Sprite, &Transform)>();
    let mut segments = query
        .iter(world)
        .filter_map(|(entity, sprite, transform)| {
            let size = sprite.custom_size?;
            let color = sprite.color.to_srgba();
            ([
                (color.red * 255.0).round() as u8,
                (color.green * 255.0).round() as u8,
                (color.blue * 255.0).round() as u8,
            ] == LOCATION_CORNER_FRAME_COLOR
                && (size.x == 2.0 || size.y == 2.0))
                .then_some(CornerFrameSegment {
                    entity,
                    x: transform.translation.x,
                    y: transform.translation.y,
                    z: transform.translation.z,
                    width: size.x,
                    height: size.y,
                    alpha: (color.alpha * 255.0).round() as u8,
                })
        })
        .collect::<Vec<_>>();
    segments.sort_by(|left, right| left.entity.to_bits().cmp(&right.entity.to_bits()));
    segments
}

fn normalized_location_hit_regions(app: &mut App) -> Vec<(String, i64, i64, i64, i64)> {
    let mut regions = hit_regions(app)
        .into_iter()
        .filter(|region| region.kind == "location")
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

fn assert_corner_frame_geometry(segments: &[CornerFrameSegment], location: &VisualProbeRow) {
    assert_eq!(
        segments.len(),
        4,
        "an eligible location needs upper-left and lower-right L brackets with two arms each"
    );
    let arm_length = (location.size_px * 0.30).clamp(3.0, 6.0);
    let expected_alpha = (location.alpha * 0.85 * 255.0).round() as u8;
    assert!(segments.iter().all(|segment| {
        (segment.width == 2.0 && segment.height == arm_length)
            || (segment.width == arm_length && segment.height == 2.0)
    }));
    assert!(
        segments
            .iter()
            .all(|segment| { segment.z == location.z + 0.01 && segment.alpha == expected_alpha })
    );

    let outer_offset = (location.size_px / 2.0) + 3.0;
    for (x_sign, y_sign) in [(-1.0_f32, 1.0_f32), (1.0_f32, -1.0_f32)] {
        let corner = segments
            .iter()
            .filter(|segment| {
                (segment.x - location.x).signum() == x_sign
                    && (segment.y - location.y).signum() == y_sign
            })
            .collect::<Vec<_>>();
        assert_eq!(corner.len(), 2, "each eligible corner has two L arms");
        assert!(corner.iter().any(|segment| segment.width > segment.height));
        assert!(corner.iter().any(|segment| segment.height > segment.width));
        assert!(corner.iter().all(|segment| {
            ((segment.x - location.x).abs() - outer_offset).abs() <= (arm_length / 2.0)
                && ((segment.y - location.y).abs() - outer_offset).abs() <= (arm_length / 2.0)
        }));
    }
}

#[test]
fn location_corner_frame_renders_four_mint_segments_reuses_them_and_keeps_hit_regions() {
    let state = normal_location_corner_frame_state();
    let mut app = render_test_app(state);
    let location = visual_probe_summary(&mut app)
        .locations
        .into_iter()
        .find(|location| location.id == "loc-corner-frame")
        .expect("normal location renders its base marker");
    let baseline_hit_regions = normalized_location_hit_regions(&mut app);
    let initial = location_corner_frame_segments(&mut app);

    assert_corner_frame_geometry(&initial, &location);
    assert_eq!(
        normalized_location_hit_regions(&mut app),
        baseline_hit_regions,
        "display-only corner frames must not alter location hit-region identity or bounds"
    );

    app.update();
    let updated_location = visual_probe_summary(&mut app)
        .locations
        .into_iter()
        .find(|location| location.id == "loc-corner-frame")
        .expect("normal location remains rendered after pulse update");
    let updated = location_corner_frame_segments(&mut app);
    assert_eq!(
        updated
            .iter()
            .map(|segment| segment.entity)
            .collect::<Vec<_>>(),
        initial
            .iter()
            .map(|segment| segment.entity)
            .collect::<Vec<_>>(),
        "unchanged reconcile must reuse the same four corner-frame entities"
    );
    assert_corner_frame_geometry(&updated, &updated_location);

    {
        let mut runtime = app.world_mut().resource_mut::<BevyRuntimeState>();
        runtime
            .render_state
            .as_mut()
            .expect("render state")
            .locations
            .clear();
        runtime.render_version += 1;
        runtime.hit_regions_dirty = true;
    }
    app.update();
    assert!(
        location_corner_frame_segments(&mut app).is_empty(),
        "removing an eligible location must clean up every corner-frame arm"
    );
}

#[test]
fn location_corner_frame_excludes_selected_logic_anchor_off_canvas_unmounted_and_absent_state() {
    let mut selected_state = normal_location_corner_frame_state();
    selected_state.selection = Some(Selection {
        kind: "location".to_string(),
        id: "loc-corner-frame".to_string(),
    });
    let mut selected = render_test_app(selected_state);
    assert!(
        location_corner_frame_segments(&mut selected).is_empty(),
        "selected locations retain their amber selection treatment instead of mint corner frames"
    );

    let mut logic_anchor_state = normal_location_corner_frame_state();
    logic_anchor_state.locations[0].marker_role = Some("logic_anchor".to_string());
    let mut logic_anchor = render_test_app(logic_anchor_state);
    assert!(
        location_corner_frame_segments(&mut logic_anchor).is_empty(),
        "logic anchors must not receive landmark corner frames"
    );

    let mut off_canvas_state = normal_location_corner_frame_state();
    off_canvas_state.locations[0].pos.x_cm = -1.0;
    let mut off_canvas = render_test_app(off_canvas_state);
    assert!(
        location_corner_frame_segments(&mut off_canvas).is_empty(),
        "off-canvas locations must not leave visible corner-frame geometry"
    );

    let mut unmounted = render_test_app(normal_location_corner_frame_state());
    unmounted
        .world_mut()
        .resource_mut::<BevyRuntimeState>()
        .mounted = false;
    unmounted.update();
    assert!(
        location_corner_frame_segments(&mut unmounted).is_empty(),
        "unmount must clear every location corner-frame entity"
    );

    let mut absent = render_test_app(normal_location_corner_frame_state());
    absent
        .world_mut()
        .resource_mut::<BevyRuntimeState>()
        .render_state = None;
    absent.update();
    assert!(
        location_corner_frame_segments(&mut absent).is_empty(),
        "an absent render state must clear every location corner-frame entity"
    );
}
