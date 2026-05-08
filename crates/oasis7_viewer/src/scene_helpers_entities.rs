use super::*;
use oasis7::simulator::{AssetKind, ResourceKind};

const TWO_D_AGENT_MARKER_MIN_RADIUS_WORLD: f32 = 0.00045;
const TWO_D_AGENT_MARKER_MIN_THICKNESS_WORLD: f32 = 0.00006;
const TWO_D_AGENT_MARKER_MAX_THICKNESS_WORLD: f32 = 0.0016;
const TWO_D_AGENT_MARKER_MIN_LIFT_M: f32 = 0.22;
const TWO_D_AGENT_MARKER_MAX_LIFT_M: f32 = 0.85;
const FACILITY_MARKER_MIN_SCALE_M: f32 = 4.8;
const POWER_PLANT_SCALE_RATIO_TO_LOCATION_RADIUS: f32 = 0.88;

pub(super) fn spawn_agent_two_d_map_marker(
    parent: &mut ChildSpawnerCommands,
    assets: &Viewer3dAssets,
    agent_id: &str,
    height_cm: i64,
    module_count: usize,
    cm_to_unit: f32,
) {
    let (world_radius, thickness, y) = two_d_agent_marker_profile(height_cm, cm_to_unit);

    let base_scale = Vec3::new(world_radius * 2.0, thickness, world_radius * 2.0);
    parent.spawn((
        Mesh3d(assets.agent_module_marker_mesh.clone()),
        MeshMaterial3d(assets.agent_module_marker_material.clone()),
        Transform::from_translation(Vec3::new(0.0, y, 0.0)).with_scale(base_scale),
        BaseScale(base_scale),
        Visibility::Visible,
        Name::new(format!("map2d:agent:plate:{agent_id}")),
        SceneZoomLayer::TwoDOverviewMarker,
        TwoDOverviewMarkerTag,
    ));

    let module_ratio =
        module_count.min(AGENT_MODULE_MARKER_MAX) as f32 / AGENT_MODULE_MARKER_MAX as f32;
    if module_ratio > 0.0 {
        let outer_radius = world_radius * (1.18 + module_ratio * 0.52);
        let outer_scale = Vec3::new(outer_radius * 2.0, thickness * 0.55, outer_radius * 2.0);
        parent.spawn((
            Mesh3d(assets.agent_module_marker_mesh.clone()),
            MeshMaterial3d(assets.chunk_generated_material.clone()),
            Transform::from_translation(Vec3::new(0.0, y - thickness * 0.45, 0.0))
                .with_scale(outer_scale),
            BaseScale(outer_scale),
            Visibility::Visible,
            Name::new(format!("map2d:agent:module_band:{agent_id}")),
            SceneZoomLayer::TwoDOverviewMarker,
            TwoDOverviewMarkerTag,
        ));
    }

    parent.spawn((
        Mesh3d(assets.location_mesh.clone()),
        MeshMaterial3d(assets.agent_material.clone()),
        Transform::from_translation(Vec3::new(0.0, y + thickness * 0.65, 0.0))
            .with_scale(Vec3::splat(world_radius * 0.68)),
        BaseScale(Vec3::splat(world_radius * 0.68)),
        Visibility::Visible,
        Name::new(format!("map2d:agent:center:{agent_id}")),
        SceneZoomLayer::TwoDOverviewMarker,
        TwoDOverviewMarkerTag,
    ));
}

fn two_d_agent_marker_profile(height_cm: i64, cm_to_unit: f32) -> (f32, f32, f32) {
    let agent_height_m = (agent_height_cm(Some(height_cm)) as f32 / 100.0)
        .clamp(AGENT_HEIGHT_MIN_M, AGENT_HEIGHT_MAX_M);
    let units_per_m = world_units_per_meter(cm_to_unit);
    let physical_radius = (agent_height_m * 0.62).clamp(0.38, 0.95) * units_per_m;
    let world_radius = physical_radius.max(TWO_D_AGENT_MARKER_MIN_RADIUS_WORLD);
    let thickness = (world_radius * 0.18).clamp(
        TWO_D_AGENT_MARKER_MIN_THICKNESS_WORLD,
        TWO_D_AGENT_MARKER_MAX_THICKNESS_WORLD,
    );
    let y = (agent_height_m * 0.35)
        .clamp(TWO_D_AGENT_MARKER_MIN_LIFT_M, TWO_D_AGENT_MARKER_MAX_LIFT_M)
        * units_per_m
        + thickness * 0.5;
    (world_radius, thickness, y)
}

pub(super) fn id_hash_fraction(id: &str) -> f32 {
    let hash = id.bytes().fold(0u32, |acc, value| {
        acc.wrapping_mul(31).wrapping_add(value as u32)
    });
    (hash % 1024) as f32 / 1024.0
}

fn asset_translation(base: Vec3, asset_id: &str) -> Vec3 {
    let angle = id_hash_fraction(asset_id) * std::f32::consts::TAU;
    let lateral = Vec3::new(angle.cos(), 0.0, angle.sin()) * ASSET_MARKER_RING_RADIUS;
    base + lateral + Vec3::Y * ASSET_MARKER_VERTICAL_OFFSET
}

fn facility_marker_scale_for_location(
    scene: &Viewer3dScene,
    location_id: &str,
    cm_to_unit: f32,
    ratio_to_location_radius: f32,
) -> f32 {
    let location_radius_cm = scene
        .location_radii_cm
        .get(location_id)
        .copied()
        .unwrap_or(600);
    let location_radius_units = location_render_radius_units(location_radius_cm, cm_to_unit);
    let min_scale_units = world_units_per_meter(cm_to_unit) * FACILITY_MARKER_MIN_SCALE_M;
    (location_radius_units * ratio_to_location_radius).max(min_scale_units)
}

pub(super) fn module_visual_anchor_pos_in_snapshot(
    snapshot: &WorldSnapshot,
    anchor: &ModuleVisualAnchor,
) -> Option<GeoPos> {
    match anchor {
        ModuleVisualAnchor::Agent { agent_id } => {
            snapshot.model.agents.get(agent_id).map(|agent| agent.pos)
        }
        ModuleVisualAnchor::Location { location_id } => snapshot
            .model
            .locations
            .get(location_id)
            .map(|location| location.pos),
        ModuleVisualAnchor::Absolute { pos } => Some(*pos),
    }
}

pub(super) fn module_visual_anchor_pos_in_scene(
    scene: &Viewer3dScene,
    anchor: &ModuleVisualAnchor,
) -> Option<GeoPos> {
    match anchor {
        ModuleVisualAnchor::Agent { agent_id } => scene.agent_positions.get(agent_id).copied(),
        ModuleVisualAnchor::Location { location_id } => {
            scene.location_positions.get(location_id).copied()
        }
        ModuleVisualAnchor::Absolute { pos } => Some(*pos),
    }
}

fn module_visual_translation(base: Vec3, module_id: &str, entity_id: &str) -> Vec3 {
    let hash_key = format!("{module_id}:{entity_id}");
    let angle = id_hash_fraction(hash_key.as_str()) * std::f32::consts::TAU;
    let lateral = Vec3::new(angle.cos(), 0.0, angle.sin()) * MODULE_VISUAL_RING_RADIUS;
    base + lateral + Vec3::Y * MODULE_VISUAL_VERTICAL_OFFSET
}

fn location_shell_ring_layers(config: &Viewer3dConfig) -> usize {
    match config.assets.geometry_tier {
        ViewerGeometryTier::Debug => 1,
        ViewerGeometryTier::Balanced => 2,
        ViewerGeometryTier::Cinematic => 3,
    }
}

fn location_shell_halo_layers(config: &Viewer3dConfig) -> usize {
    match config.assets.geometry_tier {
        ViewerGeometryTier::Debug => 1,
        ViewerGeometryTier::Balanced => 1,
        ViewerGeometryTier::Cinematic => 2,
    }
}

fn location_damage_overlay_layers(damage_tier: LocationDamageTier) -> usize {
    match damage_tier {
        LocationDamageTier::Intact => 0,
        LocationDamageTier::Light => 1,
        LocationDamageTier::Heavy => 2,
        LocationDamageTier::Severe => 3,
    }
}

fn location_core_vertical_scale(damage_tier: LocationDamageTier) -> f32 {
    match damage_tier {
        LocationDamageTier::Intact => 1.0,
        LocationDamageTier::Light => 0.92,
        LocationDamageTier::Heavy => 0.8,
        LocationDamageTier::Severe => 0.62,
    }
}

fn location_core_material(
    assets: &Viewer3dAssets,
    material: MaterialKind,
) -> Handle<StandardMaterial> {
    match material {
        MaterialKind::Silicate => assets.location_core_silicate_material.clone(),
        MaterialKind::Metal => assets.location_core_metal_material.clone(),
        MaterialKind::Ice => assets.location_core_ice_material.clone(),
        MaterialKind::Carbon => assets.chunk_exhausted_material.clone(),
        MaterialKind::Composite => assets.chunk_generated_material.clone(),
    }
}

pub(super) fn spawn_location_shell_details(
    parent: &mut ChildSpawnerCommands,
    config: &Viewer3dConfig,
    assets: &Viewer3dAssets,
    location_id: &str,
    material: MaterialKind,
    radiation_emission_per_tick: i64,
    damage_tier: LocationDamageTier,
) {
    if !config.assets.location_shell_enabled {
        return;
    }

    let effective_damage_tier = if config.visual.location_damage_visual {
        damage_tier
    } else {
        LocationDamageTier::Intact
    };
    let core_scale = Vec3::new(
        1.0,
        location_core_vertical_scale(effective_damage_tier),
        1.0,
    );
    parent.spawn((
        Mesh3d(assets.location_mesh.clone()),
        MeshMaterial3d(location_core_material(assets, material)),
        Transform::from_scale(core_scale),
        BaseScale(core_scale),
        Name::new(format!("location:detail:core:{location_id}")),
        SceneZoomLayer::Detail,
    ));

    for ring_idx in 0..location_shell_ring_layers(config) {
        let scale_factor = 1.12 + ring_idx as f32 * 0.18;
        let ring_scale = Vec3::new(scale_factor, 0.13 + ring_idx as f32 * 0.02, scale_factor);
        parent.spawn((
            Mesh3d(assets.world_box_mesh.clone()),
            MeshMaterial3d(assets.world_grid_material.clone()),
            Transform::from_translation(Vec3::new(0.0, ring_idx as f32 * 0.018, 0.0))
                .with_scale(ring_scale),
            BaseScale(ring_scale),
            Name::new(format!("location:detail:ring:{location_id}:{ring_idx}")),
            SceneZoomLayer::Detail,
        ));
    }

    if config.visual.location_radiation_glow && radiation_emission_per_tick > 0 {
        for halo_idx in 0..location_shell_halo_layers(config) {
            let halo_scale = Vec3::splat(1.24 + halo_idx as f32 * 0.18);
            parent.spawn((
                Mesh3d(assets.location_mesh.clone()),
                MeshMaterial3d(assets.location_halo_material.clone()),
                Transform::from_scale(halo_scale),
                BaseScale(halo_scale),
                Name::new(format!("location:detail:halo:{location_id}:{halo_idx}")),
                SceneZoomLayer::Detail,
            ));
        }
    }

    for overlay_idx in 0..location_damage_overlay_layers(effective_damage_tier) {
        let overlay_scale = Vec3::new(
            1.05 + overlay_idx as f32 * 0.14,
            (0.085 - overlay_idx as f32 * 0.015).max(0.03),
            1.05 + overlay_idx as f32 * 0.14,
        );
        parent.spawn((
            Mesh3d(assets.world_box_mesh.clone()),
            MeshMaterial3d(assets.chunk_exhausted_material.clone()),
            Transform::from_translation(Vec3::new(0.0, -0.01 - overlay_idx as f32 * 0.01, 0.0))
                .with_scale(overlay_scale),
            BaseScale(overlay_scale),
            Name::new(format!(
                "location:detail:damage:{location_id}:{overlay_idx}"
            )),
            SceneZoomLayer::Detail,
        ));
    }

    if material == MaterialKind::Carbon {
        for grain_idx in 0..2 {
            let grain_scale = Vec3::new(
                1.16 + grain_idx as f32 * 0.12,
                0.055 + grain_idx as f32 * 0.01,
                1.16 + grain_idx as f32 * 0.12,
            );
            parent.spawn((
                Mesh3d(assets.world_box_mesh.clone()),
                MeshMaterial3d(assets.chunk_exhausted_material.clone()),
                Transform::from_translation(Vec3::new(0.0, 0.02 + grain_idx as f32 * 0.02, 0.0))
                    .with_scale(grain_scale),
                BaseScale(grain_scale),
                Name::new(format!(
                    "location:detail:carbon:grain:{location_id}:{grain_idx}"
                )),
                SceneZoomLayer::Detail,
            ));
        }
    }

    if material == MaterialKind::Composite {
        for layer_idx in 0..2 {
            let layer_scale = Vec3::new(
                1.10 + layer_idx as f32 * 0.14,
                0.07 + layer_idx as f32 * 0.01,
                1.10 + layer_idx as f32 * 0.14,
            );
            let layer_material = if layer_idx % 2 == 0 {
                assets.chunk_generated_material.clone()
            } else {
                assets.chunk_unexplored_material.clone()
            };
            parent.spawn((
                Mesh3d(assets.world_box_mesh.clone()),
                MeshMaterial3d(layer_material),
                Transform::from_translation(Vec3::new(0.0, 0.01 + layer_idx as f32 * 0.02, 0.0))
                    .with_scale(layer_scale),
                BaseScale(layer_scale),
                Name::new(format!(
                    "location:detail:composite:layer:{location_id}:{layer_idx}"
                )),
                SceneZoomLayer::Detail,
            ));
        }
    }
}

pub(super) fn spawn_module_visual_entity(
    commands: &mut Commands,
    config: &Viewer3dConfig,
    assets: &Viewer3dAssets,
    scene: &mut Viewer3dScene,
    origin: GeoPos,
    module_entity: &ModuleVisualEntity,
    anchor_pos: GeoPos,
) {
    let translation = module_visual_translation(
        geo_to_vec3(anchor_pos, origin, config.effective_cm_to_unit()),
        module_entity.module_id.as_str(),
        module_entity.entity_id.as_str(),
    );

    if let Some(entity) = scene
        .module_visual_entities
        .remove(module_entity.entity_id.as_str())
    {
        commands.entity(entity).despawn();
    }

    let visual_id = module_entity.entity_id.clone();
    let visual_label = module_entity.resolved_label();
    let visual_name = format!(
        "module_visual:{}:{}:{}",
        module_entity.module_id, module_entity.kind, module_entity.entity_id
    );

    let entity = commands
        .spawn((
            Mesh3d(assets.asset_mesh.clone()),
            MeshMaterial3d(assets.asset_material.clone()),
            Transform::from_translation(translation).with_scale(Vec3::splat(0.9)),
            Name::new(visual_name),
            AssetMarker {
                id: visual_id.clone(),
            },
            BaseScale(Vec3::splat(0.9)),
        ))
        .id();
    attach_to_scene_root(commands, scene, entity);

    commands.entity(entity).with_children(|parent| {
        spawn_label(
            parent,
            assets,
            visual_label,
            AGENT_LABEL_OFFSET,
            format!("label:module_visual:{visual_id}"),
        );
    });

    scene.module_visual_entities.insert(visual_id, entity);
}

pub(crate) fn spawn_power_plant_entity(
    commands: &mut Commands,
    config: &Viewer3dConfig,
    assets: &Viewer3dAssets,
    scene: &mut Viewer3dScene,
    origin: GeoPos,
    facility_id: &str,
    location_id: &str,
    location_pos: GeoPos,
) {
    let cm_to_unit = config.effective_cm_to_unit();
    let base = geo_to_vec3(location_pos, origin, config.effective_cm_to_unit());
    let translation = base
        + Vec3::new(
            FACILITY_MARKER_LATERAL_OFFSET,
            FACILITY_MARKER_VERTICAL_OFFSET,
            0.0,
        );
    let marker_scale = Vec3::splat(facility_marker_scale_for_location(
        scene,
        location_id,
        cm_to_unit,
        POWER_PLANT_SCALE_RATIO_TO_LOCATION_RADIUS,
    ));

    if let Some(entity) = scene.power_plant_entities.get(facility_id) {
        commands.entity(*entity).insert((
            Transform::from_translation(translation).with_scale(marker_scale),
            BaseScale(marker_scale),
        ));
        return;
    }

    let entity = commands
        .spawn((
            Mesh3d(assets.power_plant_mesh.clone()),
            MeshMaterial3d(assets.power_plant_material.clone()),
            Transform::from_translation(translation).with_scale(marker_scale),
            Name::new(format!("power_plant:{facility_id}:{location_id}")),
            PowerPlantMarker {
                id: facility_id.to_string(),
            },
            BaseScale(marker_scale),
        ))
        .id();
    attach_to_scene_root(commands, scene, entity);
    commands.entity(entity).with_children(|parent| {
        spawn_label(
            parent,
            assets,
            format!("plant:{facility_id}"),
            LOCATION_LABEL_OFFSET,
            format!("label:power_plant:{facility_id}"),
        );
    });
    scene
        .power_plant_entities
        .insert(facility_id.to_string(), entity);
}

pub(super) fn spawn_asset_entity(
    commands: &mut Commands,
    config: &Viewer3dConfig,
    assets: &Viewer3dAssets,
    scene: &mut Viewer3dScene,
    origin: GeoPos,
    asset_id: &str,
    owner_pos: GeoPos,
    quantity: i64,
    kind: Option<&AssetKind>,
) {
    let base = geo_to_vec3(owner_pos, origin, config.effective_cm_to_unit());
    let translation = asset_translation(base, asset_id);
    let kind_label = asset_kind_label(kind);
    let label = asset_label_text(config, asset_id, quantity, kind_label);
    let marker = AssetMarker {
        id: asset_id.to_string(),
    };

    if let Some(entity) = scene.asset_entities.get(asset_id) {
        commands.entity(*entity).insert((
            Transform::from_translation(translation),
            marker,
            BaseScale(Vec3::ONE),
        ));
        commands.entity(*entity).despawn_children();
        commands.entity(*entity).with_children(|parent| {
            spawn_label(
                parent,
                assets,
                label,
                AGENT_LABEL_OFFSET,
                format!("label:asset:{asset_id}"),
            );
            spawn_asset_visual_details(parent, config, assets, asset_id, quantity, kind);
        });
        return;
    }

    let entity = commands
        .spawn((
            Mesh3d(assets.asset_mesh.clone()),
            MeshMaterial3d(assets.asset_material.clone()),
            Transform::from_translation(translation),
            Name::new(format!("asset:{asset_id}")),
            marker,
            BaseScale(Vec3::ONE),
        ))
        .id();
    attach_to_scene_root(commands, scene, entity);
    commands.entity(entity).with_children(|parent| {
        spawn_label(
            parent,
            assets,
            label,
            AGENT_LABEL_OFFSET,
            format!("label:asset:{asset_id}"),
        );
        spawn_asset_visual_details(parent, config, assets, asset_id, quantity, kind);
    });
    scene.asset_entities.insert(asset_id.to_string(), entity);
}

fn asset_kind_label(kind: Option<&AssetKind>) -> Option<&'static str> {
    match kind {
        Some(AssetKind::Resource {
            kind: ResourceKind::Electricity,
        }) => Some("electricity"),
        Some(AssetKind::Resource {
            kind: ResourceKind::Data,
        }) => Some("data"),
        _ => None,
    }
}

fn asset_label_text(
    config: &Viewer3dConfig,
    asset_id: &str,
    quantity: i64,
    kind_label: Option<&str>,
) -> String {
    let mut text = format!("asset:{asset_id}");
    if config.visual.asset_quantity_visual {
        text.push_str(format!(" q={}", quantity.max(0)).as_str());
    }
    if config.visual.asset_type_color {
        if let Some(kind_label) = kind_label {
            text.push_str(format!(" {kind_label}").as_str());
        }
    }
    text
}

fn spawn_asset_visual_details(
    parent: &mut ChildSpawnerCommands,
    config: &Viewer3dConfig,
    assets: &Viewer3dAssets,
    asset_id: &str,
    quantity: i64,
    kind: Option<&AssetKind>,
) {
    if config.visual.asset_type_color {
        let type_material = match kind {
            Some(AssetKind::Resource {
                kind: ResourceKind::Electricity,
            }) => Some(assets.flow_power_material.clone()),
            Some(AssetKind::Resource {
                kind: ResourceKind::Data,
            }) => Some(assets.flow_trade_material.clone()),
            _ => None,
        };
        if let Some(type_material) = type_material {
            let type_scale = Vec3::new(1.24, 0.09, 1.24);
            parent.spawn((
                Mesh3d(assets.world_box_mesh.clone()),
                MeshMaterial3d(type_material),
                Transform::from_translation(Vec3::new(0.0, -0.10, 0.0)).with_scale(type_scale),
                BaseScale(type_scale),
                Name::new(format!("asset:type_color:{asset_id}")),
                SceneZoomLayer::Detail,
            ));
        }
    }

    if config.visual.asset_quantity_visual {
        let capped_quantity = quantity.max(0).min(50_000) as f32;
        let quantity_ratio = (capped_quantity / 50_000.0).sqrt();
        let ring_radius = 0.66 + quantity_ratio * 0.84;
        let quantity_scale = Vec3::new(ring_radius, 0.06, ring_radius);
        parent.spawn((
            Mesh3d(assets.agent_module_marker_mesh.clone()),
            MeshMaterial3d(assets.chunk_generated_material.clone()),
            Transform::from_translation(Vec3::new(0.0, 0.16, 0.0)).with_scale(quantity_scale),
            BaseScale(quantity_scale),
            Name::new(format!("asset:quantity:{asset_id}")),
            SceneZoomLayer::Detail,
        ));
    }
}

fn chunk_coord_id(coord: ChunkCoord) -> String {
    format!("{},{},{}", coord.x, coord.y, coord.z)
}

fn chunk_state_name(state: ChunkState) -> String {
    match state {
        ChunkState::Unexplored => "unexplored".to_string(),
        ChunkState::Generated => "generated".to_string(),
        ChunkState::Exhausted => "exhausted".to_string(),
    }
}

fn chunk_material(assets: &Viewer3dAssets, state: ChunkState) -> Handle<StandardMaterial> {
    match state {
        ChunkState::Unexplored => assets.chunk_unexplored_material.clone(),
        ChunkState::Generated => assets.chunk_generated_material.clone(),
        ChunkState::Exhausted => assets.chunk_exhausted_material.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_d_agent_marker_profile_enforces_readable_radius() {
        let (radius, thickness, y) = two_d_agent_marker_profile(100, 0.00001);
        assert!(radius >= TWO_D_AGENT_MARKER_MIN_RADIUS_WORLD);
        assert!(thickness >= TWO_D_AGENT_MARKER_MIN_THICKNESS_WORLD);
        assert!(thickness <= TWO_D_AGENT_MARKER_MAX_THICKNESS_WORLD);
        assert!(y > 0.0);
    }

    #[test]
    fn two_d_agent_marker_profile_boosts_small_scale_readability() {
        let (radius, thickness, y) = two_d_agent_marker_profile(100, 0.00001);
        assert!(radius >= 0.00045);
        assert!(thickness >= 0.00006);
        assert!(y >= 0.00006);
    }
}

fn spawn_chunk_line_segments(
    commands: &mut Commands,
    assets: &Viewer3dAssets,
    scene: &Viewer3dScene,
    min_x: f32,
    max_x: f32,
    min_z: f32,
    max_z: f32,
    y: f32,
    chunk_id: &str,
    state_name: &str,
    state: ChunkState,
) -> Vec<Entity> {
    let mut entities = Vec::new();
    let thickness = grid_line_thickness(GridLineKind::Chunk, ViewerCameraMode::TwoD);

    let x_span = max_z - min_z;
    let x_line_scale = grid_line_scale(GridLineAxis::AlongZ, x_span, thickness);
    for (idx, x) in [min_x, max_x].into_iter().enumerate() {
        let entity = commands
            .spawn((
                Mesh3d(assets.world_box_mesh.clone()),
                MeshMaterial3d(chunk_material(assets, state)),
                Transform::from_translation(Vec3::new(x, y, (min_z + max_z) * 0.5))
                    .with_scale(x_line_scale),
                Name::new(format!("chunk:grid:x:{chunk_id}:{idx}")),
                ChunkMarker {
                    id: chunk_id.to_string(),
                    state: state_name.to_string(),
                    min_x,
                    max_x,
                    min_z,
                    max_z,
                    pick_y: y,
                },
                BaseScale(x_line_scale),
                GridLineVisual {
                    kind: GridLineKind::Chunk,
                    axis: GridLineAxis::AlongZ,
                    span: x_span,
                },
            ))
            .id();
        attach_to_scene_root(commands, scene, entity);
        entities.push(entity);
    }

    let z_span = max_x - min_x;
    let z_line_scale = grid_line_scale(GridLineAxis::AlongX, z_span, thickness);
    for (idx, z) in [min_z, max_z].into_iter().enumerate() {
        let entity = commands
            .spawn((
                Mesh3d(assets.world_box_mesh.clone()),
                MeshMaterial3d(chunk_material(assets, state)),
                Transform::from_translation(Vec3::new((min_x + max_x) * 0.5, y, z))
                    .with_scale(z_line_scale),
                Name::new(format!("chunk:grid:z:{chunk_id}:{idx}")),
                ChunkMarker {
                    id: chunk_id.to_string(),
                    state: state_name.to_string(),
                    min_x,
                    max_x,
                    min_z,
                    max_z,
                    pick_y: y,
                },
                BaseScale(z_line_scale),
                GridLineVisual {
                    kind: GridLineKind::Chunk,
                    axis: GridLineAxis::AlongX,
                    span: z_span,
                },
            ))
            .id();
        attach_to_scene_root(commands, scene, entity);
        entities.push(entity);
    }

    entities
}

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
    let Some(bounds) = chunk_bounds(coord, space) else {
        return;
    };
    let cm_to_unit = config.effective_cm_to_unit();
    let chunk_id = chunk_coord_id(coord);
    let state_name = chunk_state_name(state);

    if let Some(lines) = scene.chunk_line_entities.remove(&chunk_id) {
        for entity in lines {
            commands.entity(entity).despawn();
        }
    }
    scene.chunk_entities.remove(&chunk_id);

    let min_x = ((bounds.min.x_cm - origin.x_cm) as f64 * cm_to_unit as f64) as f32;
    let max_x = ((bounds.max.x_cm - origin.x_cm) as f64 * cm_to_unit as f64) as f32;
    let min_z = ((bounds.min.y_cm - origin.y_cm) as f64 * cm_to_unit as f64) as f32;
    let max_z = ((bounds.max.y_cm - origin.y_cm) as f64 * cm_to_unit as f64) as f32;
    let thickness = grid_line_thickness(GridLineKind::Chunk, ViewerCameraMode::TwoD);
    let y = -((space.height_cm as f32) * cm_to_unit * 0.5) + thickness * 0.7;

    let lines = spawn_chunk_line_segments(
        commands,
        assets,
        scene,
        min_x,
        max_x,
        min_z,
        max_z,
        y,
        &chunk_id,
        &state_name,
        state,
    );

    if let Some(anchor) = lines.first().copied() {
        commands.entity(anchor).with_children(|parent| {
            spawn_label(
                parent,
                assets,
                format!("chunk {chunk_id}"),
                LOCATION_LABEL_OFFSET,
                format!("label:chunk:{chunk_id}"),
            );
        });
        scene.chunk_entities.insert(chunk_id.clone(), anchor);
    }

    scene.chunk_line_entities.insert(chunk_id, lines);
}
