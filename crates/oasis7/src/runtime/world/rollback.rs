use std::collections::{BTreeMap, BTreeSet};

use ed25519_dalek::{Signature, Verifier, VerifyingKey};

use super::super::util::{hash_json, sha256_hex, to_canonical_cbor};
use super::super::{
    Journal, RollbackAuthorityRegistry, RollbackAuthorityRole, RollbackAuthorizationEnvelope,
    RollbackEvent, RollbackNonceOutcome, RollbackOutcomeRecoveryMetadata, RollbackReadiness,
    RollbackReadinessEvidence, RollbackReceiptProjection, RollbackSourceEventIdentity, Snapshot,
    WorldError,
};
use super::World;

#[derive(serde::Serialize)]
struct RollbackJournalCommitment<'a> {
    domain: &'static str,
    snapshot_hash: &'a str,
    snapshot_journal_len: usize,
    target_journal_len: usize,
    events: &'a [super::super::WorldEvent],
}

fn validate_rollback_dispositions(
    affected_events: Option<&[RollbackSourceEventIdentity]>,
    dispositions: &[super::super::RollbackEventDisposition],
) -> Result<(), WorldError> {
    let mut found = BTreeSet::new();
    for disposition in dispositions {
        let identity = RollbackSourceEventIdentity {
            source_batch_id: disposition.source_batch_id.clone(),
            source_event_id: disposition.source_event_id,
        };
        if identity.source_batch_id.trim().is_empty() || !found.insert(identity) {
            return Err(WorldError::DistributedValidationFailed {
                reason: "rollback dispositions require unique source batch/event identities"
                    .to_string(),
            });
        }
        match disposition.status {
            super::super::RollbackDispositionStatus::CompensationRequired => {
                let case = disposition.compensation.as_ref().ok_or_else(|| {
                    WorldError::DistributedValidationFailed {
                        reason: "compensation_required disposition lacks case reference"
                            .to_string(),
                    }
                })?;
                if case.owner_id.trim().is_empty() || case.ticket_id.trim().is_empty() {
                    return Err(WorldError::DistributedValidationFailed {
                        reason: "compensation case requires owner and ticket".to_string(),
                    });
                }
            }
            _ if disposition.compensation.is_some() => {
                return Err(WorldError::DistributedValidationFailed {
                    reason: "only compensation_required may reference compensation".to_string(),
                });
            }
            _ => {}
        }
    }
    if let Some(affected) = affected_events {
        let expected = affected.iter().cloned().collect::<BTreeSet<_>>();
        if expected.len() != affected.len() || expected != found {
            return Err(WorldError::DistributedValidationFailed {
                reason: "rollback disposition coverage does not exactly match affected events"
                    .to_string(),
            });
        }
    }
    Ok(())
}

pub fn rollback_journal_commitment(
    snapshot: &Snapshot,
    journal: &Journal,
    target_journal_len: usize,
) -> Result<String, WorldError> {
    if target_journal_len > journal.len() || snapshot.journal_len > target_journal_len {
        return Err(WorldError::RollbackReplayTargetInvalid {
            snapshot_journal_len: snapshot.journal_len,
            target_journal_len,
            supplied_journal_len: journal.len(),
        });
    }
    let snapshot_hash = hash_json(snapshot)?;
    Ok(sha256_hex(&to_canonical_cbor(
        &RollbackJournalCommitment {
            domain: "oasis7:rollback-journal-commitment:v1",
            snapshot_hash: snapshot_hash.as_str(),
            snapshot_journal_len: snapshot.journal_len,
            target_journal_len,
            events: &journal.events[..target_journal_len],
        },
    )?))
}

impl World {
    pub fn rollback_nonce_outcome(&self, nonce: &str) -> Option<RollbackNonceOutcome> {
        self.rollback_nonce_outcomes.get(nonce).cloned()
    }

    pub fn record_rollback_outcome_recovery_metadata(
        &mut self,
        nonce: &str,
        metadata: RollbackOutcomeRecoveryMetadata,
    ) -> Result<(), WorldError> {
        if !metadata.invalidated_batch_ids.is_empty() && metadata.dispositions.is_empty() {
            return Err(WorldError::DistributedValidationFailed {
                reason: "rollback disposition coverage is incomplete".to_string(),
            });
        }
        validate_rollback_dispositions(None, &metadata.dispositions)?;
        let outcome = self.rollback_nonce_outcomes.get_mut(nonce).ok_or_else(|| {
            WorldError::DistributedValidationFailed {
                reason: format!("rollback outcome for nonce {nonce} is not committed"),
            }
        })?;
        if outcome.receipt.is_some() {
            return Err(WorldError::DistributedValidationFailed {
                reason: "committed rollback receipt is immutable".to_string(),
            });
        }
        outcome.target_batch_id = metadata.target_batch_id;
        outcome.prior_reorg_epoch = metadata.prior_reorg_epoch;
        outcome.committed_reorg_epoch = metadata.committed_reorg_epoch;
        outcome.invalidated_batch_ids = metadata.invalidated_batch_ids;
        outcome.dispositions = metadata.dispositions;
        Ok(())
    }

    pub fn complete_rollback_outcome(
        &mut self,
        nonce: &str,
        metadata: RollbackOutcomeRecoveryMetadata,
        affected_events: &[RollbackSourceEventIdentity],
        receipt: RollbackReceiptProjection,
    ) -> Result<(), WorldError> {
        validate_rollback_dispositions(Some(affected_events), &metadata.dispositions)?;
        let outcome = self.rollback_nonce_outcomes.get_mut(nonce).ok_or_else(|| {
            WorldError::DistributedValidationFailed {
                reason: format!("rollback outcome for nonce {nonce} is not committed"),
            }
        })?;
        if let Some(existing) = outcome.receipt.as_ref() {
            if existing == &receipt
                && outcome.target_batch_id == metadata.target_batch_id
                && outcome.prior_reorg_epoch == metadata.prior_reorg_epoch
                && outcome.committed_reorg_epoch == metadata.committed_reorg_epoch
                && outcome.invalidated_batch_ids == metadata.invalidated_batch_ids
                && outcome.dispositions == metadata.dispositions
            {
                return Ok(());
            }
            return Err(WorldError::DistributedValidationFailed {
                reason: "committed rollback receipt is immutable".to_string(),
            });
        }
        outcome.target_batch_id = metadata.target_batch_id;
        outcome.prior_reorg_epoch = metadata.prior_reorg_epoch;
        outcome.committed_reorg_epoch = metadata.committed_reorg_epoch;
        outcome.invalidated_batch_ids = metadata.invalidated_batch_ids;
        outcome.dispositions = metadata.dispositions;
        outcome.receipt = Some(receipt);
        Ok(())
    }

    pub fn rollback_readiness(
        &self,
        nonce: &str,
        affected_events: &[RollbackSourceEventIdentity],
        evidence: &RollbackReadinessEvidence,
    ) -> RollbackReadiness {
        let Some(outcome) = self.rollback_nonce_outcomes.get(nonce) else {
            return RollbackReadiness::Blocked {
                reasons: vec!["rollback_outcome_missing".to_string()],
            };
        };
        let mut reasons = Vec::new();
        for (passed, reason) in [
            (evidence.target_root_matches, "target_root_mismatch"),
            (evidence.epoch_matches, "reorg_epoch_mismatch"),
            (evidence.drift_free, "consensus_drift_present"),
            (evidence.consensus_chain_valid, "consensus_chain_invalid"),
            (evidence.receipt_retrievable, "receipt_not_retrievable"),
            (outcome.receipt.is_some(), "immutable_receipt_missing"),
        ] {
            if !passed {
                reasons.push(reason.to_string());
            }
        }
        if let Err(error) =
            validate_rollback_dispositions(Some(affected_events), &outcome.dispositions)
        {
            reasons.push(format!("disposition_coverage_invalid:{error:?}"));
        }
        if reasons.is_empty() {
            RollbackReadiness::Ready
        } else {
            RollbackReadiness::Blocked { reasons }
        }
    }

    pub fn rollback_to_snapshot(
        &mut self,
        snapshot: Snapshot,
        mut journal: Journal,
        reason: impl Into<String>,
        target_batch_id: Option<&str>,
        approval: RollbackAuthorizationEnvelope,
        now_ms: u64,
    ) -> Result<(), WorldError> {
        let prior_journal_len = journal.len();
        journal.events.truncate(snapshot.journal_len);
        self.rollback_to_snapshot_candidate(
            snapshot,
            journal,
            prior_journal_len,
            reason,
            target_batch_id,
            approval,
            None,
            now_ms,
        )
    }

    fn rollback_to_snapshot_candidate(
        &mut self,
        snapshot: Snapshot,
        journal: Journal,
        prior_journal_len: usize,
        reason: impl Into<String>,
        target_batch_id: Option<&str>,
        approval: RollbackAuthorizationEnvelope,
        canonical_v2_payload: Option<&[u8]>,
        now_ms: u64,
    ) -> Result<(), WorldError> {
        let reason = reason.into();
        if snapshot.journal_len > journal.len() {
            return Err(WorldError::JournalMismatch);
        }
        let canonical_payload = match canonical_v2_payload {
            Some(payload)
                if approval.intent.schema_version == 2
                    && payload.starts_with(b"oasis7:governed-rollback-replay:v2\0") =>
            {
                payload.to_vec()
            }
            Some(_) => {
                return Err(WorldError::DistributedValidationFailed {
                    reason: "rollback v2 canonical payload domain mismatch".to_string(),
                });
            }
            None => approval.intent.canonical_signing_payload()?,
        };
        let supplied_intent_hash = sha256_hex(&canonical_payload);
        if let Some(committed) = self.rollback_nonce_outcomes.get(&approval.intent.nonce) {
            if committed.canonical_intent_hash == supplied_intent_hash {
                return Ok(());
            }
            return Err(WorldError::RollbackNonceConflict {
                nonce: approval.intent.nonce.clone(),
                committed_intent_hash: committed.canonical_intent_hash.clone(),
                supplied_intent_hash,
            });
        }
        let verified = self.verify_rollback_authorization(
            &snapshot,
            &reason,
            target_batch_id,
            &approval,
            canonical_payload.as_slice(),
            now_ms,
        )?;

        if approval.intent.target_journal_len < snapshot.journal_len
            || approval.intent.target_journal_len != journal.len()
        {
            return Err(WorldError::RollbackReplayTargetInvalid {
                snapshot_journal_len: snapshot.journal_len,
                target_journal_len: approval.intent.target_journal_len,
                supplied_journal_len: journal.len(),
            });
        }

        self.validate_rollback_replay_journal(&snapshot, &journal)?;
        let actual_journal_commitment =
            rollback_journal_commitment(&snapshot, &journal, approval.intent.target_journal_len)?;
        if approval.intent.schema_version >= 2 {
            let expected_journal_commitment = approval
                .intent
                .target_journal_commitment
                .as_deref()
                .unwrap_or_default();
            if expected_journal_commitment != actual_journal_commitment {
                return Err(WorldError::RollbackJournalCommitmentMismatch {
                    expected: expected_journal_commitment.to_string(),
                    actual: actual_journal_commitment,
                });
            }
        }

        let signer = self.receipt_signer.clone();
        let authority_registry = self.rollback_authority_registry.clone();
        let mut consumed_nonces = self.consumed_rollback_nonces.clone();
        consumed_nonces.insert(approval.intent.nonce.clone());
        let mut world = Self::from_snapshot(snapshot.clone(), journal)?;
        world.receipt_signer = signer;
        world.rollback_authority_registry = authority_registry;
        world.consumed_rollback_nonces = consumed_nonces;
        world.rollback_nonce_outcomes = self.rollback_nonce_outcomes.clone();

        let actual_target_state_root = world.current_state_root_hash()?;
        if actual_target_state_root != approval.intent.expected_target_state_root {
            return Err(WorldError::RollbackTargetStateRootMismatch {
                expected: approval.intent.expected_target_state_root.clone(),
                actual: actual_target_state_root,
            });
        }

        let snapshot_hash = hash_json(&snapshot)?;
        let outcome_nonce = approval.intent.nonce.clone();
        let outcome_ticket = approval.intent.rollback_ticket.clone();
        let outcome_commitment = approval
            .intent
            .target_journal_commitment
            .clone()
            .unwrap_or_else(|| actual_journal_commitment.clone());
        let outcome_state_root = approval.intent.expected_target_state_root.clone();
        let event = RollbackEvent {
            snapshot_hash,
            snapshot_journal_len: snapshot.journal_len,
            prior_journal_len,
            reason,
            rollback_ticket: approval.intent.rollback_ticket,
            on_call_approver: verified.0.clone(),
            governance_approver: verified.1.clone(),
            on_call_authority_id: verified.0,
            governance_authority_id: verified.1,
            authorization_nonce: approval.intent.nonce,
        };
        world.append_event(super::super::WorldEventBody::RollbackApplied(event), None)?;
        let rollback_event_id = world
            .journal
            .events
            .last()
            .map(|event| event.id)
            .unwrap_or(0);
        world.rollback_nonce_outcomes.insert(
            outcome_nonce,
            RollbackNonceOutcome {
                canonical_intent_hash: supplied_intent_hash,
                rollback_ticket: outcome_ticket,
                target_journal_commitment: outcome_commitment,
                target_state_root: outcome_state_root,
                rollback_event_id,
                target_batch_id: approval.intent.target_batch_id.unwrap_or_default(),
                prior_reorg_epoch: 0,
                committed_reorg_epoch: 0,
                invalidated_batch_ids: Vec::new(),
                dispositions: Vec::new(),
                receipt: None,
            },
        );
        *self = world;
        Ok(())
    }

    pub fn rollback_to_snapshot_with_reconciliation(
        &mut self,
        snapshot: Snapshot,
        journal: Journal,
        reason: impl Into<String>,
        target_batch_id: Option<&str>,
        approval: RollbackAuthorizationEnvelope,
        now_ms: u64,
    ) -> Result<(), WorldError> {
        self.rollback_to_snapshot_with_reconciliation_payload(
            snapshot,
            journal,
            reason,
            target_batch_id,
            approval,
            None,
            now_ms,
        )
    }

    pub(crate) fn rollback_to_snapshot_with_reconciliation_v2(
        &mut self,
        snapshot: Snapshot,
        journal: Journal,
        reason: impl Into<String>,
        target_batch_id: Option<&str>,
        approval: RollbackAuthorizationEnvelope,
        canonical_v2_payload: &[u8],
        now_ms: u64,
    ) -> Result<(), WorldError> {
        self.rollback_to_snapshot_with_reconciliation_payload(
            snapshot,
            journal,
            reason,
            target_batch_id,
            approval,
            Some(canonical_v2_payload),
            now_ms,
        )
    }

    fn rollback_to_snapshot_with_reconciliation_payload(
        &mut self,
        snapshot: Snapshot,
        journal: Journal,
        reason: impl Into<String>,
        target_batch_id: Option<&str>,
        approval: RollbackAuthorizationEnvelope,
        canonical_v2_payload: Option<&[u8]>,
        now_ms: u64,
    ) -> Result<(), WorldError> {
        let mut candidate = self.clone();
        let prior_journal_len = journal.len();
        let reconciliation_tick = snapshot.state.time;
        candidate
            .rollback_to_snapshot_candidate(
                snapshot,
                journal,
                prior_journal_len,
                reason,
                target_batch_id,
                approval,
                canonical_v2_payload,
                now_ms,
            )
            .map_err(|error| match error {
                WorldError::DistributedValidationFailed { reason }
                    if reason.contains("tick consensus") =>
                {
                    WorldError::RollbackReconciliationFailed {
                        tick: reconciliation_tick,
                        reason,
                    }
                }
                other => other,
            })?;

        if let Some(drift) = candidate.first_tick_consensus_drift() {
            return Err(WorldError::RollbackReconciliationFailed {
                tick: drift.tick,
                reason: drift.reason,
            });
        }
        *self = candidate;
        Ok(())
    }

    pub fn set_rollback_authority_registry(
        &mut self,
        registry: RollbackAuthorityRegistry,
    ) -> Result<(), WorldError> {
        if registry.is_empty() {
            return Err(WorldError::DistributedValidationFailed {
                reason: "rollback authority registry must not be empty".to_string(),
            });
        }
        self.rollback_authority_registry = registry;
        Ok(())
    }

    fn verify_rollback_authorization(
        &self,
        snapshot: &Snapshot,
        reason: &str,
        target_batch_id: Option<&str>,
        envelope: &RollbackAuthorizationEnvelope,
        canonical_payload: &[u8],
        now_ms: u64,
    ) -> Result<(String, String), WorldError> {
        let reject = |reason: String| WorldError::DistributedValidationFailed { reason };
        if self.rollback_authority_registry.is_empty() {
            return Err(reject(
                "rollback authority registry is not configured".to_string(),
            ));
        }
        let intent = &envelope.intent;
        let expected_hash = hash_json(snapshot)?;
        if intent.rollback_ticket.trim().is_empty()
            || intent.reason != reason
            || intent.snapshot_hash != expected_hash
            || intent.snapshot_journal_len != snapshot.journal_len
            || intent.expected_target_state_root.trim().is_empty()
            || intent.target_batch_id.as_deref() != target_batch_id
            || intent.nonce.trim().is_empty()
        {
            return Err(reject(
                "rollback authorization does not match the exact operation".to_string(),
            ));
        }
        if intent.issued_at_ms > now_ms
            || intent.expires_at_ms < now_ms
            || intent.expires_at_ms <= intent.issued_at_ms
        {
            return Err(reject(
                "rollback authorization is not currently valid".to_string(),
            ));
        }
        if self.consumed_rollback_nonces.contains(&intent.nonce) {
            return Err(reject(
                "rollback authorization nonce was already consumed".to_string(),
            ));
        }
        if envelope.signatures.len() != 2 {
            return Err(reject(
                "rollback authorization requires exactly two signatures".to_string(),
            ));
        }
        let mut approved = BTreeMap::new();
        let mut approved_keys = BTreeSet::new();
        for approval in &envelope.signatures {
            if approval.signature_scheme != "ed25519" {
                return Err(reject(
                    "rollback authorization requires Ed25519 signatures".to_string(),
                ));
            }
            let record = self
                .rollback_authority_registry
                .get(&approval.authority_id)
                .ok_or_else(|| {
                    reject(format!(
                        "unknown rollback authority {}",
                        approval.authority_id
                    ))
                })?;
            if !record.active || record.role != approval.role {
                return Err(reject(format!(
                    "rollback authority {} is inactive or has the wrong role",
                    approval.authority_id
                )));
            }
            let public_key: [u8; 32] = hex::decode(record.public_key_hex.trim())
                .map_err(|_| reject("invalid rollback authority public key".to_string()))?
                .try_into()
                .map_err(|_| reject("invalid rollback authority public key length".to_string()))?;
            if !approved_keys.insert(public_key)
                || approved
                    .insert(approval.role, approval.authority_id.clone())
                    .is_some()
            {
                return Err(reject(
                    "rollback approvals must use distinct authorities, keys, and roles".to_string(),
                ));
            }
            let signature: [u8; 64] = hex::decode(approval.signature_hex.trim())
                .map_err(|_| reject("invalid rollback approval signature".to_string()))?
                .try_into()
                .map_err(|_| reject("invalid rollback approval signature length".to_string()))?;
            VerifyingKey::from_bytes(&public_key)
                .map_err(|_| reject("invalid rollback authority public key".to_string()))?
                .verify(canonical_payload, &Signature::from_bytes(&signature))
                .map_err(|_| {
                    reject("rollback approval signature verification failed".to_string())
                })?;
        }
        let on_call = approved
            .remove(&RollbackAuthorityRole::OnCall)
            .ok_or_else(|| {
                reject("rollback authorization is missing on-call approval".to_string())
            })?;
        let governance = approved
            .remove(&RollbackAuthorityRole::Governance)
            .ok_or_else(|| {
                reject("rollback authorization is missing governance approval".to_string())
            })?;
        Ok((on_call, governance))
    }

    fn validate_rollback_replay_journal(
        &self,
        snapshot: &Snapshot,
        journal: &Journal,
    ) -> Result<(), WorldError> {
        if snapshot.journal_len > journal.len() {
            return Err(WorldError::JournalMismatch);
        }
        if snapshot.journal_len > 0 {
            let boundary = &journal.events[snapshot.journal_len - 1];
            if boundary.id != snapshot.last_event_id {
                return Err(WorldError::RollbackReplayJournalInvalid {
                    index: snapshot.journal_len - 1,
                    expected_event_id: snapshot.last_event_id,
                    found_event_id: boundary.id,
                });
            }
        }
        let mut expected_event_id = snapshot.last_event_id.saturating_add(1);
        for (index, event) in journal.events.iter().enumerate().skip(snapshot.journal_len) {
            if event.id != expected_event_id {
                return Err(WorldError::RollbackReplayJournalInvalid {
                    index,
                    expected_event_id,
                    found_event_id: event.id,
                });
            }
            expected_event_id = expected_event_id.saturating_add(1);
        }
        Ok(())
    }
}
