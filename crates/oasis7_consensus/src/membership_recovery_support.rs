impl MembershipSyncClient {
    pub fn new(network: Arc<dyn DistributedNetwork + Send + Sync>) -> Self {
        Self { network }
    }

    pub fn subscribe(&self, world_id: &str) -> Result<MembershipSyncSubscription, WorldError> {
        let membership_sub = self.network.subscribe(&topic_membership(world_id))?;
        let revocation_sub = self
            .network
            .subscribe(&topic_membership_revocation(world_id))?;
        let reconcile_sub = self
            .network
            .subscribe(&topic_membership_reconcile(world_id))?;
        Ok(MembershipSyncSubscription {
            membership_sub,
            revocation_sub,
            reconcile_sub,
        })
    }

    pub fn publish_membership_change(
        &self,
        world_id: &str,
        request: &ConsensusMembershipChangeRequest,
        result: &ConsensusMembershipChangeResult,
    ) -> Result<MembershipDirectoryAnnounce, WorldError> {
        let announce =
            MembershipDirectoryAnnounce::from_membership_change(world_id, request, result);
        self.publish_announcement(world_id, &announce)?;
        Ok(announce)
    }

    pub fn publish_membership_change_with_dht(
        &self,
        world_id: &str,
        request: &ConsensusMembershipChangeRequest,
        result: &ConsensusMembershipChangeResult,
        dht: &(dyn DistributedDht + Send + Sync),
    ) -> Result<MembershipDirectoryAnnounce, WorldError> {
        let announce = self.publish_membership_change(world_id, request, result)?;
        dht.put_membership_directory(world_id, &announce.clone().into_snapshot())?;
        Ok(announce)
    }

    pub fn publish_membership_change_with_dht_signed(
        &self,
        world_id: &str,
        request: &ConsensusMembershipChangeRequest,
        result: &ConsensusMembershipChangeResult,
        dht: &(dyn DistributedDht + Send + Sync),
        signer: &MembershipDirectorySigner,
    ) -> Result<MembershipDirectoryAnnounce, WorldError> {
        self.publish_membership_change_with_dht_signed_by_key_id(
            world_id, request, result, dht, signer, None,
        )
    }

    pub fn publish_membership_change_with_dht_signed_by_key_id(
        &self,
        world_id: &str,
        request: &ConsensusMembershipChangeRequest,
        result: &ConsensusMembershipChangeResult,
        dht: &(dyn DistributedDht + Send + Sync),
        signer: &MembershipDirectorySigner,
        signature_key_id: Option<&str>,
    ) -> Result<MembershipDirectoryAnnounce, WorldError> {
        let mut announce =
            MembershipDirectoryAnnounce::from_membership_change(world_id, request, result);
        let mut snapshot = announce.clone().into_snapshot();
        if let Some(key_id) = signature_key_id {
            let key_id = membership_logic::normalized_key_id(key_id.to_string())?;
            announce.signature_key_id = Some(key_id.clone());
            snapshot.signature_key_id = Some(key_id);
        }
        let signature = signer.sign_snapshot(&snapshot)?;
        announce.signature = Some(signature.clone());
        snapshot.signature = Some(signature);

        self.publish_announcement(world_id, &announce)?;
        dht.put_membership_directory(world_id, &snapshot)?;
        Ok(announce)
    }

    pub fn publish_membership_change_with_dht_signed_with_keyring(
        &self,
        world_id: &str,
        request: &ConsensusMembershipChangeRequest,
        result: &ConsensusMembershipChangeResult,
        dht: &(dyn DistributedDht + Send + Sync),
        keyring: &MembershipDirectorySignerKeyring,
    ) -> Result<MembershipDirectoryAnnounce, WorldError> {
        let mut announce =
            MembershipDirectoryAnnounce::from_membership_change(world_id, request, result);
        let mut snapshot = announce.clone().into_snapshot();
        let (signature_key_id, signature) = keyring.sign_snapshot_with_active_key(&snapshot)?;
        announce.signature_key_id = Some(signature_key_id.clone());
        announce.signature = Some(signature.clone());
        snapshot.signature_key_id = Some(signature_key_id);
        snapshot.signature = Some(signature);

        self.publish_announcement(world_id, &announce)?;
        dht.put_membership_directory(world_id, &snapshot)?;
        Ok(announce)
    }

    pub fn publish_key_revocation(
        &self,
        world_id: &str,
        requester_id: &str,
        requested_at_ms: i64,
        key_id: &str,
        reason: Option<String>,
    ) -> Result<MembershipKeyRevocationAnnounce, WorldError> {
        let announce = Self::build_key_revocation_announce(
            world_id,
            requester_id,
            requested_at_ms,
            key_id,
            reason,
        )?;
        self.publish_revocation(world_id, &announce)?;
        Ok(announce)
    }

    pub fn publish_key_revocation_signed(
        &self,
        world_id: &str,
        requester_id: &str,
        requested_at_ms: i64,
        key_id: &str,
        reason: Option<String>,
        signer: &MembershipDirectorySigner,
    ) -> Result<MembershipKeyRevocationAnnounce, WorldError> {
        self.publish_key_revocation_signed_by_key_id(
            world_id,
            requester_id,
            requested_at_ms,
            key_id,
            reason,
            signer,
            None,
        )
    }

    pub fn publish_key_revocation_signed_by_key_id(
        &self,
        world_id: &str,
        requester_id: &str,
        requested_at_ms: i64,
        key_id: &str,
        reason: Option<String>,
        signer: &MembershipDirectorySigner,
        signature_key_id: Option<&str>,
    ) -> Result<MembershipKeyRevocationAnnounce, WorldError> {
        let mut announce = Self::build_key_revocation_announce(
            world_id,
            requester_id,
            requested_at_ms,
            key_id,
            reason,
        )?;
        if let Some(key_id) = signature_key_id {
            announce.signature_key_id =
                Some(membership_logic::normalized_key_id(key_id.to_string())?);
        }
        let signature = signer.sign_revocation(&announce)?;
        announce.signature = Some(signature);
        self.publish_revocation(world_id, &announce)?;
        Ok(announce)
    }

    pub fn publish_key_revocation_signed_with_keyring(
        &self,
        world_id: &str,
        requester_id: &str,
        requested_at_ms: i64,
        key_id: &str,
        reason: Option<String>,
        keyring: &MembershipDirectorySignerKeyring,
    ) -> Result<MembershipKeyRevocationAnnounce, WorldError> {
        let mut announce = Self::build_key_revocation_announce(
            world_id,
            requester_id,
            requested_at_ms,
            key_id,
            reason,
        )?;
        let (signature_key_id, signature) = keyring.sign_revocation_with_active_key(&announce)?;
        announce.signature_key_id = Some(signature_key_id);
        announce.signature = Some(signature);
        self.publish_revocation(world_id, &announce)?;
        Ok(announce)
    }

    fn build_key_revocation_announce(
        world_id: &str,
        requester_id: &str,
        requested_at_ms: i64,
        key_id: &str,
        reason: Option<String>,
    ) -> Result<MembershipKeyRevocationAnnounce, WorldError> {
        let key_id = membership_logic::normalized_key_id(key_id.to_string())?;
        Ok(MembershipKeyRevocationAnnounce {
            world_id: world_id.to_string(),
            requester_id: requester_id.to_string(),
            requested_at_ms,
            key_id,
            reason,
            signature_key_id: None,
            signature: None,
        })
    }

    fn publish_announcement(
        &self,
        world_id: &str,
        announce: &MembershipDirectoryAnnounce,
    ) -> Result<(), WorldError> {
        let payload = to_canonical_cbor(announce)?;
        self.network
            .publish(&topic_membership(world_id), &payload)?;
        Ok(())
    }

    fn publish_revocation(
        &self,
        world_id: &str,
        announce: &MembershipKeyRevocationAnnounce,
    ) -> Result<(), WorldError> {
        let payload = to_canonical_cbor(announce)?;
        self.network
            .publish(&topic_membership_revocation(world_id), &payload)?;
        Ok(())
    }

    pub fn drain_announcements(
        &self,
        subscription: &MembershipSyncSubscription,
    ) -> Result<Vec<MembershipDirectoryAnnounce>, WorldError> {
        let raw = subscription.membership_sub.drain();
        let mut announcements = Vec::with_capacity(raw.len());
        for bytes in raw {
            announcements.push(serde_cbor::from_slice(&bytes)?);
        }
        Ok(announcements)
    }

    pub fn drain_key_revocations(
        &self,
        subscription: &MembershipSyncSubscription,
    ) -> Result<Vec<MembershipKeyRevocationAnnounce>, WorldError> {
        let raw = subscription.revocation_sub.drain();
        let mut revocations = Vec::with_capacity(raw.len());
        for bytes in raw {
            revocations.push(serde_cbor::from_slice(&bytes)?);
        }
        Ok(revocations)
    }

    pub fn sync_key_revocations(
        &self,
        subscription: &MembershipSyncSubscription,
        keyring: &mut MembershipDirectorySignerKeyring,
    ) -> Result<usize, WorldError> {
        let revocations = self.drain_key_revocations(subscription)?;
        let mut applied = 0usize;
        for revocation in revocations {
            if keyring.revoke_key(&revocation.key_id)? {
                applied =
                    checked_usize_increment(applied, "membership sync_key_revocations applied")?;
            }
        }
        Ok(applied)
    }

    pub fn sync_key_revocations_with_policy(
        &self,
        world_id: &str,
        subscription: &MembershipSyncSubscription,
        keyring: &mut MembershipDirectorySignerKeyring,
        signer: Option<&MembershipDirectorySigner>,
        policy: &MembershipRevocationSyncPolicy,
    ) -> Result<MembershipRevocationSyncReport, WorldError> {
        let accepted_signature_signer_public_keys =
            membership_logic::validate_membership_revocation_sync_policy(policy)?;
        let revocations = self.drain_key_revocations(subscription)?;
        let mut report = MembershipRevocationSyncReport {
            drained: revocations.len(),
            applied: 0,
            ignored: 0,
            rejected: 0,
        };
        let mut verification_keyring = if signer.is_none() {
            Some(keyring.clone())
        } else {
            None
        };
        for revocation in revocations {
            let keyring_ref = verification_keyring.as_ref();
            if let Err(_err) = membership_logic::validate_key_revocation(
                world_id,
                &revocation,
                signer,
                keyring_ref,
                policy,
                accepted_signature_signer_public_keys.as_ref(),
            ) {
                report.rejected = checked_usize_increment(
                    report.rejected,
                    "membership revocation report rejected",
                )?;
                continue;
            }
            if keyring.revoke_key(&revocation.key_id)? {
                report.applied = checked_usize_increment(
                    report.applied,
                    "membership revocation report applied",
                )?;
                if let Some(verifier) = verification_keyring.as_mut() {
                    let _ = verifier.revoke_key(&revocation.key_id);
                }
            } else {
                report.ignored = checked_usize_increment(
                    report.ignored,
                    "membership revocation report ignored",
                )?;
            }
        }
        Ok(report)
    }

    pub fn sync_membership_directory(
        &self,
        subscription: &MembershipSyncSubscription,
        consensus: &mut QuorumConsensus,
    ) -> Result<MembershipSyncReport, WorldError> {
        let announcements = self.drain_announcements(subscription)?;
        let mut report = MembershipSyncReport {
            drained: announcements.len(),
            applied: 0,
            ignored: 0,
        };
        for announce in announcements {
            let request = ConsensusMembershipChangeRequest {
                requester_id: announce.requester_id,
                requested_at_ms: announce.requested_at_ms,
                reason: announce.reason,
                change: ConsensusMembershipChange::ReplaceValidators {
                    validators: announce.validators,
                    quorum_threshold: announce.quorum_threshold,
                },
            };
            let result = consensus.apply_membership_change(&request)?;
            if result.applied {
                report.applied =
                    checked_usize_increment(report.applied, "membership directory report applied")?;
            } else {
                report.ignored =
                    checked_usize_increment(report.ignored, "membership directory report ignored")?;
            }
        }
        Ok(report)
    }

    pub fn restore_membership_from_dht(
        &self,
        world_id: &str,
        consensus: &mut QuorumConsensus,
        dht: &(dyn DistributedDht + Send + Sync),
    ) -> Result<Option<ConsensusMembershipChangeResult>, WorldError> {
        let policy = MembershipSnapshotRestorePolicy {
            require_signature: true,
            ..MembershipSnapshotRestorePolicy::default()
        };
        let report = self.restore_membership_from_dht_verified_with_audit(
            world_id, consensus, dht, None, None, &policy,
        )?;
        restore_result_from_audit(report)
    }

    pub fn restore_membership_from_dht_verified(
        &self,
        world_id: &str,
        consensus: &mut QuorumConsensus,
        dht: &(dyn DistributedDht + Send + Sync),
        signer: Option<&MembershipDirectorySigner>,
        policy: &MembershipSnapshotRestorePolicy,
    ) -> Result<Option<ConsensusMembershipChangeResult>, WorldError> {
        let report = self.restore_membership_from_dht_verified_with_audit(
            world_id, consensus, dht, signer, None, policy,
        )?;
        restore_result_from_audit(report)
    }

    pub fn restore_membership_from_dht_verified_with_keyring(
        &self,
        world_id: &str,
        consensus: &mut QuorumConsensus,
        dht: &(dyn DistributedDht + Send + Sync),
        keyring: Option<&MembershipDirectorySignerKeyring>,
        policy: &MembershipSnapshotRestorePolicy,
    ) -> Result<Option<ConsensusMembershipChangeResult>, WorldError> {
        let report = self.restore_membership_from_dht_verified_with_audit(
            world_id, consensus, dht, None, keyring, policy,
        )?;
        restore_result_from_audit(report)
    }

    pub fn restore_membership_from_dht_verified_with_audit(
        &self,
        world_id: &str,
        consensus: &mut QuorumConsensus,
        dht: &(dyn DistributedDht + Send + Sync),
        signer: Option<&MembershipDirectorySigner>,
        keyring: Option<&MembershipDirectorySignerKeyring>,
        policy: &MembershipSnapshotRestorePolicy,
    ) -> Result<MembershipRestoreAuditReport, WorldError> {
        let accepted_signature_signer_public_keys =
            membership_logic::validate_membership_snapshot_restore_policy(policy)?;
        let snapshot = dht.get_membership_directory(world_id)?;
        let Some(snapshot) = snapshot else {
            return Ok(MembershipRestoreAuditReport {
                restored: None,
                audit: MembershipSnapshotAuditRecord {
                    world_id: world_id.to_string(),
                    requester_id: None,
                    requested_at_ms: None,
                    signature_key_id: None,
                    outcome: MembershipSnapshotAuditOutcome::MissingSnapshot,
                    reason: "membership snapshot not found in dht".to_string(),
                },
            });
        };

        if let Err(err) = membership_logic::validate_membership_snapshot(
            world_id,
            &snapshot,
            signer,
            keyring,
            policy,
            accepted_signature_signer_public_keys.as_ref(),
        ) {
            return Ok(MembershipRestoreAuditReport {
                restored: None,
                audit: MembershipSnapshotAuditRecord {
                    world_id: world_id.to_string(),
                    requester_id: Some(snapshot.requester_id.clone()),
                    requested_at_ms: Some(snapshot.requested_at_ms),
                    signature_key_id: snapshot.signature_key_id.clone(),
                    outcome: MembershipSnapshotAuditOutcome::Rejected,
                    reason: world_error_reason(&err),
                },
            });
        }

        let request = ConsensusMembershipChangeRequest {
            requester_id: snapshot.requester_id.clone(),
            requested_at_ms: snapshot.requested_at_ms,
            reason: snapshot.reason.clone(),
            change: ConsensusMembershipChange::ReplaceValidators {
                validators: snapshot.validators.clone(),
                quorum_threshold: snapshot.quorum_threshold,
            },
        };

        match consensus.apply_membership_change(&request) {
            Ok(result) => {
                let outcome = if result.applied {
                    MembershipSnapshotAuditOutcome::Applied
                } else {
                    MembershipSnapshotAuditOutcome::Ignored
                };
                let reason = if result.applied {
                    "membership snapshot applied".to_string()
                } else {
                    "membership snapshot ignored (already in sync)".to_string()
                };
                Ok(MembershipRestoreAuditReport {
                    restored: Some(result),
                    audit: MembershipSnapshotAuditRecord {
                        world_id: world_id.to_string(),
                        requester_id: Some(snapshot.requester_id),
                        requested_at_ms: Some(snapshot.requested_at_ms),
                        signature_key_id: snapshot.signature_key_id,
                        outcome,
                        reason,
                    },
                })
            }
            Err(err) => Ok(MembershipRestoreAuditReport {
                restored: None,
                audit: MembershipSnapshotAuditRecord {
                    world_id: world_id.to_string(),
                    requester_id: Some(snapshot.requester_id),
                    requested_at_ms: Some(snapshot.requested_at_ms),
                    signature_key_id: snapshot.signature_key_id,
                    outcome: MembershipSnapshotAuditOutcome::Rejected,
                    reason: world_error_reason(&err),
                },
            }),
        }
    }

    pub fn restore_membership_from_dht_verified_with_audit_store(
        &self,
        world_id: &str,
        consensus: &mut QuorumConsensus,
        dht: &(dyn DistributedDht + Send + Sync),
        signer: Option<&MembershipDirectorySigner>,
        keyring: Option<&MembershipDirectorySignerKeyring>,
        policy: &MembershipSnapshotRestorePolicy,
        audit_store: &(dyn MembershipAuditStore + Send + Sync),
    ) -> Result<MembershipRestoreAuditReport, WorldError> {
        let report = self.restore_membership_from_dht_verified_with_audit(
            world_id, consensus, dht, signer, keyring, policy,
        )?;
        audit_store.append(&report.audit)?;
        Ok(report)
    }
}

fn restore_result_from_audit(
    report: MembershipRestoreAuditReport,
) -> Result<Option<ConsensusMembershipChangeResult>, WorldError> {
    match report.audit.outcome {
        MembershipSnapshotAuditOutcome::MissingSnapshot => Ok(None),
        MembershipSnapshotAuditOutcome::Applied | MembershipSnapshotAuditOutcome::Ignored => {
            Ok(report.restored)
        }
        MembershipSnapshotAuditOutcome::Rejected => Err(WorldError::DistributedValidationFailed {
            reason: report.audit.reason,
        }),
    }
}

fn checked_usize_add(lhs: usize, rhs: usize, context: &str) -> Result<usize, WorldError> {
    lhs.checked_add(rhs)
        .ok_or_else(|| WorldError::DistributedValidationFailed {
            reason: format!("{context} overflow: lhs={lhs}, rhs={rhs}"),
        })
}

fn checked_usize_increment(value: usize, context: &str) -> Result<usize, WorldError> {
    checked_usize_add(value, 1, context)
}

fn world_error_reason(error: &WorldError) -> String {
    match error {
        WorldError::DistributedValidationFailed { reason } => reason.clone(),
        _ => format!("{error:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_usize_add_rejects_overflow() {
        let err = checked_usize_add(usize::MAX, 1, "membership checked add")
            .expect_err("overflow should fail");
        match err {
            WorldError::DistributedValidationFailed { reason } => {
                assert!(
                    reason.contains("membership checked add overflow"),
                    "{reason}"
                );
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }
}
