use super::*;

fn resolve_owner_position(owner: &Value, agents: &[Value], locations: &[Value]) -> Option<Value> {
    let owner_type = str_key(owner, "type")?;
    let data = obj(owner, "data");
    match owner_type {
        "Agent" => {
            let agent_id = str_key(data, "agent_id")?;
            agents
                .iter()
                .find(|agent| str_key(agent, "id") == Some(agent_id))
                .and_then(|agent| normalize_position(obj(agent, "pos")))
        }
        "Location" => {
            let location_id = str_key(data, "location_id")?;
            locations
                .iter()
                .find(|location| str_key(location, "id") == Some(location_id))
                .and_then(|location| normalize_position(obj(location, "pos")))
        }
        _ => None,
    }
}

pub(super) fn build_pixel_world_social_links(
    input: &Value,
    agents: &[Value],
    locations: &[Value],
) -> Vec<Value> {
    let model = obj(obj(input, "snapshot"), "model");
    let snapshot_time = obj(obj(input, "snapshot"), "time")
        .as_u64()
        .or_else(|| {
            obj(obj(input, "snapshot"), "time")
                .as_i64()
                .and_then(|value| value.try_into().ok())
        })
        .unwrap_or(0);
    let Some(edges) = obj(model, "social_edges").as_object() else {
        return Vec::new();
    };
    let mut projected = edges
        .values()
        .filter_map(|edge| {
            let lifecycle = str_key(edge, "lifecycle").unwrap_or("active");
            if lifecycle != "active"
                || obj(edge, "expires_at_tick")
                    .as_u64()
                    .or_else(|| {
                        obj(edge, "expires_at_tick")
                            .as_i64()
                            .and_then(|value| value.try_into().ok())
                    })
                    .is_some_and(|expires_at| expires_at <= snapshot_time)
            {
                return None;
            }
            let from = resolve_owner_position(obj(edge, "from"), agents, locations)?;
            let to = resolve_owner_position(obj(edge, "to"), agents, locations)?;
            let edge_id = edge
                .get("edge_id")
                .and_then(Value::as_u64)
                .or_else(|| str_key(edge, "edge_id").and_then(|id| id.parse().ok()))?;
            Some(json!({
                "id": format!("social_edge:{edge_id}"),
                "from": from,
                "to": to,
                "relation_kind": str_key(edge, "relation_kind").unwrap_or("social"),
                "schema_id": str_key(edge, "schema_id").unwrap_or(""),
                "weight_bps": number_key(edge, "weight_bps", 0.0),
                "lifecycle": lifecycle,
            }))
        })
        .collect::<Vec<_>>();
    projected.sort_by(|left, right| str_key(left, "id").cmp(&str_key(right, "id")));
    projected
}
