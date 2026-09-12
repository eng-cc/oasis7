use super::*;
use crate::geometry::GeoPos;
use crate::runtime::WorldEventBody;
use oasis7_wasm_abi::ModuleEmitEvent;

const MODULE_ID: &str = "fixture.visual-module";
const ENTITY_ID: &str = "module-relay";

fn visual_event(kind: &str, payload: serde_json::Value, trace_id: &str) -> WorldEventBody {
    WorldEventBody::ModuleEmitted(ModuleEmitEvent {
        module_id: MODULE_ID.to_string(),
        trace_id: trace_id.to_string(),
        kind: kind.to_string(),
        payload,
    })
}

fn visual_upsert_event() -> WorldEventBody {
    visual_event(
        "ModuleVisualEntityUpserted",
        serde_json::json!({
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
        "trace-server-upsert",
    )
}

fn visual_remove_event() -> WorldEventBody {
    visual_event(
        "ModuleVisualEntityRemoved",
        serde_json::json!({ "entity_id": ENTITY_ID }),
        "trace-server-remove",
    )
}

#[test]
fn production_server_snapshot_projects_visual_upsert_remove_and_recovery() {
    let mut server =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .expect("runtime live server");
    let baseline = server.world.snapshot();

    server
        .world
        .append_event_for_test(visual_upsert_event(), None)
        .expect("publish production visual upsert");
    let upserted = server.compat_snapshot(None);
    assert_eq!(
        upserted.model.module_visual_entities[ENTITY_ID].module_id,
        MODULE_ID
    );

    server
        .world
        .append_event_for_test(visual_remove_event(), None)
        .expect("publish production visual remove");
    let removed = server.compat_snapshot(None);
    assert!(!removed.model.module_visual_entities.contains_key(ENTITY_ID));

    let replayed = crate::runtime::World::from_snapshot(baseline, server.world.journal().clone())
        .expect("recover production visual journal");
    assert!(
        !replayed
            .state()
            .module_visual_entities
            .contains_key(ENTITY_ID)
    );
}
