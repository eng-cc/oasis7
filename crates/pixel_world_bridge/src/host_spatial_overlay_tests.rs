use super::*;
fn fixture() -> Value {
    let identity = json!({"world_id":"w","reorg_epoch":"1","event_seq":"2"});
    json!({"worldView":{"world_id":"w","reorg_epoch":"1","source_cut":"C",
        "aggregates":[{"aggregate_id":"region","label":"Industry","member_count":"42","bounds":{"min_x_cm":"0","max_x_cm":"20","min_y_cm":"0","max_y_cm":"20"},"position":{"x_cm":"10","y_cm":"10","z_cm":"0"},"provenance":{"geometry_source":"display_aggregation","freshness":"current","valid_at_source_cut":"C"}}],
        "events":[{"event_id":"w:1:2","identity":identity,"feed_ref":identity,"kind":"published","label":"Published","position":{"x_cm":"30","y_cm":"40","z_cm":"0"},"source_class":"runtime_journal_payload","source_cut":"C","freshness":"current"}]}})
}
fn world() -> Value {
    json!({"width_cm":100,"depth_cm":100})
}
#[test]
fn explicit_display_types_keep_aggregate_count_and_event_original_geometry() {
    let (aggregates, events) = project(&fixture(), &world());
    assert_eq!(aggregates[0]["member_count"], "42");
    assert_eq!(events[0]["pos"]["x_cm"], 30.0);
    assert!(events[0].get("target_agent_id").is_none());
    assert!(aggregates[0].get("agent_id").is_none());
}
#[test]
fn invalid_world_epoch_cut_feed_reference_and_unknown_geometry_never_render() {
    for (field, value) in [
        ("source_cut", json!("old")),
        ("source_class", json!("inferred")),
        ("freshness", json!("unknown")),
        ("position", Value::Null),
        (
            "feed_ref",
            json!({"world_id":"w","reorg_epoch":"0","event_seq":"2"}),
        ),
    ] {
        let mut input = fixture();
        input["worldView"]["events"][0][field] = value;
        assert!(project(&input, &world()).1.is_empty(), "{field}");
    }
    for (field, value) in [
        ("world_id", json!("other")),
        ("reorg_epoch", json!("0")),
        ("event_seq", json!("-1")),
    ] {
        let mut input = fixture();
        input["worldView"]["events"][0]["identity"][field] = value.clone();
        input["worldView"]["events"][0]["feed_ref"][field] = value;
        assert!(project(&input, &world()).1.is_empty(), "{field}");
    }
    let mut input = fixture();
    input["worldView"]["events"][0]["position"] = json!({"x_cm":"200","y_cm":"0","z_cm":"0"});
    input["worldView"]["events"][0]["target"] =
        json!({"kind":"agent","id":"nearby","position":{"x_cm":10,"y_cm":10,"z_cm":0}});
    assert!(project(&input, &world()).1.is_empty());
}
#[test]
fn stale_aggregate_is_hidden_but_explicit_historical_event_point_remains_historical() {
    let mut input = fixture();
    input["worldViewStale"] = json!(true);
    let (aggregates, events) = project(&input, &world());
    assert!(aggregates.is_empty());
    assert_eq!(events[0]["historical"], true);
    input = fixture();
    input["worldView"]["aggregates"][0]["provenance"]["valid_at_source_cut"] = json!("old");
    assert!(project(&input, &world()).0.is_empty());
}

#[test]
fn event_hit_identity_cannot_be_forged_or_collide_with_another_feed_identity() {
    let mut input = fixture();
    input["worldView"]["events"][0]["event_id"] = json!("w:1:3");
    assert!(project(&input, &world()).1.is_empty());
    input = fixture();
    let mut conflicting = input["worldView"]["events"][0].clone();
    conflicting["identity"]["event_seq"] = json!("3");
    conflicting["feed_ref"]["event_seq"] = json!("3");
    input["worldView"]["events"]
        .as_array_mut()
        .unwrap()
        .push(conflicting);
    let events = project(&input, &world()).1;
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["id"], "w:1:2");
}

#[test]
fn equivalent_event_duplicates_coalesce_but_conflicting_original_points_are_suppressed() {
    let mut input = fixture();
    let event = input["worldView"]["events"][0].clone();
    input["worldView"]["events"]
        .as_array_mut()
        .unwrap()
        .push(event);
    assert_eq!(project(&input, &world()).1.len(), 1);
    input["worldView"]["events"][1]["position"]["x_cm"] = json!("35");
    assert!(project(&input, &world()).1.is_empty());
}
