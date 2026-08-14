use serde_json::{Map, Value, json};

fn u64_key(value: &Value, snake_key: &str, camel_key: &str) -> u64 {
    [snake_key, camel_key]
        .into_iter()
        .find_map(|key| {
            value.get(key).and_then(|value| {
                value
                    .as_u64()
                    .or_else(|| value.as_i64().and_then(|value| value.try_into().ok()))
            })
        })
        .unwrap_or(0)
}

fn i64_key(value: &Value, snake_key: &str, camel_key: &str) -> i64 {
    [snake_key, camel_key]
        .into_iter()
        .find_map(|key| {
            value.get(key).and_then(|value| {
                value
                    .as_i64()
                    .or_else(|| value.as_u64().and_then(|value| value.try_into().ok()))
            })
        })
        .unwrap_or(0)
}

fn object_key(value: &Value, snake_key: &str, camel_key: &str) -> Map<String, Value> {
    value
        .get(snake_key)
        .and_then(Value::as_object)
        .or_else(|| value.get(camel_key).and_then(Value::as_object))
        .cloned()
        .unwrap_or_default()
}

pub(super) fn fields(value: &Value) -> Map<String, Value> {
    Map::from_iter([
        (
            "inventory_revision".to_string(),
            json!(u64_key(value, "inventory_revision", "inventoryRevision")),
        ),
        (
            "available_units_by_kind".to_string(),
            Value::Object(object_key(
                value,
                "available_units_by_kind",
                "availableUnitsByKind",
            )),
        ),
        (
            "throughput_epoch".to_string(),
            json!(u64_key(value, "throughput_epoch", "throughputEpoch")),
        ),
        (
            "throughput_remaining_units".to_string(),
            json!(i64_key(
                value,
                "throughput_remaining_units",
                "throughputRemainingUnits",
            )),
        ),
        (
            "throughput_limit_units_per_epoch".to_string(),
            json!(i64_key(
                value,
                "throughput_limit_units_per_epoch",
                "throughputLimitUnitsPerEpoch",
            )),
        ),
    ])
}
