use serde_json::{Map, Value, json};

use super::{obj, str_key};

const AUTHORITATIVE_ASSIGNMENT_STATUS: &str = "active";
const AUTHORITATIVE_SOURCE_CLASS: &str = "runtime_projection";
const AUTHORITATIVE_FRESHNESS: &str = "current";
const AUTHORITATIVE_LINK_KINDS: &[&str] = &[
    "agent_assignment",
    "route",
    "logistics",
    "logistics_route",
    "supply_route",
    "delivery_route",
    "resource_flow",
    "resource_transfer",
    "material_transfer",
    "material_transit",
];

fn explicit_assignment_authority(agent: &Value) -> Option<&Value> {
    ["relation", "assignment"]
        .iter()
        .map(|key| obj(agent, key))
        .find(|value| value.is_object())
}

/// Emits known relation/flow links only when an explicit relation projection
/// carries all four semantic fields. `location_id` and geometry intentionally
/// remain endpoint data; they never establish relation authority on their own.
pub(super) fn build_pixel_world_links(
    agents: &[Value],
    location_by_id: &Map<String, Value>,
) -> Vec<Value> {
    agents
        .iter()
        .filter_map(|agent| {
            let relation = explicit_assignment_authority(agent)?;
            let kind = str_key(relation, "kind")?;
            if !AUTHORITATIVE_LINK_KINDS.contains(&kind)
                || str_key(relation, "status") != Some(AUTHORITATIVE_ASSIGNMENT_STATUS)
                || str_key(relation, "source_class") != Some(AUTHORITATIVE_SOURCE_CLASS)
                || str_key(relation, "freshness") != Some(AUTHORITATIVE_FRESHNESS)
            {
                return None;
            }
            let location_id = str_key(agent, "location_id")?;
            let location = location_by_id.get(location_id)?;
            let agent_pos = obj(agent, "pos");
            let location_pos = obj(location, "pos");
            if !agent_pos.is_object() || !location_pos.is_object() {
                return None;
            }
            let mut link = json!({
                "id": format!("link:{}:{location_id}", str_key(agent, "id").unwrap_or("")),
                "kind": kind,
                "from": agent_pos,
                "to": location_pos,
                "emphasis": 0.72,
                "status": AUTHORITATIVE_ASSIGNMENT_STATUS,
                "source_class": AUTHORITATIVE_SOURCE_CLASS,
                "freshness": AUTHORITATIVE_FRESHNESS,
            });
            if let Some(label) = str_key(relation, "label") {
                // Presentation text is optional and never changes the
                // authority envelope above. Host locale is carried alongside
                // RenderState so future generic route labels can use the same
                // producer supplied presentation boundary.
                link["label"] = Value::String(label.to_string());
            }
            Some(link)
        })
        .collect()
}
