use super::*;
use serde_json::json;

pub(super) fn sample_render_state_with_selection(
    fragment_footprint_cm: f64,
    kind: &str,
    id: &str,
) -> RenderState {
    let mut render_state = sample_render_state(fragment_footprint_cm);
    render_state.selection = Some(Selection {
        kind: kind.to_string(),
        id: id.to_string(),
    });
    render_state
}

pub(super) fn sample_render_state_with_unoccluded_detail_fleck() -> RenderState {
    let mut render_state = sample_render_state(20_000.0);
    render_state.selection = None;
    render_state.locations[0].pos = sample_position(1_200_000.0, 700_000.0);
    render_state.agents[0].pos = Some(sample_position(1_250_000.0, 750_000.0));
    render_state
}

pub(super) fn sample_render_state_with_beacon_candidates(kind: &str, id: &str) -> RenderState {
    let mut render_state = sample_render_state_with_selection(12_000.0, kind, id);
    render_state.locations.push(Location {
        id: "loc-1".to_string(),
        label: "Unselected Location".to_string(),
        pos: sample_position(1_620_000.0, 1_100_000.0),
        radius_cm: 30_000.0,
        resource_summary: "-".to_string(),
        size_hint_px: Some(10.0),
        marker_role: Some("logic_anchor".to_string()),
        marker_alpha: Some(0.32),
    });
    render_state.agents.push(Agent {
        id: "agent-1".to_string(),
        label: "Unselected Agent".to_string(),
        pos: Some(sample_position(1_640_000.0, 1_115_000.0)),
        location_id: Some("loc-1".to_string()),
        resource_summary: "-".to_string(),
        status_badges: vec![],
        position_source: AgentPositionSource::Snapshot,
        size_hint_px: Some(16.0),
    });
    render_state
}

pub(super) fn sample_render_state_with_hotspot_candidates() -> RenderState {
    let mut render_state = sample_render_state(12_000.0);
    render_state.visual_hotspots = vec![
        VisualHotspot {
            id: "hotspot-blocker".to_string(),
            label: "Blocked route".to_string(),
            kind: "blocker".to_string(),
            pos: sample_position(1_400_000.0, 800_000.0),
            emphasis: Some(0.8),
            size_hint_px: Some(12.0),
        },
        VisualHotspot {
            id: "hotspot-goal".to_string(),
            label: "Goal route".to_string(),
            kind: "goal".to_string(),
            pos: sample_position(1_600_000.0, 1_200_000.0),
            emphasis: Some(0.5),
            size_hint_px: Some(24.0),
        },
        VisualHotspot {
            id: "hotspot-recent-event".to_string(),
            label: "Recent route event".to_string(),
            kind: "resource_transfer".to_string(),
            pos: sample_position(1_800_000.0, 1_000_000.0),
            emphasis: Some(0.6),
            size_hint_px: Some(12.0),
        },
    ];
    render_state
}

pub(super) fn sample_render_state_with_location_resource_summary(
    resource_summary: &str,
) -> RenderState {
    let mut render_state = sample_render_state(12_000.0);
    render_state.fragment_terrain.clear();
    render_state.agents.clear();
    render_state.links.clear();
    render_state.visual_hotspots.clear();
    render_state.selection = None;
    let location = render_state
        .locations
        .first_mut()
        .expect("sample render state has a location");
    location.resource_summary = resource_summary.to_string();
    location.marker_role = Some("logic_anchor".to_string());
    location.marker_alpha = Some(0.32);
    render_state
}

pub(super) fn sample_render_state_with_receipt_target(
    receipt_state: Option<&str>,
    target_agent_id: Option<&str>,
) -> RenderState {
    serde_json::from_value(json!({
        "world_bounds": {
            "width_cm": 3_000_000.0,
            "depth_cm": 2_000_000.0,
            "height_cm": 500_000.0,
        },
        "locations": [{
            "id": "loc-0",
            "label": "Receipt Anchor",
            "pos": { "x_cm": 1_500_000.0, "y_cm": 1_000_000.0, "z_cm": 0.0 },
            "radius_cm": 30_000.0,
            "resource_summary": "-",
            "size_hint_px": 10.0,
        }],
        "fragment_terrain": [{
            "id": "fragment:loc-0:0",
            "location_id": "loc-0",
            "pos": { "x_cm": 1_503_000.0, "y_cm": 1_006_000.0, "z_cm": 0.0 },
            "footprint_cm": 12_000.0,
            "dominant_compound": "silicate_matrix",
            "color": [141, 199, 170],
            "emphasis": 0.58,
        }],
        "agents": [{
            "id": "agent-0",
            "label": "Receipt Target",
            "pos": { "x_cm": 1_520_000.0, "y_cm": 1_015_000.0, "z_cm": 0.0 },
            "location_id": "loc-0",
            "resource_summary": "-",
            "status_badges": [],
            "size_hint_px": 16.0,
        }],
        "links": [],
        "visual_hotspots": [],
        "selection": null,
        "receipt_target": receipt_state.zip(target_agent_id).map(|(state, agent_id)| json!({
            "state": state,
            "agent_id": agent_id,
        })),
    }))
    .expect("receipt cue fixture must remain a valid RenderState DTO")
}

pub(super) fn sample_render_state_with_recommended_target(
    target_agent_id: Option<&str>,
) -> RenderState {
    let mut render_state = sample_render_state_with_receipt_target(None, None);
    render_state.recommended_target = target_agent_id.map(|agent_id| RecommendedTarget {
        agent_id: agent_id.to_string(),
    });
    render_state
}

pub(super) fn test_runtime(render_state: RenderState) -> BevyRuntimeState {
    BevyRuntimeState {
        mounted: true,
        render_state: Some(render_state),
        render_version: 1,
        camera: CameraState {
            zoom: 3.0,
            pan_x_px: 0.0,
            pan_y_px: 0.0,
        },
        camera_fit_version: 1,
        camera_user_override: true,
        ..Default::default()
    }
}
