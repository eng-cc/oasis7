use std::collections::{HashMap, HashSet};

use super::*;

const MAP_OBJECTIVE_COLOR: Color = Color::srgba_u8(254, 240, 138, 244);
const MAP_BLOCKER_COLOR: Color = Color::srgba_u8(253, 186, 116, 244);
const MAP_ROUTE_COLOR: Color = Color::srgba_u8(186, 230, 253, 236);
// Keep map callouts above location identities while letting selected/active
// agent identities retain the top label layer at a shared anchor.
const MAP_LABEL_LAYER_Z: f32 = AGENT_LAYER_Z + 0.002;
const MAP_LABEL_FONT_SIZE_PX: f32 = 11.0;
const MAP_LABEL_GLYPH_ADVANCE_PX: f64 = 6.0;
const MAP_LABEL_HEIGHT_PX: f64 = 13.0;
const MAP_LABEL_MAX_CHARS: usize = 28;
const MAP_OBJECTIVE_OFFSET_Y_PX: f64 = 22.0;
const MAP_BLOCKER_OFFSET_Y_PX: f64 = 22.0;
const MAP_ROUTE_OFFSET_Y_PX: f64 = 14.0;

#[derive(Component)]
pub(crate) struct PixelWorldMapLabel {
    pub(crate) id: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum MapLabelKind {
    Objective,
    Blocker,
    Route,
}

#[derive(SystemParam)]
pub(crate) struct MapLabelQueries<'w, 's> {
    labels: Query<'w, 's, (Entity, &'static PixelWorldMapLabel)>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct MapLabelRect {
    pub(super) left: f64,
    pub(super) right: f64,
    pub(super) top: f64,
    pub(super) bottom: f64,
}

impl MapLabelRect {
    fn centered(canvas_x: f64, canvas_y: f64, display: &str) -> Self {
        let half_width = display.chars().count() as f64 * MAP_LABEL_GLYPH_ADVANCE_PX / 2.0;
        Self {
            left: canvas_x - half_width,
            right: canvas_x + half_width,
            top: canvas_y - MAP_LABEL_HEIGHT_PX,
            bottom: canvas_y,
        }
    }

    pub(super) fn overlaps(self, other: Self) -> bool {
        self.left < other.right
            && self.right > other.left
            && self.top < other.bottom
            && self.bottom > other.top
    }

    pub(super) fn overlaps_bounds(self, left: f64, right: f64, top: f64, bottom: f64) -> bool {
        self.left < right && self.right > left && self.top < bottom && self.bottom > top
    }
}

#[derive(Clone, Debug)]
struct MapLabelCandidate {
    id: String,
    text: String,
    kind: MapLabelKind,
    canvas_x: f64,
    canvas_y: f64,
}

impl MapLabelCandidate {
    fn rect(&self) -> MapLabelRect {
        MapLabelRect::centered(self.canvas_x, self.canvas_y, &self.text)
    }
}

pub(crate) fn despawn_map_labels(commands: &mut Commands, queries: &MapLabelQueries) {
    for (entity, _) in queries.labels.iter() {
        commands.entity(entity).despawn();
    }
}

pub(crate) fn reconcile_map_labels(
    commands: &mut Commands,
    runtime: &BevyRuntimeState,
    queries: &MapLabelQueries,
    width: f64,
    height: f64,
) {
    let existing = queries
        .labels
        .iter()
        .map(|(entity, label)| (label.id.clone(), entity))
        .collect::<HashMap<_, _>>();
    let Some(render_state) = runtime.render_state.as_ref() else {
        despawn_map_labels(commands, queries);
        return;
    };

    let candidates = map_label_candidates(render_state, width, height, &runtime.camera);
    let mut active_ids = HashSet::new();
    let mut accepted_rects = Vec::new();
    for candidate in candidates {
        let rect = candidate.rect();
        if accepted_rects
            .iter()
            .any(|accepted: &MapLabelRect| accepted.overlaps(rect))
        {
            continue;
        }
        accepted_rects.push(rect);
        active_ids.insert(candidate.id.clone());
        let visuals = (
            Text2d::new(candidate.text),
            TextFont {
                font_size: FontSize::Px(MAP_LABEL_FONT_SIZE_PX),
                ..default()
            },
            TextColor(map_label_color(candidate.kind)),
            Transform::from_translation(to_bevy_translation(
                candidate.canvas_x,
                candidate.canvas_y,
                width,
                height,
                MAP_LABEL_LAYER_Z,
            )),
        );
        let label = PixelWorldMapLabel {
            id: candidate.id.clone(),
        };
        if let Some(entity) = existing.get(&candidate.id) {
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

/// Returns the occupied rectangles for objective/blocker/route labels. Agent
/// and Location labels use this as a deterministic high-priority exclusion
/// set, while selected/active identities may still take precedence over it.
pub(super) fn map_label_obstacles(
    render_state: &RenderState,
    width: f64,
    height: f64,
    camera: &CameraState,
) -> Vec<MapLabelRect> {
    let mut accepted = Vec::new();
    for candidate in map_label_candidates(render_state, width, height, camera) {
        let rect = candidate.rect();
        if accepted
            .iter()
            .any(|other: &MapLabelRect| other.overlaps(rect))
        {
            continue;
        }
        accepted.push(rect);
    }
    accepted
}

fn map_label_candidates(
    render_state: &RenderState,
    width: f64,
    height: f64,
    camera: &CameraState,
) -> Vec<MapLabelCandidate> {
    let Some(bounds) = render_state.world_bounds.as_ref() else {
        return Vec::new();
    };
    let mut candidates = Vec::new();
    let mut hotspots = render_state
        .visual_hotspots
        .iter()
        .filter(|hotspot| matches!(hotspot.kind.as_str(), "goal" | "blocker"))
        .collect::<Vec<_>>();
    hotspots.sort_by(|left, right| {
        map_hotspot_priority(&left.kind)
            .cmp(&map_hotspot_priority(&right.kind))
            .then_with(|| left.id.cmp(&right.id))
    });
    for hotspot in hotspots {
        let Some((canvas_x, canvas_y)) =
            to_canvas_point(&hotspot.pos, bounds, width, height, camera)
        else {
            continue;
        };
        let text = truncate_map_label(&hotspot.label);
        if text.is_empty() {
            continue;
        }
        let offset_y = match hotspot.kind.as_str() {
            "goal" => -MAP_OBJECTIVE_OFFSET_Y_PX,
            "blocker" => MAP_BLOCKER_OFFSET_Y_PX,
            _ => unreachable!("filtered map hotspot kind"),
        };
        candidates.push(MapLabelCandidate {
            id: format!("map:{}", hotspot.id),
            text,
            kind: if hotspot.kind == "goal" {
                MapLabelKind::Objective
            } else {
                MapLabelKind::Blocker
            },
            canvas_x,
            canvas_y: canvas_y + offset_y,
        });
    }
    let mut route_links = render_state
        .links
        .iter()
        .filter(|link| super::links::is_authoritative_directional_link(link))
        .collect::<Vec<_>>();
    route_links.sort_by(|left, right| left.id.cmp(&right.id));
    for link in route_links {
        let Some((from_x, from_y)) = to_canvas_point(&link.from, bounds, width, height, camera)
        else {
            continue;
        };
        let Some((to_x, to_y)) = to_canvas_point(&link.to, bounds, width, height, camera) else {
            continue;
        };
        let length = ((to_x - from_x).powi(2) + (to_y - from_y).powi(2)).sqrt();
        if !length.is_finite() || length < super::links::LINK_DIRECTION_MIN_LENGTH_PX {
            continue;
        }
        candidates.push(MapLabelCandidate {
            id: format!("map:route:{}", link.id),
            text: truncate_map_label(&route_label_text(
                &link.kind,
                link.label.as_deref(),
                &render_state.locale,
            )),
            kind: MapLabelKind::Route,
            canvas_x: (from_x + to_x) / 2.0,
            canvas_y: ((from_y + to_y) / 2.0) - MAP_ROUTE_OFFSET_Y_PX,
        });
    }
    candidates
}

fn map_hotspot_priority(kind: &str) -> u8 {
    match kind {
        "goal" => 1,
        "blocker" => 2,
        _ => 3,
    }
}

fn route_label_text(kind: &str, published_label: Option<&str>, locale: &str) -> String {
    if let Some(label) = published_label.filter(|label| !label.trim().is_empty()) {
        return label.trim().to_string();
    }
    let is_zh = locale.trim().to_ascii_lowercase().starts_with("zh");
    let fallback =
        if is_zh {
            match kind {
                "logistics" => "物流",
                "logistics_route" => "物流路线",
                "supply_route" => "供应路线",
                "delivery_route" => "交付路线",
                "resource_flow" | "resource_transfer" | "material_transfer"
                | "material_transit" => "资源流",
                "route" => "路线",
                _ => "路线",
            }
        } else {
            match kind {
                "logistics" => "Logistics",
                "logistics_route" => "Logistics route",
                "supply_route" => "Supply route",
                "delivery_route" => "Delivery route",
                "resource_flow" | "resource_transfer" | "material_transfer"
                | "material_transit" => "Resource flow",
                "route" => "Route",
                _ => "Route",
            }
        };
    fallback.to_string()
}

pub(super) fn truncate_map_label(label: &str) -> String {
    let mut characters = label.trim().chars();
    let visible = characters
        .by_ref()
        .take(MAP_LABEL_MAX_CHARS)
        .collect::<String>();
    if characters.next().is_some() {
        format!("{visible}…")
    } else {
        visible
    }
}

fn map_label_color(kind: MapLabelKind) -> Color {
    match kind {
        MapLabelKind::Objective => MAP_OBJECTIVE_COLOR,
        MapLabelKind::Blocker => MAP_BLOCKER_COLOR,
        MapLabelKind::Route => MAP_ROUTE_COLOR,
    }
}
