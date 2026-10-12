use super::dto::*;
use crate::{Error, Result};
use rusqlite::{Connection, params};
pub(crate) fn migrate(db: &Connection) -> Result<()> {
    let version: u32 = db.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if version > 2 {
        return Err(Error::Invalid(
            "database belongs to a newer application version".into(),
        ));
    }
    db.busy_timeout(std::time::Duration::from_secs(2))?;
    db.execute_batch("PRAGMA journal_mode=WAL;")?;
    let tx = db.unchecked_transaction()?;
    tx.execute_batch("CREATE TABLE IF NOT EXISTS personal(key TEXT PRIMARY KEY,value TEXT NOT NULL); CREATE TABLE IF NOT EXISTS cache(key TEXT PRIMARY KEY,value TEXT NOT NULL); CREATE TABLE IF NOT EXISTS history(key TEXT PRIMARY KEY,value TEXT NOT NULL); CREATE TABLE IF NOT EXISTS search(id TEXT PRIMARY KEY,body TEXT NOT NULL,value TEXT NOT NULL); CREATE VIRTUAL TABLE IF NOT EXISTS search_fts USING fts5(id UNINDEXED,body,tokenize='trigram'); CREATE TABLE IF NOT EXISTS cc_runtime_state(scope TEXT PRIMARY KEY,value TEXT NOT NULL,sequence INTEGER NOT NULL DEFAULT 0); CREATE TABLE IF NOT EXISTS cc_runtime_operations(id TEXT PRIMARY KEY,scope TEXT NOT NULL,target TEXT NOT NULL,state TEXT NOT NULL,value TEXT NOT NULL); CREATE INDEX IF NOT EXISTS cc_runtime_ops_scope ON cc_runtime_operations(scope,state); CREATE TABLE IF NOT EXISTS cc_runtime_private_inputs(scope TEXT NOT NULL,thread TEXT NOT NULL,reference TEXT NOT NULL,PRIMARY KEY(scope,thread,reference)); PRAGMA user_version=2;")?;
    let has_sequence: bool = {
        let mut stmt = tx.prepare("PRAGMA table_info(cc_runtime_state)")?;
        let names = stmt
            .query_map([], |r| r.get::<_, String>(1))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        names.iter().any(|n| n == "sequence")
    };
    if !has_sequence {
        tx.execute_batch(
            "ALTER TABLE cc_runtime_state ADD COLUMN sequence INTEGER NOT NULL DEFAULT 0;",
        )?;
    }
    tx.commit()?;
    Ok(())
}
pub(crate) struct Store {
    pub db: Connection,
    pub path: std::path::PathBuf,
}
impl Store {
    pub fn open(path: &std::path::Path) -> Result<Self> {
        let db = Connection::open(path)?;
        migrate(&db)?;
        let floor: i64 = db.query_row(
            "SELECT COALESCE(MAX(sequence),0) FROM cc_runtime_state",
            [],
            |r| r.get(0),
        )?;
        SEQUENCE.fetch_max(floor, std::sync::atomic::Ordering::SeqCst);
        db.execute("UPDATE cc_runtime_operations SET state='unknown' WHERE state IN ('pending','awaiting')",[])?;
        Ok(Self {
            db,
            path: path.to_path_buf(),
        })
    }
    pub fn scopes(&self) -> Result<Vec<RuntimeSnapshot>> {
        let mut stmt = self.db.prepare("SELECT value FROM cc_runtime_state")?;
        stmt.query_map([], |r| r.get::<_, String>(0))?
            .map(|r| {
                let mut snapshot: RuntimeSnapshot = serde_json::from_str(&r?)?;
                let scope = format!("{}:{}", snapshot.project_id, snapshot.environment);
                self.apply_private_sources(&scope, &mut snapshot)?;
                Ok(snapshot)
            })
            .collect()
    }
    pub fn writer(path: &std::path::Path) -> Result<Self> {
        let db = Connection::open(path)?;
        db.busy_timeout(std::time::Duration::from_secs(2))?;
        let version: u32 = db.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if version != 2 {
            return Err(Error::Invalid(
                "event writer requires initialized schema 2".into(),
            ));
        }
        let floor: i64 = db.query_row(
            "SELECT COALESCE(MAX(sequence),0) FROM cc_runtime_state",
            [],
            |r| r.get(0),
        )?;
        SEQUENCE.fetch_max(floor, std::sync::atomic::Ordering::SeqCst);
        Ok(Self {
            db,
            path: path.into(),
        })
    }
    pub fn save(&self, scope: &str, snapshot: &RuntimeSnapshot) -> Result<()> {
        self.save_at(scope, snapshot, sequence(), false)
    }
    pub fn save_at(
        &self,
        scope: &str,
        snapshot: &RuntimeSnapshot,
        sequence: i64,
        event: bool,
    ) -> Result<()> {
        let mut saved = snapshot.clone();
        self.apply_private_sources(scope, &mut saved)?;
        saved.logs.clear();
        let changed=self.db.execute("INSERT INTO cc_runtime_state(scope,value,sequence) VALUES(?1,?2,?3) ON CONFLICT(scope) DO UPDATE SET value=excluded.value,sequence=excluded.sequence WHERE excluded.sequence>=cc_runtime_state.sequence AND (?4=0 OR json_extract(cc_runtime_state.value,'$.instanceId') IS json_extract(excluded.value,'$.instanceId'))",params![scope,serde_json::to_string(&saved)?,sequence,event])?;
        if changed == 0 && !event {
            return Err(Error::Invalid(
                "runtime snapshot persistence rejected stale sequence".into(),
            ));
        }
        Ok(())
    }
    pub fn operations(&self, scope: &str) -> Result<Vec<RuntimeOperation>> {
        let mut stmt=self.db.prepare("SELECT value,state FROM cc_runtime_operations WHERE scope=?1 AND (state IN ('pending','awaiting','unknown') OR rowid IN (SELECT rowid FROM cc_runtime_operations WHERE scope=?1 ORDER BY rowid DESC LIMIT 200)) ORDER BY rowid DESC")?;
        stmt.query_map([scope], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })?
        .map(|r| {
            let (value, state) = r?;
            let mut op: RuntimeOperation = serde_json::from_str(&value)?;
            op.state = state;
            Ok(op)
        })
        .collect()
    }
    pub fn insert(
        &self,
        scope: &str,
        target: &str,
        op: &RuntimeOperation,
        management: bool,
    ) -> Result<()> {
        let tx = self.db.unchecked_transaction()?;
        let exists:usize=tx.query_row("SELECT COUNT(*) FROM cc_runtime_operations WHERE scope=?1 AND target=?2 AND state IN ('pending','awaiting','unknown')",params![scope,target],|r|r.get(0))?;
        if exists > 0 && !management {
            return Err(Error::Invalid(
                "target has an unresolved operation; reconcile before sending again".into(),
            ));
        }
        let unresolved:usize=tx.query_row("SELECT COUNT(*) FROM cc_runtime_operations WHERE scope=?1 AND state IN ('pending','awaiting','unknown')",[scope],|r|r.get(0))?;
        if unresolved >= 128 {
            return Err(Error::Invalid("unresolved operation limit reached".into()));
        }
        tx.execute(
            "INSERT INTO cc_runtime_operations(id,scope,target,state,value) VALUES(?1,?2,?3,?4,?5)",
            params![op.id, scope, target, op.state, serde_json::to_string(op)?],
        )?;
        tx.commit()?;
        Ok(())
    }
    pub fn settle(&self, op: &RuntimeOperation) -> Result<()> {
        self.db.execute(
            "UPDATE cc_runtime_operations SET state=?1,value=?2 WHERE id=?3",
            params![op.state, serde_json::to_string(op)?, op.id],
        )?;
        Ok(())
    }
}

static SEQUENCE: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);
pub(crate) fn sequence() -> i64 {
    let clock = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
        .min(i64::MAX as u128) as i64;
    SEQUENCE.fetch_max(clock, std::sync::atomic::Ordering::SeqCst);
    SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
}
