use bevy::prelude::*;

use super::{GridLineKind, Viewer3dConfig, ViewerCameraMode};

const TWO_D_GRID_LOD_DISTANCE_FACTOR: f32 = 1.35;
const THREE_D_GRID_LOD_DISTANCE_FACTOR: f32 = 1.0;
const TWO_D_LABEL_FADE_START_MULTIPLIER: f32 = 1.15;
const TWO_D_LABEL_FADE_END_MULTIPLIER: f32 = 2.0;
const TWO_D_LABEL_MAX_VISIBLE_MULTIPLIER: usize = 2;
const TWO_D_LABEL_MAX_VISIBLE_CAP: usize = 220;
const TWO_D_LABEL_OCCLUSION_CELL_SPAN_MULTIPLIER: f32 = 1.25;
const TWO_D_LABEL_OCCLUSION_CAP_BONUS: usize = 1;
const TWO_D_LABEL_OCCLUSION_CAP_LIMIT: usize = 8;
const FLOW_2D_THICKNESS_MULTIPLIER: f32 = 1.65;

pub(crate) const FLOW_THICKNESS_MIN: f32 = 0.03;
pub(crate) const FLOW_THICKNESS_MAX: f32 = 0.12;
pub(crate) const FLOW_2D_PLANE_Y: f32 = 0.3;
pub(crate) const FLOW_2D_THICKNESS_MAX: f32 = 0.24;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct LabelLodProfile {
    pub fade_start: f32,
    pub fade_end: f32,
    pub max_visible_labels: usize,
    pub occlusion_cell_span: f32,
    pub occlusion_cap: usize,
}

pub(crate) fn grid_line_thickness(kind: GridLineKind, mode: ViewerCameraMode) -> f32 {
    match (kind, mode) {
        (GridLineKind::World, ViewerCameraMode::TwoD) => super::WORLD_GRID_LINE_THICKNESS_2D,
        (GridLineKind::World, ViewerCameraMode::ThreeD) => super::WORLD_GRID_LINE_THICKNESS_3D,
        (GridLineKind::Chunk, ViewerCameraMode::TwoD) => super::CHUNK_GRID_LINE_THICKNESS_2D,
        (GridLineKind::Chunk, ViewerCameraMode::ThreeD) => super::CHUNK_GRID_LINE_THICKNESS_3D,
    }
}

pub(crate) fn grid_lod_distance_factor(mode: ViewerCameraMode) -> f32 {
    match mode {
        ViewerCameraMode::TwoD => TWO_D_GRID_LOD_DISTANCE_FACTOR,
        ViewerCameraMode::ThreeD => THREE_D_GRID_LOD_DISTANCE_FACTOR,
    }
}

pub(crate) fn label_lod_profile(
    mode: ViewerCameraMode,
    config: &Viewer3dConfig,
) -> LabelLodProfile {
    let base_fade_start = config.label_lod.fade_start_distance.max(0.0);
    let base_fade_end = config
        .label_lod
        .fade_end_distance
        .max(base_fade_start + f32::EPSILON);
    let base_max_visible = config.label_lod.max_visible_labels.max(1);
    let base_cell_span = config.label_lod.occlusion_cell_span.max(0.5);
    let base_occlusion_cap = config.label_lod.occlusion_cap_per_cell.max(1);

    match mode {
        ViewerCameraMode::TwoD => LabelLodProfile {
            fade_start: base_fade_start * TWO_D_LABEL_FADE_START_MULTIPLIER,
            fade_end: base_fade_end * TWO_D_LABEL_FADE_END_MULTIPLIER,
            max_visible_labels: (base_max_visible
                .saturating_mul(TWO_D_LABEL_MAX_VISIBLE_MULTIPLIER))
            .min(TWO_D_LABEL_MAX_VISIBLE_CAP),
            occlusion_cell_span: base_cell_span * TWO_D_LABEL_OCCLUSION_CELL_SPAN_MULTIPLIER,
            occlusion_cap: (base_occlusion_cap + TWO_D_LABEL_OCCLUSION_CAP_BONUS)
                .min(TWO_D_LABEL_OCCLUSION_CAP_LIMIT),
        },
        ViewerCameraMode::ThreeD => LabelLodProfile {
            fade_start: base_fade_start,
            fade_end: base_fade_end,
            max_visible_labels: base_max_visible,
            occlusion_cell_span: base_cell_span,
            occlusion_cap: base_occlusion_cap,
        },
    }
}

pub(crate) fn flow_render_profile(
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
                .clamp(FLOW_THICKNESS_MIN, FLOW_2D_THICKNESS_MAX),
        ),
        ViewerCameraMode::ThreeD => (from, to, thickness),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
