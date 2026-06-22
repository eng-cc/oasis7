use super::super::util::{hash_json, sha256_hex};
use super::super::{
    CausedBy, RuntimeCommittedTickContext, TICK_BLOCK_HEADER_SCHEMA_V1,
    TICK_BLOCK_HEADER_SCHEMA_V2, TickBlock, TickBlockHeader, TickCertificate,
    TickConsensusDriftReport, TickConsensusRecord, TickConsensusRejectionAuditEvent,
    TickConsensusSubmissionRole, TickExecutionDigest, WorldError, WorldEvent, WorldEventBody,
    WorldEventId, WorldTime,
};
use super::World;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

const TICK_CONSENSUS_EPOCH_LEN: u64 = 360;
const TICK_CONSENSUS_GENESIS_PARENT_HASH: &str = "genesis";
const TICK_CERT_SIGNATURE_PREFIX: &str = "sha256:";
const MAX_TICK_CONSENSUS_REJECTION_AUDIT_EVENTS: usize = 2_048;

#[derive(Serialize)]
struct TickEventHashInput<'a> {
    id: WorldEventId,
    caused_by: &'a Option<CausedBy>,
    body: &'a WorldEventBody,
}

#[derive(Serialize)]
struct StateRootProjection<'a> {
    state: &'a super::super::WorldState,
    manifest_hash: &'a str,
    policy_hash: &'a str,
}

impl World {
    pub fn latest_tick_consensus_record(&self) -> Option<&TickConsensusRecord> {
        self.tick_consensus_records.last()
    }

    pub fn first_tick_consensus_drift(&self) -> Option<TickConsensusDriftReport> {
        let mut expected_parent = TICK_CONSENSUS_GENESIS_PARENT_HASH.to_string();
        let mut expected_height = 1_u64;
        for record in &self.tick_consensus_records {
            if let Err(err) = self.validate_tick_consensus_record_metadata(record) {
                return Some(TickConsensusDriftReport {
                    tick: record.block.header.tick,
                    reason: format!("{err:?}"),
                });
            }
            if record.block.header.parent_hash != expected_parent {
                return Some(TickConsensusDriftReport {
                    tick: record.block.header.tick,
                    reason: format!(
                        "tick consensus parent hash mismatch tick={} expected={} found={}",
                        record.block.header.tick, expected_parent, record.block.header.parent_hash
                    ),
                });
            }
            if record.certificate.consensus_height != expected_height {
                return Some(TickConsensusDriftReport {
                    tick: record.block.header.tick,
                    reason: format!(
                        "tick consensus height mismatch tick={} expected={} found={}",
                        record.block.header.tick,
                        expected_height,
                        record.certificate.consensus_height
                    ),
                });
            }
            let block_hash = record.block.block_hash();
            if record.certificate.block_hash != block_hash {
                return Some(TickConsensusDriftReport {
                    tick: record.block.header.tick,
                    reason: format!(
                        "tick consensus block hash mismatch tick={} expected={} found={}",
                        record.block.header.tick, block_hash, record.certificate.block_hash
                    ),
                });
            }
            let events_hash = match self.hash_tick_events_from_ids(
                record.block.header.tick,
                &record.block.ordered_event_ids,
            ) {
                Ok(value) => value,
                Err(err) => {
                    return Some(TickConsensusDriftReport {
                        tick: record.block.header.tick,
                        reason: format!("{err:?}"),
                    });
                }
            };
            if events_hash != record.block.header.events_hash {
                return Some(TickConsensusDriftReport {
                    tick: record.block.header.tick,
                    reason: format!(
                        "tick events hash mismatch tick={} expected={} found={}",
                        record.block.header.tick, events_hash, record.block.header.events_hash
                    ),
                });
            }
            expected_parent = record.certificate.block_hash.clone();
            expected_height = expected_height.saturating_add(1);
        }
        if let Some(record) = self.tick_consensus_records.last() {
            let current_state_root = match self.current_state_root_hash() {
                Ok(value) => value,
                Err(err) => {
                    return Some(TickConsensusDriftReport {
                        tick: record.block.header.tick,
                        reason: format!("{err:?}"),
                    });
                }
            };
            if record.block.header.state_root != current_state_root {
                return Some(TickConsensusDriftReport {
                    tick: record.block.header.tick,
                    reason: format!(
                        "latest state root mismatch tick={} expected={} found={}",
                        record.block.header.tick,
                        current_state_root,
                        record.block.header.state_root
                    ),
                });
            }
        }
        None
    }

    pub fn set_tick_consensus_authority_source(
        &mut self,
        source_node_id: &str,
    ) -> Result<(), WorldError> {
        let validated = self.validate_tick_consensus_source_node(source_node_id)?;
        self.tick_consensus_authority_source = validated;
        Ok(())
    }

    pub fn clear_tick_consensus_rejection_audit_events(&mut self) {
        self.tick_consensus_rejection_audit_events.clear();
    }

    pub fn record_tick_consensus_propagation_for_tick(
        &mut self,
        tick: WorldTime,
        source_node_id: &str,
    ) -> Result<(), WorldError> {
        self.record_tick_consensus_submission_for_tick(
            tick,
            source_node_id,
            TickConsensusSubmissionRole::Propagation,
        )
    }

    pub fn record_tick_consensus_authority_for_tick(
        &mut self,
        tick: WorldTime,
        source_node_id: &str,
    ) -> Result<(), WorldError> {
        self.record_tick_consensus_submission_for_tick(
            tick,
            source_node_id,
            TickConsensusSubmissionRole::Authority,
        )
    }

    pub fn verify_tick_consensus_chain(&self) -> Result<(), WorldError> {
        if let Some(report) = self.first_tick_consensus_drift() {
            return Err(WorldError::DistributedValidationFailed {
                reason: report.reason,
            });
        }
        Ok(())
    }

    pub(super) fn record_tick_consensus(&mut self) -> Result<(), WorldError> {
        self.record_tick_consensus_for_tick(self.state.time)
    }

    pub(super) fn record_tick_consensus_for_tick(
        &mut self,
        tick: WorldTime,
    ) -> Result<(), WorldError> {
        let authority_source = self.tick_consensus_authority_source.clone();
        self.record_tick_consensus_submission_for_tick(
            tick,
            authority_source.as_str(),
            TickConsensusSubmissionRole::Authority,
        )
    }

    pub(super) fn record_tick_consensus_for_committed_context(
        &mut self,
        context: &RuntimeCommittedTickContext,
    ) -> Result<(), WorldError> {
        let authority_source = self.tick_consensus_authority_source.clone();
        self.record_tick_consensus_submission_for_tick_with_context(
            context.height,
            authority_source.as_str(),
            TickConsensusSubmissionRole::Authority,
            Some(context),
        )
    }

    fn record_tick_consensus_submission_for_tick(
        &mut self,
        tick: WorldTime,
        source_node_id: &str,
        submission_role: TickConsensusSubmissionRole,
    ) -> Result<(), WorldError> {
        self.record_tick_consensus_submission_for_tick_with_context(
            tick,
            source_node_id,
            submission_role,
            None,
        )
    }

    fn record_tick_consensus_submission_for_tick_with_context(
        &mut self,
        tick: WorldTime,
        source_node_id: &str,
        submission_role: TickConsensusSubmissionRole,
        committed_context: Option<&RuntimeCommittedTickContext>,
    ) -> Result<(), WorldError> {
        let source_node_id = self.validate_tick_consensus_source_node(source_node_id)?;
        let record = self.build_tick_consensus_record_for_submission(
            tick,
            source_node_id.as_str(),
            submission_role,
            committed_context,
        )?;
        self.commit_tick_consensus_record_submission(record)?;
        self.verify_latest_tick_consensus_record()
    }

    fn build_tick_consensus_record_for_submission(
        &self,
        tick: WorldTime,
        source_node_id: &str,
        submission_role: TickConsensusSubmissionRole,
        committed_context: Option<&RuntimeCommittedTickContext>,
    ) -> Result<TickConsensusRecord, WorldError> {
        let tick_events: Vec<WorldEvent> = self
            .journal
            .events
            .iter()
            .filter(|event| event.time == tick)
            .cloned()
            .collect();
        let ordered_event_ids: Vec<WorldEventId> =
            tick_events.iter().map(|event| event.id).collect();
        let ordered_action_ids = Self::extract_ordered_action_ids(&tick_events);
        let events_hash = self.hash_tick_events(&tick_events)?;
        let state_root = self.current_state_root_hash()?;
        let parent_hash = self.parent_hash_for_tick(tick);
        let executor_version = env!("CARGO_PKG_VERSION").to_string();
        let randomness_seed = Self::derive_tick_randomness_seed(parent_hash.as_str(), tick);
        let consensus_height = self.consensus_height_for_tick(tick);
        let header = TickBlockHeader {
            schema_version: committed_context
                .map(|_| TICK_BLOCK_HEADER_SCHEMA_V2)
                .unwrap_or(TICK_BLOCK_HEADER_SCHEMA_V1),
            epoch: tick / TICK_CONSENSUS_EPOCH_LEN,
            tick,
            parent_hash,
            events_hash: events_hash.clone(),
            state_root: state_root.clone(),
            executor_version,
            randomness_seed,
            chain_height: committed_context.map(|context| context.height),
            chain_slot: committed_context.map(|context| context.slot),
            chain_epoch: committed_context.map(|context| context.epoch),
            node_block_hash: committed_context.map(|context| context.node_block_hash.clone()),
            action_root: committed_context.map(|context| context.action_root.clone()),
            committed_at_unix_ms: committed_context.map(|context| context.committed_at_unix_ms),
        };
        let execution_digest = TickExecutionDigest {
            action_batch_hash: committed_context
                .map(|context| Ok(context.action_root.clone()))
                .unwrap_or_else(|| hash_json(&ordered_action_ids))?,
            domain_events_hash: Self::hash_tick_domain_events(&tick_events)?,
            state_projection_hash: state_root,
        };
        let block = TickBlock {
            header,
            ordered_action_ids,
            ordered_event_ids: ordered_event_ids.clone(),
            event_count: ordered_event_ids.len() as u32,
            execution_digest,
        };
        let block_hash = block.block_hash();
        let mut signatures = BTreeMap::new();
        signatures.insert(
            source_node_id.to_string(),
            format!(
                "{}{}",
                TICK_CERT_SIGNATURE_PREFIX,
                sha256_hex(format!("tickcert:v1|{}|{}", source_node_id, block_hash).as_bytes())
            ),
        );
        Ok(TickConsensusRecord {
            block,
            certificate: TickCertificate {
                block_hash,
                consensus_height,
                threshold: 1,
                authority_source: source_node_id.to_string(),
                submission_role,
                signatures,
            },
        })
    }

    fn commit_tick_consensus_record_submission(
        &mut self,
        candidate: TickConsensusRecord,
    ) -> Result<(), WorldError> {
        let tick = candidate.block.header.tick;
        let existing_index = self
            .tick_consensus_records
            .iter()
            .position(|record| record.block.header.tick == tick);
        if let Some(index) = existing_index {
            let existing = self.tick_consensus_records[index].clone();
            self.validate_tick_consensus_candidate_against_policy(&candidate, Some(&existing))?;
            self.validate_tick_consensus_candidate_against_existing(&candidate, &existing)?;
            self.tick_consensus_records[index] = candidate;
            return Ok(());
        }
        self.validate_tick_consensus_candidate_against_policy(&candidate, None)?;
        self.tick_consensus_records.push(candidate);
        Ok(())
    }

    fn validate_tick_consensus_candidate_against_policy(
        &mut self,
        candidate: &TickConsensusRecord,
        existing: Option<&TickConsensusRecord>,
    ) -> Result<(), WorldError> {
        let source = candidate.certificate.authority_source.trim();
        if source.is_empty() {
            return self.reject_tick_consensus_submission(
                candidate,
                existing,
                "tick consensus authority_source cannot be empty".to_string(),
            );
        }
        if self.node_identity_public_key(source).is_none() {
            return self.reject_tick_consensus_submission(
                candidate,
                existing,
                format!("tick consensus source node is not trusted: {source}"),
            );
        }
        if !candidate.certificate.signatures.contains_key(source) {
            return self.reject_tick_consensus_submission(
                candidate,
                existing,
                format!("tick consensus signatures missing source signer: {source}"),
            );
        }
        if candidate.certificate.submission_role == TickConsensusSubmissionRole::Authority
            && source != self.tick_consensus_authority_source
        {
            return self.reject_tick_consensus_submission(
                candidate,
                existing,
                format!(
                    "authority submission source mismatch: expected={} found={source}",
                    self.tick_consensus_authority_source
                ),
            );
        }
        Ok(())
    }

    fn validate_tick_consensus_candidate_against_existing(
        &mut self,
        candidate: &TickConsensusRecord,
        existing: &TickConsensusRecord,
    ) -> Result<(), WorldError> {
        let existing_role = existing.certificate.submission_role;
        let candidate_role = candidate.certificate.submission_role;
        if existing_role == TickConsensusSubmissionRole::Authority {
            if candidate_role != TickConsensusSubmissionRole::Authority {
                return self.reject_tick_consensus_submission(
                    candidate,
                    Some(existing),
                    format!(
                        "non-authoritative submission rejected at tick {} because authoritative commitment already exists",
                        candidate.block.header.tick
                    ),
                );
            }
            if existing.certificate.authority_source != candidate.certificate.authority_source {
                return self.reject_tick_consensus_submission(
                    candidate,
                    Some(existing),
                    format!(
                        "conflicting authority sources at tick {}: existing={} attempted={}",
                        candidate.block.header.tick,
                        existing.certificate.authority_source,
                        candidate.certificate.authority_source
                    ),
                );
            }
            return Ok(());
        }

        if candidate_role == TickConsensusSubmissionRole::Propagation
            && existing.certificate.authority_source != candidate.certificate.authority_source
            && existing.certificate.block_hash != candidate.certificate.block_hash
        {
            return self.reject_tick_consensus_submission(
                candidate,
                Some(existing),
                format!(
                    "propagation conflict at tick {} requires authoritative adjudication",
                    candidate.block.header.tick
                ),
            );
        }
        Ok(())
    }

    fn reject_tick_consensus_submission(
        &mut self,
        candidate: &TickConsensusRecord,
        existing: Option<&TickConsensusRecord>,
        reason: String,
    ) -> Result<(), WorldError> {
        self.push_tick_consensus_rejection_audit_event(TickConsensusRejectionAuditEvent {
            recorded_at_tick: self.state.time,
            tick: candidate.block.header.tick,
            consensus_height: candidate.certificate.consensus_height,
            attempted_source: candidate.certificate.authority_source.clone(),
            attempted_role: candidate.certificate.submission_role,
            existing_source: existing.map(|record| record.certificate.authority_source.clone()),
            existing_role: existing.map(|record| record.certificate.submission_role),
            reason: reason.clone(),
        });
        Err(WorldError::DistributedValidationFailed { reason })
    }

    fn push_tick_consensus_rejection_audit_event(
        &mut self,
        event: TickConsensusRejectionAuditEvent,
    ) {
        if self.tick_consensus_rejection_audit_events.len()
            >= MAX_TICK_CONSENSUS_REJECTION_AUDIT_EVENTS
        {
            self.tick_consensus_rejection_audit_events.remove(0);
        }
        self.tick_consensus_rejection_audit_events.push(event);
    }

    fn validate_tick_consensus_source_node(
        &self,
        source_node_id: &str,
    ) -> Result<String, WorldError> {
        let source_node_id = source_node_id.trim();
        if source_node_id.is_empty() {
            return Err(WorldError::DistributedValidationFailed {
                reason: "tick consensus source_node_id cannot be empty".to_string(),
            });
        }
        if self.node_identity_public_key(source_node_id).is_none() {
            return Err(WorldError::DistributedValidationFailed {
                reason: format!("tick consensus source node is not trusted: {source_node_id}"),
            });
        }
        Ok(source_node_id.to_string())
    }

    fn validate_tick_consensus_record_metadata(
        &self,
        record: &TickConsensusRecord,
    ) -> Result<(), WorldError> {
        let source = record.certificate.authority_source.trim();
        if source.is_empty() {
            return Err(WorldError::DistributedValidationFailed {
                reason: format!(
                    "tick consensus authority_source is empty at tick {}",
                    record.block.header.tick
                ),
            });
        }
        if self.node_identity_public_key(source).is_none() {
            return Err(WorldError::DistributedValidationFailed {
                reason: format!(
                    "tick consensus authority_source is untrusted at tick {}: {}",
                    record.block.header.tick, source
                ),
            });
        }
        if !record.certificate.signatures.contains_key(source) {
            return Err(WorldError::DistributedValidationFailed {
                reason: format!(
                    "tick consensus source signature missing at tick {} source={}",
                    record.block.header.tick, source
                ),
            });
        }
        Ok(())
    }

    fn verify_latest_tick_consensus_record(&self) -> Result<(), WorldError> {
        let Some(record) = self.tick_consensus_records.last() else {
            return Ok(());
        };
        self.validate_tick_consensus_record_metadata(record)?;
        let events_hash = self.hash_tick_events_from_ids(
            record.block.header.tick,
            record.block.ordered_event_ids.as_slice(),
        )?;
        if events_hash != record.block.header.events_hash {
            return Err(WorldError::DistributedValidationFailed {
                reason: format!(
                    "latest tick events hash mismatch tick={} expected={} found={}",
                    record.block.header.tick, events_hash, record.block.header.events_hash
                ),
            });
        }
        let block_hash = record.block.block_hash();
        if block_hash != record.certificate.block_hash {
            return Err(WorldError::DistributedValidationFailed {
                reason: format!(
                    "latest tick block hash mismatch tick={} expected={} found={}",
                    record.block.header.tick, block_hash, record.certificate.block_hash
                ),
            });
        }
        let current_state_root = self.current_state_root_hash()?;
        if record.block.header.tick == self.state.time
            && current_state_root != record.block.header.state_root
        {
            return Err(WorldError::DistributedValidationFailed {
                reason: format!(
                    "latest tick state root mismatch tick={} expected={} found={}",
                    record.block.header.tick, current_state_root, record.block.header.state_root
                ),
            });
        }
        Ok(())
    }

    fn hash_tick_events_from_ids(
        &self,
        tick: WorldTime,
        ordered_event_ids: &[WorldEventId],
    ) -> Result<String, WorldError> {
        if ordered_event_ids.is_empty() {
            let empty_inputs: Vec<TickEventHashInput<'_>> = Vec::new();
            return hash_json(&empty_inputs);
        }
        let mut events = Vec::with_capacity(ordered_event_ids.len());
        for event_id in ordered_event_ids {
            let event = self
                .journal
                .events
                .iter()
                .find(|event| event.id == *event_id && event.time == tick)
                .ok_or_else(|| WorldError::DistributedValidationFailed {
                    reason: format!(
                        "tick event missing during hash rebuild tick={} event_id={}",
                        tick, event_id
                    ),
                })?;
            events.push(event.clone());
        }
        self.hash_tick_events(events.as_slice())
    }

    fn hash_tick_events(&self, events: &[WorldEvent]) -> Result<String, WorldError> {
        let inputs: Vec<TickEventHashInput<'_>> = events
            .iter()
            .map(|event| TickEventHashInput {
                id: event.id,
                caused_by: &event.caused_by,
                body: &event.body,
            })
            .collect();
        hash_json(&inputs)
    }

    fn hash_tick_domain_events(events: &[WorldEvent]) -> Result<String, WorldError> {
        let inputs: Vec<TickEventHashInput<'_>> = events
            .iter()
            .filter(|event| matches!(event.body, WorldEventBody::Domain(_)))
            .map(|event| TickEventHashInput {
                id: event.id,
                caused_by: &event.caused_by,
                body: &event.body,
            })
            .collect();
        hash_json(&inputs)
    }

    fn extract_ordered_action_ids(events: &[WorldEvent]) -> Vec<u64> {
        let mut seen = BTreeSet::new();
        let mut ordered = Vec::new();
        for event in events {
            let Some(CausedBy::Action(action_id)) = &event.caused_by else {
                continue;
            };
            if seen.insert(*action_id) {
                ordered.push(*action_id);
            }
        }
        ordered
    }

    fn current_state_root_hash(&self) -> Result<String, WorldError> {
        let manifest_hash = self.current_manifest_hash()?;
        let policy_hash = hash_json(&self.policies)?;
        let projection = StateRootProjection {
            state: &self.state,
            manifest_hash: manifest_hash.as_str(),
            policy_hash: policy_hash.as_str(),
        };
        hash_json(&projection)
    }

    fn consensus_height_for_tick(&self, tick: WorldTime) -> u64 {
        match self
            .tick_consensus_records
            .iter()
            .position(|record| record.block.header.tick == tick)
        {
            Some(index) => index as u64 + 1,
            None => self.tick_consensus_records.len() as u64 + 1,
        }
    }

    fn parent_hash_for_tick(&self, tick: WorldTime) -> String {
        self.tick_consensus_records
            .iter()
            .rev()
            .find(|record| record.block.header.tick < tick)
            .map(|record| record.certificate.block_hash.clone())
            .unwrap_or_else(|| TICK_CONSENSUS_GENESIS_PARENT_HASH.to_string())
    }

    fn derive_tick_randomness_seed(parent_hash: &str, tick: WorldTime) -> String {
        sha256_hex(format!("tick-rand:v1|{parent_hash}|{tick}").as_bytes())
    }
}
