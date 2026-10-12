//! Application operations shared by the CLI and desktop host. No UI or database dependencies.
use crate::{
    error::{ContextError, Result},
    model::*,
    store::Store,
};
use serde::Serialize;
use std::path::Path;

#[derive(Clone, Debug, Serialize)]
pub struct Capability {
    pub id: &'static str,
    pub supported: bool,
    pub unavailable_reason: Option<&'static str>,
}
#[derive(Clone, Debug, Serialize)]
pub struct ToolInfo {
    pub schema_version: u32,
    pub tool_id: &'static str,
    pub version: &'static str,
    pub source_head_oid: Option<&'static str>,
    pub release_id: Option<&'static str>,
    pub target: Option<&'static str>,
    pub protocol_schemas: Vec<&'static str>,
    pub capabilities: Vec<Capability>,
}
pub fn info() -> ToolInfo {
    ToolInfo {
        schema_version: 1,
        tool_id: "oasis7_context",
        version: env!("CARGO_PKG_VERSION"),
        source_head_oid: option_env!("OASIS7_SOURCE_HEAD_OID"),
        release_id: option_env!("OASIS7_RELEASE_ID"),
        target: option_env!("OASIS7_BUILD_TARGET"),
        protocol_schemas: vec![
            "oasis7.context_store.v1",
            "oasis7.context_entry.v1",
            "oasis7.context_result.v1",
        ],
        capabilities: [
            "context.read",
            "context.write",
            "context.history",
            "context.snapshot",
        ]
        .into_iter()
        .map(|id| Capability {
            id,
            supported: true,
            unavailable_reason: None,
        })
        .collect(),
    }
}
pub fn open(explicit_root: Option<&Path>) -> Result<Store> {
    let config = crate::config::load()?;
    let root = crate::config::select_root(explicit_root, &config)?;
    let mut store = Store::open(&root)?;
    store.verification_max_age_days = config.verification_max_age_days.unwrap_or(30);
    Ok(store)
}
pub fn save(
    store: &Store,
    entry: &Entry,
    condition: WriteCondition,
    actor: &str,
) -> Result<SaveResult> {
    save_raw(
        store,
        &entry.metadata.id,
        &crate::format::encode(entry)?,
        condition,
        actor,
    )
}
pub fn save_raw(
    store: &Store,
    id: &str,
    candidate: &[u8],
    condition: WriteCondition,
    actor: &str,
) -> Result<SaveResult> {
    store.put(id, candidate, condition, actor)
}
pub fn restore_entry(
    store: &Store,
    id: &str,
    commit: &str,
    condition: WriteCondition,
    actor: &str,
) -> Result<(String, SaveResult)> {
    let old = crate::git::read_at_commit(&store.root, id, commit)
        .map_err(|e| ContextError::new(e.code, e.message))?;
    let config: StoreConfig = crate::format::strict_json(&old.config_bytes)?;
    config.validate()?;
    if config.store_id != store.config.store_id || config.project_id != store.config.project_id {
        return Err(ContextError::new(
            "STORE_MISMATCH",
            "historical store identity does not match",
        ));
    }
    Ok((
        old.commit,
        save_raw(store, id, &old.entry_bytes, condition, actor)?,
    ))
}

pub fn snapshot_export(
    root: &Path,
    output: &Path,
    selection: &[String],
    passphrase: &[u8],
    lock_timeout_ms: u64,
) -> Result<crate::snapshot::SnapshotSummary> {
    crate::snapshot::export(root, output, selection, passphrase, lock_timeout_ms)
}
pub fn snapshot_inspect(
    input: &Path,
    passphrase: &[u8],
) -> Result<crate::snapshot::SnapshotSummary> {
    crate::snapshot::inspect(input, passphrase)
}
pub fn snapshot_restore(
    input: &Path,
    target: &Path,
    expected_project: Option<&str>,
    expected_store_id: Option<&str>,
    passphrase: &[u8],
) -> Result<crate::snapshot::SnapshotSummary> {
    crate::snapshot::restore(
        input,
        target,
        expected_project,
        expected_store_id,
        passphrase,
    )
}
