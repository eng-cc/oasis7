use std::collections::{BTreeMap, BTreeSet};

use ed25519_dalek::{Signature, Verifier, VerifyingKey};

use super::super::util::hash_json;
use super::super::{
    Journal, RollbackAuthorityRegistry, RollbackAuthorityRole, RollbackAuthorizationEnvelope,
    RollbackEvent, Snapshot, WorldError,
};
use super::World;

impl World {
    pub fn rollback_to_snapshot(
        &mut self,
        snapshot: Snapshot,
        mut journal: Journal,
        reason: impl Into<String>,
        target_batch_id: Option<&str>,
        approval: RollbackAuthorizationEnvelope,
        now_ms: u64,
    ) -> Result<(), WorldError> {
        let reason = reason.into();
        if snapshot.journal_len > journal.len() {
            return Err(WorldError::JournalMismatch);
        }
        let verified = self.verify_rollback_authorization(
            &snapshot,
            &reason,
            target_batch_id,
            &approval,
            now_ms,
        )?;

        let prior_len = journal.len();
        journal.events.truncate(snapshot.journal_len);

        let signer = self.receipt_signer.clone();
        let authority_registry = self.rollback_authority_registry.clone();
        let mut consumed_nonces = self.consumed_rollback_nonces.clone();
        consumed_nonces.insert(approval.intent.nonce.clone());
        let mut world = Self::from_snapshot(snapshot.clone(), journal)?;
        world.receipt_signer = signer;
        world.rollback_authority_registry = authority_registry;
        world.consumed_rollback_nonces = consumed_nonces;

        let snapshot_hash = hash_json(&snapshot)?;
        let event = RollbackEvent {
            snapshot_hash,
            snapshot_journal_len: snapshot.journal_len,
            prior_journal_len: prior_len,
            reason,
            rollback_ticket: approval.intent.rollback_ticket,
            on_call_approver: verified.0.clone(),
            governance_approver: verified.1.clone(),
            on_call_authority_id: verified.0,
            governance_authority_id: verified.1,
            authorization_nonce: approval.intent.nonce,
        };
        world.append_event(super::super::WorldEventBody::RollbackApplied(event), None)?;
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
        let mut candidate = self.clone();
        candidate.rollback_to_snapshot(
            snapshot,
            journal,
            reason,
            target_batch_id,
            approval,
            now_ms,
        )?;

        if let Some(drift) = candidate.first_tick_consensus_drift() {
            return Err(WorldError::DistributedValidationFailed {
                reason: format!(
                    "rollback reconciliation drift detected at tick {}: {}",
                    drift.tick, drift.reason
                ),
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
        let payload = intent.canonical_signing_payload()?;
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
                .verify(&payload, &Signature::from_bytes(&signature))
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
}
