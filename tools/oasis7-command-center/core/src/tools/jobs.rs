//! Durable admission before side effects, immutable operation identity and resource reservations.
use crate::{Error, Result};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use ts_rs::TS;
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ToolJob {
    pub id: String,
    pub fingerprint: String,
    pub resource: String,
    pub kind: String,
    pub state: String,
    pub observed_at: String,
    pub result: Option<String>,
}
/// Match the same ASCII filesystem alias rule used by signer admission only.
/// Operation IDs and fingerprints always retain exact comparison.
pub(crate) fn same_reserved_resource(recorded: &str, current: &str) -> bool {
    if current.starts_with("signer:") {
        recorded.eq_ignore_ascii_case(current)
    } else {
        recorded == current
    }
}
pub struct ToolJobs {
    inner: Mutex<Inner>,
    admission: Mutex<bool>,
}
struct Inner {
    db: Connection,
}
impl ToolJobs {
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let db = Connection::open(path)?;
        db.execute_batch("CREATE TABLE IF NOT EXISTS tool_recovery_evidence(id TEXT PRIMARY KEY,value TEXT NOT NULL); CREATE TABLE IF NOT EXISTS tool_jobs(id TEXT PRIMARY KEY,fingerprint TEXT NOT NULL,resource TEXT NOT NULL,kind TEXT NOT NULL,state TEXT NOT NULL,observed_at TEXT NOT NULL,result TEXT); UPDATE tool_jobs SET state='unknown' WHERE state='running';")?;
        Ok(Self {
            inner: Mutex::new(Inner { db }),
            admission: Mutex::new(false),
        })
    }
    pub fn admit(
        &self,
        id: &str,
        fingerprint: &str,
        resource: &str,
        kind: &str,
    ) -> Result<ToolJob> {
        self.admit_once(id, fingerprint, resource, kind)
            .map(|(job, _)| job)
    }
    pub fn admit_once(
        &self,
        id: &str,
        fingerprint: &str,
        resource: &str,
        kind: &str,
    ) -> Result<(ToolJob, bool)> {
        self.admit_recorded(id, fingerprint, resource, kind, None)
    }
    pub fn admit_once_with_evidence(
        &self,
        id: &str,
        fingerprint: &str,
        resource: &str,
        kind: &str,
        evidence: &crate::context::ContextRecoveryEvidence,
    ) -> Result<(ToolJob, bool)> {
        let value = serde_json::to_string(evidence)?;
        self.admit_recorded(id, fingerprint, resource, kind, Some(&value))
    }
    fn admit_recorded(
        &self,
        id: &str,
        fingerprint: &str,
        resource: &str,
        kind: &str,
        evidence: Option<&str>,
    ) -> Result<(ToolJob, bool)> {
        let frozen = self
            .admission
            .lock()
            .map_err(|_| Error::Invalid("tool admission lock".into()))?;
        if *frozen {
            return Err(Error::Invalid(
                "App exit has frozen new side effects".into(),
            ));
        }
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| Error::Invalid("tool jobs lock".into()))?;
        if id.is_empty() || fingerprint.is_empty() || resource.is_empty() {
            return Err(Error::Invalid("operation identity required".into()));
        }
        let tx = inner.db.transaction()?;
        if let Some(job) = get(&tx, id)? {
            if job.fingerprint != fingerprint || job.resource != resource || job.kind != kind {
                return Err(Error::Invalid(
                    "Operation ID conflicts with original input".into(),
                ));
            }
            if let Some(value) = evidence {
                let previous: Option<String> = tx
                    .query_row(
                        "SELECT value FROM tool_recovery_evidence WHERE id=?1",
                        [id],
                        |r| r.get(0),
                    )
                    .optional()?;
                if previous.as_deref() != Some(value) {
                    return Err(Error::Invalid(
                        "Operation recovery target conflicts with original evidence".into(),
                    ));
                }
            }
            return Ok((job, false));
        }
        // Valid signer identifiers are ASCII. On ordinary macOS volumes their
        // case variants can name the same Job directory, including old ledger
        // rows. Conservatively serialize those aliases without changing either
        // the immutable operation fingerprint or any non-signer resource.
        let resource_query = if resource.starts_with("signer:") {
            "SELECT EXISTS(SELECT 1 FROM tool_jobs WHERE resource=?1 COLLATE NOCASE AND state IN ('running','unknown'))"
        } else {
            "SELECT EXISTS(SELECT 1 FROM tool_jobs WHERE resource=?1 AND state IN ('running','unknown'))"
        };
        let busy: bool = tx.query_row(resource_query, [resource], |r| r.get(0))?;
        if busy {
            return Err(Error::Invalid(
                "Target already has an active or unknown operation; reconcile it first".into(),
            ));
        }
        let job = ToolJob {
            id: id.into(),
            fingerprint: fingerprint.into(),
            resource: resource.into(),
            kind: kind.into(),
            state: "running".into(),
            observed_at: crate::now(),
            result: None,
        };
        tx.execute(
            "INSERT INTO tool_jobs VALUES (?1,?2,?3,?4,?5,?6,NULL)",
            params![
                job.id,
                job.fingerprint,
                job.resource,
                job.kind,
                job.state,
                job.observed_at
            ],
        )?;
        if let Some(value) = evidence {
            tx.execute(
                "INSERT INTO tool_recovery_evidence(id,value) VALUES (?1,?2)",
                params![id, value],
            )?;
        }
        tx.commit()?;
        Ok((job, true))
    }
    pub fn recovery_evidence(
        &self,
        id: &str,
    ) -> Result<Option<crate::context::ContextRecoveryEvidence>> {
        let inner = self
            .inner
            .lock()
            .map_err(|_| Error::Invalid("tool jobs lock".into()))?;
        let value: Option<String> = inner
            .db
            .query_row(
                "SELECT value FROM tool_recovery_evidence WHERE id=?1",
                [id],
                |r| r.get(0),
            )
            .optional()?;
        value
            .map(|value| serde_json::from_str(&value).map_err(Error::from))
            .transpose()
    }
    pub fn reconcile_context(
        &self,
        id: &str,
        service: &crate::context::ContextService,
        passphrase: Option<&[u8]>,
    ) -> Result<ToolJob> {
        {
            let inner = self
                .inner
                .lock()
                .map_err(|_| Error::Invalid("tool jobs lock".into()))?;
            let job = get(&inner.db, id)?
                .ok_or_else(|| Error::Invalid("Unknown original operation".into()))?;
            if job.state != "unknown" {
                return Ok(job);
            }
        }
        let evidence = self.recovery_evidence(id)?.ok_or_else(|| {
            Error::Invalid("Original operation has no recoverable target evidence".into())
        })?;
        let conclusion = service.reconcile(&evidence, passphrase)?;
        self.finish(id, &conclusion.state, Some(&conclusion.reason))
    }
    pub fn finish(&self, id: &str, state: &str, result: Option<&str>) -> Result<ToolJob> {
        if !["succeeded", "failed", "cancelled", "partial", "unknown"].contains(&state) {
            return Err(Error::Invalid("invalid operation terminal state".into()));
        }
        let inner = self
            .inner
            .lock()
            .map_err(|_| Error::Invalid("tool jobs lock".into()))?;
        let old = get(&inner.db, id)?.ok_or_else(|| Error::Invalid("Unknown operation".into()))?;
        if ["succeeded", "failed", "cancelled", "partial"].contains(&old.state.as_str()) {
            return Ok(old);
        }
        inner.db.execute(
            "UPDATE tool_jobs SET state=?2,result=?3,observed_at=?4 WHERE id=?1",
            params![id, state, result, crate::now()],
        )?;
        Ok(get(&inner.db, id)?.unwrap())
    }
    pub fn list(&self) -> Result<Vec<ToolJob>> {
        let inner = self
            .inner
            .lock()
            .map_err(|_| Error::Invalid("tool jobs lock".into()))?;
        let mut query=inner.db.prepare("SELECT id,fingerprint,resource,kind,state,observed_at,result FROM tool_jobs ORDER BY observed_at DESC")?;
        Ok(query
            .query_map([], row)?
            .collect::<std::result::Result<Vec<_>, _>>()?)
    }
    /// Runtime admission shares the freeze gate, without holding the job database
    /// lock during network calls. Queries and operation receipts stay available.
    pub fn with_open<T>(&self, work: impl FnOnce() -> Result<T>) -> Result<T> {
        let frozen = self
            .admission
            .lock()
            .map_err(|_| Error::Invalid("tool admission lock".into()))?;
        if *frozen {
            return Err(Error::Invalid(
                "App exit has frozen new side effects".into(),
            ));
        }
        work()
    }
    pub fn ensure_open(&self) -> Result<()> {
        self.with_open(|| Ok(()))
    }
    pub fn freeze(&self) -> Result<()> {
        *self
            .admission
            .lock()
            .map_err(|_| Error::Invalid("tool admission lock".into()))? = true;
        Ok(())
    }
    pub fn unfreeze(&self) -> Result<()> {
        *self
            .admission
            .lock()
            .map_err(|_| Error::Invalid("tool admission lock".into()))? = false;
        Ok(())
    }
    pub fn quit_pending(&self) -> Result<bool> {
        Ok(self
            .list()?
            .iter()
            .any(|j| ["running", "unknown"].contains(&j.state.as_str())))
    }
}
fn row(r: &rusqlite::Row<'_>) -> rusqlite::Result<ToolJob> {
    Ok(ToolJob {
        id: r.get(0)?,
        fingerprint: r.get(1)?,
        resource: r.get(2)?,
        kind: r.get(3)?,
        state: r.get(4)?,
        observed_at: r.get(5)?,
        result: r.get(6)?,
    })
}
fn get(db: &Connection, id: &str) -> Result<Option<ToolJob>> {
    Ok(db.query_row("SELECT id,fingerprint,resource,kind,state,observed_at,result FROM tool_jobs WHERE id=?1",[id],row).optional()?)
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signer_job_case_aliases_preserve_existing_running_and_unknown_reservations() {
        let jobs = ToolJobs::open(":memory:").unwrap();
        jobs.admit(
            "original",
            "original-input",
            "signer:install:JobOne",
            "file_prepare",
        )
        .unwrap();
        assert!(
            jobs.admit(
                "another",
                "other-input",
                "signer:install:jobone",
                "file_prepare"
            )
            .is_err()
        );
        jobs.finish("original", "unknown", None).unwrap();
        assert!(
            jobs.admit(
                "another",
                "other-input",
                "signer:install:JOBONE",
                "file_prepare"
            )
            .is_err()
        );
        assert!(
            jobs.admit(
                "original",
                "original-input",
                "signer:install:jobone",
                "file_prepare"
            )
            .is_err()
        );
        jobs.finish("original", "succeeded", None).unwrap();
        assert!(
            jobs.admit(
                "another",
                "other-input",
                "signer:install:jobone",
                "file_prepare"
            )
            .is_ok()
        );
        // Only signer Jobs have this conservative filesystem-alias guard.
        jobs.admit("cli-upper", "upper", "cli:Release", "install")
            .unwrap();
        assert!(
            jobs.admit("cli-lower", "lower", "cli:release", "install")
                .is_ok()
        );
    }
    #[test]
    fn runtime_admission_does_not_block_job_queries_or_receipts() {
        let jobs = std::sync::Arc::new(ToolJobs::open(":memory:").unwrap());
        jobs.admit("save", "hash", "store", "save").unwrap();
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let worker = jobs.clone();
        let thread = std::thread::spawn(move || {
            worker
                .with_open(|| {
                    started_tx.send(()).unwrap();
                    release_rx.recv().unwrap();
                    Ok(())
                })
                .unwrap()
        });
        started_rx.recv().unwrap();
        assert_eq!(jobs.list().unwrap().len(), 1);
        jobs.finish("save", "succeeded", None).unwrap();
        let (frozen_tx, frozen_rx) = std::sync::mpsc::channel();
        let freeze_jobs = jobs.clone();
        let freeze = std::thread::spawn(move || {
            freeze_jobs.freeze().unwrap();
            frozen_tx.send(()).unwrap();
        });
        assert!(
            frozen_rx
                .recv_timeout(std::time::Duration::from_millis(50))
                .is_err()
        );
        release_tx.send(()).unwrap();
        thread.join().unwrap();
        freeze.join().unwrap();
        assert!(jobs.with_open(|| Ok(())).is_err());
    }
    #[test]
    fn idempotent_admission_freeze_and_terminal_monotonicity() {
        let jobs = ToolJobs::open(":memory:").unwrap();
        jobs.admit("one", "hash", "root", "save").unwrap();
        assert!(jobs.admit("one", "different", "root", "save").is_err());
        assert!(jobs.admit("two", "hash", "root", "save").is_err());
        jobs.finish("one", "succeeded", None).unwrap();
        assert_eq!(
            jobs.finish("one", "unknown", None).unwrap().state,
            "succeeded"
        );
        jobs.freeze().unwrap();
        assert!(jobs.admit("two", "hash", "other", "save").is_err());
        jobs.unfreeze().unwrap();
        jobs.admit("two", "hash", "other", "save").unwrap();
        assert!(jobs.quit_pending().unwrap());
    }
}
