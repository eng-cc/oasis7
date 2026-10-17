//! Bounded projection of the connection's existing visible snapshot.
//! This does not grant a private Agent audience or fabricate Runtime freshness.
use crate::simulator::WorldSnapshot;
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Query {
    request_generation: String,
    #[serde(default)]
    bounds: Option<Bounds>,
    #[serde(default)]
    search: Option<String>,
    page_size: usize,
    #[serde(default)]
    cursor: Option<String>,
}
#[derive(Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct Bounds {
    min_x_cm: String,
    max_x_cm: String,
    min_y_cm: String,
    max_y_cm: String,
}

pub(super) fn build(
    snapshot: &WorldSnapshot,
    world: &str,
    epoch: u64,
    audience: &str,
    query: Value,
) -> Value {
    build_with_authority(snapshot, world, epoch, audience, query, false)
}

pub(super) fn build_with_authority(
    snapshot: &WorldSnapshot,
    world: &str,
    epoch: u64,
    audience: &str,
    query: Value,
    local_runtime: bool,
) -> Value {
    let Some(prepared) = prepare(snapshot, world, epoch, audience, local_runtime) else {
        return json!({"world_id":world,"request_generation":query.get("request_generation"),"completeness":"unknown","entities":[],"unavailable_reasons":["projection_limit_exceeded"]});
    };
    build_inner(&snapshot.config.space, world, query, &prepared, false)
}

fn build_inner(
    space: &crate::simulator::SpaceConfig,
    world: &str,
    query: Value,
    prepared: &Prepared,
    source_frozen: bool,
) -> Value {
    let generation = query
        .get("request_generation")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();
    let unavailable = |reason: &str| {
        json!({"world_id":world,"scope_id":"viewer_snapshot","request_generation":generation,
        "completeness":"unknown","entities":[],"relations":[],"events":[],"unavailable_reasons":[reason]})
    };
    let Ok(query) = serde_json::from_value::<Query>(query) else {
        return unavailable("invalid_query");
    };
    if query.page_size == 0
        || query.page_size > 200
        || query.request_generation.len() > 64
        || query.search.as_ref().is_some_and(|s| s.len() > 128)
    {
        return unavailable("invalid_query");
    }
    let bounds = if let Some(b) = &query.bounds {
        let parsed = [&b.min_x_cm, &b.max_x_cm, &b.min_y_cm, &b.max_y_cm].map(|v| v.parse::<i64>());
        let [Ok(x0), Ok(x1), Ok(y0), Ok(y1)] = parsed else {
            return unavailable("invalid_bounds");
        };
        if x0 > x1 || y0 > y1 {
            return unavailable("invalid_bounds");
        }
        Some((x0, x1, y0, y1))
    } else {
        None
    };
    let scope = &prepared.scope;
    let cut = &prepared.cut;
    let mut entities: Vec<&Value> = prepared.entities.iter().collect();
    let selector = json!([&scope, &cut, &query.bounds, &query.search, query.page_size]);
    let fingerprint = blake3::hash(selector.to_string().as_bytes())
        .to_hex()
        .to_string();
    let after = if let Some(cursor) = &query.cursor {
        if cursor.len() > 4096 {
            return unavailable("cursor_reset_required");
        }
        let parsed = hex::decode(cursor)
            .ok()
            .and_then(|b| serde_json::from_slice::<(String, String)>(&b).ok());
        match parsed {
            Some((binding, key)) if binding == fingerprint => Some(key),
            _ => return unavailable("cursor_reset_required"),
        }
    } else {
        None
    };
    let search = query.search.as_deref().unwrap_or("").to_lowercase();
    entities.retain(|e| {
        if !search.is_empty()
            && !e["label"]
                .as_str()
                .unwrap_or("")
                .to_lowercase()
                .contains(&search)
            && !e["entity_id"]
                .as_str()
                .unwrap_or("")
                .to_lowercase()
                .contains(&search)
        {
            return false;
        }
        if let Some((x0, x1, y0, y1)) = bounds {
            let p = &e["position"];
            let x = p["x_cm"].as_str().and_then(|s| s.parse::<i64>().ok());
            let y = p["y_cm"].as_str().and_then(|s| s.parse::<i64>().ok());
            if !matches!((x,y),(Some(x),Some(y)) if x>=x0 && x<=x1 && y>=y0 && y<=y1) {
                return false;
            }
        }
        true
    });
    let key = |e: &Value| {
        format!(
            "{}:{}",
            e["kind"].as_str().unwrap_or(""),
            e["entity_id"].as_str().unwrap_or("")
        )
    };
    if let Some(after) = &after {
        entities.retain(|e| key(e) > *after)
    }
    let partial = entities.len() > query.page_size;
    entities.truncate(query.page_size);
    let next = if partial {
        entities.last().map(|e| {
            hex::encode(serde_json::to_vec(&(&fingerprint, key(e))).expect("string tuple"))
        })
    } else {
        None
    };
    if next.as_ref().is_some_and(|cursor| cursor.len() > 4096) {
        return unavailable("cursor_limit_exceeded");
    }
    let mut entities: Vec<Value> = entities.into_iter().cloned().collect();
    if source_frozen {
        for entity in &mut entities {
            if entity["provenance"]["freshness"] == "current" {
                entity["provenance"]["freshness"] = json!("last_known");
                entity["provenance"]["reason"] = json!("frozen_pagination_cut");
            }
        }
    }
    let view = json!({"api_revision":1,"world_id":world,"scope_id":scope,"source_cut":cut,"request_generation":generation,
        "world_bounds":{"min_x_cm":"0","min_y_cm":"0","max_x_cm":space.width_cm.to_string(),"max_y_cm":space.depth_cm.to_string(),"width_cm":space.width_cm.to_string(),"depth_cm":space.depth_cm.to_string(),"height_cm":space.height_cm.to_string()},
        "coverage":{"kind":"viewer_snapshot_projection","bounds":query.bounds,"search":query.search,"replacement_scope":"query_only","absence_means_world_deletion":false},
        "entities":entities,"relations":[],"events":[],"next_cursor":next,"completeness":if partial {"partial"} else {"complete"},
        "source_frozen":source_frozen,"unavailable_reasons":[],"excluded_kinds":["factory","resource","event","relation"]});
    if view.to_string().len() > 2 * 1024 * 1024 {
        return unavailable("response_limit_exceeded");
    }
    view
}

fn prepare(
    snapshot: &WorldSnapshot,
    world: &str,
    epoch: u64,
    audience: &str,
    local_runtime: bool,
) -> Option<Prepared> {
    if snapshot
        .model
        .agents
        .len()
        .saturating_add(snapshot.model.locations.len())
        > 50_000
    {
        return None;
    }
    let scope = format!(
        "viewer_snapshot:{}",
        blake3::hash(audience.as_bytes()).to_hex()
    );
    let mut entities = Vec::new();
    let agents=snapshot.model.agents.iter().map(|(id,a)|("agent",id,json!({"id":a.id,"pos":a.pos,"location_id":a.location_id,"activity":a.activity,"power":a.power,"resources":a.resources})));
    let locations = snapshot.model.locations.iter().map(|(id, l)| {
        (
            "location",
            id,
            json!({"id":l.id,"name":l.name,"pos":l.pos,"resources":l.resources}),
        )
    });
    let mut bytes = 0usize;
    for (kind, id, record) in agents.chain(locations) {
        let label = record
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or(id)
            .to_owned();
        let position = record.get("pos").cloned().unwrap_or(Value::Null);
        let current = local_runtime && kind == "agent";
        let entity = json!({"entity_id":id,"kind":kind,"label":label,"position":decimal(position),
            "render_record":decimal(record),"provenance":{"geometry_source":if kind=="agent" {"authoritative"} else {"location_derived"},"freshness":if current {"current"} else {"unknown"},
            "source_ref":"viewer_snapshot","observed_at_tick":snapshot.time.to_string(),"reason":if current {"local_authoritative_runtime_cut"} else {"snapshot_without_independent_observation_binding"}}});
        bytes = bytes.saturating_add(entity.to_string().len());
        if bytes > 8 * 1024 * 1024 {
            return None;
        }
        entities.push(entity);
    }
    // The cut covers the complete visible projection, never a filtered page.
    let cut_material = json!([
        world,
        epoch.to_string(),
        &scope,
        snapshot.time.to_string(),
        &snapshot.config.space,
        &entities
    ]);
    let cut = blake3::hash(cut_material.to_string().as_bytes())
        .to_hex()
        .to_string();
    for entity in &mut entities {
        entity["provenance"]["valid_at_source_cut"] = json!(&cut);
    }
    if entities.iter().map(|e| e.to_string().len()).sum::<usize>() > 8 * 1024 * 1024 {
        return None;
    }
    Some(Prepared {
        scope,
        cut,
        entities,
    })
}
#[derive(Clone)]
struct Prepared {
    scope: String,
    cut: String,
    entities: Vec<Value>,
}
fn decimal(value: Value) -> Value {
    match value {
        Value::Number(n) if n.is_i64() || n.is_u64() => Value::String(n.to_string()),
        Value::Array(v) => Value::Array(v.into_iter().map(decimal).collect()),
        Value::Object(v) => Value::Object(v.into_iter().map(|(k, v)| (k, decimal(v))).collect()),
        v => v,
    }
}

pub(super) struct SessionProjection {
    world: String,
    epoch: u64,
    audience: String,
    created: std::time::Instant,
    space: crate::simulator::SpaceConfig,
    prepared: Prepared,
}
pub(super) fn build_session(
    cache: &mut Option<SessionProjection>,
    snapshot: impl FnOnce() -> WorldSnapshot,
    world: &str,
    epoch: u64,
    audience: &str,
    query: Value,
    local_runtime: bool,
) -> Value {
    let valid_query = serde_json::from_value::<Query>(query.clone())
        .ok()
        .is_some_and(|q| {
            q.page_size > 0
                && q.page_size <= 200
                && q.request_generation.len() <= 64
                && q.search.as_ref().is_none_or(|s| s.len() <= 128)
        });
    if !valid_query {
        return json!({"world_id":world,"request_generation":query.get("request_generation"),"completeness":"unknown","entities":[],"unavailable_reasons":["invalid_query"]});
    }
    let bounds_valid = serde_json::from_value::<Query>(query.clone())
        .ok()
        .is_some_and(|q| {
            q.bounds.as_ref().is_none_or(|b| {
                let parsed =
                    [&b.min_x_cm, &b.max_x_cm, &b.min_y_cm, &b.max_y_cm].map(|v| v.parse::<i64>());
                matches!(parsed,[Ok(x0),Ok(x1),Ok(y0),Ok(y1)] if x0<=x1 && y0<=y1)
            })
        });
    if !bounds_valid {
        return json!({"world_id":world,"request_generation":query.get("request_generation"),"completeness":"unknown","entities":[],"unavailable_reasons":["invalid_bounds"]});
    }
    let continuation = query.get("cursor").is_some_and(|c| !c.is_null());
    let valid = cache.as_ref().is_some_and(|c| {
        c.world == world
            && c.epoch == epoch
            && c.audience == audience
            && c.created.elapsed() < std::time::Duration::from_secs(10)
    });
    if continuation && !valid {
        *cache = None;
        return json!({"world_id":world,"request_generation":query.get("request_generation"),"completeness":"unknown","entities":[],"unavailable_reasons":["cursor_reset_required"]});
    }
    if !continuation {
        *cache = None;
        let snapshot = snapshot();
        let Some(prepared) = prepare(&snapshot, world, epoch, audience, local_runtime) else {
            return json!({"world_id":world,"request_generation":query.get("request_generation"),"completeness":"unknown","entities":[],"unavailable_reasons":["projection_limit_exceeded"]});
        };
        *cache = Some(SessionProjection {
            world: world.into(),
            epoch,
            audience: audience.into(),
            created: std::time::Instant::now(),
            space: snapshot.config.space.clone(),
            prepared,
        });
    }
    let cached = cache.as_ref().expect("prepared above");
    build_inner(&cached.space, world, query, &cached.prepared, continuation)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulator::WorldKernel;
    fn snapshot() -> WorldSnapshot {
        let mut snapshot = WorldKernel::new().snapshot();
        snapshot.model.agents.insert(
            "page-a".into(),
            crate::simulator::Agent::new("page-a", "", crate::geometry::GeoPos::new(1, 2, 0)),
        );
        snapshot.model.agents.insert(
            "page-b".into(),
            crate::simulator::Agent::new("page-b", "", crate::geometry::GeoPos::new(3, 4, 0)),
        );
        snapshot
    }
    fn query() -> Value {
        json!({"request_generation":"1","page_size":1})
    }
    #[test]
    fn pages_bind_scope_cut_and_selector_without_claiming_deletion() {
        let snapshot = snapshot();
        let first = build(&snapshot, "w", 0, "alice", query());
        assert_eq!(first["coverage"]["absence_means_world_deletion"], false);
        assert_eq!(first["entities"][0]["provenance"]["freshness"], "unknown");
        assert!(first["next_cursor"].is_string());
        {
            let mut q = query();
            q["cursor"] = first["next_cursor"].clone();
            let second = build(&snapshot, "w", 0, "alice", q.clone());
            assert_eq!(first["source_cut"], second["source_cut"]);
            assert_ne!(
                first["entities"][0]["entity_id"],
                second["entities"][0]["entity_id"]
            );
            for (world, epoch, audience) in [("x", 0, "alice"), ("w", 1, "alice"), ("w", 0, "bob")]
            {
                assert_eq!(
                    build(&snapshot, world, epoch, audience, q.clone())["unavailable_reasons"][0],
                    "cursor_reset_required"
                );
            }
            let mut changed = snapshot.clone();
            changed.time += 1;
            assert_eq!(
                build(&changed, "w", 0, "alice", q)["unavailable_reasons"][0],
                "cursor_reset_required"
            );
        }
    }
    #[test]
    fn session_pages_use_bounded_frozen_cut_and_drop_other_audiences() {
        let source = snapshot();
        let mut cache = None;
        let first = build_session(
            &mut cache,
            || source.clone(),
            "w",
            0,
            "alice",
            query(),
            true,
        );
        let mut q = query();
        q["cursor"] = first["next_cursor"].clone();
        let second = build_session(
            &mut cache,
            || panic!("must not rebuild snapshot for continuation"),
            "w",
            0,
            "alice",
            q.clone(),
            true,
        );
        assert_eq!(first["source_cut"], second["source_cut"]);
        assert_eq!(
            second["entities"][0]["provenance"]["freshness"],
            "last_known"
        );
        let denied = build_session(
            &mut cache,
            || panic!("reject before access"),
            "w",
            0,
            "bob",
            q,
            true,
        );
        assert_eq!(denied["unavailable_reasons"][0], "cursor_reset_required");
        assert!(cache.is_none());
    }
    #[test]
    fn huge_private_pending_actions_do_not_enter_projection_cache() {
        let mut source = snapshot();
        let secret = "private_pending_action".repeat(1024 * 1024);
        source
            .pending_actions
            .push(crate::simulator::ActionEnvelope {
                id: 1,
                action: crate::simulator::Action::RegisterAgent {
                    agent_id: secret,
                    location_id: "private".into(),
                },
                submitter: Default::default(),
            });
        let mut cache = None;
        let view = build_session(&mut cache, || source, "w", 0, "alice", query(), true);
        let cached = cache
            .as_ref()
            .expect("cache accepts large unrelated source");
        let bytes = cached
            .prepared
            .entities
            .iter()
            .map(|e| e.to_string().len())
            .sum::<usize>();
        assert!(bytes < 1024 * 1024);
        assert!(!view.to_string().contains("private_pending_action"));
        // Metadata is only SpaceConfig; there is no WorldSnapshot/pending-actions field.
        assert_eq!(
            std::mem::size_of_val(&cached.space),
            std::mem::size_of::<crate::simulator::SpaceConfig>()
        );
    }
    #[test]
    fn invalid_session_bounds_fail_before_snapshot_work() {
        let mut q = query();
        q["bounds"] = json!({"min_x_cm":"9","max_x_cm":"2","min_y_cm":"0","max_y_cm":"1"});
        let result = build_session(
            &mut None,
            || panic!("invalid bounds must not build snapshot"),
            "w",
            0,
            "public",
            q,
            false,
        );
        assert_eq!(result["unavailable_reasons"][0], "invalid_bounds");
    }
    #[test]
    fn search_and_bounds_do_not_return_unmatched_entities() {
        let snapshot = snapshot();
        let mut q = query();
        q["search"] = json!("nonexistent-needle");
        let view = build(&snapshot, "w", 0, "public", q);
        assert_eq!(view["entities"], json!([]));
        assert_eq!(view["completeness"], "complete");
        let mut q = query();
        q["bounds"] = json!({"min_x_cm":"3","max_x_cm":"2","min_y_cm":"0","max_y_cm":"1"});
        assert_eq!(
            build(&snapshot, "w", 0, "public", q)["unavailable_reasons"][0],
            "invalid_bounds"
        );
    }
}
