//! Bounded, source-bound projection of this connection's authorized snapshot.
use crate::simulator::WorldSnapshot;
use crate::viewer::world_view_authority::SourceAuthority;
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::BTreeSet;
#[path = "world_view_index.rs"]
mod index;
#[path = "world_view_projection.rs"]
mod projection;
use index::Rect;
use projection::{Prepared, prepare};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Query {
    request_generation: String,
    #[serde(default)]
    bounds: Option<Bounds>,
    #[serde(default)]
    search: Option<String>,
    #[serde(default)]
    pinned_entities: Vec<Target>,
    page_size: usize,
    #[serde(default)]
    cursor: Option<String>,
}
#[derive(Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct Target {
    kind: String,
    id: String,
}
#[derive(Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct Bounds {
    min_x_cm: String,
    max_x_cm: String,
    min_y_cm: String,
    max_y_cm: String,
}
fn validate(query: &Value) -> Result<(Query, Option<Rect>), &'static str> {
    let q: Query = serde_json::from_value(query.clone()).map_err(|_| "invalid_query")?;
    if q.page_size == 0
        || q.page_size > 200
        || q.request_generation.len() > 64
        || q.search.as_ref().is_some_and(|s| s.len() > 128)
        || q.pinned_entities.len() > 3
        || q.pinned_entities.iter().any(|p| {
            !matches!(p.kind.as_str(), "agent" | "location" | "factory")
                || p.id.is_empty()
                || p.id.len() > 256
        })
    {
        return Err("invalid_query");
    }
    let bounds = if let Some(b) = &q.bounds {
        let [Ok(x0), Ok(x1), Ok(y0), Ok(y1)] =
            [&b.min_x_cm, &b.max_x_cm, &b.min_y_cm, &b.max_y_cm].map(|v| v.parse::<i64>())
        else {
            return Err("invalid_bounds");
        };
        if x0 > x1 || y0 > y1 {
            return Err("invalid_bounds");
        }
        Some(Rect(x0, x1, y0, y1))
    } else {
        None
    };
    Ok((q, bounds))
}
fn unavailable(world: &str, query: &Value, reason: &str) -> Value {
    json!({"world_id":world,"request_generation":query.get("request_generation"),"completeness":"unknown","entities":[],"relations":[],"events":[],"unavailable_reasons":[reason]})
}
pub fn build(
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
    if let Err(reason) = validate(&query) {
        return unavailable(world, &query, reason);
    }
    let authority = if local_runtime {
        SourceAuthority::local_runtime()
    } else {
        SourceAuthority::unknown()
    };
    let Some(prepared) = prepare(snapshot, world, epoch, audience, &authority) else {
        return unavailable(world, &query, "projection_limit_exceeded");
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
    let (q, bounds) = match validate(&query) {
        Ok(q) => q,
        Err(reason) => return unavailable(world, &query, reason),
    };
    let fingerprint = blake3::hash(
        json!([
            &prepared.scope,
            &prepared.cut,
            &q.bounds,
            &q.search,
            &q.pinned_entities,
            q.page_size
        ])
        .to_string()
        .as_bytes(),
    )
    .to_hex()
    .to_string();
    let after = if let Some(cursor) = &q.cursor {
        if cursor.len() > 4096 {
            return unavailable(world, &query, "cursor_reset_required");
        }
        match hex::decode(cursor)
            .ok()
            .and_then(|b| serde_json::from_slice::<(String, String)>(&b).ok())
        {
            Some((binding, key)) if binding == fingerprint => Some(key),
            _ => return unavailable(world, &query, "cursor_reset_required"),
        }
    } else {
        None
    };
    let search = q.search.as_deref().unwrap_or("").to_lowercase();
    let candidates = bounds.map_or_else(
        || (0..prepared.objects.len()).collect(),
        |b| prepared.index.query(b),
    );
    let mut chosen: BTreeSet<usize> = candidates
        .into_iter()
        .filter(|i| {
            let object = &prepared.objects[*i];
            search.is_empty()
                || ["label", "entity_id", "relation_id"].iter().any(|k| {
                    object.value[*k]
                        .as_str()
                        .is_some_and(|s| s.to_lowercase().contains(&search))
                })
        })
        .collect();
    let mut pinned_keys = BTreeSet::new();
    for pin in &q.pinned_entities {
        if let Some(i) = prepared.identities.get(&(pin.kind.clone(), pin.id.clone())) {
            chosen.insert(*i);
            pinned_keys.insert(prepared.objects[*i].key.clone());
        }
    }
    // Relationship endpoints must be from this same authorized cut, never old cache.
    for i in chosen.iter().copied().collect::<Vec<_>>() {
        let object = &prepared.objects[i];
        if object.relation {
            for key in ["from", "to"] {
                let e = &object.value[key];
                if let (Some(kind), Some(id)) = (e["kind"].as_str(), e["id"].as_str())
                    && let Some(i) = prepared.identities.get(&(kind.into(), id.into()))
                {
                    chosen.insert(*i);
                }
            }
        }
    }
    let order_key =
        |o: &projection::Object| format!("{}:{}", u8::from(!pinned_keys.contains(&o.key)), o.key);
    let mut objects: Vec<_> = chosen
        .into_iter()
        .map(|i| &prepared.objects[i])
        .filter(|o| after.as_ref().is_none_or(|a| order_key(o) > *a))
        .collect();
    objects.sort_by_key(|o| order_key(o));
    let partial = objects.len() > q.page_size;
    objects.truncate(q.page_size);
    let next = if partial {
        objects.last().map(|o| {
            hex::encode(serde_json::to_vec(&(&fingerprint, order_key(o))).expect("string tuple"))
        })
    } else {
        None
    };
    if next.as_ref().is_some_and(|s| s.len() > 4096) {
        return unavailable(world, &query, "cursor_limit_exceeded");
    }
    let mut entities = Vec::new();
    let mut relations = Vec::new();
    for object in objects {
        let mut value = object.value.clone();
        let provenance = if object.relation {
            &mut value
        } else {
            &mut value["provenance"]
        };
        if source_frozen && provenance["freshness"] == "current" {
            provenance["freshness"] = json!("last_known");
            provenance["reason"] = json!("frozen_pagination_cut");
        }
        if object.relation {
            relations.push(value);
        } else {
            entities.push(value);
        }
    }
    let view = json!({"api_revision":1,"world_id":world,"scope_id":prepared.scope,"source_cut":prepared.cut,"source_binding":prepared.source_binding,"request_generation":q.request_generation,
    "world_bounds":{"min_x_cm":"0","min_y_cm":"0","max_x_cm":space.width_cm.to_string(),"max_y_cm":space.depth_cm.to_string(),"width_cm":space.width_cm.to_string(),"depth_cm":space.depth_cm.to_string(),"height_cm":space.height_cm.to_string()},
    "coverage":{"kind":"viewer_snapshot_projection","bounds":q.bounds,"search":q.search,"replacement_scope":"query_only","absence_means_world_deletion":false},
    "entities":entities,"relations":relations,"events":[],"next_cursor":next,"completeness":if partial {"partial"}else{"complete"},"source_frozen":source_frozen,"unavailable_reasons":[],"excluded_kinds":["resource","event"]});
    if view.to_string().len() > 2 * 1024 * 1024 {
        return unavailable(world, &query, "response_limit_exceeded");
    }
    view
}
pub(super) struct SessionProjection {
    world: String,
    epoch: u64,
    audience: String,
    source_scope: String,
    source_cut_key: String,
    created: std::time::Instant,
    space: crate::simulator::SpaceConfig,
    prepared: Prepared,
}
#[cfg(test)]
pub(super) fn build_session(
    cache: &mut Option<SessionProjection>,
    snapshot: impl FnOnce() -> WorldSnapshot,
    world: &str,
    epoch: u64,
    audience: &str,
    query: Value,
    local_runtime: bool,
) -> Value {
    let authority = if local_runtime {
        SourceAuthority::local_runtime()
    } else {
        SourceAuthority::unknown()
    };
    build_session_bound(cache, snapshot, world, epoch, audience, query, authority)
}
pub(super) fn build_session_bound(
    cache: &mut Option<SessionProjection>,
    snapshot: impl FnOnce() -> WorldSnapshot,
    world: &str,
    epoch: u64,
    audience: &str,
    query: Value,
    authority: SourceAuthority,
) -> Value {
    if let Err(reason) = validate(&query) {
        return unavailable(world, &query, reason);
    }
    let continuation = query.get("cursor").is_some_and(|c| !c.is_null());
    let source_scope = authority.scope_key();
    let source_cut_key = authority.cut_key();
    let valid = cache.as_ref().is_some_and(|c| {
        c.world == world
            && c.epoch == epoch
            && c.audience == audience
            && c.source_scope == source_scope
            && c.created.elapsed() < std::time::Duration::from_secs(10)
    });
    if continuation && !valid {
        *cache = None;
        return unavailable(world, &query, "cursor_reset_required");
    }
    let freeze_continuation = continuation
        && (!authority.has_bound_cut()
            || cache
                .as_ref()
                .is_some_and(|c| c.source_cut_key != source_cut_key));
    let same_source = valid
        && authority.current()
        && cache
            .as_ref()
            .is_some_and(|c| c.source_cut_key == source_cut_key);
    if !continuation && !same_source {
        *cache = None;
        let snapshot = snapshot();
        let source = if authority.validate_snapshot(&snapshot) {
            authority
        } else {
            SourceAuthority::unknown()
        };
        let Some(prepared) = prepare(&snapshot, world, epoch, audience, &source) else {
            return unavailable(world, &query, "projection_limit_exceeded");
        };
        *cache = Some(SessionProjection {
            world: world.into(),
            epoch,
            audience: audience.into(),
            source_scope,
            source_cut_key,
            created: std::time::Instant::now(),
            space: snapshot.config.space.clone(),
            prepared,
        });
    }
    let cached = cache.as_ref().expect("prepared above");
    build_inner(
        &cached.space,
        world,
        query,
        &cached.prepared,
        freeze_continuation,
    )
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
            .objects
            .iter()
            .map(|e| e.value.to_string().len())
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

#[cfg(test)]
#[path = "world_view_spatial_tests.rs"]
mod spatial_tests;
