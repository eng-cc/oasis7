use super::*;

pub(super) fn build_world_bounds(input: &Value) -> Value {
    let space = obj(obj(input, "snapshot"), "config")
        .get("space")
        .unwrap_or(&Value::Null);
    let Some(width_cm) = space.get("width_cm").and_then(Value::as_f64) else {
        return Value::Null;
    };
    let Some(depth_cm) = space.get("depth_cm").and_then(Value::as_f64) else {
        return Value::Null;
    };
    let Some(height_cm) = space.get("height_cm").and_then(Value::as_f64) else {
        return Value::Null;
    };
    if !width_cm.is_finite()
        || !depth_cm.is_finite()
        || !height_cm.is_finite()
        || width_cm <= 0.0
        || depth_cm <= 0.0
        || height_cm <= 0.0
    {
        return Value::Null;
    }
    json!({
        "width_cm": width_cm,
        "depth_cm": depth_cm,
        "height_cm": height_cm,
    })
}
