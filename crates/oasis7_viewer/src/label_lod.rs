use std::cmp::Ordering;
use std::collections::HashMap;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::{
    label_lod_profile, SelectionKind, Viewer3dCamera, Viewer3dConfig, ViewerCameraMode,
    ViewerSelection,
};

const LABEL_COLOR_R: f32 = 0.9;
const LABEL_COLOR_G: f32 = 0.9;
const LABEL_COLOR_B: f32 = 0.9;
const MIN_LABEL_ALPHA: f32 = 0.12;
const HIDDEN_LABEL_ALPHA: f32 = 0.05;
const OCCLUSION_ALPHA_DROP_PER_LAYER: f32 = 0.26;
const SELECTED_LABEL_BIAS: f32 = 0.5;

#[derive(Resource, Clone, Copy, Debug, Default)]
pub(super) struct LabelLodStats {
    pub total_labels: usize,
    pub visible_labels: usize,
    pub hidden_by_distance: usize,
    pub hidden_by_capacity: usize,
    pub hidden_by_occlusion: usize,
}

impl LabelLodStats {
    pub(super) fn degraded(self) -> bool {
        self.hidden_by_capacity > 0 || self.hidden_by_occlusion > 0
    }
}

#[derive(SystemParam)]
pub(super) struct LabelLodParams<'w, 's> {
    camera: Query<'w, 's, &'static GlobalTransform, With<Viewer3dCamera>>,
    labels: ParamSet<
        'w,
        's,
        (
            Query<'w, 's, (Entity, &'static GlobalTransform, Option<&'static Name>), With<Text2d>>,
            Query<'w, 's, (Entity, &'static mut Visibility, &'static mut TextColor), With<Text2d>>,
        ),
    >,
}

#[derive(Clone, Copy, Debug)]
struct LabelCandidate {
    entity: Entity,
    alpha: f32,
    score: f32,
    cell: (i32, i32),
    selection_bias: f32,
}

pub(super) fn update_label_lod(
    config: Res<Viewer3dConfig>,
    camera_mode: Res<ViewerCameraMode>,
    selection: Res<ViewerSelection>,
    mut params: LabelLodParams,
    mut stats: ResMut<LabelLodStats>,
) {
    let Ok(camera_transform) = params.camera.single() else {
        return;
    };

    let (_, camera_rotation, camera_translation) = camera_transform.to_scale_rotation_translation();
    let camera_forward = camera_rotation * -Vec3::Z;
    let camera_right = camera_rotation * Vec3::X;
    let camera_up = camera_rotation * Vec3::Y;

    let label_profile = label_lod_profile(*camera_mode, &config);
    let selected = selection
        .current
        .as_ref()
        .map(|current| (current.kind, current.id.as_str()));

    let mut next_stats = LabelLodStats::default();
    let mut candidates = Vec::new();

    for (entity, global_transform, name) in &params.labels.p0() {
        next_stats.total_labels += 1;

        let label_pos = global_transform.translation();
        let to_label = label_pos - camera_translation;
        let depth = to_label.dot(camera_forward);
        if depth <= 0.0 {
            next_stats.hidden_by_distance += 1;
            continue;
        }

        let distance = to_label.length();
        let alpha = label_alpha(distance, label_profile.fade_start, label_profile.fade_end);
        if alpha <= 0.0 {
            next_stats.hidden_by_distance += 1;
            continue;
        }

        let projected_x = to_label.dot(camera_right) / depth.max(0.001);
        let projected_y = to_label.dot(camera_up) / depth.max(0.001);
        let cell = (
            (projected_x * label_profile.occlusion_cell_span).round() as i32,
            (projected_y * label_profile.occlusion_cell_span).round() as i32,
        );

        let selection_bias = selected_label_bias(name.map(Name::as_str), selected);
        let score = alpha + selection_bias - distance * 0.001;
        candidates.push(LabelCandidate {
            entity,
            alpha,
            score,
            cell,
            selection_bias,
        });
    }

    candidates.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(Ordering::Equal)
    });

    let mut visible = HashMap::<Entity, f32>::new();
    let mut cell_occupancy = HashMap::<(i32, i32), usize>::new();

    for candidate in candidates {
        let occupancy = cell_occupancy.get(&candidate.cell).copied().unwrap_or(0);

        if visible.len() >= label_profile.max_visible_labels && candidate.selection_bias <= 0.0 {
            next_stats.hidden_by_capacity += 1;
            continue;
        }

        if occupancy >= label_profile.occlusion_cap && candidate.selection_bias <= 0.0 {
            next_stats.hidden_by_occlusion += 1;
            continue;
        }

        let occlusion_penalty = (occupancy as f32) * OCCLUSION_ALPHA_DROP_PER_LAYER;
        let final_alpha = (candidate.alpha - occlusion_penalty).clamp(0.0, 1.0);
        if final_alpha < MIN_LABEL_ALPHA && candidate.selection_bias <= 0.0 {
            next_stats.hidden_by_occlusion += 1;
            continue;
        }

        visible.insert(candidate.entity, final_alpha.max(MIN_LABEL_ALPHA));
        cell_occupancy.insert(candidate.cell, occupancy.saturating_add(1));
    }

    next_stats.visible_labels = visible.len();

    for (entity, mut visibility, mut text_color) in &mut params.labels.p1() {
        if let Some(alpha) = visible.get(&entity).copied() {
            *visibility = Visibility::Visible;
            text_color.0 = Color::srgba(LABEL_COLOR_R, LABEL_COLOR_G, LABEL_COLOR_B, alpha);
        } else {
            *visibility = Visibility::Hidden;
            text_color.0 = Color::srgba(
                LABEL_COLOR_R,
                LABEL_COLOR_G,
                LABEL_COLOR_B,
                HIDDEN_LABEL_ALPHA,
            );
        }
    }

    *stats = next_stats;
}

fn label_alpha(distance: f32, fade_start: f32, fade_end: f32) -> f32 {
    if distance <= fade_start {
        return 1.0;
    }
    if distance >= fade_end {
        return 0.0;
    }
    let span = (fade_end - fade_start).max(f32::EPSILON);
    1.0 - ((distance - fade_start) / span)
}

fn selected_label_bias(name: Option<&str>, selected: Option<(SelectionKind, &str)>) -> f32 {
    let (kind, id) = match selected {
        Some(value) => value,
        None => return 0.0,
    };
    let Some(name) = name else {
        return 0.0;
    };

    let prefix = match kind {
        SelectionKind::Agent => "label:agent:",
        SelectionKind::Location => "label:location:",
        SelectionKind::Fragment => "label:fragment:",
        SelectionKind::Asset => "label:asset:",
        SelectionKind::PowerPlant => "label:power_plant:",
        SelectionKind::Chunk => "label:chunk:",
    };

    if name.starts_with(prefix) && name.ends_with(id) {
        SELECTED_LABEL_BIAS
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SelectionInfo, SelectionKind, ViewerSelection};

    #[test]
    fn label_alpha_fades_between_start_and_end() {
        assert!((label_alpha(20.0, 30.0, 80.0) - 1.0).abs() < f32::EPSILON);
        assert!((label_alpha(80.0, 30.0, 80.0) - 0.0).abs() < f32::EPSILON);
        let mid = label_alpha(55.0, 30.0, 80.0);
        assert!(mid < 1.0);
        assert!(mid > 0.0);
    }

    #[test]
    fn selected_label_bias_matches_selection_kind_and_id() {
        let selected = Some((SelectionKind::Agent, "agent-2"));
        assert!(selected_label_bias(Some("label:agent:agent-2"), selected) > 0.0);
        assert_eq!(
            selected_label_bias(Some("label:agent:agent-1"), selected),
            0.0
        );
        assert_eq!(
            selected_label_bias(Some("label:location:loc-1"), selected),
            0.0
        );
    }

    #[test]
    fn update_label_lod_hides_far_labels_and_updates_stats() {
        let mut app = App::new();
        app.add_systems(Update, update_label_lod);
        app.insert_resource(Viewer3dConfig::default());
        app.insert_resource(ViewerCameraMode::ThreeD);
        app.insert_resource(ViewerSelection::default());
        app.insert_resource(LabelLodStats::default());

        app.world_mut()
            .spawn((Viewer3dCamera, GlobalTransform::default()));

        spawn_label(
            app.world_mut(),
            "label:agent:near",
            Vec3::new(0.0, 0.0, -10.0),
        );
        spawn_label(
            app.world_mut(),
            "label:agent:far",
            Vec3::new(0.0, 0.0, -220.0),
        );

        app.update();

        let mut query = app.world_mut().query::<(&Name, &Visibility)>();
        let mut seen_near = None;
        let mut seen_far = None;
        for (name, visibility) in query.iter(app.world()) {
            if name.as_str() == "label:agent:near" {
                seen_near = Some(*visibility);
            }
            if name.as_str() == "label:agent:far" {
                seen_far = Some(*visibility);
            }
        }

        assert_eq!(seen_near, Some(Visibility::Visible));
        assert_eq!(seen_far, Some(Visibility::Hidden));

        let stats = app.world().resource::<LabelLodStats>();
        assert_eq!(stats.total_labels, 2);
        assert_eq!(stats.visible_labels, 1);
        assert!(stats.hidden_by_distance >= 1);
    }

    #[test]
    fn update_label_lod_prioritizes_selected_label_when_capacity_is_tight() {
        let mut app = App::new();
        app.add_systems(Update, update_label_lod);

        let mut config = Viewer3dConfig::default();
        config.label_lod.max_visible_labels = 1;
        config.label_lod.occlusion_cap_per_cell = 1;
        app.insert_resource(config);
        app.insert_resource(ViewerCameraMode::ThreeD);
        app.insert_resource(LabelLodStats::default());

        let selected_entity = app.world_mut().spawn_empty().id();
        app.insert_resource(ViewerSelection {
            current: Some(SelectionInfo {
                entity: selected_entity,
                kind: SelectionKind::Agent,
                id: "agent-b".to_string(),
                name: None,
            }),
        });

        app.world_mut()
            .spawn((Viewer3dCamera, GlobalTransform::default()));

        spawn_label(
            app.world_mut(),
            "label:agent:agent-a",
            Vec3::new(0.0, 0.0, -18.0),
        );
        spawn_label(
            app.world_mut(),
            "label:agent:agent-b",
            Vec3::new(0.1, 0.0, -18.0),
        );

        app.update();

        let mut query = app.world_mut().query::<(&Name, &Visibility)>();
        let mut selected_visible = None;
        let mut other_visible = None;
        for (name, visibility) in query.iter(app.world()) {
            if name.as_str() == "label:agent:agent-b" {
                selected_visible = Some(*visibility);
            }
            if name.as_str() == "label:agent:agent-a" {
                other_visible = Some(*visibility);
            }
        }

        assert_eq!(selected_visible, Some(Visibility::Visible));
        assert_eq!(other_visible, Some(Visibility::Hidden));

        let stats = app.world().resource::<LabelLodStats>();
        assert!(stats.degraded());
    }

    #[test]
    fn label_lod_profile_two_d_is_more_permissive() {
        let mut config = Viewer3dConfig::default();
        config.label_lod.fade_start_distance = 40.0;
        config.label_lod.fade_end_distance = 90.0;
        config.label_lod.max_visible_labels = 20;
        config.label_lod.occlusion_cell_span = 6.0;
        config.label_lod.occlusion_cap_per_cell = 2;

        let three_d = label_lod_profile(ViewerCameraMode::ThreeD, &config);
        let two_d = label_lod_profile(ViewerCameraMode::TwoD, &config);

        assert!(two_d.fade_start > three_d.fade_start);
        assert!(two_d.fade_end > three_d.fade_end);
        assert!(two_d.max_visible_labels > three_d.max_visible_labels);
        assert!(two_d.occlusion_cell_span > three_d.occlusion_cell_span);
        assert!(two_d.occlusion_cap > three_d.occlusion_cap);
    }

    fn spawn_label(world: &mut World, name: &str, position: Vec3) {
        world.spawn((
            Text2d::new("label"),
            TextColor(Color::WHITE),
            Visibility::Visible,
            Transform::from_translation(position),
            GlobalTransform::from_translation(position),
            Name::new(name.to_string()),
        ));
    }
}
