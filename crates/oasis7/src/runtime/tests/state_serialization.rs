use crate::runtime::WorldState;

#[test]
fn empty_starter_oc_claims_do_not_change_legacy_state_projection_json() {
    let state = WorldState::default();
    let value = serde_json::to_value(&state).expect("serialize world state");
    let object = value.as_object().expect("world state serializes as object");

    assert!(
        !object.contains_key("starter_oc_claims"),
        "empty starter OC claims must stay out of canonical state JSON so old state roots remain stable"
    );
}

#[test]
fn legacy_world_state_json_defaults_empty_starter_oc_claims() {
    let mut value = serde_json::to_value(WorldState::default()).expect("serialize world state");
    let object = value
        .as_object_mut()
        .expect("world state serializes as object");
    object.remove("starter_oc_claims");

    let decoded: WorldState = serde_json::from_value(value).expect("decode legacy world state");

    assert!(decoded.starter_oc_claims.is_empty());
}
