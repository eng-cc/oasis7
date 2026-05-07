use super::*;
use crate::GridLineKind;

#[test]
fn drag_delta_requires_active_dragging() {
    let current = Vec2::new(40.0, 20.0);
    let (delta, next_cursor) = drag_delta(Some(Vec2::new(10.0, 10.0)), Some(current), false);
    assert_eq!(delta, Vec2::ZERO);
    assert_eq!(next_cursor, None);
}

#[test]
fn drag_delta_uses_cursor_position_difference() {
    let previous = Vec2::new(10.0, 10.0);
    let current = Vec2::new(24.0, 30.0);
    let (delta, next_cursor) = drag_delta(Some(previous), Some(current), true);
    assert_eq!(delta, Vec2::new(14.0, 20.0));
    assert_eq!(next_cursor, Some(current));
}

#[test]
fn normalized_mouse_wheel_delta_converts_pixel_to_line_scale() {
    let line = normalized_mouse_wheel_delta(MouseScrollUnit::Line, 1.5);
    let pixel = normalized_mouse_wheel_delta(
        MouseScrollUnit::Pixel,
        MouseScrollUnit::SCROLL_UNIT_CONVERSION_FACTOR * 1.5,
    );
    assert!((line - pixel).abs() < f32::EPSILON);
}

#[test]
fn pinch_scroll_delta_expands_small_magnify_values() {
    let zoom_in = pinch_scroll_delta(0.25);
    let zoom_out = pinch_scroll_delta(-0.25);
    assert!(zoom_in > 0.0);
    assert!(zoom_out < 0.0);
    assert!((zoom_in + zoom_out).abs() < f32::EPSILON);
}

#[test]
fn wasd_axis_maps_pressed_keys() {
    let mut keys = ButtonInput::<KeyCode>::default();
    assert_eq!(wasd_axis(&keys), Vec2::ZERO);

    keys.press(KeyCode::KeyW);
    keys.press(KeyCode::KeyD);
    assert_eq!(wasd_axis(&keys), Vec2::new(1.0, 1.0));

    keys.release(KeyCode::KeyW);
    keys.press(KeyCode::KeyS);
    assert_eq!(wasd_axis(&keys), Vec2::new(1.0, -1.0));

    keys.press(KeyCode::KeyA);
    keys.release(KeyCode::KeyD);
    assert_eq!(wasd_axis(&keys), Vec2::new(-1.0, -1.0));
}

#[test]
fn cursor_in_3d_view_respects_right_panel_bound() {
    let mut window = Window::default();
    window.resolution.set(1200.0, 800.0);

    assert!(cursor_in_3d_view(&window, Vec2::new(879.5, 100.0), 320.0));
    assert!(!cursor_in_3d_view(&window, Vec2::new(880.5, 100.0), 320.0));
}

#[test]
fn two_d_ortho_scale_decreases_when_radius_decreases() {
    let cm_to_unit = Viewer3dConfig::default().effective_cm_to_unit();
    let reference = two_d_reference_radius(cm_to_unit);
    let zoom_in_scale = two_d_ortho_scale_for_radius(
        (reference * 0.5).max(orbit_min_radius(cm_to_unit)),
        cm_to_unit,
    );
    let zoom_out_scale =
        two_d_ortho_scale_for_radius((reference * 1.5).min(ORBIT_MAX_RADIUS), cm_to_unit);
    assert!(zoom_in_scale < zoom_out_scale);
}

#[test]
fn sync_2d_zoom_projection_updates_orthographic_scale() {
    let config = Viewer3dConfig::default();
    let cm_to_unit = config.effective_cm_to_unit();
    let mut projection = camera_projection_for_mode(ViewerCameraMode::TwoD, &config);
    let before = match &projection {
        Projection::Orthographic(ortho) => ortho.scale,
        _ => panic!("expected orthographic projection"),
    };

    let zoom_in_radius =
        (two_d_reference_radius(cm_to_unit) * 0.6).max(orbit_min_radius(cm_to_unit));
    sync_2d_zoom_projection(&mut projection, zoom_in_radius, cm_to_unit);
    let after = match &projection {
        Projection::Orthographic(ortho) => ortho.scale,
        _ => panic!("expected orthographic projection"),
    };
    assert!(after < before);
}

#[test]
fn two_d_ortho_scale_supports_large_zoom_out() {
    let cm_to_unit = 0.0002;
    let reference = two_d_reference_radius(cm_to_unit);
    let far_zoom_out_scale =
        two_d_ortho_scale_for_radius((reference * 4.0).min(ORBIT_MAX_RADIUS), cm_to_unit);
    assert!(far_zoom_out_scale > 8.0);
}

#[test]
fn two_d_detail_default_zoom_is_deep_enough_for_agent_readability() {
    let cm_to_unit = Viewer3dConfig::default().effective_cm_to_unit();
    let detail_radius = two_d_detail_default_radius(cm_to_unit);
    let detail_scale = two_d_ortho_scale_for_radius(detail_radius, cm_to_unit);
    assert!(detail_scale < 0.001);
}

#[test]
fn apply_orbit_input_updates_focus_and_radius() {
    let mut orbit = OrbitCamera {
        focus: Vec3::ZERO,
        radius: 20.0,
        yaw: 0.0,
        pitch: 0.0,
    };

    let changed = apply_orbit_input(
        &mut orbit,
        Vec2::new(6.0, -4.0),
        1.0,
        false,
        true,
        ViewerCameraMode::ThreeD,
        ORBIT_MIN_RADIUS,
        ORBIT_MAX_RADIUS,
    );
    assert!(changed);
    assert_ne!(orbit.focus, Vec3::ZERO);
    assert!(orbit.radius < 20.0);
}

#[test]
fn apply_orbit_input_zoom_out_clamps_to_expanded_max_radius() {
    let mut orbit = OrbitCamera {
        focus: Vec3::ZERO,
        radius: 100.0,
        yaw: 0.0,
        pitch: 0.0,
    };

    let changed = apply_orbit_input(
        &mut orbit,
        Vec2::ZERO,
        -1_000.0,
        false,
        false,
        ViewerCameraMode::TwoD,
        ORBIT_MIN_RADIUS,
        ORBIT_MAX_RADIUS,
    );
    assert!(changed);
    assert!(ORBIT_MAX_RADIUS > 300.0);
    assert!((orbit.radius - ORBIT_MAX_RADIUS).abs() < f32::EPSILON);
}

#[test]
fn apply_orbit_input_2d_mode_ignores_rotation_drag() {
    let mut orbit = OrbitCamera {
        focus: Vec3::ZERO,
        radius: 20.0,
        yaw: 1.0,
        pitch: -1.53,
    };

    let changed = apply_orbit_input(
        &mut orbit,
        Vec2::new(8.0, 5.0),
        0.0,
        true,
        false,
        ViewerCameraMode::TwoD,
        ORBIT_MIN_RADIUS,
        ORBIT_MAX_RADIUS,
    );

    assert!(!changed);
    assert!((orbit.yaw - 1.0).abs() < f32::EPSILON);
    assert!((orbit.pitch + 1.53).abs() < f32::EPSILON);
}

#[test]
fn apply_keyboard_pan_two_d_moves_focus_on_horizontal_plane() {
    let mut orbit = OrbitCamera {
        focus: Vec3::new(0.0, 3.0, 0.0),
        radius: 90.0,
        yaw: 0.0,
        pitch: -1.53,
    };

    let changed = apply_keyboard_pan(&mut orbit, Vec2::new(0.0, 1.0), 1.0, false);
    assert!(changed);
    assert!((orbit.focus.y - 3.0).abs() < 1e-6);
    assert!(orbit.focus.z < 0.0);
}

#[test]
fn apply_keyboard_pan_three_d_follows_camera_heading() {
    let mut orbit = OrbitCamera {
        focus: Vec3::ZERO,
        radius: 48.0,
        yaw: std::f32::consts::FRAC_PI_2,
        pitch: 0.55,
    };

    let changed = apply_keyboard_pan(&mut orbit, Vec2::new(0.0, 1.0), 1.0, false);
    assert!(changed);
    assert!(orbit.focus.x < 0.0);
    assert!(orbit.focus.z.abs() < orbit.focus.x.abs());
}

#[test]
fn apply_keyboard_pan_shift_boost_moves_faster() {
    let mut normal = OrbitCamera {
        focus: Vec3::ZERO,
        radius: 64.0,
        yaw: -0.7,
        pitch: 0.55,
    };
    let mut boosted = OrbitCamera {
        focus: Vec3::ZERO,
        radius: 64.0,
        yaw: -0.7,
        pitch: 0.55,
    };

    let normal_changed = apply_keyboard_pan(&mut normal, Vec2::new(1.0, 0.0), 1.0, false);
    let boosted_changed = apply_keyboard_pan(&mut boosted, Vec2::new(1.0, 0.0), 1.0, true);
    assert!(normal_changed && boosted_changed);
    assert!(boosted.focus.distance(Vec3::ZERO) > normal.focus.distance(Vec3::ZERO));
}

#[test]
fn camera_projection_matches_mode() {
    let config = Viewer3dConfig::default();
    let two_d = camera_projection_for_mode(ViewerCameraMode::TwoD, &config);
    match two_d {
        Projection::Orthographic(projection) => {
            assert!(projection.scale > 0.0);
            assert!(projection.scale < 1.0);
        }
        _ => panic!("expected orthographic projection for 2D mode"),
    }

    let three_d = camera_projection_for_mode(ViewerCameraMode::ThreeD, &config);
    assert!(matches!(three_d, Projection::Perspective(_)));
}

#[test]
fn camera_projection_scales_near_and_keeps_far_covering_world() {
    let config = Viewer3dConfig::default();
    let units_per_meter = config.effective_cm_to_unit() * 100.0;
    let expected_near = config.physical.camera_near_m * units_per_meter;

    let projection = camera_projection_for_mode(ViewerCameraMode::ThreeD, &config);
    let Projection::Perspective(perspective) = projection else {
        panic!("expected perspective projection");
    };
    assert!((perspective.near - expected_near).abs() < 1e-6);
    assert!(perspective.far >= world_view_radius(config.effective_cm_to_unit()));
}

#[test]
fn orbit_min_radius_scales_with_world_units() {
    let config = Viewer3dConfig::default();
    let min_radius = orbit_min_radius(config.effective_cm_to_unit());
    assert!((min_radius - 0.004).abs() < 1e-6);
}

#[test]
fn grid_line_thickness_uses_mode_and_kind() {
    let world_2d = grid_line_thickness(GridLineKind::World, ViewerCameraMode::TwoD);
    let world_3d = grid_line_thickness(GridLineKind::World, ViewerCameraMode::ThreeD);
    let chunk_2d = grid_line_thickness(GridLineKind::Chunk, ViewerCameraMode::TwoD);
    let chunk_3d = grid_line_thickness(GridLineKind::Chunk, ViewerCameraMode::ThreeD);

    assert!(world_2d < world_3d);
    assert!(chunk_2d < chunk_3d);
    assert!(chunk_2d > world_2d);
    assert!(chunk_3d > world_3d);
}

#[test]
fn camera_orbit_preset_two_d_has_top_down_pitch() {
    let cm_to_unit = Viewer3dConfig::default().effective_cm_to_unit();
    let orbit = camera_orbit_preset(ViewerCameraMode::TwoD, None, cm_to_unit);
    assert!(orbit.pitch < -1.5);
    assert!(orbit.radius >= orbit_min_radius(cm_to_unit));
    assert!(orbit.radius < world_view_radius(cm_to_unit));
}

#[test]
fn sync_camera_mode_two_d_projection_matches_orbit_radius() {
    let mut app = App::new();
    app.add_systems(Update, sync_camera_mode);
    app.insert_resource(Viewer3dConfig::default());
    app.insert_resource(ViewerCameraMode::ThreeD);
    app.insert_resource(AutoFocusState::default());

    let config = *app.world().resource::<Viewer3dConfig>();
    let mut transform = Transform::default();
    let orbit = camera_orbit_preset(
        ViewerCameraMode::ThreeD,
        Some(Vec3::ZERO),
        config.effective_cm_to_unit(),
    );
    orbit.apply_to_transform(&mut transform);
    app.world_mut().spawn((
        Viewer3dCamera,
        orbit,
        transform,
        camera_projection_for_mode(ViewerCameraMode::ThreeD, &config),
    ));

    app.world_mut().insert_resource(ViewerCameraMode::TwoD);
    app.update();

    let mut query = app.world_mut().query::<(&OrbitCamera, &Projection)>();
    let (orbit, projection) = query
        .single(app.world())
        .expect("camera query should contain one entity");
    let Projection::Orthographic(ortho) = projection else {
        panic!("expected orthographic projection");
    };
    let expected = two_d_ortho_scale_for_radius(orbit.radius, config.effective_cm_to_unit());
    assert!((ortho.scale - expected).abs() < 1e-6);
}

#[test]
fn sync_camera_mode_ignores_redundant_three_d_assignment() {
    let mut app = App::new();
    app.add_systems(Update, sync_camera_mode);
    app.insert_resource(Viewer3dConfig::default());
    app.insert_resource(ViewerCameraMode::ThreeD);
    app.insert_resource(AutoFocusState::default());

    let config = *app.world().resource::<Viewer3dConfig>();
    let custom_radius = 13.44;
    let mut transform = Transform::default();
    let orbit = OrbitCamera {
        focus: Vec3::new(10.0, 2.0, -8.0),
        radius: custom_radius,
        yaw: 0.35,
        pitch: -0.8,
    };
    orbit.apply_to_transform(&mut transform);
    app.world_mut().spawn((
        Viewer3dCamera,
        orbit,
        transform,
        camera_projection_for_mode(ViewerCameraMode::ThreeD, &config),
    ));

    app.world_mut().insert_resource(ViewerCameraMode::ThreeD);
    app.update();

    let mut query = app.world_mut().query::<&OrbitCamera>();
    let orbit = query
        .single(app.world())
        .expect("camera query should contain one entity");
    assert!((orbit.radius - custom_radius).abs() < 1e-6);
}

#[test]
fn world_background_surfaces_are_always_hidden() {
    let mut app = App::new();
    app.add_systems(Update, sync_world_background_visibility);
    app.insert_resource(ViewerCameraMode::default());
    let floor = app
        .world_mut()
        .spawn((WorldFloorSurface, Visibility::Visible))
        .id();
    let bounds = app
        .world_mut()
        .spawn((WorldBoundsSurface, Visibility::Visible))
        .id();

    app.world_mut().insert_resource(ViewerCameraMode::TwoD);
    app.update();

    let floor_visibility = app
        .world()
        .get::<Visibility>(floor)
        .expect("floor visibility");
    let bounds_visibility = app
        .world()
        .get::<Visibility>(bounds)
        .expect("bounds visibility");
    assert_eq!(*floor_visibility, Visibility::Hidden);
    assert_eq!(*bounds_visibility, Visibility::Hidden);

    app.world_mut().insert_resource(ViewerCameraMode::ThreeD);
    app.update();
    let floor_visibility = app
        .world()
        .get::<Visibility>(floor)
        .expect("floor visibility");
    let bounds_visibility = app
        .world()
        .get::<Visibility>(bounds)
        .expect("bounds visibility");
    assert_eq!(*floor_visibility, Visibility::Hidden);
    assert_eq!(*bounds_visibility, Visibility::Hidden);
}

#[test]
fn two_d_zoom_tier_switches_with_hysteresis() {
    let cm_to_unit = Viewer3dConfig::default().effective_cm_to_unit();
    let (exit, enter) = two_d_overview_thresholds(cm_to_unit);

    let to_overview = two_d_zoom_tier_for_radius(enter + 0.01, cm_to_unit, TwoDZoomTier::Detail);
    assert_eq!(to_overview, TwoDZoomTier::Overview);

    let stay_overview =
        two_d_zoom_tier_for_radius((enter + exit) * 0.5, cm_to_unit, TwoDZoomTier::Overview);
    assert_eq!(stay_overview, TwoDZoomTier::Overview);

    let back_to_detail =
        two_d_zoom_tier_for_radius(exit - 0.01, cm_to_unit, TwoDZoomTier::Overview);
    assert_eq!(back_to_detail, TwoDZoomTier::Detail);
}

#[test]
fn two_d_map_marker_visibility_follows_zoom_tier() {
    let mut app = App::new();
    app.add_systems(Update, sync_two_d_map_marker_visibility);
    app.insert_resource(ViewerCameraMode::TwoD);
    app.insert_resource(TwoDZoomTier::Detail);

    let marker = app
        .world_mut()
        .spawn((SceneZoomLayer::TwoDOverviewMarker, Visibility::Hidden))
        .id();

    app.update();
    let visibility = app
        .world()
        .get::<Visibility>(marker)
        .expect("marker visibility");
    assert_eq!(*visibility, Visibility::Hidden);

    *app.world_mut().resource_mut::<TwoDZoomTier>() = TwoDZoomTier::Overview;
    app.update();
    let visibility = app
        .world()
        .get::<Visibility>(marker)
        .expect("marker visibility");
    assert_eq!(*visibility, Visibility::Visible);

    *app.world_mut().resource_mut::<ViewerCameraMode>() = ViewerCameraMode::ThreeD;
    app.update();
    let visibility = app
        .world()
        .get::<Visibility>(marker)
        .expect("marker visibility");
    assert_eq!(*visibility, Visibility::Hidden);
}

#[test]
fn two_d_map_marker_scale_boosts_in_overview_and_resets_in_detail() {
    let mut app = App::new();
    app.add_systems(Update, sync_two_d_map_marker_scale);
    app.insert_resource(ViewerCameraMode::TwoD);
    app.insert_resource(TwoDZoomTier::Overview);
    app.insert_resource(Viewer3dConfig::default());

    let config = *app.world().resource::<Viewer3dConfig>();
    let cm_to_unit = config.effective_cm_to_unit();
    let mut camera_transform = Transform::default();
    let orbit = OrbitCamera {
        focus: Vec3::ZERO,
        radius: two_d_detail_default_radius(cm_to_unit) * 24.0,
        yaw: 0.0,
        pitch: -1.53,
    };
    orbit.apply_to_transform(&mut camera_transform);
    app.world_mut()
        .spawn((Viewer3dCamera, orbit, camera_transform));

    let base = Vec3::new(0.002, 0.0002, 0.002);
    let marker = app
        .world_mut()
        .spawn((
            SceneZoomLayer::TwoDOverviewMarker,
            BaseScale(base),
            Transform::from_scale(base),
            Visibility::Visible,
        ))
        .id();

    app.update();
    let boosted = app
        .world()
        .get::<Transform>(marker)
        .expect("marker transform in overview")
        .scale;
    assert!(boosted.x > base.x);

    *app.world_mut().resource_mut::<TwoDZoomTier>() = TwoDZoomTier::Detail;
    app.update();
    let reset = app
        .world()
        .get::<Transform>(marker)
        .expect("marker transform in detail")
        .scale;
    assert!((reset.x - base.x).abs() < 1e-6);
    assert!((reset.y - base.y).abs() < 1e-6);
    assert!((reset.z - base.z).abs() < 1e-6);
}

#[test]
fn detail_zoom_visibility_hides_detail_entities_in_overview() {
    let mut app = App::new();
    app.add_systems(Update, sync_detail_zoom_visibility);
    app.insert_resource(ViewerCameraMode::TwoD);
    app.insert_resource(TwoDZoomTier::Detail);

    let detail_entity = app
        .world_mut()
        .spawn((SceneZoomLayer::Detail, Visibility::Visible))
        .id();

    app.update();
    let visibility = app
        .world()
        .get::<Visibility>(detail_entity)
        .expect("detail visibility");
    assert_eq!(*visibility, Visibility::Visible);

    *app.world_mut().resource_mut::<TwoDZoomTier>() = TwoDZoomTier::Overview;
    app.update();
    let visibility = app
        .world()
        .get::<Visibility>(detail_entity)
        .expect("detail visibility");
    assert_eq!(*visibility, Visibility::Hidden);

    *app.world_mut().resource_mut::<ViewerCameraMode>() = ViewerCameraMode::ThreeD;
    app.update();
    let visibility = app
        .world()
        .get::<Visibility>(detail_entity)
        .expect("detail visibility");
    assert_eq!(*visibility, Visibility::Visible);
}

#[test]
fn grid_line_lod_hides_far_chunk_lines_and_keeps_world_lines() {
    let mut app = App::new();
    app.add_systems(Update, update_grid_line_lod_visibility);
    app.insert_resource(ViewerCameraMode::ThreeD);
    app.insert_resource(Viewer3dConfig::default());
    app.insert_resource(WorldOverlayConfig::default());

    app.world_mut().spawn((
        Viewer3dCamera,
        Transform::from_xyz(0.0, 0.0, 0.0),
        GlobalTransform::default(),
    ));

    let world_line = app
        .world_mut()
        .spawn((
            GridLineVisual {
                kind: GridLineKind::World,
                axis: crate::GridLineAxis::AlongX,
                span: 10.0,
            },
            GlobalTransform::from_translation(Vec3::new(0.0, 0.0, -200.0)),
            Visibility::Visible,
        ))
        .id();

    let chunk_line = app
        .world_mut()
        .spawn((
            GridLineVisual {
                kind: GridLineKind::Chunk,
                axis: crate::GridLineAxis::AlongX,
                span: 10.0,
            },
            GlobalTransform::from_translation(Vec3::new(0.0, 0.0, -220.0)),
            Visibility::Visible,
        ))
        .id();

    app.update();

    let world_visibility = app
        .world()
        .get::<Visibility>(world_line)
        .expect("world visibility");
    let chunk_visibility = app
        .world()
        .get::<Visibility>(chunk_line)
        .expect("chunk visibility");

    assert_eq!(*world_visibility, Visibility::Visible);
    assert_eq!(*chunk_visibility, Visibility::Hidden);
}
