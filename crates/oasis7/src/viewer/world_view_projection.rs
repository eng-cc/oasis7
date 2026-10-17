//! Allowlisted spatial records; no private Runtime state is retained in the cache.
use super::index::{Rect, SpatialIndex};
use crate::simulator::{ModuleVisualAnchor, WorldSnapshot};
use crate::viewer::world_view_authority::SourceAuthority;
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub(super) struct Object {
    pub key: String,
    pub value: Value,
    pub relation: bool,
    pub bounds: Option<Rect>,
}
pub(super) struct Prepared {
    pub scope: String,
    pub cut: String,
    pub objects: Vec<Object>,
    pub index: SpatialIndex,
    pub identities: BTreeMap<(String, String), usize>,
    pub source_binding: Option<Value>,
}
fn decimal(value: Value) -> Value {
    match value {
        Value::Number(n) if n.is_i64() || n.is_u64() => Value::String(n.to_string()),
        Value::Array(v) => Value::Array(v.into_iter().map(decimal).collect()),
        Value::Object(v) => Value::Object(v.into_iter().map(|(k, v)| (k, decimal(v))).collect()),
        v => v,
    }
}
fn point(value: &Value) -> Option<(i64, i64)> {
    Some((
        value.get("x_cm")?.as_str()?.parse().ok()?,
        value.get("y_cm")?.as_str()?.parse().ok()?,
    ))
}
fn entity(
    kind: &str,
    id: &str,
    label: &str,
    record: Value,
    (position, geometry, current): (Value, &str, bool),
    snapshot: &WorldSnapshot,
    authority: &SourceAuthority,
) -> Object {
    let position = decimal(position);
    let bounds = point(&position).map(|(x, y)| Rect(x, x, y, y));
    Object {
        key: format!("entity:{kind}:{id}"),
        relation: false,
        bounds,
        value: json!({"entity_id":id,"kind":kind,"label":label,"position":position,"render_record":decimal(record),
        "provenance":{"geometry_source":geometry,"freshness":if current {"current"} else {"unknown"},"source_ref":authority.source_ref(),"observed_at_tick":snapshot.time.to_string(),
        "reason":if current {"authority_bound_runtime_cut"} else {"position_without_current_source_binding"}}}),
    }
}
fn factory_position(
    snapshot: &WorldSnapshot,
    id: &str,
    fallback_location: &str,
) -> (Value, String, String, bool) {
    if let Some(runtime) = &snapshot.runtime_snapshot {
        let state = &runtime.state;
        // An explicit module anchor names this exact facility. The builder may have moved.
        if let Some(visual) = state.module_visual_entities.get(id)
            && visual.entity_id == id
            && visual.kind == "factory"
            && let ModuleVisualAnchor::Absolute { pos } = &visual.anchor
        {
            return (json!(pos), String::new(), "authoritative".into(), true);
        }
        if let Some(factory) = state.factories.get(id) {
            let location = state
                .factory_site_authorities
                .get(&factory.site_id)
                .filter(|site| site.active && site.site_id == factory.site_id)
                .map(|site| site.location_id.as_str())
                .unwrap_or(&factory.site_id);
            let pos = snapshot
                .model
                .locations
                .get(location)
                .map(|l| json!(l.pos))
                .unwrap_or(Value::Null);
            return (pos, location.into(), "location_derived".into(), false);
        }
    }
    let pos = snapshot
        .model
        .locations
        .get(fallback_location)
        .map(|l| json!(l.pos))
        .unwrap_or(Value::Null);
    (
        pos,
        fallback_location.into(),
        "location_derived".into(),
        false,
    )
}
pub(super) fn prepare(
    snapshot: &WorldSnapshot,
    world: &str,
    epoch: u64,
    audience: &str,
    authority: &SourceAuthority,
) -> Option<Prepared> {
    let route_count = snapshot
        .runtime_snapshot
        .as_ref()
        .map_or(0, |r| r.state.logistics_routes.len());
    if snapshot
        .model
        .agents
        .len()
        .saturating_add(snapshot.model.locations.len())
        .saturating_add(snapshot.model.factories.len())
        .saturating_add(route_count)
        > 50_000
    {
        return None;
    }
    let scope = format!(
        "viewer_snapshot:{}",
        blake3::hash(audience.as_bytes()).to_hex()
    );
    let mut objects = Vec::new();
    let mut bytes = 0usize;
    let mut push = |object: Object| -> Option<()> {
        bytes = bytes.checked_add(object.value.to_string().len())?;
        if bytes > 8 * 1024 * 1024 {
            return None;
        }
        objects.push(object);
        Some(())
    };
    for (id, a) in &snapshot.model.agents {
        push(entity(
            "agent",
            id,
            id,
            json!({"id":a.id,"pos":a.pos,"location_id":a.location_id,"activity":a.activity,"power":a.power,"resources":a.resources}),
            (json!(a.pos), "authoritative", authority.current()),
            snapshot,
            authority,
        ))?;
    }
    for (id, l) in &snapshot.model.locations {
        push(entity(
            "location",
            id,
            &l.name,
            json!({"id":l.id,"name":l.name,"pos":l.pos,"resources":l.resources}),
            (json!(l.pos), "location_derived", false),
            snapshot,
            authority,
        ))?;
    }
    for (id, f) in &snapshot.model.factories {
        let (position, location, geometry, explicit) =
            factory_position(snapshot, id, &f.location_id);
        let label = format!("{} ({id})", f.kind);
        push(entity(
            "factory",
            id,
            &label,
            json!({"id":id,"name":label,"kind":f.kind,"pos":position,"location_id":location}),
            (position.clone(), &geometry, explicit && authority.current()),
            snapshot,
            authority,
        ))?;
    }
    objects.sort_by(|a, b| a.key.cmp(&b.key));
    let identities: BTreeMap<_, _> = objects
        .iter()
        .enumerate()
        .map(|(i, o)| {
            (
                (
                    o.value["kind"].as_str().unwrap().to_owned(),
                    o.value["entity_id"].as_str().unwrap().to_owned(),
                ),
                i,
            )
        })
        .collect();
    if let Some(runtime) = &snapshot.runtime_snapshot {
        let endpoint = |ledger: &crate::runtime::MaterialLedgerId| -> Option<Value> {
            use crate::runtime::MaterialLedgerId;
            let (kind, id) = match ledger {
                MaterialLedgerId::Agent(id) => ("agent", id),
                MaterialLedgerId::Factory(id) => ("factory", id),
                MaterialLedgerId::Site(id) => {
                    let site = runtime
                        .state
                        .factory_site_authorities
                        .get(id)
                        .filter(|site| site.active && site.site_id == *id)?;
                    ("location", &site.location_id)
                }
                MaterialLedgerId::World => return None,
            };
            identities
                .contains_key(&(kind.into(), id.clone()))
                .then(|| json!({"kind":kind,"id":id}))
        };
        for (id, route) in &runtime.state.logistics_routes {
            if route.route_id != *id {
                continue;
            }
            let (Some(from), Some(to)) = (endpoint(&route.from_ledger), endpoint(&route.to_ledger))
            else {
                continue;
            };
            let get = |target: &Value| {
                &objects[identities[&(
                    target["kind"].as_str().unwrap().into(),
                    target["id"].as_str().unwrap().into(),
                )]]
            };
            let (a, b) = (get(&from), get(&to));
            let bounds = a
                .bounds
                .zip(b.bounds)
                .map(|(a, b)| Rect(a.0.min(b.0), a.1.max(b.1), a.2.min(b.2), a.3.max(b.3)));
            let current = authority.current()
                && a.value["provenance"]["freshness"] == "current"
                && b.value["provenance"]["freshness"] == "current";
            let value = json!({"relation_id":id,"kind":"logistics_route","label":id,"status":if route.available {"active"} else {"unavailable"},"source_class":"runtime_projection","source_ref":authority.source_ref(),"freshness":if current {"current"} else {"unknown"},"observed_at_tick":snapshot.time.to_string(),"from":from,"to":to});
            bytes = bytes.checked_add(value.to_string().len())?;
            if bytes > 8 * 1024 * 1024 {
                return None;
            }
            objects.push(Object {
                key: format!("relation:{id}"),
                value,
                relation: true,
                bounds,
            });
        }
    }
    let source_binding = authority.binding().cloned();
    let cut_material = json!([
        world,
        epoch.to_string(),
        &scope,
        snapshot.time.to_string(),
        &snapshot.config.space,
        &source_binding,
        objects.iter().map(|o| &o.value).collect::<Vec<_>>()
    ]);
    let cut = blake3::hash(cut_material.to_string().as_bytes())
        .to_hex()
        .to_string();
    for object in &mut objects {
        if object.relation {
            object.value["valid_at_source_cut"] = json!(&cut);
        } else {
            object.value["provenance"]["valid_at_source_cut"] = json!(&cut);
        }
    }
    if objects
        .iter()
        .map(|o| o.value.to_string().len())
        .sum::<usize>()
        > 8 * 1024 * 1024
    {
        return None;
    }
    let index = SpatialIndex::new(
        objects
            .iter()
            .enumerate()
            .filter_map(|(i, o)| o.bounds.map(|r| (i, r)))
            .collect(),
    );
    Some(Prepared {
        scope,
        cut,
        objects,
        index,
        identities,
        source_binding,
    })
}
