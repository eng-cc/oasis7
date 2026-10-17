use super::{arr, obj, str_key};
use serde_json::{Value, json};

fn coordinate(value: &Value) -> Option<f64> {
    let n = value.as_f64().or_else(|| value.as_str()?.parse().ok())?;
    (n.is_finite() && n.abs() <= 9_007_199_254_740_991.0).then_some(n)
}
fn point(value: &Value, world: &Value) -> Option<Value> {
    let x = coordinate(&value["x_cm"])?;
    let y = coordinate(&value["y_cm"])?;
    let z = coordinate(&value["z_cm"])?;
    let width = coordinate(&world["width_cm"])?;
    let depth = coordinate(&world["depth_cm"])?;
    (x >= 0.0 && y >= 0.0 && x <= width && y <= depth).then(|| json!({"x_cm":x,"y_cm":y,"z_cm":z}))
}
fn nonempty<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    str_key(value, key).filter(|s| !s.trim().is_empty())
}
fn decimal(value: &Value) -> Option<&str> {
    value.as_str().filter(|s| {
        !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()) && s.parse::<u64>().is_ok()
    })
}

pub(super) fn project(input: &Value, world: &Value) -> (Vec<Value>, Vec<Value>) {
    let view = obj(input, "worldView");
    let Some(cut) = nonempty(view, "source_cut") else {
        return (vec![], vec![]);
    };
    let stale = input["worldViewStale"].as_bool().unwrap_or(false);
    let aggregates=arr(view,"aggregates").iter().filter_map(|a|{
        let provenance=obj(a,"provenance");
        if stale || str_key(provenance,"geometry_source")!=Some("display_aggregation") || str_key(provenance,"freshness")!=Some("current") || str_key(provenance,"valid_at_source_cut")!=Some(cut) {return None;}
        let pos=point(obj(a,"position"),world)?;
        let b=obj(a,"bounds");
        let x0=coordinate(&b["min_x_cm"])?; let x1=coordinate(&b["max_x_cm"])?;
        let y0=coordinate(&b["min_y_cm"])?; let y1=coordinate(&b["max_y_cm"])?;
        let x=pos["x_cm"].as_f64()?;let y=pos["y_cm"].as_f64()?;
        if x0>x1 || y0>y1 || x<x0 || x>x1 || y<y0 || y>y1 {return None;}
        let count=decimal(&a["member_count"])?;
        Some(json!({"id":nonempty(a,"aggregate_id")?,"label":nonempty(a,"label").unwrap_or(""),"member_count":count,"pos":pos,"historical":false}))
    }).collect();
    let events:Vec<Value>=arr(view,"events").iter().filter_map(|event|{
        let identity=obj(event,"identity");
        let epoch=decimal(&identity["reorg_epoch"])?; let sequence=decimal(&identity["event_seq"])?;
        if nonempty(identity,"world_id")!=nonempty(view,"world_id") || nonempty(identity,"world_id").is_none()
            || Some(epoch)!=decimal(&view["reorg_epoch"]) || event.get("feed_ref")!=Some(identity)
            || str_key(event,"source_class")!=Some("runtime_journal_payload") || str_key(event,"source_cut")!=Some(cut) {return None;}
        let world_id=nonempty(identity,"world_id")?;
        let canonical_id=format!("{world_id}:{epoch}:{sequence}");
        if str_key(event,"event_id")!=Some(canonical_id.as_str()) {return None;}
        let freshness=str_key(event,"freshness")?;
        if !matches!(freshness,"current"|"last_known") {return None;}
        Some(json!({"id":nonempty(event,"event_id")?,"label":nonempty(event,"label").unwrap_or(""),"pos":point(obj(event,"position"),world)?,"historical":stale || freshness=="last_known","member_count":Value::Null}))
    }).collect();
    let mut unique = std::collections::BTreeMap::<String, Option<Value>>::new();
    for event in events {
        let id = event["id"]
            .as_str()
            .expect("validated event identity")
            .to_owned();
        match unique.entry(id) {
            std::collections::btree_map::Entry::Vacant(slot) => {
                slot.insert(Some(event));
            }
            std::collections::btree_map::Entry::Occupied(mut slot) => {
                if slot.get().as_ref() != Some(&event) {
                    slot.insert(None);
                }
            }
        }
    }
    (aggregates, unique.into_values().flatten().collect())
}

#[cfg(test)]
#[path = "host_spatial_overlay_tests.rs"]
mod tests;
