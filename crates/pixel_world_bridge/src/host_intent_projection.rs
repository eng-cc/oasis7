use serde_json::{Value, json};

use super::{normalize_position, obj, str_key};

fn nonnegative_integer(value: &Value) -> Option<u64> {
    match value {
        Value::Number(number) => number.as_u64(),
        Value::String(value) => value.parse::<u64>().ok(),
        _ => None,
    }
}

fn required_position_field(intent: &Value, key: &str) -> bool {
    nonnegative_integer(obj(intent, key)).is_some()
}

fn has_snapshot_authoritative_position(agent: &Value) -> bool {
    str_key(agent, "position_source") == Some("snapshot")
        && normalize_position(obj(agent, "pos")).is_some()
}

/// Projects only an active Intent whose complete v2 authority envelope and
/// exact snapshot position survived the host boundary.  In particular,
/// location-derived positions are not sufficient for an Intent cue.
pub(super) fn project_active_intent_target(
    input: &Value,
    world_bounds: &Value,
    agents: &[Value],
) -> Value {
    if !world_bounds.is_object() {
        return Value::Null;
    }
    let intent = obj(
        obj(obj(input, "snapshot"), "player_gameplay"),
        "primary_intent",
    );
    if obj(intent, "schema_version").as_u64() != Some(2)
        || str_key(intent, "source_class") != Some("runtime_projection")
        || str_key(intent, "freshness") != Some("current")
        || str_key(intent, "control_state") != Some("controllable")
        || !matches!(
            str_key(intent, "status"),
            Some("submitted" | "accepted" | "blocked")
        )
        || str_key(intent, "intent_id").is_none()
        || str_key(intent, "agent_id").is_none()
        || str_key(intent, "world_id").is_none()
        || !required_position_field(intent, "reorg_epoch")
        || !required_position_field(intent, "logical_time")
        || !required_position_field(intent, "event_seq")
        || !required_position_field(intent, "updated_at")
    {
        return Value::Null;
    }

    let Some(agent_id) = str_key(intent, "agent_id") else {
        return Value::Null;
    };
    let Some(status) = str_key(intent, "status") else {
        return Value::Null;
    };
    if !agents.iter().any(|agent| {
        str_key(agent, "id") == Some(agent_id) && has_snapshot_authoritative_position(agent)
    }) {
        return Value::Null;
    }
    json!({ "agent_id": agent_id, "status": status })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_counters_are_accepted_but_float_counters_are_not() {
        assert_eq!(nonnegative_integer(&json!("12")), Some(12));
        assert_eq!(nonnegative_integer(&json!(12)), Some(12));
        assert_eq!(nonnegative_integer(&json!(12.5)), None);
        assert_eq!(nonnegative_integer(&json!(-1)), None);
    }
}
