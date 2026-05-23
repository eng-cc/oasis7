use std::collections::VecDeque;
use std::fs;
use std::path::Path;

use super::super::persist::{PersistError, WorldJournal, WorldSnapshot};
use super::super::types::{CHUNK_GENERATION_SCHEMA_VERSION, JOURNAL_VERSION, SNAPSHOT_VERSION};
use super::WorldKernel;

impl WorldKernel {
    fn restore_persisted_state(
        snapshot: WorldSnapshot,
        journal_events: Vec<super::WorldEvent>,
    ) -> Self {
        Self {
            time: snapshot.time,
            config: snapshot.config.sanitized(),
            next_event_id: snapshot.next_event_id,
            next_action_id: snapshot.next_action_id,
            pending_actions: VecDeque::from(snapshot.pending_actions),
            journal: journal_events,
            model: snapshot.model,
            chunk_runtime: snapshot.chunk_runtime,
            intel_ttl_ticks: snapshot.intel_ttl_ticks,
            // Runtime-only caches and hooks are intentionally rebuilt per process.
            intel_cache: Default::default(),
            last_intent_batch_report: None,
            rule_hooks: Default::default(),
        }
    }

    pub fn snapshot(&self) -> WorldSnapshot {
        WorldSnapshot {
            version: SNAPSHOT_VERSION,
            chunk_generation_schema_version: CHUNK_GENERATION_SCHEMA_VERSION,
            time: self.time,
            config: self.config.clone(),
            model: self.model.clone(),
            runtime_snapshot: None,
            player_gameplay: None,
            chunk_runtime: self.chunk_runtime.clone(),
            intel_ttl_ticks: self.intel_ttl_ticks,
            next_event_id: self.next_event_id,
            next_action_id: self.next_action_id,
            pending_actions: self.pending_actions.iter().cloned().collect(),
            journal_len: self.journal.len(),
        }
    }

    pub fn journal_snapshot(&self) -> WorldJournal {
        WorldJournal {
            version: JOURNAL_VERSION,
            events: self.journal.clone(),
        }
    }

    pub fn from_snapshot(
        snapshot: WorldSnapshot,
        journal: WorldJournal,
    ) -> Result<Self, PersistError> {
        snapshot.validate_version()?;
        journal.validate_version()?;
        if snapshot.journal_len != journal.events.len() {
            return Err(PersistError::SnapshotMismatch {
                expected: snapshot.journal_len,
                actual: journal.events.len(),
            });
        }
        Ok(Self::restore_persisted_state(snapshot, journal.events))
    }

    pub fn replay_from_snapshot(
        snapshot: WorldSnapshot,
        journal: WorldJournal,
    ) -> Result<Self, PersistError> {
        snapshot.validate_version()?;
        journal.validate_version()?;
        if journal.events.len() < snapshot.journal_len {
            return Err(PersistError::SnapshotMismatch {
                expected: snapshot.journal_len,
                actual: journal.events.len(),
            });
        }
        if !snapshot.pending_actions.is_empty() && journal.events.len() > snapshot.journal_len {
            return Err(PersistError::ReplayConflict {
                message: "cannot replay with pending actions in snapshot".to_string(),
            });
        }

        let journal_len = snapshot.journal_len;
        let mut kernel = Self::restore_persisted_state(snapshot, journal.events.clone());

        for event in journal.events.iter().skip(journal_len) {
            kernel.apply_event(event)?;
        }
        let events_after_snapshot = journal.events.len() - journal_len;
        if events_after_snapshot > 0 {
            kernel.next_action_id = kernel
                .next_action_id
                .saturating_add(events_after_snapshot as u64);
        }

        Ok(kernel)
    }

    pub fn save_to_dir(&self, dir: impl AsRef<Path>) -> Result<(), PersistError> {
        let dir = dir.as_ref();
        fs::create_dir_all(dir)?;
        let snapshot_path = dir.join("snapshot.json");
        let journal_path = dir.join("journal.json");
        self.snapshot().save_json(&snapshot_path)?;
        self.journal_snapshot().save_json(&journal_path)?;
        Ok(())
    }

    pub fn load_from_dir(dir: impl AsRef<Path>) -> Result<Self, PersistError> {
        let dir = dir.as_ref();
        let snapshot_path = dir.join("snapshot.json");
        let journal_path = dir.join("journal.json");
        let snapshot = WorldSnapshot::load_json(&snapshot_path)?;
        let journal = WorldJournal::load_json(&journal_path)?;
        Self::from_snapshot(snapshot, journal)
    }
}
