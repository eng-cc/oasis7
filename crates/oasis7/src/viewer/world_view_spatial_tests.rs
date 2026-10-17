use super::*;
use crate::{
    geometry::GeoPos,
    simulator::{
        Agent, Factory, Location, ModuleVisualAnchor, ModuleVisualEntity, ResourceOwner,
        WorldKernel,
    },
};
fn fixture() -> WorldSnapshot {
    let mut snapshot = WorldKernel::new().snapshot();
    snapshot.model.agents.clear();
    snapshot.model.locations.clear();
    snapshot.model.factories.clear();
    snapshot
        .model
        .agents
        .insert("a".into(), Agent::new("a", "l", GeoPos::new(0, 0, 0)));
    snapshot.model.locations.insert(
        "l".into(),
        Location::new("l", "site", GeoPos::new(200, 200, 0)),
    );
    snapshot.model.factories.insert(
        "f".into(),
        Factory {
            id: "f".into(),
            owner: ResourceOwner::Agent {
                agent_id: "a".into(),
            },
            location_id: "l".into(),
            kind: "smelter".into(),
        },
    );
    let mut runtime =
        crate::runtime::World::new_with_state(crate::runtime::WorldState::default()).snapshot();
    runtime.state.module_visual_entities.insert(
        "f".into(),
        ModuleVisualEntity {
            entity_id: "f".into(),
            module_id: "industry".into(),
            kind: "factory".into(),
            label: None,
            anchor: ModuleVisualAnchor::Absolute {
                pos: GeoPos::new(1000, 1000, 0),
            },
        },
    );
    runtime.state.logistics_routes.insert(
        "r".into(),
        crate::runtime::LogisticsRouteV1 {
            route_id: "r".into(),
            from_ledger: crate::runtime::MaterialLedgerId::Agent("a".into()),
            to_ledger: crate::runtime::MaterialLedgerId::Factory("f".into()),
            kind: "road".into(),
            distance_km: 1,
            priority: Default::default(),
            owner_agent_id: "private-owner-marker".into(),
            available: true,
            capacity_units: 20,
            reserved_capacity_units: 4,
            tariff_electricity_per_unit: 2,
        },
    );
    runtime.state.agents.insert(
        "a".into(),
        crate::runtime::AgentCell::new(
            crate::models::AgentState::new("a", GeoPos::new(0, 0, 0)),
            0,
        ),
    );
    snapshot.time = runtime.state.time;
    snapshot.model.agents.get_mut("a").unwrap().resources =
        runtime.state.agents["a"].state.resources.clone();
    snapshot.runtime_snapshot = Some(runtime);
    snapshot
}
fn query() -> Value {
    json!({"request_generation":"q","page_size":200})
}
#[test]
fn factories_require_explicit_fixed_geometry_and_routes_require_visible_endpoints() {
    let mut source = fixture();
    let result = build_with_authority(&source, "w", 0, "public", query(), true);
    let f = result["entities"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["kind"] == "factory")
        .unwrap();
    assert_eq!(f["position"]["x_cm"], "1000");
    assert_eq!(f["provenance"]["freshness"], "current");
    assert_eq!(
        result["relations"][0]["from"],
        json!({"kind":"agent","id":"a"})
    );
    assert!(!result.to_string().contains("private-owner-marker"));
    source
        .runtime_snapshot
        .as_mut()
        .unwrap()
        .state
        .module_visual_entities
        .clear();
    let without_anchor = build_with_authority(&source, "w", 0, "public", query(), true);
    let f = without_anchor["entities"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["kind"] == "factory")
        .unwrap();
    assert_eq!(f["provenance"]["freshness"], "unknown");
    assert_eq!(without_anchor["relations"][0]["freshness"], "unknown");
    source.model.agents.clear();
    assert!(
        build_with_authority(&source, "w", 0, "public", query(), true)["relations"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}
#[test]
fn viewport_route_intersection_includes_both_endpoints_and_authorized_pins() {
    let source = fixture();
    let mut q = query();
    q["bounds"] = json!({"min_x_cm":"400","max_x_cm":"600","min_y_cm":"400","max_y_cm":"600"});
    q["pinned_entities"] = json!([{"kind":"location","id":"l"},{"kind":"agent","id":"hidden"}]);
    let result = build_with_authority(&source, "w", 0, "public", q, true);
    assert_eq!(result["relations"].as_array().unwrap().len(), 1);
    let ids: Vec<_> = result["entities"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["entity_id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, vec!["l", "a", "f"]);
    assert!(!result.to_string().contains("hidden"));
}
#[test]
fn joint_paging_never_duplicates_or_mixes_entity_and_relation_budgets() {
    let source = fixture();
    let mut q = query();
    q["page_size"] = json!(1);
    let mut objects = BTreeSet::new();
    loop {
        let result = build_with_authority(&source, "w", 0, "public", q.clone(), true);
        let entities = result["entities"].as_array().unwrap();
        let relations = result["relations"].as_array().unwrap();
        assert_eq!(entities.len() + relations.len(), 1);
        for e in entities {
            assert!(objects.insert(format!("{}:{}", e["kind"], e["entity_id"])));
        }
        for r in relations {
            assert!(objects.insert(format!("relation:{}", r["relation_id"])));
        }
        if result["next_cursor"].is_null() {
            break;
        }
        q["cursor"] = result["next_cursor"].clone();
    }
    assert_eq!(objects.len(), 4);
}
#[test]
fn stable_source_reuses_index_without_snapshot_and_permission_change_resets() {
    let source = fixture();
    let mut cache = None;
    // Local wrapper permits a synthetic simulator fixture; production uses a bound Runtime digest.
    let a = SourceAuthority::local_runtime();
    let first = build_session_bound(&mut cache, || source, "w", 0, "public", query(), a.clone());
    let mut q = query();
    q["bounds"] = json!({"min_x_cm":"0","max_x_cm":"20","min_y_cm":"0","max_y_cm":"20"});
    let next = build_session_bound(
        &mut cache,
        || panic!("same source viewport must reuse the indexed projection"),
        "w",
        0,
        "public",
        q,
        a,
    );
    assert_eq!(first["source_cut"], next["source_cut"]);
    let mut page = query();
    page["page_size"] = json!(1);
    let source = fixture();
    let first = build_session(&mut cache, || source, "w", 0, "public", page.clone(), true);
    page["cursor"] = first["next_cursor"].clone();
    let denied = build_session_bound(
        &mut cache,
        || panic!("invalid scope must fail before source access"),
        "w",
        0,
        "public",
        page,
        SourceAuthority::unknown(),
    );
    assert_eq!(denied["unavailable_reasons"][0], "cursor_reset_required");
}

#[test]
fn stable_bound_pagination_stays_current_but_advanced_source_freezes_old_cut() {
    use crate::viewer::world_view_authority::spatial_state_digest;
    let source = fixture();
    let authority = SourceAuthority::local_bound(
        spatial_state_digest(&source.runtime_snapshot.as_ref().unwrap().state).unwrap(),
    );
    let mut cache = None;
    let mut q = query();
    q["page_size"] = json!(1);
    let first = build_session_bound(
        &mut cache,
        || source.clone(),
        "w",
        0,
        "public",
        q.clone(),
        authority.clone(),
    );
    q["cursor"] = first["next_cursor"].clone();
    let second = build_session_bound(
        &mut cache,
        || panic!("continuation must not rebuild"),
        "w",
        0,
        "public",
        q.clone(),
        authority,
    );
    assert_eq!(second["source_frozen"], false);
    assert_eq!(second["entities"][0]["provenance"]["freshness"], "current");
    let mut advanced = source.runtime_snapshot.unwrap().state;
    advanced.time += 1;
    let later = SourceAuthority::local_bound(spatial_state_digest(&advanced).unwrap());
    let frozen = build_session_bound(
        &mut cache,
        || panic!("use frozen old cut"),
        "w",
        0,
        "public",
        q,
        later,
    );
    assert_eq!(frozen["source_cut"], first["source_cut"]);
    assert_eq!(frozen["source_frozen"], true);
    assert_eq!(
        frozen["entities"][0]["provenance"]["freshness"],
        "last_known"
    );
}
#[test]
fn pinned_target_is_on_first_page_even_when_lexically_last_and_outside_viewport() {
    let mut source = fixture();
    for i in 0..1001 {
        let id = format!("agent-{i:04}");
        source
            .model
            .agents
            .insert(id.clone(), Agent::new(id, "l", GeoPos::new(0, 0, 0)));
    }
    source.model.agents.insert(
        "z-target".into(),
        Agent::new("z-target", "l", GeoPos::new(999, 999, 0)),
    );
    let mut q = query();
    q["bounds"] = json!({"min_x_cm":"0","max_x_cm":"10","min_y_cm":"0","max_y_cm":"10"});
    q["pinned_entities"] = json!([{"kind":"agent","id":"z-target"}]);
    let view = build_with_authority(&source, "w", 0, "public", q, true);
    assert_eq!(view["entities"][0]["entity_id"], "z-target");
    assert_eq!(
        view["entities"].as_array().unwrap().len() + view["relations"].as_array().unwrap().len(),
        200
    );
    assert_eq!(view["completeness"], "partial");
}
#[test]
fn site_ledger_resolves_distinct_location_identity_and_inactive_site_does_not_project() {
    let mut source = fixture();
    let state = &mut source.runtime_snapshot.as_mut().unwrap().state;
    state.factory_site_authorities.insert(
        "site-S".into(),
        crate::runtime::FactorySiteAuthorityV1 {
            site_id: "site-S".into(),
            location_id: "l".into(),
            active: true,
            ..Default::default()
        },
    );
    state.logistics_routes.get_mut("r").unwrap().from_ledger =
        crate::runtime::MaterialLedgerId::Site("site-S".into());
    let view = build_with_authority(&source, "w", 0, "public", query(), true);
    assert_eq!(
        view["relations"][0]["from"],
        json!({"kind":"location","id":"l"})
    );
    source
        .runtime_snapshot
        .as_mut()
        .unwrap()
        .state
        .factory_site_authorities
        .get_mut("site-S")
        .unwrap()
        .active = false;
    assert!(
        build_with_authority(&source, "w", 0, "public", query(), true)["relations"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}
