#[path = "context/recovery.rs"]
mod recovery;
pub use recovery::{ContextReconciliation, ContextRecoveryEvidence};
pub fn builtin_info() -> serde_json::Value {
    serde_json::to_value(application::info()).expect("static capability information serializes")
}

// On-demand private library access. Only binding identities are persisted here.
use crate::{Error, Result};
use oasis7_context::{application, model::WriteCondition, store::Store};
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use ts_rs::TS;

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ContextBinding {
    pub id: String,
    #[ts(type = "number")]
    pub binding_version: u64,
    pub root: String,
    pub store_id: String,
    pub project_id: String,
    #[serde(default)]
    pub registered_project_id: String,
    pub name: String,
    pub directory_identity: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ContextEntry {
    pub id: String,
    pub revision: String,
    pub raw_text: String,
    pub metadata_json: String,
    pub verification_state: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct ContextSummary {
    pub id: String,
    pub title: String,
    pub kind: String,
    pub environment: String,
    pub state: String,
    pub revision: String,
    pub verification_state: String,
    pub summary: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct ContextQuery {
    pub entries: Vec<ContextSummary>,
    pub complete: bool,
    pub total_matches: Option<usize>,
    pub has_more: bool,
    pub next_cursor: Option<String>,
    pub warnings: Vec<String>,
    pub read_at: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ContextSaveRequest {
    pub binding_id: String,
    #[ts(type = "number")]
    pub binding_version: u64,
    pub id: String,
    pub raw_text: String,
    /// None means create only, never unconditional overwrite.
    pub expected_revision: Option<String>,
    pub actor: String,
}
pub struct ContextService {
    db: Connection,
}
pub(crate) fn migrate(db: &Connection) -> Result<()> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS context_bindings(id TEXT PRIMARY KEY,value TEXT NOT NULL); CREATE TABLE IF NOT EXISTS ordinary_projects(path TEXT PRIMARY KEY); CREATE TABLE IF NOT EXISTS context_reserved_roots(path TEXT PRIMARY KEY);")?;
    Ok(())
}
fn failure(e: impl std::fmt::Display) -> Error {
    Error::Invalid(e.to_string())
}
fn canonical_candidate(path: &Path) -> Result<PathBuf> {
    if !path.is_absolute() {
        return Err(failure("private directory must be absolute"));
    }
    if path.exists() {
        return Ok(path.canonicalize()?);
    }
    let parent = path
        .parent()
        .ok_or_else(|| failure("directory needs parent"))?
        .canonicalize()?;
    let name = path
        .file_name()
        .ok_or_else(|| failure("directory needs name"))?;
    Ok(parent.join(name))
}
fn overlaps(a: &Path, b: &Path) -> bool {
    a.starts_with(b) || b.starts_with(a)
}
fn bindings(db: &Connection) -> Result<Vec<ContextBinding>> {
    let mut statement = db.prepare("SELECT value FROM context_bindings ORDER BY id")?;
    let values = statement.query_map([], |r| r.get::<_, String>(0))?;
    values.map(|v| Ok(serde_json::from_str(&v?)?)).collect()
}
pub(crate) fn check_project(db: &Connection, path: &Path) -> Result<()> {
    // Cached source facts remain readable when a worktree is temporarily absent.
    // Its registered absolute path still participates in the privacy exclusion.
    let root = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    for binding in bindings(db)? {
        let registered = Path::new(&binding.root);
        let current = canonical_candidate(registered)?;
        if overlaps(&root, registered) || overlaps(&root, &current) {
            return Err(failure(
                "ordinary project overlaps a private library; choose independent directories",
            ));
        }
    }
    let mut statement = db.prepare("SELECT path FROM context_reserved_roots")?;
    for value in statement.query_map([], |r| r.get::<_, String>(0))? {
        let value = value?;
        let registered = Path::new(&value);
        let current = registered
            .canonicalize()
            .unwrap_or_else(|_| registered.to_path_buf());
        if overlaps(&root, registered) || overlaps(&root, &current) {
            return Err(failure(
                "ordinary project overlaps an original private restore target",
            ));
        }
    }
    Ok(())
}
pub(crate) fn register_project(db: &Connection, path: &Path) -> Result<()> {
    let tx = Transaction::new_unchecked(db, TransactionBehavior::Immediate)?;
    check_project(&tx, path)?;
    tx.execute(
        "INSERT OR IGNORE INTO ordinary_projects(path) VALUES (?1)",
        [path.to_string_lossy()],
    )?;
    tx.commit()?;
    Ok(())
}
fn directory_identity(path: &Path) -> Result<String> {
    use std::os::unix::fs::MetadataExt;
    let metadata = std::fs::metadata(path)?;
    Ok(format!("{}:{}", metadata.dev(), metadata.ino()))
}
impl ContextService {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let db = Connection::open(path)?;
        migrate(&db)?;
        db.busy_timeout(std::time::Duration::from_secs(5))?;
        Ok(Self { db })
    }
    pub fn bindings(&self) -> Result<Vec<ContextBinding>> {
        bindings(&self.db)
    }
    pub fn bind(
        &self,
        path: &Path,
        project_id: &str,
        name: &str,
        create: bool,
    ) -> Result<ContextBinding> {
        let root = canonical_candidate(path)?;
        let tx = Transaction::new_unchecked(&self.db, TransactionBehavior::Immediate)?;
        let mut statement = tx.prepare("SELECT path FROM ordinary_projects")?;
        for value in statement.query_map([], |r| r.get::<_, String>(0))? {
            let value = value?;
            let registered = Path::new(&value);
            let current = canonical_candidate(registered)?;
            if overlaps(&root, registered) || overlaps(&root, &current) {
                return Err(failure(
                    "private library overlaps an ordinary project; choose an independent directory",
                ));
            }
        }
        drop(statement);
        let store = if create {
            Store::init(&root, project_id, name)
        } else {
            application::open(Some(&root))
        }
        .map_err(Error::Context)?;
        let actual = root.canonicalize()?;
        if !store
            .directory()
            .matches_path(&actual)
            .map_err(Error::Context)?
        {
            return Err(failure("private library replaced while binding"));
        }
        let (device, inode) = store.directory().identity().map_err(Error::Context)?;
        let identity = format!("{device}:{inode}");
        let id = crate::index::digest(actual.to_string_lossy().as_bytes());
        let previous: Option<String> = tx
            .query_row(
                "SELECT value FROM context_bindings WHERE id=?1",
                [&id],
                |r| r.get(0),
            )
            .optional()?;
        let version = previous
            .map(|v| serde_json::from_str::<ContextBinding>(&v))
            .transpose()?
            .map_or(1, |b| b.binding_version + 1);
        let binding = ContextBinding {
            id,
            binding_version: version,
            root: actual.to_string_lossy().into(),
            directory_identity: identity,
            store_id: store.config.store_id,
            project_id: store.config.project_id,
            registered_project_id: project_id.into(),
            name: store.config.name,
        };
        tx.execute("INSERT INTO context_bindings(id,value) VALUES (?1,?2) ON CONFLICT(id) DO UPDATE SET value=excluded.value", params![binding.id, serde_json::to_string(&binding)?])?;
        tx.commit()?;
        Ok(binding)
    }
    pub fn validate_binding(&self, id: &str, version: u64) -> Result<ContextBinding> {
        let binding = self
            .bindings()?
            .into_iter()
            .find(|b| b.id == id)
            .ok_or_else(|| failure("unknown private library binding"))?;
        if binding.binding_version != version {
            return Err(failure("private library binding changed"));
        }
        let root = Path::new(&binding.root);
        if root.canonicalize()? != root || directory_identity(root)? != binding.directory_identity {
            return Err(failure("private library directory identity changed"));
        }
        let mut statement = self.db.prepare("SELECT path FROM ordinary_projects")?;
        for value in statement.query_map([], |r| r.get::<_, String>(0))? {
            let value = value?;
            if overlaps(root, &canonical_candidate(Path::new(&value))?) {
                return Err(failure(
                    "ordinary project directory now overlaps private library",
                ));
            }
        }
        let store = application::open(Some(root)).map_err(Error::Context)?;
        if store.config.store_id != binding.store_id
            || store.config.project_id != binding.project_id
        {
            return Err(failure("private library identity changed"));
        }
        Ok(binding)
    }
    fn store(&self, id: &str, version: u64) -> Result<Store> {
        let binding = self.validate_binding(id, version)?;
        let store = application::open(Some(Path::new(&binding.root))).map_err(Error::Context)?;
        let (device, inode) = store.directory().identity().map_err(Error::Context)?;
        if format!("{device}:{inode}") != binding.directory_identity
            || store.config.store_id != binding.store_id
            || store.config.project_id != binding.project_id
        {
            return Err(failure("private library replaced while opening"));
        }
        Ok(store)
    }
    pub fn get_bound(&self, id: &str, version: u64, entry_id: &str) -> Result<ContextEntry> {
        let snapshot = self
            .store(id, version)?
            .get(entry_id)
            .map_err(Error::Context)?;
        Ok(ContextEntry {
            id: snapshot.entry.metadata.id.clone(),
            revision: snapshot.revision,
            raw_text: String::from_utf8(snapshot.raw_bytes).map_err(failure)?,
            metadata_json: serde_json::to_string(&snapshot.entry.metadata)?,
            verification_state: serde_json::to_value(snapshot.verification_state)?
                .as_str()
                .unwrap_or("unverified")
                .into(),
        })
    }
    pub fn list(
        &self,
        id: &str,
        version: u64,
        query: &str,
        include_archived: bool,
    ) -> Result<ContextQuery> {
        let store = self.store(id, version)?;
        let filters = oasis7_context::query::ListQuery {
            include_archived,
            limit: 200,
            ..Default::default()
        };
        let result = if query.trim().is_empty() {
            store.list(&filters)
        } else {
            store.search(&oasis7_context::query::SearchQuery {
                query: query.into(),
                filters,
            })
        }
        .map_err(Error::Context)?;
        Ok(serde_json::from_value(serde_json::to_value(result)?)?)
    }
    pub fn save(&self, request: &ContextSaveRequest) -> Result<serde_json::Value> {
        let store = self.store(&request.binding_id, request.binding_version)?;
        let result = application::save_raw(
            &store,
            &request.id,
            request.raw_text.as_bytes(),
            condition(request.expected_revision.as_deref()),
            &request.actor,
        )
        .map_err(Error::Context)?;
        Ok(serde_json::to_value(result)?)
    }
    pub fn archive(
        &self,
        id: &str,
        version: u64,
        entry_id: &str,
        revision: &str,
        actor: &str,
    ) -> Result<serde_json::Value> {
        Ok(serde_json::to_value(
            self.store(id, version)?
                .archive(entry_id, revision, actor)
                .map_err(Error::Context)?,
        )?)
    }
    pub fn history(&self, id: &str, version: u64, entry_id: &str) -> Result<serde_json::Value> {
        let store = self.store(id, version)?;
        Ok(serde_json::to_value(
            application::history(&store, entry_id, 100).map_err(Error::Context)?,
        )?)
    }
    pub fn restore_entry(
        &self,
        id: &str,
        version: u64,
        entry_id: &str,
        commit: &str,
        revision: Option<&str>,
        actor: &str,
    ) -> Result<serde_json::Value> {
        let store = self.store(id, version)?;
        let (commit, result) =
            application::restore_entry(&store, entry_id, commit, condition(revision), actor)
                .map_err(Error::Context)?;
        Ok(serde_json::json!({"commit":commit,"save":result}))
    }
    pub fn snapshot_export(
        &self,
        id: &str,
        version: u64,
        output: &Path,
        selection: &[String],
        passphrase: &[u8],
    ) -> Result<serde_json::Value> {
        let store = self.store(id, version)?;
        Ok(serde_json::to_value(
            application::snapshot_export(&store.root, output, selection, passphrase, 0)
                .map_err(Error::Context)?,
        )?)
    }
    pub fn snapshot_inspect(&self, input: &Path, passphrase: &[u8]) -> Result<serde_json::Value> {
        Ok(serde_json::to_value(
            application::snapshot_inspect(input, passphrase).map_err(Error::Context)?,
        )?)
    }
    pub fn snapshot_restore(
        &self,
        id: &str,
        version: u64,
        input: &Path,
        target: &Path,
        passphrase: &[u8],
    ) -> Result<ContextBinding> {
        self.snapshot_restore_checked(id, version, input, target, passphrase, None)
    }
    pub fn snapshot_restore_frozen(
        &self,
        id: &str,
        version: u64,
        input: &Path,
        target: &Path,
        passphrase: &[u8],
        expected_input_sha256: &str,
    ) -> Result<ContextBinding> {
        self.snapshot_restore_checked(
            id,
            version,
            input,
            target,
            passphrase,
            Some(expected_input_sha256),
        )
    }
    fn snapshot_restore_checked(
        &self,
        id: &str,
        version: u64,
        input: &Path,
        target: &Path,
        passphrase: &[u8],
        expected_digest: Option<&str>,
    ) -> Result<ContextBinding> {
        let binding = self.validate_binding(id, version)?;
        let root = canonical_candidate(target)?;
        let tx = Transaction::new_unchecked(&self.db, TransactionBehavior::Immediate)?;
        let mut statement = tx.prepare("SELECT path FROM ordinary_projects")?;
        for value in statement.query_map([], |r| r.get::<_, String>(0))? {
            let value = value?;
            if overlaps(&root, &canonical_candidate(Path::new(&value))?) {
                return Err(failure("restore target overlaps ordinary project"));
            }
        }
        drop(statement);
        tx.execute(
            "INSERT OR IGNORE INTO context_reserved_roots(path) VALUES (?1)",
            [root.to_string_lossy()],
        )?;
        tx.commit()?;
        oasis7_context::snapshot::restore_frozen(
            input,
            &root,
            Some(&binding.project_id),
            Some(&binding.store_id),
            passphrase,
            expected_digest,
        )
        .map_err(Error::Context)?;
        self.bind(&root, &binding.registered_project_id, &binding.name, false)
    }
    pub fn doctor(&self, id: &str, version: u64) -> Result<serde_json::Value> {
        self.store(id, version)?.doctor().map_err(Error::Context)
    }
}
fn condition(revision: Option<&str>) -> WriteCondition {
    revision
        .map(|r| WriteCondition::Revision(r.into()))
        .unwrap_or(WriteCondition::Absent)
}
