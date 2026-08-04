use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum SelectedLocationCueEdge {
    Top,
    Bottom,
    Left,
    Right,
}

#[derive(Component)]
pub(super) struct PixelWorldSelectedLocationCue {
    pub(super) location_id: String,
    pub(super) edge: SelectedLocationCueEdge,
}

pub(super) fn selected_location_cue_visuals(
    location: &Location,
    animation_ms: f64,
) -> [(SelectedLocationCueEdge, Vec2, Vec2); 4] {
    let location_style = selected_location_visual_style(location, true, animation_ms);
    let outer_size = location_style.size_px as f32
        + (SELECTED_LOCATION_CUE_PADDING_PX + SELECTED_LOCATION_CUE_THICKNESS_PX) * 2.0;
    let edge_offset = (outer_size - SELECTED_LOCATION_CUE_THICKNESS_PX) / 2.0;
    [
        (
            SelectedLocationCueEdge::Top,
            Vec2::new(0.0, edge_offset),
            Vec2::new(outer_size, SELECTED_LOCATION_CUE_THICKNESS_PX),
        ),
        (
            SelectedLocationCueEdge::Bottom,
            Vec2::new(0.0, -edge_offset),
            Vec2::new(outer_size, SELECTED_LOCATION_CUE_THICKNESS_PX),
        ),
        (
            SelectedLocationCueEdge::Left,
            Vec2::new(-edge_offset, 0.0),
            Vec2::new(SELECTED_LOCATION_CUE_THICKNESS_PX, outer_size),
        ),
        (
            SelectedLocationCueEdge::Right,
            Vec2::new(edge_offset, 0.0),
            Vec2::new(SELECTED_LOCATION_CUE_THICKNESS_PX, outer_size),
        ),
    ]
}
