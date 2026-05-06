use super::super::util::{hash_json, sha256_hex};
use super::super::{
    apply_manifest_patch, GovernanceEvent, GovernanceExecutionPolicy,
    GovernanceFinalityCertificate, GovernanceFinalityEpochSnapshot,
    GovernanceFinalitySignerRegistry, GovernanceIdentityPenaltyRecord,
    GovernanceIdentityPenaltyStatus, GovernanceIdentityProfileState, GovernanceIdentityStatus,
    GovernanceMainTokenControllerRegistry, GovernanceThresholdSignerPolicy, Manifest,
    ManifestPatch, ManifestUpdate, Proposal, ProposalDecision, ProposalId, ProposalStatus,
    WorldError, WorldEventBody, WorldTime, MAIN_TOKEN_TREASURY_BUCKET_ECOSYSTEM_POOL,
};
use super::World;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use std::collections::{BTreeMap, BTreeSet};
#[path = "governance_internal.rs"]
mod governance_internal;
#[path = "governance_validator_admission.rs"]
mod governance_validator_admission;

const LOCAL_GOVERNANCE_FINALITY_SIGNERS: [(&str, &str); 2] = [
    (
        "governance.local.finality.signer.1",
        "oasis7-governance-local-finality-signer-1-v1",
    ),
    (
        "governance.local.finality.signer.2",
        "oasis7-governance-local-finality-signer-2-v1",
    ),
];
const IDENTITY_PENALTY_DETECTION_SOURCE: &str = "world.threat_heatmap.v1";

pub(super) fn local_governance_finality_signer_public_keys() -> Vec<(String, String)> {
    let mut keys = Vec::with_capacity(LOCAL_GOVERNANCE_FINALITY_SIGNERS.len());
    for (node_id, seed_label) in LOCAL_GOVERNANCE_FINALITY_SIGNERS {
        let signing_key = local_governance_finality_signing_key(seed_label);
        keys.push((
            node_id.to_string(),
            hex::encode(signing_key.verifying_key().to_bytes()),
        ));
    }
    keys
}

fn local_governance_finality_signing_key(seed_label: &str) -> SigningKey {
    let seed = sha256_hex(seed_label.as_bytes());
    let seed_bytes = hex::decode(seed).expect("decode governance finality seed");
    let private_key_bytes: [u8; 32] = seed_bytes
        .as_slice()
        .try_into()
        .expect("governance finality seed is 32 bytes");
    SigningKey::from_bytes(&private_key_bytes)
}

fn decode_hex_array<const N: usize>(raw: &str, label: &str) -> Result<[u8; N], WorldError> {
    let bytes = hex::decode(raw).map_err(|_| WorldError::GovernanceFinalityInvalid {
        reason: format!("{label} is not valid hex"),
    })?;
    bytes
        .as_slice()
        .try_into()
        .map_err(|_| WorldError::GovernanceFinalityInvalid {
            reason: format!("{label} has invalid length"),
        })
}

pub(super) fn governance_finality_validator_set_hash(signer_node_ids: &[String]) -> String {
    sha256_hex(signer_node_ids.join("|").as_bytes())
}

pub(super) fn governance_finality_stake_root(signer_node_ids: &[String]) -> String {
    let payload = signer_node_ids
        .iter()
        .map(|node_id| format!("{node_id}:1"))
        .collect::<Vec<_>>()
        .join("|");
    sha256_hex(payload.as_bytes())
}

fn governance_finality_signed_stake_bps(total_signers: usize, signed_signers: usize) -> u16 {
    if total_signers == 0 {
        return 0;
    }
    let signed = u128::from(signed_signers as u64)
        .saturating_mul(10_000)
        .saturating_div(u128::from(total_signers as u64));
    signed.min(10_000) as u16
}

fn governance_threshold_bps(required_signers: u16, total_signers: usize) -> u16 {
    if required_signers == 0 || total_signers == 0 {
        return 0;
    }
    let total_signers = total_signers as u128;
    let required_signers = u128::from(required_signers);
    required_signers
        .saturating_mul(10_000)
        .saturating_add(total_signers.saturating_sub(1))
        .saturating_div(total_signers)
        .min(10_000) as u16
}

impl World {
    pub(super) fn treasury_bucket_controller_account_id<'a>(
        registry: &'a GovernanceMainTokenControllerRegistry,
        bucket_id: &str,
        context_label: &str,
    ) -> Result<&'a str, WorldError> {
        registry
            .treasury_bucket_controller_slots
            .get(bucket_id)
            .map(String::as_str)
            .ok_or_else(|| WorldError::GovernancePolicyInvalid {
                reason: format!(
                    "{context_label} controller slot is not configured for bucket {bucket_id}",
                ),
            })
    }

    pub(super) fn ecosystem_treasury_controller_account_id<'a>(
        registry: &'a GovernanceMainTokenControllerRegistry,
        context_label: &str,
    ) -> Result<&'a str, WorldError> {
        Self::treasury_bucket_controller_account_id(
            registry,
            MAIN_TOKEN_TREASURY_BUCKET_ECOSYSTEM_POOL,
            context_label,
        )
    }

    pub(super) fn restricted_starter_claim_admin_registry_controller_account_id<'a>(
        registry: &'a GovernanceMainTokenControllerRegistry,
    ) -> Result<&'a str, WorldError> {
        Self::ecosystem_treasury_controller_account_id(registry, "restricted claim admin registry")
    }

    pub(super) fn validator_admission_controller_account_id<'a>(
        registry: &'a GovernanceMainTokenControllerRegistry,
    ) -> Result<&'a str, WorldError> {
        let controller_account_id = registry.genesis_controller_account_id.trim();
        if controller_account_id.is_empty() {
            return Err(WorldError::GovernancePolicyInvalid {
                reason: "validator admission controller account_id cannot be empty".to_string(),
            });
        }
        Ok(controller_account_id)
    }

    // ---------------------------------------------------------------------
    // Manifest and governance
    // ---------------------------------------------------------------------

    pub fn current_manifest_hash(&self) -> Result<String, WorldError> {
        hash_json(&self.manifest)
    }

    pub fn governance_identity_profile(
        &self,
        agent_id: &str,
    ) -> Option<&GovernanceIdentityProfileState> {
        self.state.governance_identity_profiles.get(agent_id)
    }

    pub fn set_agent_reputation_score(
        &mut self,
        agent_id: &str,
        reputation_score: i64,
    ) -> Result<(), WorldError> {
        self.state.set_reputation_score(agent_id, reputation_score)
    }

    pub fn set_governance_identity_profile(
        &mut self,
        agent_id: impl Into<String>,
        stake_locked: u64,
        warmup_until_tick: WorldTime,
        status: GovernanceIdentityStatus,
    ) -> Result<(), WorldError> {
        let agent_id = agent_id.into();
        let slash_count = self
            .state
            .governance_identity_profiles
            .get(agent_id.as_str())
            .map(|profile| profile.slash_count)
            .unwrap_or(0);
        self.state
            .set_governance_identity_profile(GovernanceIdentityProfileState {
                agent_id,
                stake_locked,
                warmup_until_tick,
                status,
                slash_count,
                updated_at: self.state.time,
            })
    }

    pub fn set_governance_execution_policy(
        &mut self,
        policy: GovernanceExecutionPolicy,
    ) -> Result<(), WorldError> {
        Self::validate_governance_execution_policy(&policy)?;
        self.governance_execution_policy = policy;
        Ok(())
    }

    pub fn set_governance_finality_epoch_snapshot(
        &mut self,
        mut snapshot: GovernanceFinalityEpochSnapshot,
    ) -> Result<(), WorldError> {
        self.normalize_governance_finality_epoch_snapshot(&mut snapshot)?;
        self.governance_finality_epoch_snapshots
            .insert(snapshot.epoch_id, snapshot);
        Ok(())
    }

    pub fn remove_governance_finality_epoch_snapshot(&mut self, epoch_id: u64) -> bool {
        self.governance_finality_epoch_snapshots
            .remove(&epoch_id)
            .is_some()
    }

    pub fn set_governance_finality_signer_registry(
        &mut self,
        registry: GovernanceFinalitySignerRegistry,
    ) -> Result<(), WorldError> {
        let registry = self.validate_governance_finality_signer_registry(registry)?;
        for (node_id, public_key_hex) in &registry.signer_bindings {
            match self.node_identity_public_key(node_id.as_str()) {
                Some(bound_public_key_hex) if bound_public_key_hex != public_key_hex => {
                    return Err(WorldError::GovernancePolicyInvalid {
                        reason: format!(
                            "finality signer binding conflicts with existing node identity slot_id={} node_id={}",
                            registry.slot_id, node_id
                        ),
                    });
                }
                Some(_) => {}
                None => self.bind_node_identity(node_id.as_str(), public_key_hex.as_str())?,
            }
        }
        self.state.governance_finality_signer_registry = Some(registry);
        Ok(())
    }

    pub fn set_governance_main_token_controller_registry(
        &mut self,
        registry: GovernanceMainTokenControllerRegistry,
    ) -> Result<(), WorldError> {
        let registry = Self::validate_governance_main_token_controller_registry(registry)?;
        self.state.governance_main_token_controller_registry = Some(registry);
        Ok(())
    }

    pub fn resolve_governance_effective_finality_signer_registry(
        &self,
    ) -> Result<Option<GovernanceFinalitySignerRegistry>, WorldError> {
        self.resolve_governance_effective_finality_signer_registry_from_admissions(
            &self.state.governance_validator_admissions,
        )
    }

    pub fn governance_effective_finality_epoch_snapshot(
        &self,
        epoch_id: u64,
    ) -> GovernanceFinalityEpochSnapshot {
        self.governance_finality_epoch_snapshot_for_epoch(epoch_id)
    }

    pub fn activate_emergency_brake(
        &mut self,
        initiator: impl Into<String>,
        reason: impl Into<String>,
        duration_ticks: u64,
        signer_node_ids: Vec<String>,
    ) -> Result<(), WorldError> {
        let threshold = self
            .governance_execution_policy
            .emergency_brake_guardian_threshold;
        let signer_node_ids = self.validate_guardian_signers(&signer_node_ids, threshold)?;
        if duration_ticks == 0 {
            return Err(WorldError::GovernancePolicyInvalid {
                reason: "emergency brake duration must be > 0".to_string(),
            });
        }
        if duration_ticks > self.governance_execution_policy.emergency_brake_max_ticks {
            return Err(WorldError::GovernancePolicyInvalid {
                reason: format!(
                    "emergency brake duration exceeds max: duration_ticks={} max={}",
                    duration_ticks, self.governance_execution_policy.emergency_brake_max_ticks
                ),
            });
        }
        let active_until_tick = self.state.time.saturating_add(duration_ticks);
        let event = GovernanceEvent::EmergencyBrakeActivated {
            initiator: initiator.into(),
            reason: reason.into(),
            active_until_tick,
            threshold,
            signer_node_ids,
        };
        self.append_event(WorldEventBody::Governance(event), None)?;
        Ok(())
    }

    pub fn release_emergency_brake(
        &mut self,
        initiator: impl Into<String>,
        reason: impl Into<String>,
        signer_node_ids: Vec<String>,
    ) -> Result<(), WorldError> {
        let threshold = self
            .governance_execution_policy
            .emergency_brake_guardian_threshold;
        let signer_node_ids = self.validate_guardian_signers(&signer_node_ids, threshold)?;
        if !self.is_governance_emergency_brake_active() {
            return Err(WorldError::GovernancePolicyInvalid {
                reason: "emergency brake is not active".to_string(),
            });
        }
        let event = GovernanceEvent::EmergencyBrakeReleased {
            initiator: initiator.into(),
            reason: reason.into(),
            threshold,
            signer_node_ids,
        };
        self.append_event(WorldEventBody::Governance(event), None)?;
        Ok(())
    }

    pub fn emergency_veto_proposal(
        &mut self,
        proposal_id: ProposalId,
        initiator: impl Into<String>,
        reason: impl Into<String>,
        signer_node_ids: Vec<String>,
    ) -> Result<(), WorldError> {
        let proposal = self
            .proposals
            .get(&proposal_id)
            .ok_or(WorldError::ProposalNotFound { proposal_id })?;
        if !matches!(proposal.status, ProposalStatus::Approved { .. }) {
            return Err(WorldError::ProposalInvalidState {
                proposal_id,
                expected: "approved".to_string(),
                found: proposal.status.label(),
            });
        }
        if proposal.not_before_tick.is_none() || proposal.activate_epoch.is_none() {
            return Err(WorldError::GovernancePolicyInvalid {
                reason: format!("proposal_id={} is not queued for activation", proposal_id),
            });
        }
        let threshold = self
            .governance_execution_policy
            .emergency_veto_guardian_threshold;
        let signer_node_ids = self.validate_guardian_signers(&signer_node_ids, threshold)?;
        let event = GovernanceEvent::EmergencyVetoed {
            proposal_id,
            initiator: initiator.into(),
            reason: reason.into(),
            threshold,
            signer_node_ids,
        };
        self.append_event(WorldEventBody::Governance(event), None)?;
        Ok(())
    }

    pub fn propose_manifest_update(
        &mut self,
        manifest: Manifest,
        author: impl Into<String>,
    ) -> Result<ProposalId, WorldError> {
        let proposal_id = self.allocate_next_proposal_id();
        let base_manifest_hash = self.current_manifest_hash()?;
        let event = GovernanceEvent::Proposed {
            proposal_id,
            author: author.into(),
            base_manifest_hash,
            manifest,
            patch: None,
        };
        self.append_event(WorldEventBody::Governance(event), None)?;
        Ok(proposal_id)
    }

    pub fn propose_manifest_patch(
        &mut self,
        patch: ManifestPatch,
        author: impl Into<String>,
    ) -> Result<ProposalId, WorldError> {
        let base_manifest_hash = self.current_manifest_hash()?;
        if patch.base_manifest_hash != base_manifest_hash {
            return Err(WorldError::PatchBaseMismatch {
                expected: base_manifest_hash,
                found: patch.base_manifest_hash,
            });
        }

        let manifest = apply_manifest_patch(&self.manifest, &patch)?;
        let proposal_id = self.allocate_next_proposal_id();
        let event = GovernanceEvent::Proposed {
            proposal_id,
            author: author.into(),
            base_manifest_hash,
            manifest,
            patch: Some(patch),
        };
        self.append_event(WorldEventBody::Governance(event), None)?;
        Ok(proposal_id)
    }

    pub fn shadow_proposal(&mut self, proposal_id: ProposalId) -> Result<String, WorldError> {
        let proposal = self
            .proposals
            .get(&proposal_id)
            .ok_or(WorldError::ProposalNotFound { proposal_id })?;
        if !matches!(proposal.status, ProposalStatus::Proposed) {
            return Err(WorldError::ProposalInvalidState {
                proposal_id,
                expected: "proposed".to_string(),
                found: proposal.status.label(),
            });
        }
        if let Some(changes) = proposal.manifest.module_changes()? {
            self.validate_module_changes(&changes)?;
            self.shadow_validate_module_changes(&changes)?;
        }
        let manifest_hash = hash_json(&proposal.manifest)?;
        let event = GovernanceEvent::ShadowReport {
            proposal_id,
            manifest_hash: manifest_hash.clone(),
        };
        self.append_event(WorldEventBody::Governance(event), None)?;
        Ok(manifest_hash)
    }

    pub fn approve_proposal(
        &mut self,
        proposal_id: ProposalId,
        approver: impl Into<String>,
        decision: ProposalDecision,
    ) -> Result<(), WorldError> {
        let mut queued_manifest_hash: Option<String> = None;
        let proposal = self
            .proposals
            .get(&proposal_id)
            .ok_or(WorldError::ProposalNotFound { proposal_id })?;

        match (&decision, &proposal.status) {
            (ProposalDecision::Approve, ProposalStatus::Shadowed { manifest_hash }) => {
                queued_manifest_hash = Some(manifest_hash.clone());
            }
            (ProposalDecision::Reject { .. }, ProposalStatus::Applied { .. })
            | (ProposalDecision::Reject { .. }, ProposalStatus::Rejected { .. }) => {
                return Err(WorldError::ProposalInvalidState {
                    proposal_id,
                    expected: "proposed".to_string(),
                    found: proposal.status.label(),
                });
            }
            (ProposalDecision::Approve, _) => {
                return Err(WorldError::ProposalInvalidState {
                    proposal_id,
                    expected: "shadowed".to_string(),
                    found: proposal.status.label(),
                });
            }
            _ => {}
        }

        let event = GovernanceEvent::Approved {
            proposal_id,
            approver: approver.into(),
            decision,
        };
        self.append_event(WorldEventBody::Governance(event), None)?;
        if let Some(manifest_hash) = queued_manifest_hash {
            let queued_at_tick = self.state.time;
            let timelock_ticks = self.governance_execution_policy.timelock_ticks;
            let not_before_tick = queued_at_tick.saturating_add(timelock_ticks);
            let activate_epoch = self
                .current_governance_epoch()
                .saturating_add(self.governance_execution_policy.activation_delay_epochs);
            self.append_event(
                WorldEventBody::Governance(GovernanceEvent::Queued {
                    proposal_id,
                    manifest_hash,
                    queued_at_tick,
                    not_before_tick,
                    activate_epoch,
                    timelock_ticks,
                }),
                None,
            )?;
        }
        Ok(())
    }

    pub fn build_local_finality_certificate(
        &self,
        proposal_id: ProposalId,
    ) -> Result<GovernanceFinalityCertificate, WorldError> {
        if self.governance_finality_signer_registry().is_some() {
            return Err(WorldError::GovernancePolicyInvalid {
                reason: "local finality certificate builder is unavailable when governance finality signer registry is configured".to_string(),
            });
        }
        let proposal = self
            .proposals
            .get(&proposal_id)
            .ok_or(WorldError::ProposalNotFound { proposal_id })?;
        let manifest_hash = match &proposal.status {
            ProposalStatus::Approved { manifest_hash, .. } => manifest_hash.clone(),
            other => {
                return Err(WorldError::ProposalInvalidState {
                    proposal_id,
                    expected: "approved".to_string(),
                    found: other.label(),
                })
            }
        };
        let consensus_height = self.journal.events.len() as u64 + 1;
        let epoch_id = self.current_governance_epoch();
        let snapshot = self.governance_finality_epoch_snapshot_for_epoch(epoch_id);
        let min_unique_signers = snapshot.effective_min_unique_signers();
        let mut signatures = BTreeMap::new();
        for (node_id, seed_label) in LOCAL_GOVERNANCE_FINALITY_SIGNERS {
            let payload = GovernanceFinalityCertificate::signing_payload_v1(
                proposal_id,
                manifest_hash.as_str(),
                consensus_height,
                epoch_id,
                snapshot.validator_set_hash.as_str(),
                snapshot.stake_root.as_str(),
                snapshot.threshold_bps,
                min_unique_signers,
                node_id,
            );
            let signing_key = local_governance_finality_signing_key(seed_label);
            let signature = signing_key.sign(payload.as_slice());
            signatures.insert(
                node_id.to_string(),
                format!(
                    "{}{}",
                    GovernanceFinalityCertificate::SIGNATURE_PREFIX_ED25519_V1,
                    hex::encode(signature.to_bytes())
                ),
            );
        }
        Ok(GovernanceFinalityCertificate {
            proposal_id,
            manifest_hash,
            consensus_height,
            epoch_id,
            validator_set_hash: snapshot.validator_set_hash,
            stake_root: snapshot.stake_root,
            threshold_bps: snapshot.threshold_bps,
            min_unique_signers,
            threshold: min_unique_signers,
            signatures,
        })
    }

    pub fn apply_proposal(&mut self, proposal_id: ProposalId) -> Result<String, WorldError> {
        if !self.release_security_policy.allow_local_finality_signing {
            return Err(WorldError::GovernancePolicyInvalid {
                reason: format!(
                    "apply_proposal local finality path is disabled by release policy proposal_id={proposal_id}"
                ),
            });
        }
        let finality_certificate = self.build_local_finality_certificate(proposal_id)?;
        self.apply_proposal_with_finality(proposal_id, &finality_certificate)
    }

    pub fn apply_proposal_with_finality(
        &mut self,
        proposal_id: ProposalId,
        finality_certificate: &GovernanceFinalityCertificate,
    ) -> Result<String, WorldError> {
        let proposal = self
            .proposals
            .get(&proposal_id)
            .ok_or(WorldError::ProposalNotFound { proposal_id })?;
        let (manifest, actor, approved_manifest_hash) = match &proposal.status {
            ProposalStatus::Approved { manifest_hash, .. } => (
                proposal.manifest.clone(),
                proposal.author.clone(),
                manifest_hash.clone(),
            ),
            other => {
                return Err(WorldError::ProposalInvalidState {
                    proposal_id,
                    expected: "approved".to_string(),
                    found: other.label(),
                })
            }
        };
        if self.is_governance_emergency_brake_active() {
            return Err(WorldError::GovernancePolicyInvalid {
                reason: format!(
                    "governance apply blocked by emergency brake until_tick={}",
                    self.governance_emergency_brake_until_tick
                        .unwrap_or(self.state.time)
                ),
            });
        }
        if let Some(not_before_tick) = proposal.not_before_tick {
            if self.state.time < not_before_tick {
                return Err(WorldError::GovernancePolicyInvalid {
                    reason: format!(
                        "proposal_id={} timelock pending current_tick={} not_before_tick={}",
                        proposal_id, self.state.time, not_before_tick
                    ),
                });
            }
        }
        if let Some(activate_epoch) = proposal.activate_epoch {
            let current_epoch = self.current_governance_epoch();
            if current_epoch < activate_epoch {
                return Err(WorldError::GovernancePolicyInvalid {
                    reason: format!(
                        "proposal_id={} activation epoch pending current_epoch={} activate_epoch={}",
                        proposal_id, current_epoch, activate_epoch
                    ),
                });
            }
        }

        let module_changes = manifest.module_changes()?;
        if let Some(changes) = &module_changes {
            self.validate_module_changes(changes)?;
        }
        let applied_manifest = if module_changes.is_some() {
            manifest.without_module_changes()?
        } else {
            manifest.clone()
        };
        let proposal_manifest_hash = hash_json(&manifest)?;
        if proposal_manifest_hash != approved_manifest_hash {
            return Err(WorldError::GovernanceFinalityInvalid {
                reason: "approved manifest hash drift".to_string(),
            });
        }
        let applied_hash = hash_json(&applied_manifest)?;
        let finality_epoch_id = self.current_governance_epoch();
        self.validate_governance_finality_certificate(
            proposal_id,
            approved_manifest_hash.as_str(),
            finality_epoch_id,
            finality_certificate,
        )?;

        if let Some(changes) = module_changes {
            self.apply_module_changes(proposal_id, &changes, &actor)?;
        }
        let update = ManifestUpdate {
            manifest: applied_manifest,
            manifest_hash: applied_hash.clone(),
        };
        self.append_event(WorldEventBody::ManifestUpdated(update), None)?;
        let event = GovernanceEvent::Applied {
            proposal_id,
            manifest_hash: Some(applied_hash.clone()),
            consensus_height: Some(finality_certificate.consensus_height),
            threshold: Some(finality_certificate.effective_min_unique_signers()),
            signer_node_ids: finality_certificate.signatures.keys().cloned().collect(),
        };
        self.append_event(WorldEventBody::Governance(event), None)?;
        Ok(applied_hash)
    }

    pub(super) fn allocate_next_governance_identity_penalty_id(&mut self) -> u64 {
        let id = self.next_governance_identity_penalty_id;
        self.next_governance_identity_penalty_id =
            self.next_governance_identity_penalty_id.saturating_add(1);
        id
    }

    pub(super) fn validate_governance_execution_policy(
        policy: &GovernanceExecutionPolicy,
    ) -> Result<(), WorldError> {
        if policy.epoch_length_ticks == 0 {
            return Err(WorldError::GovernancePolicyInvalid {
                reason: "epoch_length_ticks must be > 0".to_string(),
            });
        }
        if policy.emergency_brake_guardian_threshold == 0 {
            return Err(WorldError::GovernancePolicyInvalid {
                reason: "emergency_brake_guardian_threshold must be > 0".to_string(),
            });
        }
        if policy.emergency_veto_guardian_threshold == 0 {
            return Err(WorldError::GovernancePolicyInvalid {
                reason: "emergency_veto_guardian_threshold must be > 0".to_string(),
            });
        }
        if policy.emergency_brake_max_ticks == 0 {
            return Err(WorldError::GovernancePolicyInvalid {
                reason: "emergency_brake_max_ticks must be > 0".to_string(),
            });
        }
        Ok(())
    }

    pub(super) fn governance_finality_epoch_snapshot_for_epoch(
        &self,
        epoch_id: u64,
    ) -> GovernanceFinalityEpochSnapshot {
        if let Some(snapshot) = self.governance_finality_epoch_snapshots.get(&epoch_id) {
            return snapshot.clone();
        }
        if let Some(snapshot) = self.governance_finality_registry_epoch_snapshot(epoch_id) {
            return snapshot;
        }
        let signer_node_ids: Vec<String> = LOCAL_GOVERNANCE_FINALITY_SIGNERS
            .iter()
            .map(|(node_id, _)| (*node_id).to_string())
            .collect();
        let min_unique_signers = LOCAL_GOVERNANCE_FINALITY_SIGNERS.len() as u16;
        GovernanceFinalityEpochSnapshot {
            epoch_id,
            threshold_bps: 10_000,
            min_unique_signers,
            validator_set_hash: governance_finality_validator_set_hash(signer_node_ids.as_slice()),
            stake_root: governance_finality_stake_root(signer_node_ids.as_slice()),
            threshold: min_unique_signers,
            signer_node_ids,
        }
    }
}
