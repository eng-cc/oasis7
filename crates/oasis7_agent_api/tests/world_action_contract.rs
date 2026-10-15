use oasis7_agent_api::world_action::{Action, GeoPos, MaterialLedgerId};

#[test]
fn action_contract_preserves_typed_json_and_canonical_preimage() {
    let action = Action::MoveAgent {
        agent_id: "agent".into(),
        to: GeoPos::new(1, 2, 3),
    };
    let wire = serde_json::json!({"type":"MoveAgent","data":{
        "agent_id":"agent","to":{"x_cm":1,"y_cm":2,"z_cm":3}
    }});
    assert_eq!(serde_json::to_value(&action).unwrap(), wire);
    let decoded: Action = serde_json::from_value(wire).unwrap();
    assert_eq!(decoded, action);
    assert_eq!(action.actor_id(), Some("agent"));
    assert_eq!(
        oasis7_wasm_abi::encode_canonical_cbor(&decoded).unwrap(),
        oasis7_wasm_abi::encode_canonical_cbor(&action).unwrap()
    );
}

#[test]
fn coordinate_and_ledger_compatibility_rules_are_preserved() {
    let point: GeoPos = serde_json::from_str(r#"{"x_cm":1.0,"y_cm":2,"z_cm":3}"#).unwrap();
    assert_eq!(point, GeoPos::new(1, 2, 3));
    assert!(serde_json::from_str::<GeoPos>(r#"{"x_cm":1.5,"y_cm":2,"z_cm":3}"#).is_err());
    for value in ["world", "agent:owner", "site:s", "factory:smelter"] {
        let ledger: MaterialLedgerId = serde_json::from_value(serde_json::json!(value)).unwrap();
        assert_eq!(serde_json::to_value(ledger).unwrap(), value);
    }
    assert!(serde_json::from_value::<MaterialLedgerId>(serde_json::json!("agent:")).is_err());
}
