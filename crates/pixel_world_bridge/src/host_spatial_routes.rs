use super::{arr, obj, str_key};
use serde_json::{Value, json};

// Geometry never proves a logistics relation. Both the published relation and
// each visible endpoint must survive the same authorized projection.
pub(super) fn project(
    input: &Value,
    agents: &[Value],
    locations: &[Value],
    facilities: &[Value],
) -> Vec<Value> {
    let view = obj(input, "worldView");
    if input.get("worldViewStale").and_then(Value::as_bool) == Some(true) {
        return Vec::new();
    }
    let endpoint = |reference: &Value| {
        let id = str_key(reference, "id")?;
        let list = match str_key(reference, "kind")? {
            "agent" => agents,
            "location" => locations,
            "factory" | "module_visual" => facilities,
            _ => return None,
        };
        list.iter()
            .find(|entry| str_key(entry, "id") == Some(id))
            .and_then(|entry| {
                obj(entry, "pos")
                    .as_object()
                    .map(|_| obj(entry, "pos").clone())
            })
    };
    arr(view,"relations").iter().filter_map(|relation| {
        let kind = str_key(relation,"kind")?;
        if !matches!(kind,"route"|"logistics_route"|"supply_route"|"delivery_route")
            || str_key(relation,"status") != Some("active")
            || str_key(relation,"source_class") != Some("runtime_projection")
            || str_key(relation,"freshness") != Some("current")
            || relation.get("valid_at_source_cut").filter(|cut|cut.is_string()) != view.get("source_cut") { return None; }
        Some(json!({"id":str_key(relation,"relation_id")?,"kind":kind,"from":endpoint(obj(relation,"from"))?,"to":endpoint(obj(relation,"to"))?,
            "status":"active","source_class":"runtime_projection","freshness":"current","emphasis":0.72,"label":relation.get("label")}))
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn render_state_projects_factory_selection_and_explicit_factory_route() {
        let provenance = json!({"geometry_source":"authoritative","freshness":"current","valid_at_source_cut":"C"});
        let input = json!({"selectedKind":"factory","selectedId":"F","snapshot":{"model":{},"config":{"space":{"width_cm":100,"depth_cm":100,"height_cm":10}}},
            "worldView":{"source_cut":"C","entities":[
                {"entity_id":"L","kind":"location","render_record":{"id":"L","name":"Site","pos":{"x_cm":30,"y_cm":40,"z_cm":0}},"provenance":provenance},
                {"entity_id":"A","kind":"agent","render_record":{"id":"A","name":"Courier","pos":{"x_cm":10,"y_cm":20,"z_cm":0}},"provenance":provenance},
                {"entity_id":"F","kind":"factory","render_record":{"name":"Forge","location_id":"L"},"provenance":{"geometry_source":"location_derived","freshness":"current","valid_at_source_cut":"C"}}
            ],"relations":[{"relation_id":"R","kind":"logistics_route","status":"active","source_class":"runtime_projection","freshness":"current","valid_at_source_cut":"C","from":{"kind":"agent","id":"A"},"to":{"kind":"factory","id":"F"}}]}});
        let rendered = super::super::build_render_state(&input);
        assert_eq!(rendered["module_visual_entities"][0]["id"], "F");
        assert_eq!(rendered["module_visual_entities"][0]["pos"]["x_cm"], 30.0);
        assert_eq!(rendered["selection"]["kind"], "module_visual");
        assert_eq!(rendered["links"][0]["id"], "R");
        assert_eq!(rendered["links"][0]["to"]["x_cm"], 30.0);
    }

    #[test]
    fn routes_need_explicit_same_cut_authority_and_authorized_endpoints() {
        let agents = vec![json!({"id":"A","pos":{"x_cm":10,"y_cm":20,"z_cm":0}})];
        let facilities = vec![json!({"id":"F","pos":{"x_cm":30,"y_cm":40,"z_cm":0}})];
        let input = json!({"worldView":{"source_cut":"C","relations":[{"relation_id":"R","kind":"route","status":"active","source_class":"runtime_projection","freshness":"current","valid_at_source_cut":"C","from":{"kind":"agent","id":"A"},"to":{"kind":"factory","id":"F"}}]}});
        assert_eq!(project(&input, &agents, &[], &facilities).len(), 1);
        assert!(project(&input, &agents, &[], &[]).is_empty());
        for (field, value) in [
            ("valid_at_source_cut", json!("old")),
            ("freshness", json!("expired")),
            ("source_class", json!("inferred")),
            ("status", json!("inactive")),
        ] {
            let mut invalid = input.clone();
            invalid["worldView"]["relations"][0][field] = value;
            assert!(project(&invalid, &agents, &[], &facilities).is_empty());
        }
        assert!(
            project(
                &json!({"worldView":{"source_cut":"C"}}),
                &agents,
                &[],
                &facilities
            )
            .is_empty()
        );
    }
}
