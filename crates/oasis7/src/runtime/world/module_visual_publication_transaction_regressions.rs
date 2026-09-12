use super::*;
use crate::geometry::GeoPos;
use crate::runtime::WorldEventBody;
use oasis7_wasm_abi::ModuleEmitEvent;

const MODULE_ID: &str = "fixture.visual-module";
const ENTITY_ID: &str = "module-relay";

fn visual_upsert_body(module_id: &str) -> WorldEventBody {
    WorldEventBody::ModuleEmitted(ModuleEmitEvent {
        module_id: module_id.to_string(),
        trace_id: "trace-visual-upsert".to_string(),
        kind: "ModuleVisualEntityUpserted".to_string(),
        payload: serde_json::json!({
            "entity": {
                "entity_id": ENTITY_ID,
                "module_id": MODULE_ID,
                "kind": "relay",
                "label": "Relay",
                "anchor": {
                    "type": "absolute",
                    "data": { "pos": GeoPos::new(100, 200, 0) }
                }
            }
        }),
    })
}

fn visual_remove_body() -> WorldEventBody {
    WorldEventBody::ModuleEmitted(ModuleEmitEvent {
        module_id: MODULE_ID.to_string(),
        trace_id: "trace-visual-remove".to_string(),
        kind: "ModuleVisualEntityRemoved".to_string(),
        payload: serde_json::json!({ "entity_id": ENTITY_ID }),
    })
}

fn state_json(world: &World) -> serde_json::Value {
    serde_json::to_value(world.snapshot()).expect("encode runtime snapshot")
}

#[test]
fn production_module_visual_upsert_remove_persist_and_replay() {
    let mut world = World::new();
    let baseline = world.snapshot();

    world
        .append_event_for_test(visual_upsert_body(MODULE_ID), None)
        .expect("publish visual upsert");
    let updated = state_json(&world);
    assert_eq!(
        updated["state"]["module_visual_entities"][ENTITY_ID]["module_id"],
        MODULE_ID
    );

    world
        .append_event_for_test(visual_remove_body(), None)
        .expect("publish visual removal");
    let removed = state_json(&world);
    assert!(
        removed["state"]["module_visual_entities"]
            .get(ENTITY_ID)
            .is_none()
    );

    let replayed = World::from_snapshot(baseline, world.journal().clone())
        .expect("replay visual mutations from baseline");
    assert!(
        state_json(&replayed)["state"]["module_visual_entities"]
            .get(ENTITY_ID)
            .is_none()
    );
}

#[test]
fn production_module_visual_payload_requires_emitter_owned_module() {
    let mut world = World::new();
    let mut body = visual_upsert_body(MODULE_ID);
    let WorldEventBody::ModuleEmitted(event) = &mut body else {
        unreachable!()
    };
    event.payload["entity"]["module_id"] = serde_json::json!("different-module");

    assert!(world.append_event_for_test(body, None).is_err());
}

#[test]
fn production_module_visual_payload_requires_module_and_anchor_fields() {
    for field in ["module_id", "anchor"] {
        let mut world = World::new();
        let mut body = visual_upsert_body(MODULE_ID);
        let WorldEventBody::ModuleEmitted(event) = &mut body else {
            unreachable!()
        };
        event.payload["entity"]
            .as_object_mut()
            .unwrap()
            .remove(field);

        assert!(
            world.append_event_for_test(body, None).is_err(),
            "missing {field} must reject the durable mutation"
        );
    }
}

#[test]
fn production_module_visual_remove_requires_existing_owner() {
    let mut world = World::new();
    world
        .append_event_for_test(visual_upsert_body(MODULE_ID), None)
        .expect("publish visual upsert");

    let mut remove = visual_remove_body();
    let WorldEventBody::ModuleEmitted(event) = &mut remove else {
        unreachable!()
    };
    event.module_id = "different-module".to_string();
    assert!(world.append_event_for_test(remove, None).is_err());
}
