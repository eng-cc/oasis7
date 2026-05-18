use std::collections::HashSet;

use super::*;

const LOCATION_HIT_HALF_SIZE: f64 = 8.0;
const AGENT_HIT_HALF_SIZE: f64 = 8.0;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GridLayoutKey {
    width: i32,
    height: i32,
    step_milli: i32,
    offset_x_milli: i32,
    offset_y_milli: i32,
}

#[derive(Component)]
pub(crate) struct PixelWorldGridVisual;

#[derive(Component)]
pub(crate) struct PixelWorldLocationVisual {
    id: String,
}

#[derive(Component)]
pub(crate) struct PixelWorldAgentVisual {
    id: String,
}

#[derive(Component)]
pub(crate) struct PixelWorldLinkVisual {
    id: String,
}

#[derive(Component)]
pub(crate) struct PixelWorldHotspotVisual {
    id: String,
}

fn maybe_auto_fit_camera(runtime: &mut BevyRuntimeState, width: f64, height: f64) {
    if runtime.camera_fit_version == runtime.render_version || runtime.camera_user_override {
        return;
    }
    let Some(render_state) = runtime.render_state.as_ref() else {
        return;
    };
    let Some(world_bounds) = render_state.world_bounds.as_ref() else {
        return;
    };

    let mut points = Vec::new();
    let base_camera = CameraState::default();

    for location in &render_state.locations {
        if let Some(point) =
            to_canvas_point(&location.pos, world_bounds, width, height, &base_camera)
        {
            points.push(point);
        }
    }
    for agent in &render_state.agents {
        if let Some(pos) = agent.pos.as_ref() {
            if let Some(point) = to_canvas_point(pos, world_bounds, width, height, &base_camera) {
                points.push(point);
            }
        }
    }
    for hotspot in &render_state.visual_hotspots {
        if let Some(point) =
            to_canvas_point(&hotspot.pos, world_bounds, width, height, &base_camera)
        {
            points.push(point);
        }
    }

    if points.is_empty() {
        runtime.camera_fit_version = runtime.render_version;
        return;
    }

    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for (x, y) in points {
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }

    let content_width = (max_x - min_x).max(40.0);
    let content_height = (max_y - min_y).max(40.0);
    let target_zoom_x = ((width - 180.0).max(120.0) / content_width).clamp(0.6, 3.5);
    let target_zoom_y = ((height - 180.0).max(120.0) / content_height).clamp(0.6, 3.5);
    let target_zoom = target_zoom_x.min(target_zoom_y);
    let content_center_x = (min_x + max_x) / 2.0;
    let content_center_y = (min_y + max_y) / 2.0;
    let centered_x = content_center_x - (width / 2.0);
    let centered_y = content_center_y - (height / 2.0);

    runtime.camera.zoom = target_zoom;
    runtime.camera.pan_x_px = -(centered_x * target_zoom);
    runtime.camera.pan_y_px = -(centered_y * target_zoom);
    runtime.camera_fit_version = runtime.render_version;
    let _ = emit_camera_state(&runtime.camera);
}

fn build_grid_layout(camera: &CameraState, width: f64, height: f64) -> GridLayoutKey {
    let grid_step = clamp(24.0 * camera.zoom.max(0.5), 12.0, 72.0);
    let offset_x = ((camera.pan_x_px % grid_step) + grid_step) % grid_step;
    let offset_y = ((camera.pan_y_px % grid_step) + grid_step) % grid_step;
    GridLayoutKey {
        width: width.round() as i32,
        height: height.round() as i32,
        step_milli: (grid_step * 1000.0).round() as i32,
        offset_x_milli: (offset_x * 1000.0).round() as i32,
        offset_y_milli: (offset_y * 1000.0).round() as i32,
    }
}

fn grid_geometry(layout: &GridLayoutKey) -> (f64, f64, f64, f64, Color) {
    (
        layout.step_milli as f64 / 1000.0,
        layout.offset_x_milli as f64 / 1000.0,
        layout.offset_y_milli as f64 / 1000.0,
        layout.width as f64,
        Color::srgba_u8(99, 179, 255, 26),
    )
}

fn reconcile_grid(
    commands: &mut Commands,
    runtime: &mut BevyRuntimeState,
    existing_grid: &Query<Entity, With<PixelWorldGridVisual>>,
    width: f64,
    height: f64,
) {
    let next_layout = build_grid_layout(&runtime.camera, width, height);
    if runtime.grid_layout.as_ref() == Some(&next_layout) {
        return;
    }

    for entity in existing_grid.iter() {
        commands.entity(entity).despawn();
    }

    let (grid_step, offset_x, offset_y, layout_width, grid_color) = grid_geometry(&next_layout);
    let layout_height = next_layout.height as f64;

    let mut x = offset_x;
    while x <= layout_width {
        commands.spawn((
            sprite_for_rect(grid_color, 1.0, layout_height as f32),
            Transform::from_translation(to_bevy_translation(
                x,
                layout_height / 2.0,
                layout_width,
                layout_height,
                0.0,
            )),
            PixelWorldGridVisual,
        ));
        x += grid_step;
    }

    let mut y = offset_y;
    while y <= layout_height {
        commands.spawn((
            sprite_for_rect(grid_color, layout_width as f32, 1.0),
            Transform::from_translation(to_bevy_translation(
                layout_width / 2.0,
                y,
                layout_width,
                layout_height,
                0.0,
            )),
            PixelWorldGridVisual,
        ));
        y += grid_step;
    }

    runtime.grid_layout = Some(next_layout);
}

fn reconcile_locations(
    commands: &mut Commands,
    runtime: &mut BevyRuntimeState,
    width: f64,
    height: f64,
    animation_ms: f64,
) {
    let mut active_ids = HashSet::new();
    let Some(render_state) = runtime.render_state.as_ref() else {
        return;
    };
    let Some(world_bounds) = render_state.world_bounds.as_ref() else {
        for (_, entity) in runtime.location_entities.drain() {
            commands.entity(entity).despawn();
        }
        return;
    };

    for location in &render_state.locations {
        let Some((canvas_x, canvas_y)) =
            to_canvas_point(&location.pos, world_bounds, width, height, &runtime.camera)
        else {
            continue;
        };
        active_ids.insert(location.id.clone());
        let pulse = 1.0 + (0.08 * ((animation_ms / 360.0) + location.id.len() as f64).sin());
        let size = location.size_hint_px.unwrap_or(16.0) * pulse;
        let transform = Transform::from_translation(to_bevy_translation(
            canvas_x, canvas_y, width, height, 1.0,
        ));
        let sprite = sprite_for_square(Color::srgba_u8(110, 231, 183, 184), size as f32);

        if let Some(entity) = runtime.location_entities.get(&location.id).copied() {
            commands.entity(entity).insert((sprite, transform));
        } else {
            let entity = commands
                .spawn((
                    sprite,
                    transform,
                    PixelWorldLocationVisual {
                        id: location.id.clone(),
                    },
                ))
                .id();
            runtime
                .location_entities
                .insert(location.id.clone(), entity);
        }

        runtime.hit_regions.push(HitRegion {
            kind: "location",
            id: location.id.clone(),
            left: canvas_x - LOCATION_HIT_HALF_SIZE,
            top: canvas_y - LOCATION_HIT_HALF_SIZE,
            right: canvas_x + LOCATION_HIT_HALF_SIZE,
            bottom: canvas_y + LOCATION_HIT_HALF_SIZE,
        });
    }

    let stale_ids: Vec<String> = runtime
        .location_entities
        .keys()
        .filter(|id| !active_ids.contains(*id))
        .cloned()
        .collect();
    for id in stale_ids {
        if let Some(entity) = runtime.location_entities.remove(&id) {
            commands.entity(entity).despawn();
        }
    }
}

fn reconcile_agents(
    commands: &mut Commands,
    runtime: &mut BevyRuntimeState,
    width: f64,
    height: f64,
    animation_ms: f64,
) {
    let Some(render_state) = runtime.render_state.as_ref() else {
        for (_, entity) in runtime.agent_entities.drain() {
            commands.entity(entity).despawn();
        }
        return;
    };

    let mut active_ids = HashSet::new();
    for (index, agent) in render_state.agents.iter().enumerate() {
        active_ids.insert(agent.id.clone());
        let (canvas_x, canvas_y) = render_state
            .world_bounds
            .as_ref()
            .and_then(|world_bounds| {
                agent.pos.as_ref().and_then(|pos| {
                    to_canvas_point(pos, world_bounds, width, height, &runtime.camera)
                })
            })
            .unwrap_or_else(|| {
                fallback_point_for_entity(&agent.id, width, height, &runtime.camera)
            });
        let is_selected = render_state
            .selection
            .as_ref()
            .map(|selection| selection.kind == "agent" && selection.id == agent.id)
            .unwrap_or(false);
        let pulse = 1.0 + (0.12 * ((animation_ms / 240.0) + index as f64).sin());
        let base_size = if is_selected {
            agent.size_hint_px.unwrap_or(15.0).max(15.0)
        } else {
            agent.size_hint_px.unwrap_or(12.0)
        };
        let size = base_size * pulse;
        let color = if is_selected {
            Color::srgb_u8(251, 191, 36)
        } else {
            Color::srgb_u8(99, 179, 255)
        };
        let transform = Transform::from_translation(to_bevy_translation(
            canvas_x, canvas_y, width, height, 2.0,
        ));
        let sprite = sprite_for_square(color, size as f32);

        if let Some(entity) = runtime.agent_entities.get(&agent.id).copied() {
            commands.entity(entity).insert((sprite, transform));
        } else {
            let entity = commands
                .spawn((
                    sprite,
                    transform,
                    PixelWorldAgentVisual {
                        id: agent.id.clone(),
                    },
                ))
                .id();
            runtime.agent_entities.insert(agent.id.clone(), entity);
        }

        runtime.hit_regions.push(HitRegion {
            kind: "agent",
            id: agent.id.clone(),
            left: canvas_x - AGENT_HIT_HALF_SIZE,
            top: canvas_y - AGENT_HIT_HALF_SIZE,
            right: canvas_x + AGENT_HIT_HALF_SIZE,
            bottom: canvas_y + AGENT_HIT_HALF_SIZE,
        });
    }

    let stale_ids: Vec<String> = runtime
        .agent_entities
        .keys()
        .filter(|id| !active_ids.contains(*id))
        .cloned()
        .collect();
    for id in stale_ids {
        if let Some(entity) = runtime.agent_entities.remove(&id) {
            commands.entity(entity).despawn();
        }
    }
}

fn reconcile_links(
    commands: &mut Commands,
    runtime: &mut BevyRuntimeState,
    width: f64,
    height: f64,
) {
    let Some(render_state) = runtime.render_state.as_ref() else {
        for (_, entity) in runtime.link_entities.drain() {
            commands.entity(entity).despawn();
        }
        return;
    };
    let Some(world_bounds) = render_state.world_bounds.as_ref() else {
        for (_, entity) in runtime.link_entities.drain() {
            commands.entity(entity).despawn();
        }
        return;
    };

    let mut active_ids = HashSet::new();
    for link in &render_state.links {
        let Some((from_x, from_y)) =
            to_canvas_point(&link.from, world_bounds, width, height, &runtime.camera)
        else {
            continue;
        };
        let Some((to_x, to_y)) =
            to_canvas_point(&link.to, world_bounds, width, height, &runtime.camera)
        else {
            continue;
        };
        active_ids.insert(link.id.clone());
        let length = ((to_x - from_x).powi(2) + (to_y - from_y).powi(2))
            .sqrt()
            .max(4.0);
        let emphasis = clamp(link.emphasis.unwrap_or(0.7), 0.25, 1.0);
        let sprite = sprite_for_rect(
            Color::srgba(0.49, 0.83, 0.98, (0.18 + (emphasis * 0.34)) as f32),
            length as f32,
            (1.4 + (emphasis * 2.2)) as f32,
        );
        let transform = transform_for_line(from_x, from_y, to_x, to_y, width, height, 0.5);

        if let Some(entity) = runtime.link_entities.get(&link.id).copied() {
            commands.entity(entity).insert((sprite, transform));
        } else {
            let entity = commands
                .spawn((
                    sprite,
                    transform,
                    PixelWorldLinkVisual {
                        id: link.id.clone(),
                    },
                ))
                .id();
            runtime.link_entities.insert(link.id.clone(), entity);
        }
    }

    let stale_ids: Vec<String> = runtime
        .link_entities
        .keys()
        .filter(|id| !active_ids.contains(*id))
        .cloned()
        .collect();
    for id in stale_ids {
        if let Some(entity) = runtime.link_entities.remove(&id) {
            commands.entity(entity).despawn();
        }
    }
}

fn reconcile_hotspots(
    commands: &mut Commands,
    runtime: &mut BevyRuntimeState,
    width: f64,
    height: f64,
    animation_ms: f64,
) {
    let Some(render_state) = runtime.render_state.as_ref() else {
        for (_, entity) in runtime.hotspot_entities.drain() {
            commands.entity(entity).despawn();
        }
        return;
    };
    let Some(world_bounds) = render_state.world_bounds.as_ref() else {
        for (_, entity) in runtime.hotspot_entities.drain() {
            commands.entity(entity).despawn();
        }
        return;
    };

    let mut active_ids = HashSet::new();
    for (index, hotspot) in render_state.visual_hotspots.iter().enumerate() {
        let Some((canvas_x, canvas_y)) =
            to_canvas_point(&hotspot.pos, world_bounds, width, height, &runtime.camera)
        else {
            continue;
        };
        active_ids.insert(hotspot.id.clone());
        let emphasis = clamp(hotspot.emphasis.unwrap_or(0.7), 0.35, 1.0);
        let pulse = 1.0 + (0.1 * ((animation_ms / 280.0) + index as f64).sin());
        let size = hotspot.size_hint_px.unwrap_or(10.0) * pulse;
        let color = match hotspot.kind.as_str() {
            "blocker" => Color::srgba_u8(249, 115, 22, 210),
            "goal" => Color::srgba_u8(250, 204, 21, 196),
            _ => Color::srgba(0.56, 0.84, 1.0, (0.28 + (emphasis * 0.48)) as f32),
        };
        let mut transform = Transform::from_translation(to_bevy_translation(
            canvas_x, canvas_y, width, height, 1.5,
        ));
        transform.rotation = Quat::from_rotation_z(std::f32::consts::FRAC_PI_4);
        let sprite = sprite_for_square(color, size as f32);

        if let Some(entity) = runtime.hotspot_entities.get(&hotspot.id).copied() {
            commands.entity(entity).insert((sprite, transform));
        } else {
            let entity = commands
                .spawn((
                    sprite,
                    transform,
                    PixelWorldHotspotVisual {
                        id: hotspot.id.clone(),
                    },
                ))
                .id();
            runtime.hotspot_entities.insert(hotspot.id.clone(), entity);
        }
    }

    let stale_ids: Vec<String> = runtime
        .hotspot_entities
        .keys()
        .filter(|id| !active_ids.contains(*id))
        .cloned()
        .collect();
    for id in stale_ids {
        if let Some(entity) = runtime.hotspot_entities.remove(&id) {
            commands.entity(entity).despawn();
        }
    }
}

fn clear_runtime_visuals(commands: &mut Commands, runtime: &mut BevyRuntimeState) {
    for (_, entity) in runtime.location_entities.drain() {
        commands.entity(entity).despawn();
    }
    for (_, entity) in runtime.agent_entities.drain() {
        commands.entity(entity).despawn();
    }
    for (_, entity) in runtime.link_entities.drain() {
        commands.entity(entity).despawn();
    }
    for (_, entity) in runtime.hotspot_entities.drain() {
        commands.entity(entity).despawn();
    }
    runtime.grid_layout = None;
    runtime.hit_regions.clear();
    runtime.hover_key = None;
}

pub(crate) fn render_scene(
    mut commands: Commands,
    mut runtime: ResMut<BevyRuntimeState>,
    windows: Query<&Window, With<PrimaryWindow>>,
    current_grid: Query<Entity, With<PixelWorldGridVisual>>,
    location_visuals: Query<(Entity, &PixelWorldLocationVisual)>,
    agent_visuals: Query<(Entity, &PixelWorldAgentVisual)>,
    link_visuals: Query<(Entity, &PixelWorldLinkVisual)>,
    hotspot_visuals: Query<(Entity, &PixelWorldHotspotVisual)>,
    time: Res<Time>,
) {
    if !runtime.mounted {
        clear_runtime_visuals(&mut commands, &mut runtime);
        for entity in current_grid.iter() {
            commands.entity(entity).despawn();
        }
        return;
    }

    for (entity, visual) in location_visuals.iter() {
        runtime
            .location_entities
            .entry(visual.id.clone())
            .or_insert(entity);
    }
    for (entity, visual) in agent_visuals.iter() {
        runtime
            .agent_entities
            .entry(visual.id.clone())
            .or_insert(entity);
    }
    for (entity, visual) in link_visuals.iter() {
        runtime
            .link_entities
            .entry(visual.id.clone())
            .or_insert(entity);
    }
    for (entity, visual) in hotspot_visuals.iter() {
        runtime
            .hotspot_entities
            .entry(visual.id.clone())
            .or_insert(entity);
    }

    let Ok(window) = windows.single() else {
        return;
    };
    let Some(_) = runtime.render_state.as_ref() else {
        clear_runtime_visuals(&mut commands, &mut runtime);
        for entity in current_grid.iter() {
            commands.entity(entity).despawn();
        }
        return;
    };

    let width = window.width() as f64;
    let height = window.height() as f64;
    let animation_ms = time.elapsed_secs_f64() * 1000.0;
    runtime.hit_regions.clear();

    maybe_auto_fit_camera(&mut runtime, width, height);
    reconcile_grid(&mut commands, &mut runtime, &current_grid, width, height);
    reconcile_links(&mut commands, &mut runtime, width, height);
    reconcile_locations(&mut commands, &mut runtime, width, height, animation_ms);
    reconcile_agents(&mut commands, &mut runtime, width, height, animation_ms);
    reconcile_hotspots(&mut commands, &mut runtime, width, height, animation_ms);
}
