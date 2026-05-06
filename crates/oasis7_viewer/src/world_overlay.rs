use bevy::ecs::hierarchy::ChildSpawnerCommands;
use bevy::prelude::*;
use oasis7::simulator::{ChunkState, ResourceKind, WorldEvent, WorldSnapshot};
use std::collections::HashMap;

use crate::i18n::{locale_or_default, UiI18n, UiLocale};
use crate::industry_graph_view_model::{
    IndustryFlowKind, IndustryGraphEdge, IndustryGraphNode, IndustryGraphViewModel,
    IndustryNodeKind, IndustrySemanticZoomLevel, IndustrySemanticZoomState, IndustryStage,
    IndustryTier,
};
use crate::ui_locale_text::{overlay_button_label, overlay_loading, overlay_status};

use super::*;

const FLOW_BATCH_QUANTIZE: f32 = 32.0;
const HEAT_BASE_HEIGHT: f32 = 0.25;
const HEAT_MAX_HEIGHT: f32 = 1.8;
const HEAT_OFFSET_Y: f32 = 0.2;
const FLOW_OFFSET_Y: f32 = 0.18;
const FLOW_MIN_THICKNESS: f32 = 0.03;
const FLOW_MAX_THICKNESS: f32 = 0.12;
const FLOW_2D_PLANE_Y: f32 = 0.3;
const FLOW_2D_THICKNESS_MULTIPLIER: f32 = 1.65;
const FLOW_2D_THICKNESS_MAX: f32 = 0.24;
const FLOW_ARROW_LENGTH_FACTOR: f32 = 3.4;
const FLOW_ARROW_WIDTH_FACTOR: f32 = 1.85;
const FLOW_ARROW_MIN_LENGTH: f32 = 0.08;
const NODE_SYMBOL_OFFSET_Y: f32 = 0.24;
const NODE_RING_HEIGHT: f32 = 0.016;
const NODE_BADGE_SIZE: f32 = 0.056;
const NODE_BADGE_LIFT: f32 = 0.08;

#[derive(Resource, Clone, Copy, PartialEq, Eq)]
pub(super) struct WorldOverlayConfig {
    pub show_chunk_overlay: bool,
    pub show_resource_heatmap: bool,
    pub show_flow_overlay: bool,
}

#[derive(Resource, Default)]
pub(super) struct OverlayRenderRuntime {
    last_snapshot_tick: Option<u64>,
    last_event_count: usize,
}

impl Default for WorldOverlayConfig {
    fn default() -> Self {
        Self {
            show_chunk_overlay: true,
            show_resource_heatmap: true,
            show_flow_overlay: true,
        }
    }
}

pub(super) fn world_overlay_config_from_env() -> WorldOverlayConfig {
    WorldOverlayConfig::default()
}

#[derive(Resource, Default)]
pub(super) struct WorldOverlayUiState {
    pub status_text: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorldOverlayKind {
    Chunk,
    Heat,
    Flow,
}

#[derive(Component)]
pub(super) struct WorldOverlayToggleButton {
    kind: WorldOverlayKind,
}

#[allow(dead_code)]
#[derive(Component)]
pub(super) struct WorldOverlayToggleLabel;

#[derive(Component)]
pub(super) struct WorldOverlayStatusText;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum FlowSegmentKind {
    Material,
    Electricity,
    Data,
}

#[derive(Debug, Clone)]
struct FlowSegment {
    from: Vec3,
    to: Vec3,
    amount: i64,
    kind: FlowSegmentKind,
}

#[derive(Debug, Clone)]
struct LocationHeatPoint {
    anchor: Vec3,
    intensity: i64,
}

#[allow(dead_code)]
pub(super) fn spawn_world_overlay_controls(
    parent: &mut ChildSpawnerCommands,
    font: Handle<Font>,
    locale: UiLocale,
) {
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                row_gap: Val::Px(4.0),
                padding: UiRect::all(Val::Px(8.0)),
                border: UiRect::all(Val::Px(1.0)),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::srgb(0.12, 0.14, 0.17)),
            BorderColor::all(Color::srgb(0.22, 0.26, 0.31)),
        ))
        .with_children(|root| {
            root.spawn(Node {
                width: Val::Percent(100.0),
                min_height: Val::Px(24.0),
                column_gap: Val::Px(6.0),
                row_gap: Val::Px(6.0),
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|buttons| {
                spawn_overlay_button(
                    buttons,
                    &font,
                    WorldOverlayKind::Chunk,
                    overlay_button_label("chunk", locale),
                    Color::srgb(0.25, 0.31, 0.37),
                );
                spawn_overlay_button(
                    buttons,
                    &font,
                    WorldOverlayKind::Heat,
                    overlay_button_label("heat", locale),
                    Color::srgb(0.35, 0.28, 0.14),
                );
                spawn_overlay_button(
                    buttons,
                    &font,
                    WorldOverlayKind::Flow,
                    overlay_button_label("flow", locale),
                    Color::srgb(0.2, 0.26, 0.38),
                );
            });

            root.spawn((
                Text::new(overlay_loading(locale)),
                TextFont {
                    font,
                    font_size: 10.0,
                    ..default()
                },
                TextColor(Color::srgb(0.76, 0.8, 0.9)),
                WorldOverlayStatusText,
            ));
        });
}

#[allow(dead_code)]
fn spawn_overlay_button(
    buttons: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    kind: WorldOverlayKind,
    label: &str,
    color: Color,
) {
    buttons
        .spawn((
            Button,
            Node {
                padding: UiRect::horizontal(Val::Px(8.0)),
                height: Val::Px(22.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(color),
            WorldOverlayToggleButton { kind },
        ))
        .with_children(|button| {
            button.spawn((
                Text::new(label),
                TextFont {
                    font: font.clone(),
                    font_size: 11.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                WorldOverlayToggleLabel,
            ));
        });
}

pub(super) fn handle_world_overlay_toggle_buttons(
    mut config: ResMut<WorldOverlayConfig>,
    mut interactions: Query<
        (&Interaction, &WorldOverlayToggleButton),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (interaction, button) in &mut interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match button.kind {
            WorldOverlayKind::Chunk => config.show_chunk_overlay = !config.show_chunk_overlay,
            WorldOverlayKind::Heat => {
                config.show_resource_heatmap = !config.show_resource_heatmap;
            }
            WorldOverlayKind::Flow => config.show_flow_overlay = !config.show_flow_overlay,
        }
    }
}

pub(super) fn update_world_overlay_status_text(
    state: Res<ViewerState>,
    viewer_3d_config: Res<Viewer3dConfig>,
    config: Res<WorldOverlayConfig>,
    zoom_state: Res<IndustrySemanticZoomState>,
    i18n: Option<Res<UiI18n>>,
    mut ui_state: ResMut<WorldOverlayUiState>,
    mut text_query: Query<&mut Text, With<WorldOverlayStatusText>>,
) {
    let locale_changed = i18n
        .as_ref()
        .map(|value| value.is_changed())
        .unwrap_or(false);
    if !state.is_changed()
        && !viewer_3d_config.is_changed()
        && !config.is_changed()
        && !locale_changed
    {
        return;
    }

    let locale = locale_or_default(i18n.as_deref());

    let summary = build_overlay_status_text(
        state.snapshot.as_ref(),
        &state.events,
        *config,
        viewer_3d_config.effective_cm_to_unit(),
        locale,
        zoom_state.level,
    );
    ui_state.status_text = summary.clone();

    if let Ok(mut text) = text_query.single_mut() {
        text.0 = summary;
    }
}

pub(super) fn update_world_overlays_3d(
    mut commands: Commands,
    state: Res<ViewerState>,
    camera_mode: Res<ViewerCameraMode>,
    viewer_3d_config: Res<Viewer3dConfig>,
    overlay_config: Res<WorldOverlayConfig>,
    zoom_state: Res<IndustrySemanticZoomState>,
    assets: Res<Viewer3dAssets>,
    mut scene: ResMut<Viewer3dScene>,
    mut runtime: ResMut<OverlayRenderRuntime>,
    mut chunk_visibility: Query<&mut Visibility>,
) {
    let chunk_visibility_value = if overlay_config.show_chunk_overlay {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    for entities in scene.chunk_line_entities.values() {
        for entity in entities {
            if let Ok(mut visibility) = chunk_visibility.get_mut(*entity) {
                *visibility = chunk_visibility_value;
            }
        }
    }

    let Some(snapshot) = state.snapshot.as_ref() else {
        return;
    };

    let refresh_interval = viewer_3d_config.render_budget.overlay_refresh_ticks.max(1);
    let changed_by_toggle = overlay_config.is_changed() || viewer_3d_config.is_changed();
    if !overlay_refresh_due(
        &runtime,
        snapshot.time,
        state.events.len(),
        refresh_interval,
        changed_by_toggle,
    ) {
        return;
    }

    runtime.last_snapshot_tick = Some(snapshot.time);
    runtime.last_event_count = state.events.len();

    for entity in scene.heat_overlay_entities.drain(..) {
        if let Ok(mut command) = commands.get_entity(entity) {
            command.despawn();
        }
    }
    for entity in scene.flow_overlay_entities.drain(..) {
        if let Ok(mut command) = commands.get_entity(entity) {
            command.despawn();
        }
    }

    let Some(origin) = scene.origin else {
        return;
    };

    let cm_to_unit = viewer_3d_config.effective_cm_to_unit();

    if overlay_config.show_resource_heatmap {
        let mut heat_points = collect_location_heat_points(snapshot, origin, cm_to_unit);
        heat_points.sort_by_key(|point| std::cmp::Reverse(point.intensity.max(0)));
        heat_points.truncate(
            viewer_3d_config
                .render_budget
                .overlay_max_heat_markers
                .max(1),
        );
        let max_intensity = heat_points
            .iter()
            .map(|point| point.intensity.max(0))
            .max()
            .unwrap_or(1)
            .max(1);

        for point in heat_points {
            let ratio = (point.intensity.max(0) as f32 / max_intensity as f32).clamp(0.0, 1.0);
            let height = HEAT_BASE_HEIGHT + ratio * HEAT_MAX_HEIGHT;
            let material = if ratio >= 0.75 {
                assets.heat_high_material.clone()
            } else if ratio >= 0.35 {
                assets.heat_mid_material.clone()
            } else {
                assets.heat_low_material.clone()
            };
            let entity = commands
                .spawn((
                    Mesh3d(assets.world_box_mesh.clone()),
                    MeshMaterial3d(material),
                    Transform::from_translation(
                        point.anchor + Vec3::Y * (HEAT_OFFSET_Y + height * 0.5),
                    )
                    .with_scale(Vec3::new(0.22, height, 0.22)),
                    Name::new("overlay:heat"),
                ))
                .id();
            attach_to_scene_root(&mut commands, &scene, entity);
            scene.heat_overlay_entities.push(entity);
        }
    }

    if overlay_config.show_flow_overlay {
        let graph = IndustryGraphViewModel::build(Some(snapshot), &state.events);
        let (slice_nodes, slice_edges) = graph.graph_slice_for_zoom(zoom_state.level);
        let mut flow_segments =
            collect_flow_segments_from_graph(slice_nodes, slice_edges, origin, cm_to_unit);
        flow_segments = batch_flow_segments(
            flow_segments,
            viewer_3d_config
                .render_budget
                .overlay_max_flow_segments
                .max(1),
        );
        let max_amount = flow_segments
            .iter()
            .map(|segment| segment.amount.abs())
            .max()
            .unwrap_or(1)
            .max(1);

        for segment in flow_segments.drain(..) {
            let ratio = (segment.amount.abs() as f32 / max_amount as f32).clamp(0.0, 1.0);
            let thickness = FLOW_MIN_THICKNESS + ratio * (FLOW_MAX_THICKNESS - FLOW_MIN_THICKNESS);
            let (from, to, thickness) =
                flow_render_profile(*camera_mode, segment.from, segment.to, thickness);
            if from.distance(to) <= 0.00001 {
                continue;
            }
            let material = flow_material_for_kind(segment.kind, &assets);
            let entity = commands
                .spawn((
                    Mesh3d(assets.world_box_mesh.clone()),
                    MeshMaterial3d(material.clone()),
                    line_transform(from, to, thickness),
                    Name::new("overlay:flow"),
                ))
                .id();
            attach_to_scene_root(&mut commands, &scene, entity);
            scene.flow_overlay_entities.push(entity);

            if *camera_mode == ViewerCameraMode::TwoD {
                let arrow_entity = commands
                    .spawn((
                        Mesh3d(assets.world_box_mesh.clone()),
                        MeshMaterial3d(material),
                        flow_arrow_transform(from, to, thickness),
                        Name::new("overlay:flow:arrow"),
                    ))
                    .id();
                attach_to_scene_root(&mut commands, &scene, arrow_entity);
                scene.flow_overlay_entities.push(arrow_entity);
            }
        }

        let mut symbol_nodes = slice_nodes.to_vec();
        symbol_nodes.sort_by(|left, right| {
            right
                .throughput
                .cmp(&left.throughput)
                .then_with(|| right.status.alert_events.cmp(&left.status.alert_events))
                .then_with(|| left.id.cmp(&right.id))
        });
        symbol_nodes.truncate(
            viewer_3d_config
                .render_budget
                .overlay_max_heat_markers
                .max(1),
        );
        for node in symbol_nodes {
            spawn_industry_node_symbol(
                &mut commands,
                scene.as_mut(),
                &assets,
                node,
                origin,
                cm_to_unit,
            );
        }
    }
}

fn overlay_refresh_due(
    runtime: &OverlayRenderRuntime,
    snapshot_tick: u64,
    event_count: usize,
    refresh_interval: u64,
    force_refresh: bool,
) -> bool {
    if force_refresh {
        return true;
    }

    let tick_due = runtime
        .last_snapshot_tick
        .map(|tick| snapshot_tick.saturating_sub(tick) >= refresh_interval.max(1))
        .unwrap_or(true);
    let event_delta = event_count.saturating_sub(runtime.last_event_count);
    tick_due || event_delta >= 8
}

fn build_overlay_status_text(
    snapshot: Option<&WorldSnapshot>,
    events: &[WorldEvent],
    config: WorldOverlayConfig,
    _cm_to_unit: f32,
    locale: UiLocale,
    zoom: IndustrySemanticZoomLevel,
) -> String {
    let Some(snapshot) = snapshot else {
        return overlay_status(
            None,
            None,
            0,
            config.show_chunk_overlay,
            config.show_resource_heatmap,
            config.show_flow_overlay,
            locale,
        );
    };

    let graph = IndustryGraphViewModel::build(Some(snapshot), events);
    let (_, edges) = graph.graph_slice_for_zoom(zoom);
    let flow_count = edges.len();
    let (unexplored, generated, exhausted) = chunk_state_counts(snapshot);
    let heat_peak = top_heat_location(snapshot)
        .map(|(id, value)| format!("{id}:{value}"))
        .unwrap_or_else(|| "-".to_string());
    let status = overlay_status(
        Some((unexplored, generated, exhausted)),
        Some(heat_peak),
        flow_count,
        config.show_chunk_overlay,
        config.show_resource_heatmap,
        config.show_flow_overlay,
        locale,
    );
    format!("{status} zoom={}", zoom.key())
}

pub(super) fn overlay_status_text_public(
    snapshot: Option<&WorldSnapshot>,
    events: &[WorldEvent],
    config: WorldOverlayConfig,
    cm_to_unit: f32,
    locale: UiLocale,
    zoom: IndustrySemanticZoomLevel,
) -> String {
    build_overlay_status_text(snapshot, events, config, cm_to_unit, locale, zoom)
}

fn chunk_state_counts(snapshot: &WorldSnapshot) -> (usize, usize, usize) {
    let mut unexplored = 0;
    let mut generated = 0;
    let mut exhausted = 0;
    for state in snapshot.model.chunks.values() {
        match state {
            ChunkState::Unexplored => unexplored += 1,
            ChunkState::Generated => generated += 1,
            ChunkState::Exhausted => exhausted += 1,
        }
    }
    (unexplored, generated, exhausted)
}

fn top_heat_location(snapshot: &WorldSnapshot) -> Option<(String, i64)> {
    snapshot
        .model
        .locations
        .iter()
        .map(|(location_id, location)| {
            let electricity = location.resources.get(ResourceKind::Electricity).max(0);
            let data = location.resources.get(ResourceKind::Data).max(0);
            let score = electricity.saturating_add(data.saturating_mul(4));
            (location_id.clone(), score)
        })
        .max_by_key(|(_, score)| *score)
}

fn collect_location_heat_points(
    snapshot: &WorldSnapshot,
    origin: GeoPos,
    cm_to_unit: f32,
) -> Vec<LocationHeatPoint> {
    snapshot
        .model
        .locations
        .values()
        .map(|location| {
            let electricity = location.resources.get(ResourceKind::Electricity).max(0);
            let data = location.resources.get(ResourceKind::Data).max(0);
            let intensity = electricity.saturating_add(data.saturating_mul(4));
            LocationHeatPoint {
                anchor: geo_to_vec3(location.pos, origin, cm_to_unit),
                intensity,
            }
        })
        .collect()
}

fn collect_flow_segments_from_graph(
    nodes: &[IndustryGraphNode],
    edges: &[IndustryGraphEdge],
    origin: GeoPos,
    cm_to_unit: f32,
) -> Vec<FlowSegment> {
    let mut node_positions = HashMap::<String, Vec3>::new();
    for node in nodes {
        if let Some(position) = node.position {
            node_positions.insert(node.id.clone(), geo_to_vec3(position, origin, cm_to_unit));
        }
    }

    edges
        .iter()
        .filter_map(|edge| {
            let from = node_positions.get(edge.from.as_str())?;
            let to = node_positions.get(edge.to.as_str())?;
            if from.distance(*to) <= 0.00001 {
                return None;
            }
            Some(FlowSegment {
                from: *from + Vec3::Y * FLOW_OFFSET_Y,
                to: *to + Vec3::Y * FLOW_OFFSET_Y,
                amount: edge.throughput.abs(),
                kind: flow_segment_kind(edge.flow_kind),
            })
        })
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct FlowBatchKey {
    from: (i32, i32, i32),
    to: (i32, i32, i32),
    kind: FlowSegmentKind,
}

fn batch_flow_segments(segments: Vec<FlowSegment>, max_segments: usize) -> Vec<FlowSegment> {
    let mut batched = HashMap::<FlowBatchKey, FlowSegment>::new();
    for segment in segments {
        let key = FlowBatchKey {
            from: quantize_vec3(segment.from),
            to: quantize_vec3(segment.to),
            kind: segment.kind,
        };
        batched
            .entry(key)
            .and_modify(|existing| {
                existing.amount = existing.amount.saturating_add(segment.amount);
            })
            .or_insert(segment);
    }

    let mut merged: Vec<_> = batched.into_values().collect();
    merged.sort_by_key(|segment| std::cmp::Reverse(segment.amount.abs()));
    merged.truncate(max_segments.max(1));
    merged
}

fn quantize_vec3(value: Vec3) -> (i32, i32, i32) {
    (
        (value.x * FLOW_BATCH_QUANTIZE).round() as i32,
        (value.y * FLOW_BATCH_QUANTIZE).round() as i32,
        (value.z * FLOW_BATCH_QUANTIZE).round() as i32,
    )
}

fn flow_segment_kind(kind: IndustryFlowKind) -> FlowSegmentKind {
    match kind {
        IndustryFlowKind::Material => FlowSegmentKind::Material,
        IndustryFlowKind::Electricity => FlowSegmentKind::Electricity,
        IndustryFlowKind::Data => FlowSegmentKind::Data,
    }
}

fn flow_material_for_kind(
    kind: FlowSegmentKind,
    assets: &Viewer3dAssets,
) -> Handle<StandardMaterial> {
    match kind {
        FlowSegmentKind::Material => assets.heat_mid_material.clone(),
        FlowSegmentKind::Electricity => assets.flow_power_material.clone(),
        FlowSegmentKind::Data => assets.flow_trade_material.clone(),
    }
}

fn spawn_industry_node_symbol(
    commands: &mut Commands,
    scene: &mut Viewer3dScene,
    assets: &Viewer3dAssets,
    node: IndustryGraphNode,
    origin: GeoPos,
    cm_to_unit: f32,
) {
    let Some(position) = node.position else {
        return;
    };

    let anchor = geo_to_vec3(position, origin, cm_to_unit) + Vec3::Y * NODE_SYMBOL_OFFSET_Y;
    let (shape_scale, shape_rotation) = tier_symbol_transform(node.tier);
    let base_translation = anchor + Vec3::Y * (shape_scale.y * 0.5);
    let base_material = node_material(node.kind, assets);

    let base_entity = commands
        .spawn((
            Mesh3d(assets.world_box_mesh.clone()),
            MeshMaterial3d(base_material),
            Transform {
                translation: base_translation,
                rotation: shape_rotation,
                scale: shape_scale,
            },
            Name::new("overlay:industry:node"),
        ))
        .id();
    attach_to_scene_root(commands, scene, base_entity);
    scene.flow_overlay_entities.push(base_entity);

    let stage_material = stage_material(node.stage, assets);
    let ring_entity = commands
        .spawn((
            Mesh3d(assets.world_box_mesh.clone()),
            MeshMaterial3d(stage_material),
            Transform::from_translation(anchor + Vec3::Y * (NODE_RING_HEIGHT * 0.5)).with_scale(
                Vec3::new(
                    (shape_scale.x * 1.65).max(0.08),
                    NODE_RING_HEIGHT,
                    (shape_scale.z * 1.65).max(0.08),
                ),
            ),
            Name::new("overlay:industry:stage"),
        ))
        .id();
    attach_to_scene_root(commands, scene, ring_entity);
    scene.flow_overlay_entities.push(ring_entity);

    let badge_base = anchor + Vec3::Y * (shape_scale.y + NODE_BADGE_LIFT);
    if node.status.bottleneck {
        let entity = spawn_status_badge(
            commands,
            scene,
            assets.heat_high_material.clone(),
            badge_base + Vec3::new(0.08, 0.0, -0.08),
            "overlay:industry:badge:bottleneck",
            assets,
        );
        scene.flow_overlay_entities.push(entity);
    }
    if node.status.congestion {
        let entity = spawn_status_badge(
            commands,
            scene,
            assets.heat_mid_material.clone(),
            badge_base + Vec3::new(-0.08, 0.0, -0.08),
            "overlay:industry:badge:congestion",
            assets,
        );
        scene.flow_overlay_entities.push(entity);
    }
    if node.status.alert {
        let entity = spawn_status_badge(
            commands,
            scene,
            assets.flow_power_material.clone(),
            badge_base + Vec3::new(0.0, 0.0, 0.08),
            "overlay:industry:badge:alert",
            assets,
        );
        scene.flow_overlay_entities.push(entity);
    }
}

fn spawn_status_badge(
    commands: &mut Commands,
    scene: &Viewer3dScene,
    material: Handle<StandardMaterial>,
    translation: Vec3,
    name: &'static str,
    assets: &Viewer3dAssets,
) -> Entity {
    let entity = commands
        .spawn((
            Mesh3d(assets.world_box_mesh.clone()),
            MeshMaterial3d(material),
            Transform::from_translation(translation).with_scale(Vec3::splat(NODE_BADGE_SIZE)),
            Name::new(name),
        ))
        .id();
    attach_to_scene_root(commands, scene, entity);
    entity
}

fn tier_symbol_transform(tier: IndustryTier) -> (Vec3, Quat) {
    match tier {
        IndustryTier::R1 => (
            Vec3::new(0.14, 0.14, 0.14),
            Quat::from_rotation_y(std::f32::consts::FRAC_PI_4),
        ),
        IndustryTier::R2 => (Vec3::new(0.18, 0.18, 0.18), Quat::IDENTITY),
        IndustryTier::R3 => (Vec3::new(0.16, 0.30, 0.16), Quat::IDENTITY),
        IndustryTier::R4 => (Vec3::new(0.30, 0.12, 0.30), Quat::IDENTITY),
        IndustryTier::R5 => (Vec3::new(0.22, 0.36, 0.22), Quat::IDENTITY),
        IndustryTier::Unknown => (Vec3::new(0.12, 0.12, 0.12), Quat::IDENTITY),
    }
}

fn node_material(kind: IndustryNodeKind, assets: &Viewer3dAssets) -> Handle<StandardMaterial> {
    match kind {
        IndustryNodeKind::Factory => assets.heat_high_material.clone(),
        IndustryNodeKind::Recipe => assets.heat_mid_material.clone(),
        IndustryNodeKind::Product => assets.heat_low_material.clone(),
        IndustryNodeKind::LogisticsStation => assets.world_grid_material.clone(),
    }
}

fn stage_material(stage: IndustryStage, assets: &Viewer3dAssets) -> Handle<StandardMaterial> {
    match stage {
        IndustryStage::Bootstrap => assets.flow_power_material.clone(),
        IndustryStage::Scale => assets.flow_trade_material.clone(),
        IndustryStage::Governance => assets.heat_high_material.clone(),
        IndustryStage::Unknown => assets.world_bounds_material.clone(),
    }
}

fn flow_render_profile(
    mode: ViewerCameraMode,
    from: Vec3,
    to: Vec3,
    thickness: f32,
) -> (Vec3, Vec3, f32) {
    match mode {
        ViewerCameraMode::TwoD => (
            Vec3::new(from.x, FLOW_2D_PLANE_Y, from.z),
            Vec3::new(to.x, FLOW_2D_PLANE_Y, to.z),
            (thickness * FLOW_2D_THICKNESS_MULTIPLIER)
                .clamp(FLOW_MIN_THICKNESS, FLOW_2D_THICKNESS_MAX),
        ),
        ViewerCameraMode::ThreeD => (from, to, thickness),
    }
}

fn line_transform(from: Vec3, to: Vec3, thickness: f32) -> Transform {
    let delta = to - from;
    let length = delta.length().max(0.0001);
    let direction = delta / length;
    let rotation = Quat::from_rotation_arc(Vec3::Y, direction);
    Transform {
        translation: (from + to) * 0.5,
        rotation,
        scale: Vec3::new(thickness, length, thickness),
    }
}

fn flow_arrow_transform(from: Vec3, to: Vec3, thickness: f32) -> Transform {
    let delta = to - from;
    let length = delta.length().max(0.0001);
    let direction = delta / length;
    let max_arrow_length = (length * 0.48).max(FLOW_ARROW_MIN_LENGTH);
    let arrow_length =
        (thickness * FLOW_ARROW_LENGTH_FACTOR).clamp(FLOW_ARROW_MIN_LENGTH, max_arrow_length);
    let arrow_width =
        (thickness * FLOW_ARROW_WIDTH_FACTOR).clamp(FLOW_MIN_THICKNESS, FLOW_2D_THICKNESS_MAX);
    let rotation = Quat::from_rotation_arc(Vec3::Y, direction);
    let translation = to - direction * (arrow_length * 0.5);
    Transform {
        translation,
        rotation,
        scale: Vec3::new(arrow_width, arrow_length, arrow_width),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oasis7::geometry::GeoPos;
    use oasis7::simulator::{
        Agent, ChunkRuntimeConfig, Location, PowerEvent, ResourceOwner, WorldConfig,
        WorldEventKind, WorldModel,
    };

    fn sample_snapshot() -> WorldSnapshot {
        let mut model = WorldModel::default();

        let mut loc_a = Location::new("loc-a", "A", GeoPos::new(0, 0, 0));
        loc_a
            .resources
            .set(ResourceKind::Electricity, 20)
            .expect("set electricity");
        let mut loc_b = Location::new("loc-b", "B", GeoPos::new(100, 0, 0));
        loc_b
            .resources
            .set(ResourceKind::Electricity, 80)
            .expect("set electricity");

        model.locations.insert("loc-a".to_string(), loc_a);
        model.locations.insert("loc-b".to_string(), loc_b);
        model.agents.insert(
            "agent-1".to_string(),
            Agent::new("agent-1", "loc-a", GeoPos::new(0, 0, 0)),
        );

        WorldSnapshot {
            version: oasis7::simulator::SNAPSHOT_VERSION,
            chunk_generation_schema_version: oasis7::simulator::CHUNK_GENERATION_SCHEMA_VERSION,
            time: 5,
            config: WorldConfig::default(),
            model,
            chunk_runtime: ChunkRuntimeConfig::default(),
            next_event_id: 2,
            next_action_id: 2,
            pending_actions: Vec::new(),
            journal_len: 0,
            runtime_snapshot: None,
            player_gameplay: None,
        }
    }

    #[test]
    fn overlay_status_contains_chunk_heat_and_flow() {
        let mut snapshot = sample_snapshot();
        snapshot.model.chunks.insert(
            oasis7::simulator::ChunkCoord { x: 0, y: 0, z: 0 },
            ChunkState::Generated,
        );

        let events = vec![WorldEvent {
            id: 1,
            time: 4,
            kind: WorldEventKind::Power(PowerEvent::PowerTransferred {
                from: ResourceOwner::Location {
                    location_id: "loc-a".to_string(),
                },
                to: ResourceOwner::Location {
                    location_id: "loc-b".to_string(),
                },
                amount: 10,
                loss: 1,
                quoted_price_per_pu: 0,
                price_per_pu: 0,
                settlement_amount: 0,
            }),
            runtime_event: None,
        }];

        let text = build_overlay_status_text(
            Some(&snapshot),
            &events,
            WorldOverlayConfig::default(),
            0.00001,
            UiLocale::EnUs,
            IndustrySemanticZoomLevel::Node,
        );
        assert!(text.contains("Overlay[chunk:on heat:on flow:on]"));
        assert!(text.contains("chunks(u/g/e)=0/1/0"));
        assert!(text.contains("heat_peak=loc-b:80"));
        assert!(text.contains("flows=1"));
        assert!(text.contains("zoom=node"));
    }

    #[test]
    fn collect_flow_segments_extracts_trade_and_power() {
        let snapshot = sample_snapshot();
        let origin = space_origin(&snapshot.config.space);
        let events = vec![
            WorldEvent {
                id: 1,
                time: 1,
                kind: WorldEventKind::ResourceTransferred {
                    from: ResourceOwner::Location {
                        location_id: "loc-a".to_string(),
                    },
                    to: ResourceOwner::Location {
                        location_id: "loc-b".to_string(),
                    },
                    kind: ResourceKind::Data,
                    amount: 5,
                },
                runtime_event: None,
            },
            WorldEvent {
                id: 2,
                time: 2,
                kind: WorldEventKind::Power(PowerEvent::PowerTransferred {
                    from: ResourceOwner::Location {
                        location_id: "loc-b".to_string(),
                    },
                    to: ResourceOwner::Agent {
                        agent_id: "agent-1".to_string(),
                    },
                    amount: 9,
                    loss: 2,
                    quoted_price_per_pu: 0,
                    price_per_pu: 0,
                    settlement_amount: 0,
                }),
                runtime_event: None,
            },
        ];

        let graph = IndustryGraphViewModel::build(Some(&snapshot), &events);
        let (nodes, edges) = graph.graph_slice_for_zoom(IndustrySemanticZoomLevel::Node);
        let segments = collect_flow_segments_from_graph(nodes, edges, origin, 0.00001);
        assert_eq!(segments.len(), 2);
        assert!(segments
            .iter()
            .any(|segment| segment.kind == FlowSegmentKind::Data));
        assert!(segments
            .iter()
            .any(|segment| segment.kind == FlowSegmentKind::Electricity));
    }

    #[test]
    fn batch_flow_segments_merges_same_path_and_applies_limit() {
        let segments = vec![
            FlowSegment {
                from: Vec3::new(0.0, 0.0, 0.0),
                to: Vec3::new(1.0, 0.0, 0.0),
                amount: 5,
                kind: FlowSegmentKind::Data,
            },
            FlowSegment {
                from: Vec3::new(0.0, 0.0, 0.0),
                to: Vec3::new(1.0, 0.0, 0.0),
                amount: 7,
                kind: FlowSegmentKind::Data,
            },
            FlowSegment {
                from: Vec3::new(0.0, 0.0, 0.0),
                to: Vec3::new(0.0, 0.0, 1.0),
                amount: 4,
                kind: FlowSegmentKind::Electricity,
            },
        ];

        let batched = batch_flow_segments(segments, 1);
        assert_eq!(batched.len(), 1);
        assert_eq!(batched[0].kind, FlowSegmentKind::Data);
        assert_eq!(batched[0].amount, 12);
    }

    #[test]
    fn overlay_refresh_due_respects_tick_interval_and_event_burst() {
        let runtime = OverlayRenderRuntime {
            last_snapshot_tick: Some(10),
            last_event_count: 20,
        };

        assert!(!overlay_refresh_due(&runtime, 12, 21, 5, false));
        assert!(overlay_refresh_due(&runtime, 15, 21, 5, false));
        assert!(overlay_refresh_due(&runtime, 11, 28, 5, false));
        assert!(overlay_refresh_due(&runtime, 11, 21, 5, true));
    }

    #[test]
    fn flow_render_profile_two_d_flattens_and_boosts_thickness() {
        let from = Vec3::new(1.2, 0.8, -2.4);
        let to = Vec3::new(-3.0, 1.4, 4.2);
        let base_thickness = 0.06;

        let (two_d_from, two_d_to, two_d_thickness) =
            flow_render_profile(ViewerCameraMode::TwoD, from, to, base_thickness);
        let (three_d_from, three_d_to, three_d_thickness) =
            flow_render_profile(ViewerCameraMode::ThreeD, from, to, base_thickness);

        assert_eq!(three_d_from, from);
        assert_eq!(three_d_to, to);
        assert!((three_d_thickness - base_thickness).abs() < f32::EPSILON);

        assert!((two_d_from.y - FLOW_2D_PLANE_Y).abs() < f32::EPSILON);
        assert!((two_d_to.y - FLOW_2D_PLANE_Y).abs() < f32::EPSILON);
        assert_eq!(two_d_from.x, from.x);
        assert_eq!(two_d_to.z, to.z);
        assert!(two_d_thickness > base_thickness);
    }

    #[test]
    fn flow_arrow_transform_tip_matches_segment_target() {
        let from = Vec3::new(0.0, FLOW_2D_PLANE_Y, 0.0);
        let to = Vec3::new(2.0, FLOW_2D_PLANE_Y, 0.0);
        let transform = flow_arrow_transform(from, to, 0.08);
        let tip =
            transform.translation + (transform.rotation * Vec3::Y) * (transform.scale.y * 0.5);

        assert!((tip.x - to.x).abs() < 1e-3);
        assert!((tip.y - to.y).abs() < 1e-3);
        assert!((tip.z - to.z).abs() < 1e-3);
        assert!(transform.scale.x > 0.08);
    }

    #[test]
    fn overlay_toggle_button_flips_flags() {
        let mut app = App::new();
        app.add_systems(Update, handle_world_overlay_toggle_buttons);
        app.world_mut()
            .insert_resource(WorldOverlayConfig::default());

        app.world_mut().spawn((
            Button,
            Interaction::Pressed,
            WorldOverlayToggleButton {
                kind: WorldOverlayKind::Heat,
            },
        ));

        app.update();

        let config = app.world().resource::<WorldOverlayConfig>();
        assert!(!config.show_resource_heatmap);
        assert!(config.show_chunk_overlay);
        assert!(config.show_flow_overlay);
    }
}
