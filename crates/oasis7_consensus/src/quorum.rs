// Quorum-based consensus helpers for distributed head commits.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::distributed::WorldHeadAnnounce;
use super::distributed_dht::DistributedDht;
use super::distributed_lease::LeaseState;
use super::error::WorldError;
use super::util::{read_json_from_path, write_json_to_path};
pub use oasis7_proto::distributed_consensus::{
    ConsensusMembershipChange, ConsensusMembershipChangeRequest, ConsensusMembershipChangeResult,
    ConsensusStatus, ConsensusVote, HeadConsensusRecord,
};

pub const CONSENSUS_SNAPSHOT_VERSION: u64 = 1;
const DEFAULT_MAX_RECORDS_PER_WORLD: usize = 4096;

fn default_max_records_per_world() -> usize {
    DEFAULT_MAX_RECORDS_PER_WORLD
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsensusConfig {
    pub validators: Vec<String>,
    pub quorum_threshold: usize,
    #[serde(default = "default_max_records_per_world")]
    pub max_records_per_world: usize,
}

impl ConsensusConfig {
    pub fn majority(validators: Vec<String>) -> Self {
        Self {
            validators,
            quorum_threshold: 0,
            max_records_per_world: default_max_records_per_world(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsensusDecision {
    pub world_id: String,
    pub height: u64,
    pub block_hash: String,
    pub status: ConsensusStatus,
    pub approvals: usize,
    pub rejections: usize,
    pub quorum_threshold: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ConsensusSnapshotFile {
    version: u64,
    validators: Vec<String>,
    quorum_threshold: usize,
    #[serde(default = "default_max_records_per_world")]
    max_records_per_world: usize,
    records: Vec<HeadConsensusRecord>,
}

#[derive(Debug, Clone)]
pub struct QuorumConsensus {
    validators: BTreeSet<String>,
    quorum_threshold: usize,
    max_records_per_world: usize,
    records: BTreeMap<(String, u64), HeadConsensusRecord>,
}

impl QuorumConsensus {
    pub fn new(config: ConsensusConfig) -> Result<Self, WorldError> {
        let mut validators = BTreeSet::new();
        for validator in config.validators {
            let trimmed = validator.trim();
            if trimmed.is_empty() {
                continue;
            }
            validators.insert(trimmed.to_string());
        }
        if validators.is_empty() {
            return Err(WorldError::DistributedValidationFailed {
                reason: "consensus validators cannot be empty".to_string(),
            });
        }

        let validator_count = validators.len();
        let quorum_threshold = if config.quorum_threshold == 0 {
            validator_count / 2 + 1
        } else {
            config.quorum_threshold
        };
        if quorum_threshold == 0 || quorum_threshold > validator_count {
            return Err(WorldError::DistributedValidationFailed {
                reason: format!(
                    "invalid quorum threshold {quorum_threshold} for {validator_count} validators"
                ),
            });
        }
        if quorum_threshold <= validator_count / 2 {
            return Err(WorldError::DistributedValidationFailed {
                reason: format!(
                    "unsafe quorum threshold {quorum_threshold}; requires > half of {validator_count}"
                ),
            });
        }
        if config.max_records_per_world == 0 {
            return Err(WorldError::DistributedValidationFailed {
                reason: "max_records_per_world must be positive".to_string(),
            });
        }

        Ok(Self {
            validators,
            quorum_threshold,
            max_records_per_world: config.max_records_per_world,
            records: BTreeMap::new(),
        })
    }

    pub fn validators(&self) -> Vec<String> {
        self.validators.iter().cloned().collect()
    }

    pub fn quorum_threshold(&self) -> usize {
        self.quorum_threshold
    }

    pub fn max_records_per_world(&self) -> usize {
        self.max_records_per_world
    }

    pub fn record(&self, world_id: &str, height: u64) -> Option<&HeadConsensusRecord> {
        self.records.get(&(world_id.to_string(), height))
    }

    pub fn export_records(&self) -> Vec<HeadConsensusRecord> {
        self.records.values().cloned().collect()
    }

    pub fn import_records(&mut self, records: Vec<HeadConsensusRecord>) -> Result<(), WorldError> {
        self.restore_records(records)
    }

    pub fn save_snapshot_to_path(&self, path: impl AsRef<Path>) -> Result<(), WorldError> {
        let path = path.as_ref();
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent)?;
        }

        let snapshot = ConsensusSnapshotFile {
            version: CONSENSUS_SNAPSHOT_VERSION,
            validators: self.validators(),
            quorum_threshold: self.quorum_threshold,
            max_records_per_world: self.max_records_per_world,
            records: self.export_records(),
        };
        write_json_atomic(&snapshot, path)
    }

    pub fn load_snapshot_from_path(path: impl AsRef<Path>) -> Result<Self, WorldError> {
        let snapshot: ConsensusSnapshotFile = read_json_from_path(path.as_ref())?;
        if snapshot.version != CONSENSUS_SNAPSHOT_VERSION {
            return Err(WorldError::DistributedValidationFailed {
                reason: format!(
                    "unsupported consensus snapshot version {} (expected {})",
                    snapshot.version, CONSENSUS_SNAPSHOT_VERSION
                ),
            });
        }

        let mut consensus = Self::new(ConsensusConfig {
            validators: snapshot.validators,
            quorum_threshold: snapshot.quorum_threshold,
            max_records_per_world: snapshot.max_records_per_world,
        })?;
        consensus.restore_records(snapshot.records)?;
        Ok(consensus)
    }

    pub fn apply_membership_change(
        &mut self,
        request: &ConsensusMembershipChangeRequest,
    ) -> Result<ConsensusMembershipChangeResult, WorldError> {
        if self.has_pending_records() {
            return Err(WorldError::DistributedValidationFailed {
                reason: "membership change is blocked while pending consensus records exist"
                    .to_string(),
            });
        }

        match &request.change {
            ConsensusMembershipChange::AddValidator { validator_id } => {
                let validator_id = validator_id.trim();
                if validator_id.is_empty() {
                    return Err(WorldError::DistributedValidationFailed {
                        reason: "validator_id cannot be empty".to_string(),
                    });
                }
                if self.validators.contains(validator_id) {
                    return Ok(ConsensusMembershipChangeResult {
                        applied: false,
                        validators: self.validators(),
                        quorum_threshold: self.quorum_threshold,
                    });
                }

                let mut validators = self.validators();
                validators.push(validator_id.to_string());
                let quorum_threshold =
                    derive_membership_threshold(self.quorum_threshold, validators.len());
                self.apply_membership_config(ConsensusConfig {
                    validators,
                    quorum_threshold,
                    max_records_per_world: self.max_records_per_world,
                })
            }
            ConsensusMembershipChange::RemoveValidator { validator_id } => {
                let validator_id = validator_id.trim();
                if validator_id.is_empty() {
                    return Err(WorldError::DistributedValidationFailed {
                        reason: "validator_id cannot be empty".to_string(),
                    });
                }
                if !self.validators.contains(validator_id) {
                    return Err(WorldError::DistributedValidationFailed {
                        reason: format!("validator not found: {validator_id}"),
                    });
                }

                let mut validators = self.validators();
                validators.retain(|candidate| candidate != validator_id);
                let quorum_threshold =
                    derive_membership_threshold(self.quorum_threshold, validators.len());
                self.apply_membership_config(ConsensusConfig {
                    validators,
                    quorum_threshold,
                    max_records_per_world: self.max_records_per_world,
                })
            }
            ConsensusMembershipChange::ReplaceValidators {
                validators,
                quorum_threshold,
            } => self.apply_membership_config(ConsensusConfig {
                validators: validators.clone(),
                quorum_threshold: *quorum_threshold,
                max_records_per_world: self.max_records_per_world,
            }),
        }
    }

    pub fn apply_membership_change_with_lease(
        &mut self,
        request: &ConsensusMembershipChangeRequest,
        lease: Option<&LeaseState>,
    ) -> Result<ConsensusMembershipChangeResult, WorldError> {
        let lease = lease.ok_or_else(|| WorldError::DistributedValidationFailed {
            reason: "membership change requires an active lease".to_string(),
        })?;
        if lease.holder_id != request.requester_id {
            return Err(WorldError::DistributedValidationFailed {
                reason: format!(
                    "membership change requester {} is not lease holder {}",
                    request.requester_id, lease.holder_id
                ),
            });
        }
        if lease.expires_at_ms <= request.requested_at_ms {
            return Err(WorldError::DistributedValidationFailed {
                reason: format!(
                    "lease {} expired at {}, request time {}",
                    lease.lease_id, lease.expires_at_ms, request.requested_at_ms
                ),
            });
        }

        self.apply_membership_change(request)
    }

    pub fn propose_head(
        &mut self,
        head: &WorldHeadAnnounce,
        proposer_id: impl Into<String>,
        proposed_at_ms: i64,
    ) -> Result<ConsensusDecision, WorldError> {
        let proposer_id = proposer_id.into();
        self.ensure_validator(&proposer_id)?;

        if let Some(committed_height) = self.latest_committed_height(&head.world_id)
            && head.height <= committed_height
        {
            return Err(WorldError::DistributedValidationFailed {
                reason: format!(
                    "stale proposal for {} at height {} (committed={committed_height})",
                    head.world_id, head.height
                ),
            });
        }

        let key = (head.world_id.clone(), head.height);
        let validator_count = self.validators.len();
        let quorum_threshold = self.quorum_threshold;
        let record = self
            .records
            .entry(key)
            .or_insert_with(|| HeadConsensusRecord {
                head: head.clone(),
                proposer_id: proposer_id.clone(),
                proposed_at_ms,
                quorum_threshold,
                validator_count,
                status: ConsensusStatus::Pending,
                votes: BTreeMap::new(),
            });
        if record.head.block_hash != head.block_hash {
            return Err(WorldError::DistributedValidationFailed {
                reason: format!(
                    "conflicting proposal for {} at height {}: existing={}, new={}",
                    head.world_id, head.height, record.head.block_hash, head.block_hash
                ),
            });
        }

        let decision = Self::apply_vote(
            record,
            record.validator_count,
            record.quorum_threshold,
            &proposer_id,
            true,
            proposed_at_ms,
            Some("proposal accepted".to_string()),
        )?;
        self.prune_world_records(head.world_id.as_str());
        Ok(decision)
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "Public vote state-machine API keeps explicit proposal and decision fields"
    )]
    pub fn vote_head(
        &mut self,
        world_id: &str,
        height: u64,
        block_hash: &str,
        validator_id: &str,
        approve: bool,
        voted_at_ms: i64,
        reason: Option<String>,
    ) -> Result<ConsensusDecision, WorldError> {
        self.ensure_validator(validator_id)?;

        let key = (world_id.to_string(), height);
        let record =
            self.records
                .get_mut(&key)
                .ok_or_else(|| WorldError::DistributedValidationFailed {
                    reason: format!("proposal not found for {world_id} at height {height}"),
                })?;
        if record.head.block_hash != block_hash {
            return Err(WorldError::DistributedValidationFailed {
                reason: format!(
                    "vote block hash mismatch for {world_id} at height {height}: expected={}, got={block_hash}",
                    record.head.block_hash
                ),
            });
        }

        let decision = Self::apply_vote(
            record,
            record.validator_count,
            record.quorum_threshold,
            validator_id,
            approve,
            voted_at_ms,
            reason,
        )?;
        self.prune_world_records(world_id);
        Ok(decision)
    }

    fn ensure_validator(&self, validator_id: &str) -> Result<(), WorldError> {
        if self.validators.contains(validator_id) {
            return Ok(());
        }
        Err(WorldError::DistributedValidationFailed {
            reason: format!("validator not allowed: {validator_id}"),
        })
    }

    fn has_pending_records(&self) -> bool {
        self.records
            .values()
            .any(|record| matches!(record.status, ConsensusStatus::Pending))
    }

    fn apply_membership_config(
        &mut self,
        config: ConsensusConfig,
    ) -> Result<ConsensusMembershipChangeResult, WorldError> {
        let next = Self::new(config)?;
        let applied =
            self.validators != next.validators || self.quorum_threshold != next.quorum_threshold;

        self.validators = next.validators;
        self.quorum_threshold = next.quorum_threshold;

        Ok(ConsensusMembershipChangeResult {
            applied,
            validators: self.validators(),
            quorum_threshold: self.quorum_threshold,
        })
    }

    fn restore_records(&mut self, records: Vec<HeadConsensusRecord>) -> Result<(), WorldError> {
        let mut restored: BTreeMap<(String, u64), HeadConsensusRecord> = BTreeMap::new();
        for mut record in records {
            if record.head.world_id.trim().is_empty() {
                return Err(WorldError::DistributedValidationFailed {
                    reason: "consensus record world_id cannot be empty".to_string(),
                });
            }
            if record.proposer_id.trim().is_empty() {
                return Err(WorldError::DistributedValidationFailed {
                    reason: format!(
                        "consensus record proposer cannot be empty for {}@{}",
                        record.head.world_id, record.head.height
                    ),
                });
            }

            if record.validator_count == 0 {
                let from_votes = record.votes.len();
                record.validator_count = self
                    .validators
                    .len()
                    .max(from_votes)
                    .max(record.quorum_threshold);
            }
            if record.validator_count == 0 {
                return Err(WorldError::DistributedValidationFailed {
                    reason: format!(
                        "invalid validator_count=0 for {}@{}",
                        record.head.world_id, record.head.height
                    ),
                });
            }

            if record.quorum_threshold == 0 {
                record.quorum_threshold = record.validator_count / 2 + 1;
            }
            if record.quorum_threshold > record.validator_count {
                return Err(WorldError::DistributedValidationFailed {
                    reason: format!(
                        "record quorum threshold mismatch for {}@{}: threshold={}, validator_count={}",
                        record.head.world_id,
                        record.head.height,
                        record.quorum_threshold,
                        record.validator_count
                    ),
                });
            }
            if record.quorum_threshold <= record.validator_count / 2 {
                return Err(WorldError::DistributedValidationFailed {
                    reason: format!(
                        "unsafe record quorum threshold {} for validator_count {} in {}@{}",
                        record.quorum_threshold,
                        record.validator_count,
                        record.head.world_id,
                        record.head.height
                    ),
                });
            }
            if record.votes.len() > record.validator_count {
                return Err(WorldError::DistributedValidationFailed {
                    reason: format!(
                        "record votes overflow for {}@{}: votes={}, validator_count={}",
                        record.head.world_id,
                        record.head.height,
                        record.votes.len(),
                        record.validator_count
                    ),
                });
            }

            for (validator_id, vote) in &record.votes {
                Self::validate_record_vote(
                    validator_id,
                    vote,
                    &record.head.world_id,
                    record.head.height,
                )?;
            }

            record.status = decide_status(
                record.validator_count,
                record.quorum_threshold,
                &record.votes,
            );
            let key = (record.head.world_id.clone(), record.head.height);
            if let Some(existing) = restored.get(&key) {
                return Err(WorldError::DistributedValidationFailed {
                    reason: format!(
                        "duplicate consensus record for {}@{}: existing={}, new={}",
                        record.head.world_id,
                        record.head.height,
                        existing.head.block_hash,
                        record.head.block_hash
                    ),
                });
            }
            restored.insert(key, record);
        }
        self.records = restored;
        let world_ids: BTreeSet<String> = self
            .records
            .keys()
            .map(|(world_id, _)| world_id.clone())
            .collect();
        for world_id in world_ids {
            self.prune_world_records(world_id.as_str());
        }
        Ok(())
    }

    fn prune_world_records(&mut self, world_id: &str) {
        let world_keys: Vec<(String, u64)> = self
            .records
            .keys()
            .filter(|(candidate_world_id, _)| candidate_world_id == world_id)
            .cloned()
            .collect();
        if world_keys.len() <= self.max_records_per_world {
            return;
        }
        let mut overflow = world_keys.len() - self.max_records_per_world;
        for key in world_keys {
            if overflow == 0 {
                break;
            }
            let should_remove = self
                .records
                .get(&key)
                .map(|record| !matches!(record.status, ConsensusStatus::Pending))
                .unwrap_or(false);
            if should_remove {
                self.records.remove(&key);
                overflow -= 1;
            }
        }
    }

    fn validate_record_vote(
        validator_id: &str,
        vote: &ConsensusVote,
        world_id: &str,
        height: u64,
    ) -> Result<(), WorldError> {
        if validator_id.trim().is_empty() {
            return Err(WorldError::DistributedValidationFailed {
                reason: format!("invalid vote validator key in {world_id}@{height}"),
            });
        }
        if vote.validator_id != validator_id {
            return Err(WorldError::DistributedValidationFailed {
                reason: format!(
                    "invalid vote payload for {world_id}@{height}: key={validator_id}, payload={}",
                    vote.validator_id
                ),
            });
        }
        Ok(())
    }

    fn apply_vote(
        record: &mut HeadConsensusRecord,
        validator_count: usize,
        quorum_threshold: usize,
        validator_id: &str,
        approve: bool,
        voted_at_ms: i64,
        reason: Option<String>,
    ) -> Result<ConsensusDecision, WorldError> {
        if let Some(existing) = record.votes.get(validator_id) {
            if existing.approve != approve {
                return Err(WorldError::DistributedValidationFailed {
                    reason: format!(
                        "conflicting vote from {validator_id} for {}@{}",
                        record.head.world_id, record.head.height
                    ),
                });
            }
            return Ok(decision_from_record(record));
        }

        if !matches!(record.status, ConsensusStatus::Pending) {
            return Err(WorldError::DistributedValidationFailed {
                reason: format!(
                    "proposal {}@{} already finalized as {:?}",
                    record.head.world_id, record.head.height, record.status
                ),
            });
        }

        record.votes.insert(
            validator_id.to_string(),
            ConsensusVote {
                validator_id: validator_id.to_string(),
                approve,
                reason,
                voted_at_ms,
            },
        );

        record.status = decide_status(validator_count, quorum_threshold, &record.votes);
        Ok(decision_from_record(record))
    }

    fn latest_committed_height(&self, world_id: &str) -> Option<u64> {
        self.records
            .iter()
            .filter(|((candidate_world_id, _), record)| {
                candidate_world_id == world_id
                    && matches!(record.status, ConsensusStatus::Committed)
            })
            .map(|((_, height), _)| *height)
            .max()
    }
}

pub fn propose_world_head_with_quorum(
    dht: &impl DistributedDht,
    consensus: &mut QuorumConsensus,
    head: &WorldHeadAnnounce,
    proposer_id: &str,
    proposed_at_ms: i64,
) -> Result<ConsensusDecision, WorldError> {
    let decision = consensus.propose_head(head, proposer_id, proposed_at_ms)?;
    if matches!(decision.status, ConsensusStatus::Committed) {
        dht.put_world_head(&head.world_id, head)?;
    }
    Ok(decision)
}

#[expect(
    clippy::too_many_arguments,
    reason = "Public network vote adapter keeps explicit DHT and vote fields for compatibility"
)]
pub fn vote_world_head_with_quorum(
    dht: &impl DistributedDht,
    consensus: &mut QuorumConsensus,
    world_id: &str,
    height: u64,
    block_hash: &str,
    validator_id: &str,
    approve: bool,
    voted_at_ms: i64,
    reason: Option<String>,
) -> Result<ConsensusDecision, WorldError> {
    let decision = consensus.vote_head(
        world_id,
        height,
        block_hash,
        validator_id,
        approve,
        voted_at_ms,
        reason,
    )?;
    if matches!(decision.status, ConsensusStatus::Committed) {
        let record = consensus.record(world_id, height).ok_or_else(|| {
            WorldError::DistributedValidationFailed {
                reason: format!("committed record missing for {world_id} at height {height}"),
            }
        })?;
        dht.put_world_head(world_id, &record.head)?;
    }
    Ok(decision)
}

pub fn ensure_lease_holder_validator(
    consensus: &mut QuorumConsensus,
    lease: Option<&LeaseState>,
    requested_at_ms: i64,
) -> Result<ConsensusMembershipChangeResult, WorldError> {
    let Some(lease) = lease else {
        return Ok(ConsensusMembershipChangeResult {
            applied: false,
            validators: consensus.validators(),
            quorum_threshold: consensus.quorum_threshold(),
        });
    };

    if consensus.validators.contains(&lease.holder_id) {
        return Ok(ConsensusMembershipChangeResult {
            applied: false,
            validators: consensus.validators(),
            quorum_threshold: consensus.quorum_threshold(),
        });
    }

    let request = ConsensusMembershipChangeRequest {
        requester_id: lease.holder_id.clone(),
        requested_at_ms,
        reason: Some("auto-add active lease holder as validator".to_string()),
        change: ConsensusMembershipChange::AddValidator {
            validator_id: lease.holder_id.clone(),
        },
    };
    consensus.apply_membership_change_with_lease(&request, Some(lease))
}

fn derive_membership_threshold(current_threshold: usize, validator_count: usize) -> usize {
    let majority = validator_count / 2 + 1;
    current_threshold.max(majority).min(validator_count)
}

fn decide_status(
    validator_count: usize,
    quorum_threshold: usize,
    votes: &BTreeMap<String, ConsensusVote>,
) -> ConsensusStatus {
    let approvals = votes.values().filter(|vote| vote.approve).count();
    if approvals >= quorum_threshold {
        return ConsensusStatus::Committed;
    }

    let rejections = votes.values().filter(|vote| !vote.approve).count();
    let max_rejections_without_losing_quorum = validator_count.saturating_sub(quorum_threshold);
    if rejections > max_rejections_without_losing_quorum {
        ConsensusStatus::Rejected
    } else {
        ConsensusStatus::Pending
    }
}

fn decision_from_record(record: &HeadConsensusRecord) -> ConsensusDecision {
    let approvals = record.votes.values().filter(|vote| vote.approve).count();
    let rejections = record.votes.values().filter(|vote| !vote.approve).count();
    ConsensusDecision {
        world_id: record.head.world_id.clone(),
        height: record.head.height,
        block_hash: record.head.block_hash.clone(),
        status: record.status,
        approvals,
        rejections,
        quorum_threshold: record.quorum_threshold,
    }
}

fn write_json_atomic<T: Serialize>(value: &T, path: &Path) -> Result<(), WorldError> {
    let tmp = path.with_extension("tmp");
    write_json_to_path(value, &tmp)?;
    fs::rename(tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests;
