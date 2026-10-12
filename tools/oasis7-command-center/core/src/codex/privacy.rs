use super::*;
use rusqlite::params;

impl Store {
    pub(super) fn private_sources(
        &self,
        scope: &str,
        thread: &str,
    ) -> Result<Vec<PrivateInputRef>> {
        let mut stmt = self.db.prepare("SELECT reference FROM cc_runtime_private_inputs WHERE scope=?1 AND thread=?2 ORDER BY rowid")?;
        stmt.query_map(params![scope, thread], |row| row.get::<_, String>(0))?
            .map(|row| Ok(serde_json::from_str(&row?)?))
            .collect()
    }
    pub(super) fn apply_private_sources(
        &self,
        scope: &str,
        snapshot: &mut RuntimeSnapshot,
    ) -> Result<()> {
        for session in &mut snapshot.sessions {
            session.private_sources = self.private_sources(scope, &session.thread_id)?;
        }
        Ok(())
    }
    fn mark_private(&self, scope: &str, thread: &str, sources: &[PrivateInputRef]) -> Result<()> {
        let tx = self.db.unchecked_transaction()?;
        for source in sources {
            tx.execute("INSERT OR IGNORE INTO cc_runtime_private_inputs(scope,thread,reference) VALUES(?1,?2,?3)", params![scope, thread, serde_json::to_string(source)?])?;
        }
        tx.commit()?;
        Ok(())
    }
}

impl Runtime {
    /// Caller must resolve and verify entry revisions before calling. Registration commits
    /// before any transport send. A failed send leaves the thread permanently private.
    pub fn start_private_turn(
        &mut self,
        project: &Project,
        thread_id: &str,
        input: &str,
        sources: Vec<PrivateInputRef>,
    ) -> Result<RuntimeSnapshot> {
        Self::input(input)?;
        let scope = self.gate(project, true)?;
        let snapshot = self.connections[&scope].snapshot();
        if !snapshot
            .sessions
            .iter()
            .any(|session| session.owned && session.thread_id == thread_id)
        {
            return Err(Error::Invalid(
                "private input requires an owned original thread".into(),
            ));
        }
        if sources.is_empty()
            || sources.len() > 32
            || sources.iter().any(|source| {
                source.binding_id.is_empty()
                    || source.binding_id.len() > 256
                    || source.binding_version == 0
                    || source.entry_id.is_empty()
                    || source.entry_id.len() > 256
                    || source.revision.is_empty()
                    || source.revision.len() > 256
                    || source.scope.is_empty()
                    || source.scope.len() > 256
            })
        {
            return Err(Error::Invalid(
                "explicit bounded private source references required".into(),
            ));
        }
        self.store.mark_private(&scope, thread_id, &sources)?;
        {
            let mut shared = self.connections[&scope].shared.lock().unwrap();
            self.store
                .apply_private_sources(&scope, &mut shared.snapshot)?;
        }
        self.start_turn(project, thread_id, input)
    }

    /// This projection is the only runtime data permitted in ordinary diagnostics and
    /// public exports. Private requests keep complete scope in the dedicated runtime UI.
    pub fn public_snapshot(&mut self, project: &Project) -> Result<RuntimeSnapshot> {
        let scope = key(project);
        let mut snapshot = self.snapshot(project)?;
        let mut stmt = self
            .store
            .db
            .prepare("SELECT DISTINCT thread FROM cc_runtime_private_inputs WHERE scope=?1")?;
        let private = stmt
            .query_map([scope], |row| row.get::<_, String>(0))?
            .collect::<std::result::Result<std::collections::HashSet<_>, _>>()?;
        snapshot
            .sessions
            .retain(|s| !private.contains(&s.thread_id));
        snapshot
            .requests
            .retain(|r| !private.contains(&r.thread_id));
        snapshot.operations.retain(|o| {
            o.thread_id
                .as_ref()
                .is_some_and(|thread| !private.contains(thread))
        });
        // Source error and general runtime logs may derive from any private thread.
        if !private.is_empty() {
            snapshot.logs.clear();
            snapshot.error = None;
        }
        Ok(snapshot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn reference() -> PrivateInputRef {
        PrivateInputRef {
            binding_id: "binding".into(),
            binding_version: 1,
            entry_id: "machine".into(),
            revision: "revision".into(),
            scope: "whole_entry".into(),
        }
    }
    #[test]
    fn private_mark_survives_stale_events_restart_and_later_unattached_turns() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("runtime.sqlite");
        let store = Store::open(&path).unwrap();
        let project = Project {
            id: "p".into(),
            path: "/tmp".into(),
            git_common_dir: "/tmp/.git".into(),
            branch: None,
            head: "h".into(),
            environment: "local".into(),
        };
        let mut stale = empty(&project);
        stale.instance_id = Some("instance".into());
        stale.sessions.push(RuntimeSession {
            purpose: "private purpose".into(),
            scope: "scope".into(),
            model: None,
            basis_revision: "h".into(),
            thread_id: "thread".into(),
            instance_id: "instance".into(),
            turn_id: None,
            state: "idle".into(),
            owned: true,
            background_state: "clear".into(),
            restorable_history: true,
            private_sources: vec![],
        });
        store.save("p:local", &stale).unwrap();
        store
            .mark_private("p:local", "thread", &[reference()])
            .unwrap();
        store
            .save_at("p:local", &stale, store::sequence(), true)
            .unwrap();
        assert_eq!(
            store.scopes().unwrap()[0].sessions[0].private_sources,
            vec![reference()]
        );
        drop(store);
        let runtime = Runtime::open(&path).unwrap();
        assert_eq!(
            runtime.contexts["p:local"].sessions[0].private_sources,
            vec![reference()]
        );
        let db = rusqlite::Connection::open(&path).unwrap();
        let raw: String = db
            .query_row("SELECT reference FROM cc_runtime_private_inputs", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert!(!raw.contains("private purpose"));
    }
    #[test]
    fn private_registration_failure_cannot_produce_a_transport_send() {
        let store = Store::open(Path::new(":memory:")).unwrap();
        store
            .db
            .execute_batch("DROP TABLE cc_runtime_private_inputs")
            .unwrap();
        assert!(
            store
                .mark_private("scope", "thread", &[reference()])
                .is_err()
        );
    }
}
