use super::*;

pub(super) fn owner_anchor_pos(snapshot: &WorldSnapshot, owner: &ResourceOwner) -> Option<GeoPos> {
    match owner {
        ResourceOwner::Agent { agent_id } => {
            snapshot.model.agents.get(agent_id).map(|agent| agent.pos)
        }
        ResourceOwner::Location { location_id } => snapshot
            .model
            .locations
            .get(location_id)
            .map(|location| location.pos),
    }
}

pub(super) fn location_render_radius_units(radius_cm: i64, cm_to_unit: f32) -> f32 {
    (radius_cm.max(1) as f32) * cm_to_unit.max(f32::EPSILON)
}

pub(super) fn world_units_per_meter(cm_to_unit: f32) -> f32 {
    cm_to_unit.max(f32::EPSILON) * 100.0
}

pub(super) fn location_visual_radius_cm(
    radius_cm: i64,
    fragment_budget: Option<&FragmentResourceBudget>,
) -> i64 {
    let base_radius = radius_cm.max(1);
    let Some(fragment_budget) = fragment_budget else {
        return base_radius;
    };
    let Some(remaining_ratio) = location_remaining_mass_ratio(fragment_budget) else {
        return base_radius;
    };

    let radius_factor = remaining_ratio
        .clamp(0.0, 1.0)
        .cbrt()
        .max(LOCATION_DEPLETION_MIN_RADIUS_FACTOR);
    ((base_radius as f32) * radius_factor).round().max(1.0) as i64
}

pub(super) fn location_damage_tier(
    fragment_budget: Option<&FragmentResourceBudget>,
) -> LocationDamageTier {
    let Some(fragment_budget) = fragment_budget else {
        return LocationDamageTier::Intact;
    };
    let Some(remaining_ratio) = location_remaining_mass_ratio(fragment_budget) else {
        return LocationDamageTier::Intact;
    };

    let damage_ratio = 1.0 - remaining_ratio.clamp(0.0, 1.0);
    if damage_ratio < 0.2 {
        LocationDamageTier::Intact
    } else if damage_ratio < 0.5 {
        LocationDamageTier::Light
    } else if damage_ratio < 0.8 {
        LocationDamageTier::Heavy
    } else {
        LocationDamageTier::Severe
    }
}

fn location_remaining_mass_ratio(fragment_budget: &FragmentResourceBudget) -> Option<f32> {
    let total_mass = fragment_budget
        .total_by_element_g
        .values()
        .copied()
        .filter(|amount| *amount > 0)
        .fold(0_i64, |acc, amount| acc.saturating_add(amount));
    if total_mass <= 0 {
        return None;
    }

    let remaining_mass = fragment_budget
        .remaining_by_element_g
        .values()
        .copied()
        .filter(|amount| *amount > 0)
        .fold(0_i64, |acc, amount| acc.saturating_add(amount));
    let clamped_remaining = remaining_mass.clamp(0, total_mass);
    Some((clamped_remaining as f32 / total_mass as f32).clamp(0.0, 1.0))
}

pub(super) fn agent_height_cm(height_cm: Option<i64>) -> i64 {
    height_cm.unwrap_or(oasis7::models::DEFAULT_AGENT_HEIGHT_CM)
}

pub(super) fn agent_body_scale(height_cm: i64, cm_to_unit: f32) -> Vec3 {
    let height_m = (agent_height_cm(Some(height_cm)) as f32 / 100.0)
        .clamp(AGENT_HEIGHT_MIN_M, AGENT_HEIGHT_MAX_M);
    let radius_m = (height_m * AGENT_BODY_RADIUS_RATIO).clamp(0.06, 0.9);
    let body_length_m = (height_m * AGENT_BODY_LENGTH_RATIO).max(radius_m * 0.1);
    let units_per_m = world_units_per_meter(cm_to_unit);
    Vec3::new(
        radius_m * 2.0 * units_per_m,
        body_length_m * units_per_m,
        radius_m * 2.0 * units_per_m,
    )
}

pub(super) fn body_half_height_units(height_cm: i64, cm_to_unit: f32) -> f32 {
    let scale = agent_body_scale(height_cm, cm_to_unit);
    scale.y * 0.5 + scale.x * 0.5
}

pub(super) fn agent_translation_for_render(
    scene: &Viewer3dScene,
    config: &Viewer3dConfig,
    origin: GeoPos,
    agent_id: &str,
    pos: GeoPos,
    height_cm: i64,
) -> Vec3 {
    let cm_to_unit = config.effective_cm_to_unit();
    let base = geo_to_vec3(pos, origin, cm_to_unit);
    let body_half_height = body_half_height_units(height_cm, cm_to_unit);
    let Some(location_id) = scene.agent_location_ids.get(agent_id) else {
        return base + Vec3::Y * body_half_height;
    };
    let Some(location_radius_cm) = scene.location_radii_cm.get(location_id.as_str()).copied()
    else {
        return base + Vec3::Y * body_half_height;
    };

    let Some(location_pos) = scene.location_positions.get(location_id.as_str()).copied() else {
        return base + Vec3::Y * body_half_height;
    };

    let location_center = geo_to_vec3(location_pos, origin, cm_to_unit);
    let location_radius = location_render_radius_units(location_radius_cm, cm_to_unit);
    let radial_offset = base - location_center;
    let radial_distance = radial_offset.length().max(location_radius);
    let surface_normal = if radial_offset.length_squared() > 1e-6 {
        radial_offset.normalize()
    } else {
        let angle = id_hash_fraction(agent_id) * std::f32::consts::TAU;
        Vec3::new(angle.cos(), 0.24, angle.sin()).normalize()
    };
    let surface_gap = (body_half_height * 0.01).max(0.006);
    location_center + surface_normal * (radial_distance + body_half_height + surface_gap)
}

#[derive(Clone, Copy)]
pub(super) struct AgentMotionVisual {
    pub(super) direction: Vec3,
    pub(super) speed_scale: f32,
}

pub(super) fn resolve_agent_motion_visual(
    scene: &Viewer3dScene,
    config: &Viewer3dConfig,
    origin: GeoPos,
    pos: GeoPos,
    kinematics: Option<&AgentKinematics>,
) -> Option<AgentMotionVisual> {
    let kinematics = kinematics?;
    if kinematics.move_remaining_cm <= 0 {
        return None;
    }

    let target_pos = kinematics.move_target.or_else(|| {
        kinematics
            .move_target_location_id
            .as_ref()
            .and_then(|location_id| scene.location_positions.get(location_id.as_str()).copied())
    })?;

    let current = geo_to_vec3(pos, origin, config.effective_cm_to_unit());
    let target = geo_to_vec3(target_pos, origin, config.effective_cm_to_unit());
    let direction = target - current;
    if direction.length_squared() <= AGENT_DIRECTION_VECTOR_EPSILON {
        return None;
    }

    let speed_scale = ((kinematics.speed_cm_per_tick.max(1) as f32)
        / AGENT_SPEED_REFERENCE_CM_PER_TICK)
        .sqrt()
        .clamp(AGENT_SPEED_EFFECT_MIN_SCALE, AGENT_SPEED_EFFECT_MAX_SCALE);
    Some(AgentMotionVisual {
        direction: direction.normalize(),
        speed_scale,
    })
}

pub(super) fn spawn_agent_motion_feedback(
    parent: &mut ChildSpawnerCommands,
    config: &Viewer3dConfig,
    assets: &Viewer3dAssets,
    agent_id: &str,
    body_scale: Vec3,
    motion_visual: Option<AgentMotionVisual>,
) {
    if !config.visual.agent_direction_indicator
        && !config.visual.agent_speed_effect
        && !config.visual.agent_trail_enabled
    {
        return;
    }

    let Some(motion_visual) = motion_visual else {
        return;
    };

    let indicator_rotation = Quat::from_rotation_arc(Vec3::Z, motion_visual.direction);
    if config.visual.agent_direction_indicator {
        let indicator_length = (body_scale.y * 0.92).max(AGENT_DIRECTION_INDICATOR_MIN_LENGTH);
        let indicator_width = (body_scale.x * 0.22).max(AGENT_DIRECTION_INDICATOR_MIN_WIDTH);
        let indicator_height = (indicator_width * 0.5).max(AGENT_DIRECTION_INDICATOR_MIN_WIDTH);
        parent.spawn((
            Mesh3d(assets.world_box_mesh.clone()),
            MeshMaterial3d(assets.chunk_generated_material.clone()),
            Transform::from_translation(Vec3::new(0.0, body_scale.y * 0.54, 0.0))
                .with_rotation(indicator_rotation)
                .with_scale(Vec3::new(
                    indicator_width,
                    indicator_height,
                    indicator_length,
                )),
            Name::new(format!("agent:direction_indicator:{agent_id}")),
            DetailZoomEntity,
        ));
    }

    if config.visual.agent_speed_effect {
        let speed_radius = (body_scale.x.max(body_scale.z) * motion_visual.speed_scale).clamp(
            body_scale.x * 0.9,
            body_scale.x * AGENT_SPEED_EFFECT_MAX_SCALE,
        );
        let speed_thickness = (body_scale.y * 0.07).max(AGENT_SPEED_EFFECT_MIN_THICKNESS);
        parent.spawn((
            Mesh3d(assets.agent_module_marker_mesh.clone()),
            MeshMaterial3d(assets.location_halo_material.clone()),
            Transform::from_translation(Vec3::new(0.0, body_scale.y * 0.22, 0.0))
                .with_scale(Vec3::new(speed_radius, speed_thickness, speed_radius)),
            Name::new(format!("agent:speed_effect:{agent_id}")),
            DetailZoomEntity,
        ));
    }

    if config.visual.agent_trail_enabled {
        let trail_length =
            (body_scale.y * motion_visual.speed_scale * 1.6).max(AGENT_TRAIL_MIN_LENGTH);
        let trail_thickness = (body_scale.y * 0.05).max(AGENT_TRAIL_MIN_THICKNESS);
        let trail_width = (body_scale.x * 0.18).max(AGENT_DIRECTION_INDICATOR_MIN_WIDTH);
        let trail_offset = -motion_visual.direction * (trail_length * 0.35);
        parent.spawn((
            Mesh3d(assets.world_box_mesh.clone()),
            MeshMaterial3d(assets.flow_power_material.clone()),
            Transform::from_translation(Vec3::new(0.0, body_scale.y * 0.36, 0.0) + trail_offset)
                .with_rotation(indicator_rotation)
                .with_scale(Vec3::new(trail_width, trail_thickness, trail_length)),
            Name::new(format!("agent:trail:{agent_id}")),
            DetailZoomEntity,
        ));
    }
}

fn agent_body_radius_m(height_cm: i64) -> f32 {
    let height_m = (agent_height_cm(Some(height_cm)) as f32 / 100.0)
        .clamp(AGENT_HEIGHT_MIN_M, AGENT_HEIGHT_MAX_M);
    (height_m * AGENT_BODY_RADIUS_RATIO).clamp(0.06, 0.9)
}

fn capped_module_marker_count(module_count: usize) -> usize {
    module_count.min(AGENT_MODULE_MARKER_MAX)
}

pub(super) fn agent_module_marker_scale(height_cm: i64, cm_to_unit: f32) -> Vec3 {
    let radius = agent_body_radius_m(height_cm);
    let units_per_m = world_units_per_meter(cm_to_unit);
    Vec3::new(
        (radius * AGENT_MODULE_MARKER_WIDTH_RATIO).clamp(AGENT_MODULE_MARKER_MIN_WIDTH, 0.62)
            * units_per_m,
        (radius * AGENT_MODULE_MARKER_HEIGHT_RATIO).clamp(AGENT_MODULE_MARKER_MIN_HEIGHT, 0.78)
            * units_per_m,
        (radius * AGENT_MODULE_MARKER_DEPTH_RATIO).clamp(AGENT_MODULE_MARKER_MIN_DEPTH, 0.42)
            * units_per_m,
    )
}

pub(super) fn agent_module_marker_world_scale(
    marker_scale: Vec3,
    cm_to_unit: f32,
    base_height_cm: i64,
) -> Vec3 {
    let base_height_m = (base_height_cm.max(1) as f32 / 100.0).max(f32::EPSILON);
    let marker_height_cm = marker_scale.y / cm_to_unit.max(f32::EPSILON);
    let min_height_cm = ((AGENT_MODULE_MARKER_WORLD_MIN_HEIGHT / base_height_m)
        * base_height_cm as f32)
        .max(base_height_cm as f32 * 0.16);
    let factor = if marker_height_cm >= min_height_cm {
        1.0
    } else {
        (min_height_cm / marker_height_cm).clamp(1.0, 3.4)
    };
    let units_per_m = world_units_per_meter(cm_to_unit);
    let min_world_width = AGENT_MODULE_MARKER_WORLD_MIN_WIDTH * units_per_m;
    let min_world_height = AGENT_MODULE_MARKER_WORLD_MIN_HEIGHT * units_per_m;
    let min_world_depth = AGENT_MODULE_MARKER_WORLD_MIN_DEPTH * units_per_m;

    Vec3::new(
        (marker_scale.x * factor).max(min_world_width),
        (marker_scale.y * factor).max(min_world_height),
        (marker_scale.z * factor).max(min_world_depth),
    )
}

fn agent_module_ring_radius(height_cm: i64, ring_idx: usize, cm_to_unit: f32) -> f32 {
    let radius = agent_body_radius_m(height_cm);
    let base = radius * AGENT_MODULE_RING_BASE_MULTIPLIER;
    let ring_gap = radius * AGENT_MODULE_RING_GAP_RATIO;
    (base + ring_gap * ring_idx as f32).clamp(0.25, 4.2) * world_units_per_meter(cm_to_unit)
}

pub(super) fn agent_module_marker_transforms(
    height_cm: i64,
    module_count: usize,
    cm_to_unit: f32,
) -> Vec<Vec3> {
    let marker_count = capped_module_marker_count(module_count);
    if marker_count == 0 {
        return Vec::new();
    }

    let body_scale = agent_body_scale(height_cm, cm_to_unit);
    let marker_scale = agent_module_marker_scale(height_cm, cm_to_unit);
    let body_half_height = body_half_height_units(height_cm, cm_to_unit);
    let module_gap_x = marker_scale.x * 2.05;
    let module_gap_z = marker_scale.z * 2.35;
    let module_layer_gap_z = marker_scale.z * 0.95;
    let shell_offset_x = body_scale.x * 0.98 + marker_scale.x * 1.05;
    let base_y = body_half_height * 0.2;
    let mut transforms = Vec::with_capacity(marker_count);

    for slot in AGENT_MODULE_LAYOUT_PRIMARY_SLOTS
        .iter()
        .take(marker_count)
        .copied()
    {
        transforms.push(Vec3::new(
            shell_offset_x + slot.0 as f32 * module_gap_x,
            base_y + slot.2 as f32 * (marker_scale.y * 0.32),
            slot.1 as f32 * module_gap_z + slot.2 as f32 * module_layer_gap_z,
        ));
    }

    if transforms.len() >= marker_count {
        return transforms;
    }

    let mut extra_idx = 0usize;
    while transforms.len() < marker_count {
        let ring_idx = extra_idx / AGENT_MODULE_MARKERS_PER_RING;
        let within_ring = extra_idx % AGENT_MODULE_MARKERS_PER_RING;
        let remaining = marker_count - transforms.len();
        let markers_in_ring = remaining.min(AGENT_MODULE_MARKERS_PER_RING);
        let angle_step = std::f32::consts::TAU / markers_in_ring as f32;
        let angle = angle_step * within_ring as f32;
        let ring_radius = agent_module_ring_radius(height_cm, ring_idx, cm_to_unit);
        let vertical = base_y + marker_scale.y * (0.28 + ring_idx as f32 * 0.24);
        transforms.push(Vec3::new(
            shell_offset_x + angle.cos() * ring_radius,
            vertical,
            angle.sin() * ring_radius,
        ));
        extra_idx += 1;
    }

    transforms
}

pub(super) fn agent_module_counts_in_snapshot(
    snapshot: &WorldSnapshot,
) -> std::collections::HashMap<String, usize> {
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for module_entity in snapshot.model.module_visual_entities.values() {
        if let ModuleVisualAnchor::Agent { agent_id } = &module_entity.anchor {
            *counts.entry(agent_id.clone()).or_insert(0) += 1;
        }
    }
    counts
}

pub(super) fn default_agent_module_count_estimate() -> usize {
    oasis7::models::AgentBodyState::default()
        .slots
        .iter()
        .filter(|slot| slot.installed_module.is_some())
        .count()
}

pub(super) fn agent_label_offset(height_cm: i64, cm_to_unit: f32) -> f32 {
    let height_m = (agent_height_cm(Some(height_cm)) as f32 / 100.0)
        .clamp(AGENT_HEIGHT_MIN_M, AGENT_HEIGHT_MAX_M);
    (height_m * 0.65).max(AGENT_LABEL_OFFSET) * world_units_per_meter(cm_to_unit)
}
