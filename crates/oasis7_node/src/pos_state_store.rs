use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{NodeError, NodeReplicationConfig, PosNodeEngine};

const POS_STATE_FILE_NAME: &str = "node_pos_state.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PosNodeStateSnapshot {
    pub next_height: u64,
    pub next_slot: u64,
    #[serde(default)]
    pub last_observed_slot: u64,
    #[serde(default)]
    pub missed_slot_count: u64,
    #[serde(default)]
    pub last_observed_tick: u64,
    #[serde(default)]
    pub missed_tick_count: u64,
    pub committed_height: u64,
    pub network_committed_height: u64,
    pub last_broadcast_proposal_height: u64,
    pub last_broadcast_local_attestation_height: u64,
    pub last_broadcast_committed_height: u64,
    #[serde(default)]
    pub last_committed_block_hash: Option<String>,
    #[serde(default)]
    pub last_execution_height: u64,
    #[serde(default)]
    pub last_execution_block_hash: Option<String>,
    #[serde(default)]
    pub last_execution_state_root: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PosNodeStateStore {
    path: PathBuf,
}

impl PosNodeStateStore {
    pub(crate) fn from_replication(replication: &NodeReplicationConfig) -> Self {
        Self {
            path: replication.root_dir.join(POS_STATE_FILE_NAME),
        }
    }

    pub(crate) fn load(&self) -> Result<Option<PosNodeStateSnapshot>, NodeError> {
        let bytes = match fs::read(&self.path) {
            Ok(bytes) => bytes,
            Err(err) if err.kind() == ErrorKind::NotFound => return Ok(None),
            Err(err) => {
                return Err(NodeError::Replication {
                    reason: format!(
                        "read node pos state {} failed: {}",
                        self.path.display(),
                        err
                    ),
                });
            }
        };
        let snapshot = serde_json::from_slice::<PosNodeStateSnapshot>(&bytes).map_err(|err| {
            NodeError::Replication {
                reason: format!(
                    "parse node pos state {} failed: {}",
                    self.path.display(),
                    err
                ),
            }
        })?;
        Ok(Some(snapshot))
    }

    pub(crate) fn save_engine_state(&self, engine: &PosNodeEngine) -> Result<(), NodeError> {
        self.save_snapshot(&engine.export_state_snapshot())
    }

    fn save_snapshot(&self, snapshot: &PosNodeStateSnapshot) -> Result<(), NodeError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|err| NodeError::Replication {
                reason: format!(
                    "create node pos state dir {} failed: {}",
                    parent.display(),
                    err
                ),
            })?;
        }
        let bytes = serde_json::to_vec_pretty(snapshot).map_err(|err| NodeError::Replication {
            reason: format!("serialize node pos state failed: {}", err),
        })?;
        let temp_path = self.path.with_extension("json.tmp");
        fs::write(&temp_path, bytes).map_err(|err| NodeError::Replication {
            reason: format!(
                "write node pos state temp {} failed: {}",
                temp_path.display(),
                err
            ),
        })?;
        fs::rename(&temp_path, &self.path).map_err(|err| NodeError::Replication {
            reason: format!(
                "rename node pos state temp {} -> {} failed: {}",
                temp_path.display(),
                self.path.display(),
                err
            ),
        })?;
        Ok(())
    }
}

impl PosNodeEngine {
    pub(super) fn export_state_snapshot(&self) -> PosNodeStateSnapshot {
        PosNodeStateSnapshot {
            next_height: self.next_height,
            next_slot: self.next_slot,
            last_observed_slot: self.last_observed_slot,
            missed_slot_count: self.missed_slot_count,
            last_observed_tick: self.last_observed_tick,
            missed_tick_count: self.missed_tick_count,
            committed_height: self.committed_height,
            network_committed_height: self.network_committed_height,
            last_broadcast_proposal_height: self.last_broadcast_proposal_height,
            last_broadcast_local_attestation_height: self.last_broadcast_local_attestation_height,
            last_broadcast_committed_height: self.last_broadcast_committed_height,
            last_committed_block_hash: self.last_committed_block_hash.clone(),
            last_execution_height: self.last_execution_height,
            last_execution_block_hash: self.last_execution_block_hash.clone(),
            last_execution_state_root: self.last_execution_state_root.clone(),
        }
    }

    pub(super) fn restore_state_snapshot(
        &mut self,
        snapshot: PosNodeStateSnapshot,
        now_ms: Option<i64>,
    ) -> Result<(), NodeError> {
        let PosNodeStateSnapshot {
            next_height: snapshot_next_height,
            next_slot,
            last_observed_slot: snapshot_last_observed_slot,
            missed_slot_count: snapshot_missed_slot_count,
            last_observed_tick: snapshot_last_observed_tick,
            missed_tick_count: snapshot_missed_tick_count,
            committed_height,
            network_committed_height: snapshot_network_committed_height,
            last_broadcast_proposal_height,
            last_broadcast_local_attestation_height,
            last_broadcast_committed_height,
            last_committed_block_hash,
            last_execution_height,
            last_execution_block_hash,
            last_execution_state_root,
        } = snapshot;
        let committed_successor =
            committed_height
                .checked_add(1)
                .ok_or_else(|| NodeError::Replication {
                    reason: format!(
                        "restore node pos state overflow: committed_height={} has no successor",
                        committed_height
                    ),
                })?;
        let restored_next_height = snapshot_next_height.max(committed_successor).max(1);
        let restored_network_committed_height =
            snapshot_network_committed_height.max(committed_height);
        if last_execution_block_hash.is_some() != last_execution_state_root.is_some() {
            return Err(NodeError::Replication {
                reason: format!(
                    "restore node pos state invalid execution binding pair: height={} block_hash_present={} state_root_present={}",
                    last_execution_height,
                    last_execution_block_hash.is_some(),
                    last_execution_state_root.is_some(),
                ),
            });
        }
        if last_execution_height > 0
            && (last_execution_block_hash.is_none() || last_execution_state_root.is_none())
        {
            return Err(NodeError::Replication {
                reason: format!(
                    "restore node pos state missing execution hashes for executed height {}",
                    last_execution_height
                ),
            });
        }
        if self.require_execution_on_commit && committed_height > last_execution_height {
            return Err(NodeError::Replication {
                reason: format!(
                    "restore node pos state invalid sequencer snapshot: committed_height={} exceeds last_execution_height={}",
                    committed_height, last_execution_height
                ),
            });
        }
        let restored_committed_hash = last_committed_block_hash.or_else(|| {
            if committed_height > 0 {
                Some(format!("legacy-height-{}", committed_height))
            } else {
                None
            }
        });

        let mut restored_next_slot = next_slot;
        let mut restored_last_observed_slot = snapshot_last_observed_slot;
        let mut restored_last_observed_tick = snapshot_last_observed_tick;
        if let (Some(genesis_unix_ms), Some(now_ms)) = (self.slot_clock_genesis_unix_ms, now_ms) {
            let elapsed_ms = if now_ms > genesis_unix_ms {
                (now_ms - genesis_unix_ms) as u64
            } else {
                0
            };
            let wall_clock_tick = (((elapsed_ms as u128)
                .saturating_mul(self.ticks_per_slot as u128))
                / self.slot_duration_ms as u128) as u64;
            let wall_clock_slot = wall_clock_tick / self.ticks_per_slot;
            restored_next_slot = restored_next_slot.min(wall_clock_slot);
            restored_last_observed_slot = restored_last_observed_slot.min(wall_clock_slot);
            restored_last_observed_tick = restored_last_observed_tick.min(wall_clock_tick);
        }

        self.pending = None;
        self.pending_consensus_actions.clear();
        self.committed_height = committed_height;
        self.network_committed_height = restored_network_committed_height;
        self.next_height = restored_next_height;
        self.next_slot = restored_next_slot;
        self.last_observed_slot = self
            .last_observed_slot
            .max(restored_last_observed_slot.max(restored_next_slot.saturating_sub(1)));
        self.missed_slot_count = self.missed_slot_count.max(snapshot_missed_slot_count);
        self.last_observed_tick = self.last_observed_tick.max(
            restored_last_observed_tick
                .max(restored_last_observed_slot.saturating_mul(self.ticks_per_slot)),
        );
        self.missed_tick_count = self.missed_tick_count.max(snapshot_missed_tick_count);
        self.last_broadcast_proposal_height = last_broadcast_proposal_height;
        self.last_broadcast_local_attestation_height = last_broadcast_local_attestation_height;
        self.last_broadcast_committed_height = last_broadcast_committed_height;
        self.last_committed_block_hash = restored_committed_hash;
        self.last_execution_height = last_execution_height;
        self.last_execution_block_hash = last_execution_block_hash;
        self.last_execution_state_root = last_execution_state_root;
        self.execution_bindings.clear();
        self.remember_execution_binding_for_height(self.last_execution_height);
        Ok(())
    }
}
