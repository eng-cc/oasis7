use super::*;

pub(super) fn recognized_metadata(bytes: &[u8]) -> bool {
    serde_cbor::from_slice::<serde_cbor::Value>(bytes)
        .ok()
        .is_some_and(|value| match value {
            serde_cbor::Value::Map(map) => {
                map.contains_key(&serde_cbor::Value::Text(
                    "gameplay_submission_origin".into(),
                )) || map.get(&serde_cbor::Value::Text("version".into()))
                    == Some(&serde_cbor::Value::Integer(2))
            }
            _ => false,
        })
}

pub(super) fn validate_metadata_presence(bytes: &[u8]) -> Result<(), String> {
    if serde_cbor::from_slice::<serde_cbor::Value>(bytes)
        .ok()
        .is_some_and(|value| match value {
            serde_cbor::Value::Map(map) => {
                map.get(&serde_cbor::Value::Text(
                    "gameplay_submission_origin".into(),
                )) == Some(&serde_cbor::Value::Null)
            }
            _ => false,
        })
    {
        return Err("gameplay submission origin must not be null".into());
    }
    Ok(())
}

pub(super) fn validate_envelope(
    envelope: &LocalConsensusActionPayloadEnvelope,
) -> Result<(), String> {
    match (
        envelope.version,
        envelope.gameplay_submission_origin.as_ref(),
    ) {
        (1, None) => Ok(()),
        (2, Some(origin)) => {
            let LocalConsensusActionPayloadBody::RuntimeAction { action } = &envelope.body else {
                return Err("gameplay origin requires Runtime ScheduleRecipe".into());
            };
            if local_runtime_action_kind(action) != Some("ScheduleRecipe") {
                return Err("gameplay origin requires ScheduleRecipe".into());
            }
            let data = local_runtime_action_data(action)?;
            let fields = ["requester_agent_id", "factory_id", "recipe_id"]
                .map(|name| data.get(name).and_then(JsonValue::as_str));
            let [Some(requester), Some(factory), Some(recipe)] = fields else {
                return Err("gameplay origin requires typed recipe identity".into());
            };
            if !origin.matches_recipe(requester, factory, recipe) {
                return Err("gameplay origin recipe identity mismatch".into());
            }
            Ok(())
        }
        _ => Err(format!(
            "unsupported consensus payload envelope version/origin combination {}",
            envelope.version
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn envelope() -> JsonValue {
        serde_json::json!({"version": 2, "gameplay_submission_origin": {
            "verified_player_id":"browser-player", "public_key":"a".repeat(64), "auth_nonce":7,
            "requester_agent_id":"builder-a", "factory_id":"factory.test", "recipe_id":"recipe.test"
        }, "body":{"type":"runtime_action", "data":{"action":{"type":"ScheduleRecipe","data":{
            "requester_agent_id":"builder-a","factory_id":"factory.test","recipe_id":"recipe.test",
            "plan":{"accepted_batches":1,"consume":[],"produce":[],"byproducts":[],"power_required":0,"duration_ticks":1},
            "logistics_route_ids":[],"logistics_path_ids":[]
        }}}}})
    }
    fn decode(value: &JsonValue) -> Result<LocalConsensusActionPayloadEnvelope, String> {
        decode_local_consensus_action_payload_envelope(&serde_cbor::to_vec(value).unwrap())
    }
    #[test]
    fn committed_recipe_origin_node_v2_is_strict_and_v1_legacy_still_decodes() {
        let valid = envelope();
        assert!(decode(&valid).is_ok(), "{:?}", decode(&valid));
        for change in [0, 1, 2, 3, 4, 5, 6, 7] {
            let mut raw = valid.clone();
            match change {
                0 => raw["version"] = serde_json::json!(1),
                1 => {
                    raw.as_object_mut()
                        .unwrap()
                        .remove("gameplay_submission_origin");
                }
                2 => raw["gameplay_submission_origin"] = JsonValue::Null,
                3 => raw["gameplay_submission_origin"]["unknown"] = serde_json::json!(true),
                4 => raw["gameplay_submission_origin"]["auth_nonce"] = serde_json::json!(true),
                5 => {
                    raw["gameplay_submission_origin"]["requester_agent_id"] =
                        serde_json::json!("other")
                }
                6 => raw["body"]["data"]["action"]["type"] = serde_json::json!("RegisterAgent"),
                _ => raw["body"] = JsonValue::Null,
            }
            assert!(decode(&raw).is_err(), "malformed v2 case {change}");
        }
        let mut legacy = valid;
        legacy["version"] = serde_json::json!(1);
        legacy
            .as_object_mut()
            .unwrap()
            .remove("gameplay_submission_origin");
        assert!(
            decode(&legacy)
                .unwrap()
                .gameplay_submission_origin
                .is_none()
        );
        let plain = legacy["body"]["data"]["action"].clone();
        assert!(decode(&plain).is_ok());
        let mut hybrid = plain.clone();
        hybrid["version"] = serde_json::json!(2);
        hybrid["body"] = JsonValue::Null;
        assert!(
            decode(&hybrid).is_err(),
            "recognized malformed v2 must not fall back to raw action"
        );
    }
}
