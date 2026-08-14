use std::collections::{HashMap, HashSet};

use super::*;

const LOCATION_LABEL_COLOR: Color = Color::srgba_u8(226, 232, 240, 220);
const LOCATION_LABEL_LAYER_Z: f32 = SELECTED_LOCATION_CUE_LAYER_Z + 0.005;
const LOCATION_LABEL_MIN_ZOOM: f64 = 1.75;
const LOCATION_LABEL_ABOVE_MARKER_PX: f64 = 14.0;
const LOCATION_LABEL_FONT_SIZE_PX: f32 = 10.0;
const LOCATION_LABEL_GLYPH_ADVANCE_PX: f64 = 5.8;
const LOCATION_LABEL_MAX_CHARS: usize = 24;
const LOCATION_LABEL_HEIGHT_PX: f64 = 12.0;

/// A zoom-gated, noninteractive identity label for an existing Location marker.
#[derive(Component)]
pub(super) struct PixelWorldLocationLabel {
    pub(super) id: String,
}

#[derive(SystemParam)]
pub(super) struct LocationLabelQueries<'w, 's> {
    labels: Query<'w, 's, (Entity, &'static PixelWorldLocationLabel)>,
}

pub(super) fn despawn_location_labels(commands: &mut Commands, queries: &LocationLabelQueries) {
    for (entity, _) in queries.labels.iter() {
        commands.entity(entity).despawn();
    }
}

pub(super) fn reconcile_location_labels(
    commands: &mut Commands,
    runtime: &BevyRuntimeState,
    queries: &LocationLabelQueries,
    width: f64,
    height: f64,
) {
    let existing = queries
        .labels
        .iter()
        .map(|(entity, label)| (label.id.clone(), entity))
        .collect::<HashMap<_, _>>();
    let Some(render_state) = runtime.render_state.as_ref() else {
        despawn_location_labels(commands, queries);
        return;
    };
    let Some(world_bounds) = render_state.world_bounds.as_ref() else {
        despawn_location_labels(commands, queries);
        return;
    };
    if runtime.camera.zoom < LOCATION_LABEL_MIN_ZOOM {
        despawn_location_labels(commands, queries);
        return;
    }

    let selected_id = render_state
        .selection
        .as_ref()
        .and_then(|selection| (selection.kind == "location").then_some(selection.id.as_str()));
    let mut locations = render_state.locations.iter().collect::<Vec<_>>();
    locations.sort_by(|left, right| {
        let left_selected = Some(left.id.as_str()) == selected_id;
        let right_selected = Some(right.id.as_str()) == selected_id;
        right_selected
            .cmp(&left_selected)
            .then_with(|| left.id.cmp(&right.id))
    });

    let mut active_ids = HashSet::new();
    let mut accepted_rects: Vec<LocationLabelRect> = Vec::new();
    for location in locations {
        let Some((canvas_x, canvas_y)) =
            to_canvas_point(&location.pos, world_bounds, width, height, &runtime.camera)
        else {
            continue;
        };
        let display = location_label_display(location);
        let label_x = canvas_x;
        let label_y = canvas_y - LOCATION_LABEL_ABOVE_MARKER_PX;
        let rect = LocationLabelRect::above_marker(label_x, label_y, &display);
        if accepted_rects
            .iter()
            .any(|accepted| accepted.overlaps(rect))
        {
            continue;
        }
        accepted_rects.push(rect);
        active_ids.insert(location.id.clone());
        let visuals = (
            Text2d::new(display),
            TextFont {
                font_size: FontSize::Px(LOCATION_LABEL_FONT_SIZE_PX),
                ..default()
            },
            TextColor(LOCATION_LABEL_COLOR),
            Transform::from_translation(to_bevy_translation(
                label_x,
                label_y,
                width,
                height,
                LOCATION_LABEL_LAYER_Z,
            )),
        );
        let label = PixelWorldLocationLabel {
            id: location.id.clone(),
        };
        if let Some(entity) = existing.get(&location.id) {
            commands.entity(*entity).insert((label, visuals));
        } else {
            commands.spawn((label, visuals));
        }
    }
    for (id, entity) in existing {
        if !active_ids.contains(&id) {
            commands.entity(entity).despawn();
        }
    }
}

fn location_label_display(location: &Location) -> String {
    let label = location.label.trim();
    truncate_location_label(if label.is_empty() {
        &location.id
    } else {
        label
    })
}

pub(super) fn truncate_location_label(label: &str) -> String {
    let mut characters = label.chars();
    let visible = characters
        .by_ref()
        .take(LOCATION_LABEL_MAX_CHARS)
        .collect::<String>();
    if characters.next().is_some() {
        format!("{visible}…")
    } else {
        visible
    }
}

#[derive(Clone, Copy)]
struct LocationLabelRect {
    left: f64,
    right: f64,
    top: f64,
    bottom: f64,
}

impl LocationLabelRect {
    fn above_marker(canvas_x: f64, canvas_y: f64, display: &str) -> Self {
        let half_width = display.chars().count() as f64 * LOCATION_LABEL_GLYPH_ADVANCE_PX / 2.0;
        Self {
            left: canvas_x - half_width,
            right: canvas_x + half_width,
            top: canvas_y - LOCATION_LABEL_HEIGHT_PX,
            bottom: canvas_y,
        }
    }

    fn overlaps(self, other: Self) -> bool {
        self.left < other.right
            && self.right > other.left
            && self.top < other.bottom
            && self.bottom > other.top
    }
}
