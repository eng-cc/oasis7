//! Display freshness evidence, separate from admission or an owner capability.
use crate::{runtime::WorldState, simulator::WorldSnapshot};
use serde_json::{Value, json};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct SourceAuthority {
    kind: SourceKind,
    binding: Option<Value>,
    scope_binding: Value,
    snapshot_digest: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
enum SourceKind {
    Unknown,
    LocalRuntime,
    VerifiedRemote,
}
impl SourceAuthority {
    pub(super) fn unknown() -> Self {
        Self {
            kind: SourceKind::Unknown,
            binding: None,
            scope_binding: json!("unknown"),
            snapshot_digest: None,
        }
    }
    /// Compatibility for the existing local-only projection entrypoint.
    pub(super) fn local_runtime() -> Self {
        Self {
            kind: SourceKind::LocalRuntime,
            binding: None,
            scope_binding: json!("local_runtime"),
            snapshot_digest: None,
        }
    }
    pub(super) fn local_bound(snapshot_digest: String) -> Self {
        Self {
            snapshot_digest: Some(snapshot_digest),
            ..Self::local_runtime()
        }
    }
    pub(super) fn remote(binding: Value, scope_binding: Value, snapshot_digest: String) -> Self {
        Self {
            kind: SourceKind::VerifiedRemote,
            binding: Some(decimal_integers(binding)),
            scope_binding,
            snapshot_digest: Some(snapshot_digest),
        }
    }
    pub(super) fn current(&self) -> bool {
        self.kind != SourceKind::Unknown
    }
    pub(super) fn has_bound_cut(&self) -> bool {
        self.current() && self.snapshot_digest.is_some()
    }
    pub(super) fn source_ref(&self) -> &'static str {
        match self.kind {
            SourceKind::Unknown => "viewer_snapshot",
            SourceKind::LocalRuntime => "local_authoritative_runtime_cut",
            SourceKind::VerifiedRemote => "verified_world_service_cut",
        }
    }
    pub(super) fn binding(&self) -> Option<&Value> {
        self.binding.as_ref()
    }
    /// Excludes the source commit position so pages can finish a frozen cut.
    pub(super) fn scope_key(&self) -> String {
        blake3::hash(self.scope_binding.to_string().as_bytes())
            .to_hex()
            .to_string()
    }
    /// Internal reuse identity; the display proof digest never goes on the wire.
    pub(super) fn cut_key(&self) -> String {
        blake3::hash(
            json!([self.source_ref(), &self.binding, &self.snapshot_digest])
                .to_string()
                .as_bytes(),
        )
        .to_hex()
        .to_string()
    }
    pub(super) fn validate_snapshot(&self, snapshot: &WorldSnapshot) -> bool {
        if !self.current() {
            return false;
        }
        let Some(expected) = &self.snapshot_digest else {
            return self.kind == SourceKind::LocalRuntime;
        };
        let Some(runtime) = &snapshot.runtime_snapshot else {
            return false;
        };
        snapshot.time == runtime.state.time
            && spatial_state_digest(&runtime.state).as_ref() == Some(expected)
            && snapshot.model.agents.len() == runtime.state.agents.len()
            && snapshot.model.agents.iter().all(|(id, agent)| {
                runtime.state.agents.get(id).is_some_and(|cell| {
                    agent.id == *id
                        && agent.pos == cell.state.pos
                        && agent.resources == cell.state.resources
                })
            })
    }
}

/// All persisted inputs used by the spatial read projection. Local-only
/// signer/retention bookkeeping is intentionally outside this display proof.
/// No digest input or private body is emitted through SourceAuthority.
pub(super) fn spatial_state_digest(state: &WorldState) -> Option<String> {
    let agents:std::collections::BTreeMap<_,_>=state.agents.iter().map(|(id,cell)|(id,json!({"agent_id":cell.state.agent_id,"pos":cell.state.pos,"resources":cell.state.resources,"activity":cell.activity}))).collect();
    let material = json!({
        "time":state.time,"agents":agents,"factories":state.factories,
        "logistics_routes":state.logistics_routes,
        "factory_site_authorities":state.factory_site_authorities,
        "location_anchors":state.location_anchors,"module_visual_entities":state.module_visual_entities
    });
    serde_json::to_vec(&material)
        .ok()
        .map(|bytes| blake3::hash(&bytes).to_hex().to_string())
}

fn decimal_integers(value: Value) -> Value {
    match value {
        Value::Number(n) if n.is_i64() || n.is_u64() => Value::String(n.to_string()),
        Value::Array(v) => Value::Array(v.into_iter().map(decimal_integers).collect()),
        Value::Object(v) => Value::Object(
            v.into_iter()
                .map(|(k, v)| (k, decimal_integers(v)))
                .collect(),
        ),
        value => value,
    }
}
