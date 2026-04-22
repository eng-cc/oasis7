use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use oasis7_proto::distributed_pos::{
    decide_pos_status as shared_decide_pos_status,
    required_supermajority_stake as shared_required_supermajority_stake,
    slot_epoch as shared_slot_epoch, weighted_expected_proposer,
    PosDecisionStatus as SharedPosDecisionStatus,
};

use super::distributed::WorldHeadAnnounce;
use super::distributed_dht::DistributedDht;
use super::error::WorldError;
use super::node_pos_core::{
    decision_from_proposal as node_decision_from_proposal,
    insert_attestation as node_insert_attestation, NodePosAttestation, NodePosError,
    NodePosPendingProposal, NodePosStatusAdapter,
};
use super::util::{read_json_from_path, write_json_to_path};

pub const POS_CONSENSUS_SNAPSHOT_VERSION: u64 = 1;
const DEFAULT_MAX_RECORDS_PER_WORLD: usize = 4096;
const DEFAULT_MAX_ATTESTATION_HISTORY_PER_VALIDATOR: usize = 8192;

fn default_max_records_per_world() -> usize {
    DEFAULT_MAX_RECORDS_PER_WORLD
}

fn default_max_attestation_history_per_validator() -> usize {
    DEFAULT_MAX_ATTESTATION_HISTORY_PER_VALIDATOR
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PosValidator {
    pub validator_id: String,
    pub stake: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PosConsensusConfig {
    pub validators: Vec<PosValidator>,
    pub supermajority_numerator: u64,
    pub supermajority_denominator: u64,
    pub epoch_length_slots: u64,
    #[serde(default = "default_max_records_per_world")]
    pub max_records_per_world: usize,
    #[serde(default = "default_max_attestation_history_per_validator")]
    pub max_attestation_history_per_validator: usize,
}

impl PosConsensusConfig {
    pub fn ethereum_like(validators: Vec<PosValidator>) -> Self {
        Self {
            validators,
            supermajority_numerator: 2,
            supermajority_denominator: 3,
            epoch_length_slots: 32,
            max_records_per_world: default_max_records_per_world(),
            max_attestation_history_per_validator: default_max_attestation_history_per_validator(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PosConsensusStatus {
    Pending,
    Committed,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PosAttestation {
    pub validator_id: String,
    pub approve: bool,
    pub source_epoch: u64,
    pub target_epoch: u64,
    pub voted_at_ms: i64,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PosHeadRecord {
    pub head: WorldHeadAnnounce,
    pub proposer_id: String,
    pub slot: u64,
    pub epoch: u64,
    pub proposed_at_ms: i64,
    pub status: PosConsensusStatus,
    pub approved_stake: u64,
    pub rejected_stake: u64,
    pub required_stake: u64,
    pub attestations: BTreeMap<String, PosAttestation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PosConsensusDecision {
    pub world_id: String,
    pub height: u64,
    pub block_hash: String,
    pub slot: u64,
    pub epoch: u64,
    pub status: PosConsensusStatus,
    pub approved_stake: u64,
    pub rejected_stake: u64,
    pub total_stake: u64,
    pub required_stake: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct PosConsensusSnapshotFile {
    version: u64,
    validators: Vec<PosValidator>,
    supermajority_numerator: u64,
    supermajority_denominator: u64,
    epoch_length_slots: u64,
    #[serde(default = "default_max_records_per_world")]
    max_records_per_world: usize,
    #[serde(default = "default_max_attestation_history_per_validator")]
    max_attestation_history_per_validator: usize,
    records: Vec<PosHeadRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct EpochAttestationRef {
    world_id: String,
    height: u64,
    block_hash: String,
    source_epoch: u64,
    target_epoch: u64,
}

#[derive(Debug, Clone)]
pub struct PosConsensus {
    validators: BTreeMap<String, u64>,
    total_stake: u64,
    required_stake: u64,
    supermajority_numerator: u64,
    supermajority_denominator: u64,
    epoch_length_slots: u64,
    max_records_per_world: usize,
    max_attestation_history_per_validator: usize,
    records: BTreeMap<(String, u64), PosHeadRecord>,
    attestation_history: BTreeMap<String, Vec<EpochAttestationRef>>,
}

type PosProgressProposal = NodePosPendingProposal<(), PosConsensusStatus>;

impl NodePosStatusAdapter for PosConsensusStatus {
    fn pending() -> Self {
        PosConsensusStatus::Pending
    }

    fn committed() -> Self {
        PosConsensusStatus::Committed
    }

    fn rejected() -> Self {
        PosConsensusStatus::Rejected
    }
}

impl PosConsensus {
    pub fn new(config: PosConsensusConfig) -> Result<Self, WorldError> {
        if config.validators.is_empty() {
            return Err(WorldError::DistributedValidationFailed {
                reason: "pos validators cannot be empty".to_string(),
            });
        }
        if config.epoch_length_slots == 0 {
            return Err(WorldError::DistributedValidationFailed {
                reason: "epoch_length_slots must be positive".to_string(),
            });
        }
        if config.supermajority_denominator == 0
            || config.supermajority_numerator == 0
            || config.supermajority_numerator > config.supermajority_denominator
        {
            return Err(WorldError::DistributedValidationFailed {
                reason: format!(
                    "invalid supermajority ratio {}/{}",
                    config.supermajority_numerator, config.supermajority_denominator
                ),
            });
        }
        if config.supermajority_numerator <= config.supermajority_denominator / 2 {
            return Err(WorldError::DistributedValidationFailed {
                reason: format!(
                    "unsafe supermajority ratio {}/{}; requires > 1/2",
                    config.supermajority_numerator, config.supermajority_denominator
                ),
            });
        }
        if config.max_records_per_world == 0 {
            return Err(WorldError::DistributedValidationFailed {
                reason: "max_records_per_world must be positive".to_string(),
            });
        }
        if config.max_attestation_history_per_validator == 0 {
            return Err(WorldError::DistributedValidationFailed {
                reason: "max_attestation_history_per_validator must be positive".to_string(),
            });
        }

        let mut validators = BTreeMap::new();
        let mut total_stake = 0u64;
        for validator in config.validators {
            let validator_id = validator.validator_id.trim();
            if validator_id.is_empty() {
                return Err(WorldError::DistributedValidationFailed {
                    reason: "validator_id cannot be empty".to_string(),
                });
            }
            if validator.stake == 0 {
                return Err(WorldError::DistributedValidationFailed {
                    reason: format!("validator {} stake must be positive", validator_id),
                });
            }
            if validators
                .insert(validator_id.to_string(), validator.stake)
                .is_some()
            {
                return Err(WorldError::DistributedValidationFailed {
                    reason: format!("duplicate validator: {}", validator_id),
                });
            }
            total_stake = total_stake.checked_add(validator.stake).ok_or_else(|| {
                WorldError::DistributedValidationFailed {
                    reason: "total stake overflow".to_string(),
                }
            })?;
        }

        if total_stake == 0 {
            return Err(WorldError::DistributedValidationFailed {
                reason: "total stake cannot be zero".to_string(),
            });
        }
        let required_stake = required_supermajority_stake(
            total_stake,
            config.supermajority_numerator,
            config.supermajority_denominator,
        )?;

        Ok(Self {
            validators,
            total_stake,
            required_stake,
            supermajority_numerator: config.supermajority_numerator,
            supermajority_denominator: config.supermajority_denominator,
            epoch_length_slots: config.epoch_length_slots,
            max_records_per_world: config.max_records_per_world,
            max_attestation_history_per_validator: config.max_attestation_history_per_validator,
            records: BTreeMap::new(),
            attestation_history: BTreeMap::new(),
        })
    }

    pub fn validators(&self) -> Vec<PosValidator> {
        self.validators
            .iter()
            .map(|(validator_id, stake)| PosValidator {
                validator_id: validator_id.clone(),
                stake: *stake,
            })
            .collect()
    }

    pub fn total_stake(&self) -> u64 {
        self.total_stake
    }

    pub fn required_stake(&self) -> u64 {
        self.required_stake
    }

    pub fn epoch_length_slots(&self) -> u64 {
        self.epoch_length_slots
    }

    pub fn max_records_per_world(&self) -> usize {
        self.max_records_per_world
    }

    pub fn max_attestation_history_per_validator(&self) -> usize {
        self.max_attestation_history_per_validator
    }

    pub fn record(&self, world_id: &str, height: u64) -> Option<&PosHeadRecord> {
        self.records.get(&(world_id.to_string(), height))
    }

    pub fn expected_proposer(&self, slot: u64) -> Option<String> {
        weighted_expected_proposer(&self.validators, self.total_stake, slot)
    }

    pub fn slot_epoch(&self, slot: u64) -> u64 {
        shared_slot_epoch(self.epoch_length_slots, slot)
    }

    pub fn save_snapshot_to_path(&self, path: impl AsRef<Path>) -> Result<(), WorldError> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        let snapshot = PosConsensusSnapshotFile {
            version: POS_CONSENSUS_SNAPSHOT_VERSION,
            validators: self.validators(),
            supermajority_numerator: self.supermajority_numerator,
            supermajority_denominator: self.supermajority_denominator,
            epoch_length_slots: self.epoch_length_slots,
            max_records_per_world: self.max_records_per_world,
            max_attestation_history_per_validator: self.max_attestation_history_per_validator,
            records: self.records.values().cloned().collect(),
        };
        write_json_atomic(&snapshot, path)
    }

    pub fn load_snapshot_from_path(path: impl AsRef<Path>) -> Result<Self, WorldError> {
        let snapshot: PosConsensusSnapshotFile = read_json_from_path(path.as_ref())?;
        if snapshot.version != POS_CONSENSUS_SNAPSHOT_VERSION {
            return Err(WorldError::DistributedValidationFailed {
                reason: format!(
                    "unsupported pos consensus snapshot version {} (expected {})",
                    snapshot.version, POS_CONSENSUS_SNAPSHOT_VERSION
                ),
            });
        }
        let mut consensus = Self::new(PosConsensusConfig {
            validators: snapshot.validators,
            supermajority_numerator: snapshot.supermajority_numerator,
            supermajority_denominator: snapshot.supermajority_denominator,
            epoch_length_slots: snapshot.epoch_length_slots,
            max_records_per_world: snapshot.max_records_per_world,
            max_attestation_history_per_validator: snapshot.max_attestation_history_per_validator,
        })?;
        consensus.restore_records(snapshot.records)?;
        Ok(consensus)
    }

    pub fn propose_head(
        &mut self,
        head: &WorldHeadAnnounce,
        proposer_id: &str,
        slot: u64,
        proposed_at_ms: i64,
    ) -> Result<PosConsensusDecision, WorldError> {
        self.ensure_validator(proposer_id)?;
        let expected = self.expected_proposer(slot).ok_or_else(|| {
            WorldError::DistributedValidationFailed {
                reason: "no proposer available".to_string(),
            }
        })?;
        if proposer_id != expected {
            return Err(WorldError::DistributedValidationFailed {
                reason: format!(
                    "unexpected proposer for slot {}: expected={}, got={}",
                    slot, expected, proposer_id
                ),
            });
        }

        if let Some(committed_height) = self.latest_committed_height(&head.world_id) {
            if head.height <= committed_height {
                return Err(WorldError::DistributedValidationFailed {
                    reason: format!(
                        "stale pos proposal for {} at height {} (committed={})",
                        head.world_id, head.height, committed_height
                    ),
                });
            }
        }

        let epoch = self.slot_epoch(slot);
        let key = (head.world_id.clone(), head.height);
        let required_stake = self.required_stake;
        let existing = self.records.entry(key).or_insert_with(|| PosHeadRecord {
            head: head.clone(),
            proposer_id: proposer_id.to_string(),
            slot,
            epoch,
            proposed_at_ms,
            status: PosConsensusStatus::Pending,
            approved_stake: 0,
            rejected_stake: 0,
            required_stake,
            attestations: BTreeMap::new(),
        });
        if existing.head.block_hash != head.block_hash {
            return Err(WorldError::DistributedValidationFailed {
                reason: format!(
                    "conflicting pos proposal for {}@{}: existing={}, new={}",
                    head.world_id, head.height, existing.head.block_hash, head.block_hash
                ),
            });
        }
        if existing.slot != slot {
            return Err(WorldError::DistributedValidationFailed {
                reason: format!(
                    "proposal slot mismatch for {}@{}: existing={}, new={}",
                    head.world_id, head.height, existing.slot, slot
                ),
            });
        }

        let decision = self.attest_head(
            &head.world_id,
            head.height,
            &head.block_hash,
            proposer_id,
            true,
            proposed_at_ms,
            epoch.saturating_sub(1),
            epoch,
            Some("proposal accepted".to_string()),
        )?;
        self.prune_world_records(head.world_id.as_str());
        Ok(decision)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn attest_head(
        &mut self,
        world_id: &str,
        height: u64,
        block_hash: &str,
        validator_id: &str,
        approve: bool,
        voted_at_ms: i64,
        source_epoch: u64,
        target_epoch: u64,
        reason: Option<String>,
    ) -> Result<PosConsensusDecision, WorldError> {
        self.ensure_validator(validator_id)?;
        if source_epoch > target_epoch {
            return Err(WorldError::DistributedValidationFailed {
                reason: format!(
                    "invalid attestation epochs for {}: source_epoch={} > target_epoch={}",
                    validator_id, source_epoch, target_epoch
                ),
            });
        }

        let key = (world_id.to_string(), height);
        {
            let record =
                self.records
                    .get(&key)
                    .ok_or_else(|| WorldError::DistributedValidationFailed {
                        reason: format!(
                            "pos proposal not found for {} at height {}",
                            world_id, height
                        ),
                    })?;
            if record.head.block_hash != block_hash {
                return Err(WorldError::DistributedValidationFailed {
                    reason: format!(
                        "pos attestation hash mismatch for {}@{}: expected={}, got={}",
                        world_id, height, record.head.block_hash, block_hash
                    ),
                });
            }
            if record.epoch != target_epoch {
                return Err(WorldError::DistributedValidationFailed {
                    reason: format!(
                        "target_epoch mismatch for {}@{}: expected={}, got={}",
                        world_id, height, record.epoch, target_epoch
                    ),
                });
            }
        }

        self.ensure_slash_free(
            validator_id,
            world_id,
            height,
            block_hash,
            source_epoch,
            target_epoch,
        )?;

        let total_stake = self.total_stake;
        let required_stake = self.required_stake;
        let decision = {
            let record = self.records.get_mut(&key).expect("record exists");
            if let Some(existing) = record.attestations.get(validator_id) {
                if existing.approve == approve
                    && existing.source_epoch == source_epoch
                    && existing.target_epoch == target_epoch
                {
                    return Ok(decision_from_record(record, total_stake));
                }
                return Err(WorldError::DistributedValidationFailed {
                    reason: format!(
                        "conflicting attestation from {} for {}@{}",
                        validator_id, world_id, height
                    ),
                });
            }
            let mut proposal = progress_proposal_from_record(record);
            insert_pos_attestation(
                &self.validators,
                total_stake,
                required_stake,
                &mut proposal,
                validator_id,
                approve,
                voted_at_ms,
                source_epoch,
                target_epoch,
                reason,
            )?;
            apply_progress_proposal_to_record(record, proposal);
            record.required_stake = required_stake;
            decision_from_record(record, total_stake)
        };

        self.record_attestation_history(
            validator_id,
            world_id,
            height,
            block_hash,
            source_epoch,
            target_epoch,
        );
        self.prune_world_records(world_id);
        Ok(decision)
    }

    fn restore_records(&mut self, records: Vec<PosHeadRecord>) -> Result<(), WorldError> {
        let mut restored = BTreeMap::new();
        for mut record in records {
            if record.head.world_id.trim().is_empty() {
                return Err(WorldError::DistributedValidationFailed {
                    reason: "pos record world_id cannot be empty".to_string(),
                });
            }
            if record.proposer_id.trim().is_empty() {
                return Err(WorldError::DistributedValidationFailed {
                    reason: format!(
                        "pos record proposer cannot be empty for {}@{}",
                        record.head.world_id, record.head.height
                    ),
                });
            }
            if !self.validators.contains_key(&record.proposer_id) {
                return Err(WorldError::DistributedValidationFailed {
                    reason: format!(
                        "unknown pos proposer {} for {}@{}",
                        record.proposer_id, record.head.world_id, record.head.height
                    ),
                });
            }

            let expected_epoch = self.slot_epoch(record.slot);
            if record.epoch != expected_epoch {
                return Err(WorldError::DistributedValidationFailed {
                    reason: format!(
                        "invalid pos epoch for {}@{}: expected={}, actual={}",
                        record.head.world_id, record.head.height, expected_epoch, record.epoch
                    ),
                });
            }

            for (validator_id, attestation) in &record.attestations {
                if attestation.validator_id != *validator_id {
                    return Err(WorldError::DistributedValidationFailed {
                        reason: format!(
                            "invalid attestation key/payload mismatch for {}@{}",
                            record.head.world_id, record.head.height
                        ),
                    });
                }
                if !self.validators.contains_key(validator_id) {
                    return Err(WorldError::DistributedValidationFailed {
                        reason: format!(
                            "unknown attestation validator {} for {}@{}",
                            validator_id, record.head.world_id, record.head.height
                        ),
                    });
                }
                if attestation.target_epoch != record.epoch {
                    return Err(WorldError::DistributedValidationFailed {
                        reason: format!(
                            "invalid target_epoch for {}@{} from {}",
                            record.head.world_id, record.head.height, validator_id
                        ),
                    });
                }
                if attestation.source_epoch > attestation.target_epoch {
                    return Err(WorldError::DistributedValidationFailed {
                        reason: format!(
                            "invalid attestation epochs for {} on {}@{}",
                            validator_id, record.head.world_id, record.head.height
                        ),
                    });
                }
            }

            let mut proposal = empty_progress_proposal(&record);
            for attestation in record.attestations.values() {
                insert_pos_attestation(
                    &self.validators,
                    self.total_stake,
                    self.required_stake,
                    &mut proposal,
                    attestation.validator_id.as_str(),
                    attestation.approve,
                    attestation.voted_at_ms,
                    attestation.source_epoch,
                    attestation.target_epoch,
                    attestation.reason.clone(),
                )?;
            }

            apply_progress_proposal_to_record(&mut record, proposal);
            record.required_stake = self.required_stake;

            let key = (record.head.world_id.clone(), record.head.height);
            if restored.insert(key, record).is_some() {
                return Err(WorldError::DistributedValidationFailed {
                    reason: "duplicate pos record in snapshot".to_string(),
                });
            }
        }

        self.records = restored;
        self.attestation_history.clear();
        let mut historical_votes = Vec::new();
        for record in self.records.values() {
            for attestation in record.attestations.values() {
                historical_votes.push((
                    attestation.validator_id.clone(),
                    record.head.world_id.clone(),
                    record.head.height,
                    record.head.block_hash.clone(),
                    attestation.source_epoch,
                    attestation.target_epoch,
                ));
            }
        }
        for (validator_id, world_id, height, block_hash, source_epoch, target_epoch) in
            historical_votes
        {
            self.record_attestation_history(
                &validator_id,
                &world_id,
                height,
                &block_hash,
                source_epoch,
                target_epoch,
            );
        }
        let world_ids: Vec<String> = self
            .records
            .keys()
            .map(|(world_id, _)| world_id.clone())
            .collect();
        for world_id in world_ids {
            self.prune_world_records(world_id.as_str());
        }
        Ok(())
    }

    fn ensure_validator(&self, validator_id: &str) -> Result<u64, WorldError> {
        self.validators.get(validator_id).copied().ok_or_else(|| {
            WorldError::DistributedValidationFailed {
                reason: format!("validator not allowed: {}", validator_id),
            }
        })
    }

    fn ensure_slash_free(
        &self,
        validator_id: &str,
        world_id: &str,
        height: u64,
        block_hash: &str,
        source_epoch: u64,
        target_epoch: u64,
    ) -> Result<(), WorldError> {
        let Some(history) = self.attestation_history.get(validator_id) else {
            return Ok(());
        };
        for previous in history {
            if previous.world_id != world_id {
                continue;
            }
            if previous.height == height && previous.block_hash == block_hash {
                continue;
            }
            if previous.target_epoch == target_epoch
                && (previous.block_hash != block_hash || previous.source_epoch != source_epoch)
            {
                return Err(WorldError::DistributedValidationFailed {
                    reason: format!(
                        "slashable double vote detected for {} at target_epoch {}",
                        validator_id, target_epoch
                    ),
                });
            }
            let previous_surrounds_new =
                previous.source_epoch < source_epoch && previous.target_epoch > target_epoch;
            let new_surrounds_previous =
                previous.source_epoch > source_epoch && previous.target_epoch < target_epoch;
            if previous_surrounds_new || new_surrounds_previous {
                return Err(WorldError::DistributedValidationFailed {
                    reason: format!(
                        "slashable surround vote detected for {} between ({},{}) and ({},{})",
                        validator_id,
                        previous.source_epoch,
                        previous.target_epoch,
                        source_epoch,
                        target_epoch
                    ),
                });
            }
        }
        Ok(())
    }

    fn record_attestation_history(
        &mut self,
        validator_id: &str,
        world_id: &str,
        height: u64,
        block_hash: &str,
        source_epoch: u64,
        target_epoch: u64,
    ) {
        let history = self
            .attestation_history
            .entry(validator_id.to_string())
            .or_default();
        let item = EpochAttestationRef {
            world_id: world_id.to_string(),
            height,
            block_hash: block_hash.to_string(),
            source_epoch,
            target_epoch,
        };
        if history.iter().any(|existing| existing == &item) {
            return;
        }
        history.push(item);
        let overflow = history
            .len()
            .saturating_sub(self.max_attestation_history_per_validator);
        if overflow > 0 {
            history.drain(0..overflow);
        }
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
                .map(|record| !matches!(record.status, PosConsensusStatus::Pending))
                .unwrap_or(false);
            if should_remove {
                self.records.remove(&key);
                overflow -= 1;
            }
        }
    }

    fn latest_committed_height(&self, world_id: &str) -> Option<u64> {
        self.records
            .iter()
            .filter(|((candidate_world_id, _), record)| {
                candidate_world_id == world_id
                    && matches!(record.status, PosConsensusStatus::Committed)
            })
            .map(|((_, height), _)| *height)
            .max()
    }
}

pub fn propose_world_head_with_pos(
    dht: &impl DistributedDht,
    consensus: &mut PosConsensus,
    head: &WorldHeadAnnounce,
    proposer_id: &str,
    slot: u64,
    proposed_at_ms: i64,
) -> Result<PosConsensusDecision, WorldError> {
    let decision = consensus.propose_head(head, proposer_id, slot, proposed_at_ms)?;
    if matches!(decision.status, PosConsensusStatus::Committed) {
        dht.put_world_head(&head.world_id, head)?;
    }
    Ok(decision)
}

#[allow(clippy::too_many_arguments)]
pub fn attest_world_head_with_pos(
    dht: &impl DistributedDht,
    consensus: &mut PosConsensus,
    world_id: &str,
    height: u64,
    block_hash: &str,
    validator_id: &str,
    approve: bool,
    voted_at_ms: i64,
    source_epoch: u64,
    target_epoch: u64,
    reason: Option<String>,
) -> Result<PosConsensusDecision, WorldError> {
    let decision = consensus.attest_head(
        world_id,
        height,
        block_hash,
        validator_id,
        approve,
        voted_at_ms,
        source_epoch,
        target_epoch,
        reason,
    )?;
    if matches!(decision.status, PosConsensusStatus::Committed) {
        let record = consensus.record(world_id, height).ok_or_else(|| {
            WorldError::DistributedValidationFailed {
                reason: format!(
                    "committed pos record missing for {} at height {}",
                    world_id, height
                ),
            }
        })?;
        dht.put_world_head(world_id, &record.head)?;
    }
    Ok(decision)
}

fn required_supermajority_stake(
    total_stake: u64,
    numerator: u64,
    denominator: u64,
) -> Result<u64, WorldError> {
    shared_required_supermajority_stake(total_stake, numerator, denominator).map_err(|reason| {
        WorldError::DistributedValidationFailed {
            reason: format!("invalid pos supermajority: {}", reason),
        }
    })
}

pub fn decide_pos_status(
    total_stake: u64,
    required_stake: u64,
    approved_stake: u64,
    rejected_stake: u64,
) -> PosConsensusStatus {
    match shared_decide_pos_status(total_stake, required_stake, approved_stake, rejected_stake) {
        SharedPosDecisionStatus::Pending => PosConsensusStatus::Pending,
        SharedPosDecisionStatus::Committed => PosConsensusStatus::Committed,
        SharedPosDecisionStatus::Rejected => PosConsensusStatus::Rejected,
    }
}

fn decision_from_record(record: &PosHeadRecord, total_stake: u64) -> PosConsensusDecision {
    let proposal = progress_proposal_from_record(record);
    let decision = node_decision_from_proposal(&proposal, record.required_stake, total_stake);
    PosConsensusDecision {
        world_id: record.head.world_id.clone(),
        height: decision.height,
        block_hash: decision.block_hash,
        slot: decision.slot,
        epoch: decision.epoch,
        status: decision.status,
        approved_stake: decision.approved_stake,
        rejected_stake: decision.rejected_stake,
        total_stake: decision.total_stake,
        required_stake: decision.required_stake,
    }
}

fn node_pos_error(err: NodePosError) -> WorldError {
    WorldError::DistributedValidationFailed { reason: err.reason }
}

fn progress_proposal_from_record(record: &PosHeadRecord) -> PosProgressProposal {
    let mut attestations = BTreeMap::new();
    for (validator_id, attestation) in &record.attestations {
        attestations.insert(
            validator_id.clone(),
            NodePosAttestation {
                validator_id: attestation.validator_id.clone(),
                approve: attestation.approve,
                source_epoch: attestation.source_epoch,
                target_epoch: attestation.target_epoch,
                voted_at_ms: attestation.voted_at_ms,
                reason: attestation.reason.clone(),
            },
        );
    }
    PosProgressProposal {
        height: record.head.height,
        slot: record.slot,
        epoch: record.epoch,
        opened_at_ms: record.proposed_at_ms,
        proposer_id: record.proposer_id.clone(),
        block_hash: record.head.block_hash.clone(),
        action_root: record.head.state_root.clone(),
        committed_actions: Vec::new(),
        attestations,
        approved_stake: record.approved_stake,
        rejected_stake: record.rejected_stake,
        status: record.status,
    }
}

fn empty_progress_proposal(record: &PosHeadRecord) -> PosProgressProposal {
    PosProgressProposal {
        approved_stake: 0,
        rejected_stake: 0,
        status: PosConsensusStatus::Pending,
        attestations: BTreeMap::new(),
        ..progress_proposal_from_record(record)
    }
}

fn apply_progress_proposal_to_record(record: &mut PosHeadRecord, proposal: PosProgressProposal) {
    record.attestations = proposal
        .attestations
        .into_values()
        .map(|attestation| {
            (
                attestation.validator_id.clone(),
                PosAttestation {
                    validator_id: attestation.validator_id,
                    approve: attestation.approve,
                    source_epoch: attestation.source_epoch,
                    target_epoch: attestation.target_epoch,
                    voted_at_ms: attestation.voted_at_ms,
                    reason: attestation.reason,
                },
            )
        })
        .collect();
    record.approved_stake = proposal.approved_stake;
    record.rejected_stake = proposal.rejected_stake;
    record.status = proposal.status;
}

#[allow(clippy::too_many_arguments)]
fn insert_pos_attestation(
    validators: &BTreeMap<String, u64>,
    total_stake: u64,
    required_stake: u64,
    proposal: &mut PosProgressProposal,
    validator_id: &str,
    approve: bool,
    voted_at_ms: i64,
    source_epoch: u64,
    target_epoch: u64,
    reason: Option<String>,
) -> Result<(), WorldError> {
    node_insert_attestation(
        validators,
        total_stake,
        required_stake,
        proposal,
        validator_id,
        approve,
        voted_at_ms,
        source_epoch,
        target_epoch,
        reason,
    )
    .map_err(node_pos_error)
}

fn write_json_atomic<T: Serialize>(value: &T, path: &Path) -> Result<(), WorldError> {
    let tmp = path.with_extension("tmp");
    write_json_to_path(value, &tmp)?;
    fs::rename(tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests;
