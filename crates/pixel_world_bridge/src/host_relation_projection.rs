use serde_json::{Map, Value, json};

use super::{obj, str_key};

const AUTHORITATIVE_ASSIGNMENT_KIND: &str = "agent_assignment";
const AUTHORITATIVE_ASSIGNMENT_STATUS: &str = "active";
const AUTHORITATIVE_SOURCE_CLASS: &str = "runtime_projection";
const AUTHORITATIVE_FRESHNESS: &str = "current";

fn explicit_assignment_authority(agent: &Value) -> Option<&Value> {
    ["relation", "assignment"]
        .iter()
        .map(|key| obj(agent, key))
        .find(|value| value.is_object())
}

/// Emits assignment links only when an explicit relation projection carries
/// all four semantic fields.  `location_id` and geometry intentionally remain
/// endpoint data; they never establish assignment authority on their own.
pub(super) fn build_pixel_world_links(
    agents: &[Value],
    location_by_id: &Map<String, Value>,
) -> Vec<Value> {
    agents
        .iter()
        .filter_map(|agent| {
            let relation = explicit_assignment_authority(agent)?;
            if str_key(relation, "kind") != Some(AUTHORITATIVE_ASSIGNMENT_KIND)
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
            Some(json!({
                "id": format!("link:{}:{location_id}", str_key(agent, "id").unwrap_or("")),
                "kind": AUTHORITATIVE_ASSIGNMENT_KIND,
                "from": agent_pos,
                "to": location_pos,
                "emphasis": 0.72,
                "status": AUTHORITATIVE_ASSIGNMENT_STATUS,
                "source_class": AUTHORITATIVE_SOURCE_CLASS,
                "freshness": AUTHORITATIVE_FRESHNESS,
            }))
        })
        .collect()
}
