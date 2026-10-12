//! Authenticated, bounded exports of current Markdown bytes, without Git or local state.
use crate::{
    error::{ContextError, Result},
    model::{MAX_ENTRIES, MAX_ENTRY_BYTES, StoreConfig, validate_id, validate_timestamp},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;
use zeroize::{Zeroize, Zeroizing};
pub const KIND: &str = "context_export_v1";
pub const MAX_RAW_BYTES: usize = 12 * 1024 * 1024;
pub const MAX_PLAINTEXT_BYTES: usize = 16 * 1024 * 1024;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Snapshot {
    schema_version: String,
    store: StoreConfig,
    created_at: String,
    scope: Scope,
    source: Source,
    entries: Vec<Manifest>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Scope {
    mode: String,
    entry_ids: Vec<String>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Source {
    #[serde(rename = "type")]
    kind: String,
    git_head: Option<String>,
    working_tree_dirty: Option<bool>,
    git_history_included: bool,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    id: String,
    byte_length: usize,
    sha256: String,
    content_utf8: String,
}
impl Drop for Manifest {
    fn drop(&mut self) {
        self.content_utf8.zeroize();
    }
}
#[derive(Debug, Serialize)]
pub struct SnapshotSummary {
    pub store_id: String,
    pub project_id: String,
    pub created_at: String,
    pub scope: String,
    pub entry_ids: Vec<String>,
    pub entry_count: usize,
    pub raw_bytes: usize,
    pub git_history_included: bool,
}
fn invalid() -> ContextError {
    ContextError::new(
        "SNAPSHOT_INVALID",
        "snapshot manifest or content is invalid",
    )
}
fn limit() -> ContextError {
    ContextError::new("LIMIT_EXCEEDED", "snapshot capacity exceeded")
}
fn digest(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}
impl Snapshot {
    fn validate(&self) -> Result<SnapshotSummary> {
        if self.schema_version != "oasis7.context_snapshot.v1"
            || !["all", "selection"].contains(&self.scope.mode.as_str())
            || self.source.kind != "working_tree"
            || self.source.git_history_included
        {
            return Err(invalid());
        }
        self.store.validate().map_err(|_| invalid())?;
        validate_timestamp(&self.created_at).map_err(|_| invalid())?;
        if let Some(oid) = &self.source.git_head {
            if ![40, 64].contains(&oid.len())
                || !oid
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            {
                return Err(invalid());
            }
        }
        if self.entries.len() > MAX_ENTRIES || self.scope.entry_ids.len() > MAX_ENTRIES {
            return Err(limit());
        }
        let ids: Vec<_> = self.entries.iter().map(|e| e.id.clone()).collect();
        if ids != self.scope.entry_ids || ids.windows(2).any(|w| w[0] >= w[1]) {
            return Err(invalid());
        }
        let mut total = 0usize;
        for item in &self.entries {
            validate_id(&item.id).map_err(|_| invalid())?;
            let bytes = item.content_utf8.as_bytes();
            if bytes.len() > MAX_ENTRY_BYTES {
                return Err(limit());
            }
            total = total.checked_add(bytes.len()).ok_or_else(limit)?;
            if total > MAX_RAW_BYTES {
                return Err(limit());
            }
            if item.byte_length != bytes.len() || item.sha256 != digest(bytes) {
                return Err(invalid());
            }
            let entry = crate::format::parse(bytes).map_err(|_| invalid())?;
            if entry.metadata.id != item.id || entry.metadata.project_id != self.store.project_id {
                return Err(invalid());
            }
        }
        Ok(SnapshotSummary {
            store_id: self.store.store_id.clone(),
            project_id: self.store.project_id.clone(),
            created_at: self.created_at.clone(),
            scope: self.scope.mode.clone(),
            entry_ids: ids,
            entry_count: self.entries.len(),
            raw_bytes: total,
            git_history_included: false,
        })
    }
}
fn read_snapshot(input: &Path, passphrase: &[u8]) -> Result<(Snapshot, SnapshotSummary)> {
    read_snapshot_frozen(input, passphrase, None)
}
fn read_snapshot_frozen(
    input: &Path,
    passphrase: &[u8],
    expected_digest: Option<&str>,
) -> Result<(Snapshot, SnapshotSummary)> {
    let bytes = crate::fs::Directory::open(input.parent().ok_or_else(invalid)?)?.read(
        input
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(invalid)?,
        oasis7_secure_envelope::MAX_ENVELOPE_BYTES,
    )?;
    if expected_digest.is_some_and(|expected| digest(&bytes) != expected) {
        return Err(ContextError::new(
            "REVISION_CONFLICT",
            "snapshot ciphertext changed before restore",
        ));
    }
    let plaintext = oasis7_secure_envelope::open(KIND, &bytes, passphrase)
        .map_err(|_| ContextError::new("CRYPTO_ERROR", "snapshot authentication failed"))?;
    if plaintext.len() > MAX_PLAINTEXT_BYTES {
        return Err(limit());
    }
    let snapshot: Snapshot = crate::format::strict_json_bounded(&plaintext, MAX_PLAINTEXT_BYTES)
        .map_err(|_| invalid())?;
    let summary = snapshot.validate()?;
    Ok((snapshot, summary))
}
pub fn inspect(input: &Path, passphrase: &[u8]) -> Result<SnapshotSummary> {
    read_snapshot(input, passphrase).map(|(_, s)| s)
}
/// Validated snapshot identity without returning private plaintext.
pub fn inspect_revisions(
    input: &Path,
    passphrase: &[u8],
) -> Result<(SnapshotSummary, Vec<(String, String)>)> {
    let (snapshot, summary) = read_snapshot(input, passphrase)?;
    let revisions = snapshot
        .entries
        .iter()
        .map(|e| {
            (
                e.id.clone(),
                crate::format::revision(e.content_utf8.as_bytes()),
            )
        })
        .collect();
    Ok((summary, revisions))
}
pub fn inspect_revisions_frozen(
    input: &Path,
    passphrase: &[u8],
    expected_digest: &str,
) -> Result<(SnapshotSummary, Vec<(String, String)>)> {
    let (snapshot, summary) = read_snapshot_frozen(input, passphrase, Some(expected_digest))?;
    let revisions = snapshot
        .entries
        .iter()
        .map(|e| {
            (
                e.id.clone(),
                crate::format::revision(e.content_utf8.as_bytes()),
            )
        })
        .collect();
    Ok((summary, revisions))
}
pub fn source_digest(
    config: &StoreConfig,
    entries: &[crate::model::EntrySnapshot],
    selection: &[String],
) -> String {
    let mut chosen = selection.to_vec();
    chosen.sort();
    let mut revisions: Vec<_> = entries
        .iter()
        .filter(|e| chosen.is_empty() || chosen.contains(&e.entry.metadata.id))
        .map(|e| (&e.entry.metadata.id, &e.revision))
        .collect();
    revisions.sort();
    crate::format::digest(&crate::format::canonical_json(
        &serde_json::json!({"config":config,"selection":chosen,"entries":revisions}),
    ))
}
/// Collect under the store lock, then release it before serialization and KDF.
pub fn export(
    root: &Path,
    output: &Path,
    selection: &[String],
    passphrase: &[u8],
    lock_timeout_ms: u64,
) -> Result<SnapshotSummary> {
    export_frozen(root, output, selection, passphrase, lock_timeout_ms, None)
}
pub fn export_frozen(
    root: &Path,
    output: &Path,
    selection: &[String],
    passphrase: &[u8],
    lock_timeout_ms: u64,
    expected_digest: Option<&str>,
) -> Result<SnapshotSummary> {
    if !output.is_absolute() {
        return Err(ContextError::new(
            "INVALID_INPUT",
            "output must be absolute",
        ));
    }
    let store = crate::store::Store::open(root)?;
    export_from_store(
        &store,
        output,
        selection,
        passphrase,
        lock_timeout_ms,
        expected_digest,
    )
}
pub fn export_from_store(
    store: &crate::store::Store,
    output: &Path,
    selection: &[String],
    passphrase: &[u8],
    lock_timeout_ms: u64,
    expected_digest: Option<&str>,
) -> Result<SnapshotSummary> {
    let root = &store.root;

    let output_parent_path = output.parent().ok_or_else(invalid)?;
    let output_directory = crate::fs::Directory::open(output_parent_path)?;
    let output_parent = output_parent_path.canonicalize()?;
    if output_parent.starts_with(root.canonicalize()?) {
        return Err(ContextError::new(
            "INVALID_INPUT",
            "snapshot output must be outside store",
        ));
    }
    let lock = store.lock(lock_timeout_ms)?;
    let locked_config: StoreConfig = crate::format::strict_json(&store.config_raw()?)?;
    if locked_config != store.config {
        return Err(ContextError::new(
            "UNSTABLE_ENTRY",
            "store configuration changed while exporting",
        ));
    }
    let mut entries = store.read_all()?;
    if expected_digest
        .is_some_and(|expected| source_digest(&store.config, &entries, selection) != expected)
    {
        return Err(ContextError::new(
            "REVISION_CONFLICT",
            "snapshot source changed before export",
        ));
    }
    entries.sort_by(|a, b| a.entry.metadata.id.cmp(&b.entry.metadata.id));
    let mut chosen = selection.to_vec();
    chosen.sort();
    if chosen.windows(2).any(|w| w[0] == w[1]) {
        return Err(ContextError::new(
            "INVALID_INPUT",
            "duplicate selection identifier",
        ));
    }
    for id in &chosen {
        validate_id(id)?;
        if !entries.iter().any(|e| &e.entry.metadata.id == id) {
            return Err(ContextError::new(
                "ENTRY_NOT_FOUND",
                "selected entry does not exist",
            ));
        }
    }
    let mut manifests = Vec::new();
    let mut total = 0usize;
    for e in entries {
        if !chosen.is_empty() && !chosen.contains(&e.entry.metadata.id) {
            continue;
        }
        total = total.checked_add(e.raw_bytes.len()).ok_or_else(limit)?;
        if total > MAX_RAW_BYTES {
            return Err(limit());
        }
        manifests.push(Manifest {
            id: e.entry.metadata.id,
            byte_length: e.raw_bytes.len(),
            sha256: digest(&e.raw_bytes),
            content_utf8: String::from_utf8(e.raw_bytes).map_err(|_| invalid())?,
        });
    }
    let snapshot = Snapshot {
        schema_version: "oasis7.context_snapshot.v1".into(),
        store: store.config.clone(),
        created_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        scope: Scope {
            mode: if selection.is_empty() {
                "all"
            } else {
                "selection"
            }
            .into(),
            entry_ids: manifests.iter().map(|m| m.id.clone()).collect(),
        },
        source: Source {
            kind: "working_tree".into(),
            git_head: None,
            working_tree_dirty: None,
            git_history_included: false,
        },
        entries: manifests,
    };
    drop(lock);
    let summary = snapshot.validate()?;
    let plaintext = Zeroizing::new(serde_json::to_vec(&snapshot).map_err(|_| invalid())?);
    if plaintext.len() > MAX_PLAINTEXT_BYTES {
        return Err(limit());
    }
    let envelope = oasis7_secure_envelope::seal(KIND, &plaintext, passphrase)
        .map_err(|_| ContextError::new("CRYPTO_ERROR", "snapshot encryption failed"))?;
    output_directory.write_atomic(
        output
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(invalid)?,
        &envelope,
        false,
    )?;
    Ok(summary)
}
pub fn restore(
    input: &Path,
    target: &Path,
    expected_project: Option<&str>,
    expected_store_id: Option<&str>,
    passphrase: &[u8],
) -> Result<SnapshotSummary> {
    restore_frozen(
        input,
        target,
        expected_project,
        expected_store_id,
        passphrase,
        None,
    )
}
pub fn restore_frozen(
    input: &Path,
    target: &Path,
    expected_project: Option<&str>,
    expected_store_id: Option<&str>,
    passphrase: &[u8],
    expected_digest: Option<&str>,
) -> Result<SnapshotSummary> {
    let parent = target.parent().ok_or_else(invalid)?;
    let parent_dir = crate::fs::Directory::open(parent)?;
    restore_in_parent(
        input,
        target,
        expected_project,
        expected_store_id,
        passphrase,
        expected_digest,
        &parent_dir,
    )
}

/// All restore writes and publication stay relative to the caller's held parent.
pub fn restore_in_parent(
    input: &Path,
    target: &Path,
    expected_project: Option<&str>,
    expected_store_id: Option<&str>,
    passphrase: &[u8],
    expected_digest: Option<&str>,
    parent_dir: &crate::fs::Directory,
) -> Result<SnapshotSummary> {
    if !target.is_absolute() {
        return Err(ContextError::new(
            "INVALID_INPUT",
            "restore target must be absolute",
        ));
    }
    let (snapshot, summary) = read_snapshot_frozen(input, passphrase, expected_digest)?;
    if expected_project.is_some_and(|p| snapshot.store.project_id != p) {
        return Err(ContextError::new(
            "PROJECT_MISMATCH",
            "snapshot project differs from expected project",
        ));
    }
    if expected_store_id.is_some_and(|id| snapshot.store.store_id != id) {
        return Err(ContextError::new(
            "STORE_MISMATCH",
            "snapshot store differs from expected store",
        ));
    }
    if std::fs::symlink_metadata(target).is_ok() {
        return Err(ContextError::new(
            "RESTORE_TARGET_EXISTS",
            "restore target already exists",
        ));
    }
    let name = target
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(invalid)?;
    let stage = format!(".context-stage-{}", random_id()?);
    parent_dir.create_dir_new(&stage)?;
    let result = (|| {
        let dir = parent_dir.child(&stage)?;
        dir.create_dir("entries")?;
        dir.create_dir(".context-local")?;
        let entries = dir.child("entries")?;
        entries.write_atomic(".gitkeep", b"", false)?;
        for item in &snapshot.entries {
            entries.write_atomic(
                &format!("{}.md", item.id),
                item.content_utf8.as_bytes(),
                false,
            )?;
        }
        entries.sync()?;
        dir.write_atomic(
            ".gitignore",
            b"/.context-local/\n/entries/.context-stage-*\n",
            false,
        )?;
        dir.write_atomic(".gitattributes", b"entries/*.md text eol=lf\n", false)?;
        dir.write_atomic(
            "context.json",
            &serde_json::to_vec_pretty(&snapshot.store).map_err(|_| invalid())?,
            false,
        )?;
        dir.sync()?;
        parent_dir
            .publish_new_directory(&stage, name)
            .map_err(|e| {
                if e.code == "ALREADY_EXISTS" {
                    ContextError::new("RESTORE_TARGET_EXISTS", "restore target already exists")
                } else {
                    e
                }
            })?;
        Ok(summary)
    })();
    // Leave private staging on failure: reopening its path could traverse a replaced parent.
    result
}
fn random_id() -> Result<String> {
    Ok(crate::fs::random_id())
}
#[cfg(test)]
#[path = "snapshot_tests.rs"]
mod tests;
