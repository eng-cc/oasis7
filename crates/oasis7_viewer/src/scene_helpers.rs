use super::*;
use oasis7::simulator::MaterialKind;
use oasis7::simulator::{
    chunk_bounds, chunk_coords, AgentKinematics, ChunkCoord, ChunkState, FragmentResourceBudget,
    ModuleVisualAnchor, ModuleVisualEntity, PowerEvent, ResourceOwner, SpaceConfig, WorldEventKind,
    CHUNK_SIZE_X_CM, CHUNK_SIZE_Y_CM,
};

const FACILITY_MARKER_LATERAL_OFFSET: f32 = 0.9;
const FACILITY_MARKER_VERTICAL_OFFSET: f32 = 0.45;
const ASSET_MARKER_VERTICAL_OFFSET: f32 = 1.1;
const ASSET_MARKER_RING_RADIUS: f32 = 0.45;
const MODULE_VISUAL_VERTICAL_OFFSET: f32 = 1.4;
const MODULE_VISUAL_RING_RADIUS: f32 = 0.7;
const LOCATION_DEPLETION_MIN_RADIUS_FACTOR: f32 = 0.24;
const AGENT_HEIGHT_MIN_M: f32 = 0.25;
const AGENT_HEIGHT_MAX_M: f32 = 4.0;
const AGENT_BODY_RADIUS_RATIO: f32 = 0.22;
const AGENT_BODY_LENGTH_RATIO: f32 = 0.56;
const AGENT_MODULE_MARKER_MAX: usize = 16;
const AGENT_MODULE_MARKERS_PER_RING: usize = 8;
const AGENT_MODULE_RING_BASE_MULTIPLIER: f32 = 3.05;
const AGENT_MODULE_RING_GAP_RATIO: f32 = 1.36;
const AGENT_MODULE_MARKER_WIDTH_RATIO: f32 = 0.96;
const AGENT_MODULE_MARKER_HEIGHT_RATIO: f32 = 1.08;
const AGENT_MODULE_MARKER_DEPTH_RATIO: f32 = 0.82;
const AGENT_MODULE_MARKER_MIN_WIDTH: f32 = 0.28;
const AGENT_MODULE_MARKER_MIN_HEIGHT: f32 = 0.34;
const AGENT_MODULE_MARKER_MIN_DEPTH: f32 = 0.24;
const AGENT_MODULE_MARKER_WORLD_MIN_WIDTH: f32 = 0.36;
const AGENT_MODULE_MARKER_WORLD_MIN_HEIGHT: f32 = 0.44;
const AGENT_MODULE_MARKER_WORLD_MIN_DEPTH: f32 = 0.32;
const AGENT_DIRECTION_VECTOR_EPSILON: f32 = 1e-6;
const AGENT_DIRECTION_INDICATOR_MIN_LENGTH: f32 = 0.01;
const AGENT_DIRECTION_INDICATOR_MIN_WIDTH: f32 = 0.005;
const AGENT_SPEED_EFFECT_MIN_SCALE: f32 = 1.05;
const AGENT_SPEED_EFFECT_MAX_SCALE: f32 = 2.4;
const AGENT_SPEED_EFFECT_MIN_THICKNESS: f32 = 0.004;
const AGENT_TRAIL_MIN_LENGTH: f32 = 0.02;
const AGENT_TRAIL_MIN_THICKNESS: f32 = 0.003;
const AGENT_SPEED_REFERENCE_CM_PER_TICK: f32 = 200_000.0;
const AGENT_MODULE_LAYOUT_PRIMARY_SLOTS: [(i32, i32, i32); 16] = [
    (0, 4, 2),
    (0, 3, 2),
    (-1, 3, 2),
    (1, 3, 2),
    (-1, 2, 2),
    (1, 2, 2),
    (-2, 2, 1),
    (2, 2, 1),
    (-2, 1, 1),
    (2, 1, 1),
    (-1, 1, 1),
    (1, 1, 1),
    (-1, 0, 1),
    (1, 0, 1),
    (-1, -1, 1),
    (1, -1, 1),
];

#[derive(Component)]
pub(super) struct AgentMarker {
    pub id: String,
    pub module_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LocationDamageTier {
    Intact,
    Light,
    Heavy,
    Severe,
}

#[derive(Component)]
pub(super) struct LocationMarker {
    pub id: String,
    pub name: String,
    pub material: MaterialKind,
    pub radiation_emission_per_tick: i64,
}

#[derive(Component)]
pub(super) struct AssetMarker {
    pub id: String,
}

#[derive(Component)]
pub(super) struct PowerPlantMarker {
    pub id: String,
}

#[derive(Component)]
pub(super) struct ChunkMarker {
    pub id: String,
    pub state: String,
    pub min_x: f32,
    pub max_x: f32,
    pub min_z: f32,
    pub max_z: f32,
    pub pick_y: f32,
}

#[derive(Component)]
pub(super) struct TwoDMapMarker;

#[derive(Component)]
pub(super) struct DetailZoomEntity;

pub(super) fn attach_to_scene_root(commands: &mut Commands, scene: &Viewer3dScene, entity: Entity) {
    if let Some(root) = scene.root_entity {
        commands.entity(root).add_child(entity);
    }
}

pub(super) fn rebuild_scene_from_snapshot(
    commands: &mut Commands,
    config: &Viewer3dConfig,
    assets: &Viewer3dAssets,
    scene: &mut Viewer3dScene,
    snapshot: &WorldSnapshot,
) {
    for entity in scene
        .agent_entities
        .values()
        .chain(scene.location_entities.values())
        .chain(scene.asset_entities.values())
        .chain(scene.module_visual_entities.values())
        .chain(scene.power_plant_entities.values())
        .chain(
            scene
                .chunk_line_entities
                .values()
                .flat_map(|items| items.iter()),
        )
        .chain(scene.background_entities.iter())
        .chain(scene.heat_overlay_entities.iter())
        .chain(scene.flow_overlay_entities.iter())
    {
        commands.entity(*entity).despawn();
    }

    scene.agent_entities.clear();
    scene.agent_positions.clear();
    scene.agent_heights_cm.clear();
    scene.agent_location_ids.clear();
    scene.agent_module_counts.clear();
    scene.agent_kinematics.clear();
    scene.location_entities.clear();
    scene.asset_entities.clear();
    scene.module_visual_entities.clear();
    scene.power_plant_entities.clear();
    scene.chunk_entities.clear();
    scene.chunk_line_entities.clear();
    scene.location_positions.clear();
    scene.location_radii_cm.clear();
    scene.background_entities.clear();
    scene.heat_overlay_entities.clear();
    scene.flow_overlay_entities.clear();
    scene.floating_origin_offset = Vec3::ZERO;

    let origin = space_origin(&snapshot.config.space);
    scene.origin = Some(origin);
    scene.space = Some(snapshot.config.space.clone());
    spawn_world_background(commands, config, assets, scene, snapshot);

    for (location_id, location) in snapshot.model.locations.iter() {
        let visual_radius_cm = location_visual_radius_cm(
            location.profile.radius_cm,
            location.fragment_budget.as_ref(),
        );
        spawn_location_entity_with_radiation(
            commands,
            config,
            assets,
            scene,
            origin,
            location_id,
            &location.name,
            location.pos,
            location.profile.material,
            visual_radius_cm,
            location.profile.radiation_emission_per_tick,
            location.fragment_budget.as_ref(),
        );

        if let (Some(fragment_profile), Some(entity)) = (
            location.fragment_profile.as_ref(),
            scene.location_entities.get(location_id).copied(),
        ) {
            commands.entity(entity).with_children(|parent| {
                location_fragment_render::spawn_location_fragment_elements(
                    parent,
                    assets,
                    location_id,
                    visual_radius_cm,
                    fragment_profile,
                );
            });
        }
    }

    let module_counts = agent_module_counts_in_snapshot(snapshot);
    for (agent_id, agent) in snapshot.model.agents.iter() {
        let module_count = module_counts
            .get(agent_id.as_str())
            .copied()
            .unwrap_or_else(default_agent_module_count_estimate);
        spawn_agent_entity(
            commands,
            config,
            assets,
            scene,
            origin,
            agent_id,
            Some(agent.location_id.as_str()),
            agent.pos,
            agent.body.height_cm,
            module_count,
            Some(&agent.kinematics),
        );
    }

    for (facility_id, plant) in snapshot.model.power_plants.iter() {
        if let Some(location) = snapshot.model.locations.get(&plant.location_id) {
            spawn_power_plant_entity(
                commands,
                config,
                assets,
                scene,
                origin,
                facility_id,
                plant.location_id.as_str(),
                location.pos,
            );
        }
    }

    for (asset_id, asset) in snapshot.model.assets.iter() {
        if let Some(anchor) = owner_anchor_pos(snapshot, &asset.owner) {
            spawn_asset_entity(
                commands,
                config,
                assets,
                scene,
                origin,
                asset_id,
                anchor,
                asset.quantity,
                Some(&asset.kind),
            );
        }
    }

    for module_entity in snapshot.model.module_visual_entities.values() {
        if let Some(anchor) = module_visual_anchor_pos_in_snapshot(snapshot, &module_entity.anchor)
        {
            spawn_module_visual_entity(
                commands,
                config,
                assets,
                scene,
                origin,
                module_entity,
                anchor,
            );
        }
    }

    for coord in chunk_coords(&snapshot.config.space) {
        let state = snapshot
            .model
            .chunks
            .get(&coord)
            .copied()
            .unwrap_or(ChunkState::Unexplored);
        spawn_chunk_entity(
            commands,
            config,
            assets,
            scene,
            origin,
            coord,
            state,
            &snapshot.config.space,
        );
    }
}

pub(super) fn apply_events_to_scene(
    commands: &mut Commands,
    config: &Viewer3dConfig,
    assets: &Viewer3dAssets,
    scene: &mut Viewer3dScene,
    snapshot: &WorldSnapshot,
    _snapshot_time: u64,
    events: &[WorldEvent],
) {
    let Some(origin) = scene.origin else {
        return;
    };
    let Some(space) = scene.space.clone() else {
        return;
    };

    let mut last_event_id = scene.last_event_id;
    let mut processed = false;

    for event in events {
        if let Some(last_id) = last_event_id {
            if event.id <= last_id {
                continue;
            }
        }

        match &event.kind {
            WorldEventKind::LocationRegistered {
                location_id,
                name,
                pos,
                profile,
            } => {
                spawn_location_entity_with_radiation(
                    commands,
                    config,
                    assets,
                    scene,
                    origin,
                    location_id,
                    name,
                    *pos,
                    profile.material,
                    profile.radius_cm,
                    profile.radiation_emission_per_tick,
                    None,
                );
            }
            WorldEventKind::AgentRegistered { agent_id, pos, .. } => {
                let snapshot_agent = snapshot.model.agents.get(agent_id);
                let height_cm = snapshot_agent
                    .map(|agent| agent.body.height_cm)
                    .or_else(|| scene.agent_heights_cm.get(agent_id).copied())
                    .unwrap_or(agent_height_cm(None));
                let location_id = snapshot_agent
                    .map(|agent| agent.location_id.clone())
                    .or_else(|| scene.agent_location_ids.get(agent_id.as_str()).cloned());
                let module_count = scene
                    .agent_module_counts
                    .get(agent_id.as_str())
                    .copied()
                    .unwrap_or(0);
                spawn_agent_entity(
                    commands,
                    config,
                    assets,
                    scene,
                    origin,
                    agent_id,
                    location_id.as_deref(),
                    *pos,
                    height_cm,
                    module_count,
                    snapshot_agent.map(|agent| &agent.kinematics),
                );
            }
            WorldEventKind::AgentMoved { agent_id, to, .. } => {
                if let Some(agent) = snapshot.model.agents.get(agent_id) {
                    let height_cm = scene
                        .agent_heights_cm
                        .get(agent_id)
                        .copied()
                        .unwrap_or(agent.body.height_cm);
                    scene
                        .agent_location_ids
                        .insert(agent_id.to_string(), to.to_string());
                    spawn_agent_entity(
                        commands,
                        config,
                        assets,
                        scene,
                        origin,
                        agent_id,
                        Some(agent.location_id.as_str()),
                        agent.pos,
                        height_cm,
                        scene
                            .agent_module_counts
                            .get(agent_id.as_str())
                            .copied()
                            .unwrap_or(0),
                        Some(&agent.kinematics),
                    );
                }
            }
            WorldEventKind::ModuleVisualEntityUpserted { entity } => {
                if let Some(anchor) = module_visual_anchor_pos_in_scene(scene, &entity.anchor) {
                    spawn_module_visual_entity(
                        commands, config, assets, scene, origin, entity, anchor,
                    );
                }
            }
            WorldEventKind::ModuleVisualEntityRemoved { entity_id } => {
                if let Some(entity) = scene.module_visual_entities.remove(entity_id.as_str()) {
                    commands.entity(entity).despawn();
                }
            }
            WorldEventKind::ChunkGenerated { coord, .. } => {
                spawn_chunk_entity(
                    commands,
                    config,
                    assets,
                    scene,
                    origin,
                    *coord,
                    ChunkState::Generated,
                    &space,
                );
            }
            WorldEventKind::Power(power_event) => match power_event {
                PowerEvent::PowerPlantRegistered { plant } => {
                    if let Some(pos) = scene.location_positions.get(&plant.location_id) {
                        spawn_power_plant_entity(
                            commands,
                            config,
                            assets,
                            scene,
                            origin,
                            &plant.id,
                            &plant.location_id,
                            *pos,
                        );
                    }
                }
                _ => {}
            },
            _ => {}
        }

        last_event_id = Some(event.id);
        processed = true;
    }

    if processed {
        scene.last_event_id = last_event_id;
    }
}

pub(super) fn spawn_world_background(
    commands: &mut Commands,
    config: &Viewer3dConfig,
    assets: &Viewer3dAssets,
    scene: &mut Viewer3dScene,
    snapshot: &WorldSnapshot,
) {
    let space = &snapshot.config.space;
    let world_width = (space.width_cm as f32 * config.effective_cm_to_unit()).max(WORLD_MIN_AXIS);
    let world_depth = (space.depth_cm as f32 * config.effective_cm_to_unit()).max(WORLD_MIN_AXIS);
    let world_height = (space.height_cm as f32 * config.effective_cm_to_unit()).max(WORLD_MIN_AXIS);

    let floor_entity = commands
        .spawn((
            Mesh3d(assets.world_box_mesh.clone()),
            MeshMaterial3d(assets.world_floor_material.clone()),
            Transform::from_translation(Vec3::new(
                0.0,
                -world_height * 0.5 - WORLD_FLOOR_THICKNESS * 0.5,
                0.0,
            ))
            .with_scale(Vec3::new(world_width, WORLD_FLOOR_THICKNESS, world_depth)),
            WorldFloorSurface,
            Name::new("world:floor"),
            BaseScale(Vec3::new(world_width, WORLD_FLOOR_THICKNESS, world_depth)),
        ))
        .id();
    attach_to_scene_root(commands, scene, floor_entity);
    scene.background_entities.push(floor_entity);

    let bounds_entity = commands
        .spawn((
            Mesh3d(assets.world_box_mesh.clone()),
            MeshMaterial3d(assets.world_bounds_material.clone()),
            Transform::from_scale(Vec3::new(world_width, world_height, world_depth)),
            WorldBoundsSurface,
            Name::new("world:bounds"),
            BaseScale(Vec3::new(world_width, world_height, world_depth)),
        ))
        .id();
    attach_to_scene_root(commands, scene, bounds_entity);
    scene.background_entities.push(bounds_entity);

    spawn_world_grid(
        commands,
        assets,
        scene,
        space,
        config.effective_cm_to_unit(),
        world_height,
    );
}

pub(super) fn spawn_world_grid(
    commands: &mut Commands,
    assets: &Viewer3dAssets,
    scene: &mut Viewer3dScene,
    space: &SpaceConfig,
    cm_to_unit: f32,
    world_height: f32,
) {
    let thickness = grid_line_thickness(GridLineKind::World, ViewerCameraMode::TwoD);
    let y = -world_height * 0.5 + thickness * 0.5;

    let mut x_idx: usize = 0;
    for x_cm in grid_positions_cm(space.width_cm, ChunkAxis::X) {
        let x = (x_cm as f32 - space.width_cm as f32 * 0.5) * cm_to_unit;
        let world_depth = (space.depth_cm as f32 * cm_to_unit).max(WORLD_MIN_AXIS);
        let x_line = commands
            .spawn((
                Mesh3d(assets.world_box_mesh.clone()),
                MeshMaterial3d(assets.world_grid_material.clone()),
                Transform::from_translation(Vec3::new(x, y, 0.0)).with_scale(grid_line_scale(
                    GridLineAxis::AlongZ,
                    world_depth,
                    thickness,
                )),
                Name::new(format!("world:grid:x:{x_idx}")),
                BaseScale(grid_line_scale(
                    GridLineAxis::AlongZ,
                    world_depth,
                    thickness,
                )),
                GridLineVisual {
                    kind: GridLineKind::World,
                    axis: GridLineAxis::AlongZ,
                    span: world_depth,
                },
            ))
            .id();
        attach_to_scene_root(commands, scene, x_line);
        scene.background_entities.push(x_line);
        x_idx += 1;
    }

    let mut z_idx: usize = 0;
    for z_cm in grid_positions_cm(space.depth_cm, ChunkAxis::Z) {
        let z = (z_cm as f32 - space.depth_cm as f32 * 0.5) * cm_to_unit;
        let world_width = (space.width_cm as f32 * cm_to_unit).max(WORLD_MIN_AXIS);
        let z_line = commands
            .spawn((
                Mesh3d(assets.world_box_mesh.clone()),
                MeshMaterial3d(assets.world_grid_material.clone()),
                Transform::from_translation(Vec3::new(0.0, y, z)).with_scale(grid_line_scale(
                    GridLineAxis::AlongX,
                    world_width,
                    thickness,
                )),
                Name::new(format!("world:grid:z:{z_idx}")),
                BaseScale(grid_line_scale(
                    GridLineAxis::AlongX,
                    world_width,
                    thickness,
                )),
                GridLineVisual {
                    kind: GridLineKind::World,
                    axis: GridLineAxis::AlongX,
                    span: world_width,
                },
            ))
            .id();
        attach_to_scene_root(commands, scene, z_line);
        scene.background_entities.push(z_line);
        z_idx += 1;
    }
}

fn grid_positions_cm(axis_cm: i64, axis: ChunkAxis) -> Vec<i64> {
    if axis_cm <= 0 {
        return vec![0];
    }
    let step_cm = grid_step_cm_for_axis(axis);
    let mut values = vec![0];
    let mut cursor = 0_i64;
    while cursor < axis_cm {
        cursor = (cursor + step_cm).min(axis_cm);
        if values.last().copied().unwrap_or(-1) != cursor {
            values.push(cursor);
        }
    }
    values
}

fn grid_step_cm_for_axis(axis: ChunkAxis) -> i64 {
    match axis {
        ChunkAxis::X => CHUNK_SIZE_X_CM,
        ChunkAxis::Z => CHUNK_SIZE_Y_CM,
    }
}

#[derive(Clone, Copy)]
enum ChunkAxis {
    X,
    Z,
}

pub(super) fn spawn_location_entity_with_radiation(
    commands: &mut Commands,
    config: &Viewer3dConfig,
    assets: &Viewer3dAssets,
    scene: &mut Viewer3dScene,
    origin: GeoPos,
    location_id: &str,
    name: &str,
    pos: GeoPos,
    material: MaterialKind,
    radius_cm: i64,
    radiation_emission_per_tick: i64,
    fragment_budget: Option<&FragmentResourceBudget>,
) {
    let damage_tier = location_damage_tier(fragment_budget);
    scene
        .location_positions
        .insert(location_id.to_string(), pos);
    scene
        .location_radii_cm
        .insert(location_id.to_string(), radius_cm.max(1));

    if !config.show_locations {
        if let Some(entity) = scene.location_entities.remove(location_id) {
            commands.entity(entity).despawn();
        }
        return;
    }

    let radius_world_units = location_render_radius_units(radius_cm, config.effective_cm_to_unit());
    let marker_scale = Vec3::splat(radius_world_units);
    let translation = geo_to_vec3(pos, origin, config.effective_cm_to_unit());
    if let Some(entity) = scene.location_entities.get(location_id) {
        commands.entity(*entity).insert((
            Transform::from_translation(translation).with_scale(marker_scale),
            Visibility::Visible,
            LocationMarker {
                id: location_id.to_string(),
                name: name.to_string(),
                material,
                radiation_emission_per_tick,
            },
            BaseScale(marker_scale),
        ));
        commands
            .entity(*entity)
            .remove::<(Mesh3d, MeshMaterial3d<StandardMaterial>)>();
        commands.entity(*entity).despawn_children();
        commands.entity(*entity).with_children(|parent| {
            spawn_location_shell_details(
                parent,
                config,
                assets,
                location_id,
                material,
                radiation_emission_per_tick,
                damage_tier,
            );
        });
        return;
    }

    let entity = commands
        .spawn((
            Transform::from_translation(translation).with_scale(marker_scale),
            Visibility::Visible,
            Name::new(format!("location:anchor:{location_id}:{name}")),
            LocationMarker {
                id: location_id.to_string(),
                name: name.to_string(),
                material,
                radiation_emission_per_tick,
            },
            BaseScale(marker_scale),
        ))
        .id();
    attach_to_scene_root(commands, scene, entity);
    commands.entity(entity).with_children(|parent| {
        spawn_location_shell_details(
            parent,
            config,
            assets,
            location_id,
            material,
            radiation_emission_per_tick,
            damage_tier,
        );
    });
    scene
        .location_entities
        .insert(location_id.to_string(), entity);
}

pub(super) fn spawn_agent_entity(
    commands: &mut Commands,
    config: &Viewer3dConfig,
    assets: &Viewer3dAssets,
    scene: &mut Viewer3dScene,
    origin: GeoPos,
    agent_id: &str,
    location_id: Option<&str>,
    pos: GeoPos,
    height_cm: i64,
    module_count: usize,
    kinematics: Option<&AgentKinematics>,
) {
    scene.agent_positions.insert(agent_id.to_string(), pos);
    scene
        .agent_heights_cm
        .insert(agent_id.to_string(), height_cm.max(1));
    scene
        .agent_module_counts
        .insert(agent_id.to_string(), module_count);
    scene.agent_kinematics.insert(
        agent_id.to_string(),
        kinematics.cloned().unwrap_or_default(),
    );
    if let Some(location_id) = location_id {
        scene
            .agent_location_ids
            .insert(agent_id.to_string(), location_id.to_string());
    }

    if !config.show_agents {
        return;
    }

    let cm_to_unit = config.effective_cm_to_unit();
    let body_scale = agent_body_scale(height_cm, cm_to_unit);
    let marker_scale = agent_module_marker_scale(height_cm, cm_to_unit);
    let marker_world_scale = agent_module_marker_world_scale(marker_scale, cm_to_unit, height_cm);
    let module_markers = agent_module_marker_transforms(height_cm, module_count, cm_to_unit);
    let translation = agent_translation_for_render(scene, config, origin, agent_id, pos, height_cm);
    let motion_visual = resolve_agent_motion_visual(scene, config, origin, pos, kinematics);
    if let Some(entity) = scene.agent_entities.get(agent_id) {
        commands.entity(*entity).insert((
            Transform::from_translation(translation),
            Visibility::Visible,
            AgentMarker {
                id: agent_id.to_string(),
                module_count,
            },
            BaseScale(Vec3::ONE),
        ));
        commands.entity(*entity).despawn_children();
        commands.entity(*entity).with_children(|parent| {
            parent.spawn((
                Mesh3d(assets.agent_mesh.clone()),
                MeshMaterial3d(assets.agent_material.clone()),
                Transform::from_scale(body_scale),
                Name::new(format!("agent:body:{agent_id}")),
                DetailZoomEntity,
            ));
            spawn_label(
                parent,
                assets,
                agent_id.to_string(),
                agent_label_offset(height_cm, cm_to_unit),
                format!("label:agent:{agent_id}"),
            );
            spawn_agent_two_d_map_marker(
                parent,
                assets,
                agent_id,
                height_cm,
                module_count,
                cm_to_unit,
            );
            spawn_agent_motion_feedback(
                parent,
                config,
                assets,
                agent_id,
                body_scale,
                motion_visual,
            );
            for (marker_idx, marker_translation) in module_markers.iter().enumerate() {
                parent.spawn((
                    Mesh3d(assets.agent_module_marker_mesh.clone()),
                    MeshMaterial3d(assets.agent_module_marker_material.clone()),
                    Transform::from_translation(*marker_translation).with_scale(marker_world_scale),
                    Name::new(format!("agent:module_marker:{agent_id}:{marker_idx}")),
                    DetailZoomEntity,
                ));
            }
        });
        return;
    }

    let entity = commands
        .spawn((
            Transform::from_translation(translation),
            Visibility::Visible,
            Name::new(format!("agent:{agent_id}")),
            AgentMarker {
                id: agent_id.to_string(),
                module_count,
            },
            BaseScale(Vec3::ONE),
        ))
        .id();
    attach_to_scene_root(commands, scene, entity);
    commands.entity(entity).with_children(|parent| {
        parent.spawn((
            Mesh3d(assets.agent_mesh.clone()),
            MeshMaterial3d(assets.agent_material.clone()),
            Transform::from_scale(body_scale),
            Name::new(format!("agent:body:{agent_id}")),
            DetailZoomEntity,
        ));
        spawn_label(
            parent,
            assets,
            agent_id.to_string(),
            agent_label_offset(height_cm, cm_to_unit),
            format!("label:agent:{agent_id}"),
        );
        spawn_agent_two_d_map_marker(
            parent,
            assets,
            agent_id,
            height_cm,
            module_count,
            cm_to_unit,
        );
        spawn_agent_motion_feedback(parent, config, assets, agent_id, body_scale, motion_visual);
        for (marker_idx, marker_translation) in module_markers.iter().enumerate() {
            parent.spawn((
                Mesh3d(assets.agent_module_marker_mesh.clone()),
                MeshMaterial3d(assets.agent_module_marker_material.clone()),
                Transform::from_translation(*marker_translation).with_scale(marker_world_scale),
                Name::new(format!("agent:module_marker:{agent_id}:{marker_idx}")),
                DetailZoomEntity,
            ));
        }
    });
    scene.agent_entities.insert(agent_id.to_string(), entity);
}

#[path = "scene_helpers_agents.rs"]
mod scene_helpers_agents;
#[path = "scene_helpers_entities.rs"]
mod scene_helpers_entities;

use scene_helpers_agents::*;

pub(super) fn location_visual_radius_cm(
    radius_cm: i64,
    fragment_budget: Option<&FragmentResourceBudget>,
) -> i64 {
    scene_helpers_agents::location_visual_radius_cm(radius_cm, fragment_budget)
}

pub(super) fn location_damage_tier(
    fragment_budget: Option<&FragmentResourceBudget>,
) -> LocationDamageTier {
    scene_helpers_agents::location_damage_tier(fragment_budget)
}

pub(super) fn agent_module_counts_in_snapshot(
    snapshot: &WorldSnapshot,
) -> std::collections::HashMap<String, usize> {
    scene_helpers_agents::agent_module_counts_in_snapshot(snapshot)
}

pub(super) use scene_helpers_entities::spawn_power_plant_entity;
use scene_helpers_entities::{
    id_hash_fraction, module_visual_anchor_pos_in_scene, module_visual_anchor_pos_in_snapshot,
    spawn_agent_two_d_map_marker, spawn_asset_entity, spawn_location_shell_details,
    spawn_module_visual_entity,
};

pub(super) fn spawn_chunk_entity(
    commands: &mut Commands,
    config: &Viewer3dConfig,
    assets: &Viewer3dAssets,
    scene: &mut Viewer3dScene,
    origin: GeoPos,
    coord: ChunkCoord,
    state: ChunkState,
    space: &SpaceConfig,
) {
    scene_helpers_entities::spawn_chunk_entity(
        commands, config, assets, scene, origin, coord, state, space,
    );
}

pub(super) fn spawn_label(
    parent: &mut ChildSpawnerCommands,
    assets: &Viewer3dAssets,
    text: String,
    offset_y: f32,
    name: String,
) {
    parent.spawn((
        Text2d::new(text),
        TextFont {
            font: assets.label_font.clone(),
            font_size: LABEL_FONT_SIZE,
            ..default()
        },
        Transform::from_translation(Vec3::new(0.0, offset_y, 0.0))
            .with_scale(Vec3::splat(LABEL_SCALE)),
        TextColor(Color::srgb(0.9, 0.9, 0.9)),
        Name::new(name),
    ));
}

pub(super) fn space_origin(space: &SpaceConfig) -> GeoPos {
    GeoPos {
        x_cm: space.width_cm / 2,
        y_cm: space.depth_cm / 2,
        z_cm: space.height_cm / 2,
    }
}

pub(super) fn geo_to_vec3(pos: GeoPos, origin: GeoPos, cm_to_unit: f32) -> Vec3 {
    let scale = cm_to_unit as f64;
    Vec3::new(
        ((pos.x_cm - origin.x_cm) as f64 * scale) as f32,
        ((pos.z_cm - origin.z_cm) as f64 * scale) as f32,
        ((pos.y_cm - origin.y_cm) as f64 * scale) as f32,
    )
}

pub(super) fn ray_point_distance(ray: Ray3d, point: Vec3) -> Option<f32> {
    let direction = ray.direction.as_vec3();
    let to_point = point - ray.origin;
    let t = direction.dot(to_point);
    if t < 0.0 {
        return None;
    }
    let closest = ray.origin + direction * t;
    Some(closest.distance(point))
}

pub(super) fn apply_entity_highlight(
    transforms: &mut Query<(&mut Transform, Option<&BaseScale>)>,
    entity: Entity,
) {
    if let Ok((mut transform, base)) = transforms.get_mut(entity) {
        let base_scale = base.map(|scale| scale.0).unwrap_or(Vec3::ONE);
        transform.scale = base_scale * 1.6;
    }
}

pub(super) fn should_apply_scale_highlight(kind: SelectionKind) -> bool {
    !matches!(kind, SelectionKind::Fragment)
}

pub(super) fn reset_entity_scale(
    transforms: &mut Query<(&mut Transform, Option<&BaseScale>)>,
    entity: Entity,
) {
    if let Ok((mut transform, base)) = transforms.get_mut(entity) {
        let base_scale = base.map(|scale| scale.0).unwrap_or(Vec3::ONE);
        transform.scale = base_scale;
    }
}

#[cfg(test)]
#[path = "scene_helpers_depletion_tests.rs"]
mod depletion_tests;
