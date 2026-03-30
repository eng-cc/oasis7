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
    pub(super) fn restricted_starter_claim_admin_registry_controller_account_id<'a>(
        registry: &'a GovernanceMainTokenControllerRegistry,
    ) -> Result<&'a str, WorldError> {
        registry
            .treasury_bucket_controller_slots
            .get(MAIN_TOKEN_TREASURY_BUCKET_ECOSYSTEM_POOL)
            .map(String::as_str)
            .ok_or_else(|| WorldError::GovernancePolicyInvalid {
                reason: format!(
                    "restricted claim admin registry controller slot is not configured for bucket {}",
                    MAIN_TOKEN_TREASURY_BUCKET_ECOSYSTEM_POOL
                ),
            })
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

    fn governance_finality_registry_epoch_snapshot(
        &self,
        epoch_id: u64,
    ) -> Option<GovernanceFinalityEpochSnapshot> {
        let registry = self.state.governance_finality_signer_registry.as_ref()?;
        let signer_node_ids: Vec<String> = registry.signer_bindings.keys().cloned().collect();
        let threshold = registry.threshold;
        let threshold_bps = if registry.threshold_bps > 0 {
            registry.threshold_bps
        } else {
            governance_threshold_bps(threshold, signer_node_ids.len())
        };
        Some(GovernanceFinalityEpochSnapshot {
            epoch_id,
            threshold_bps,
            min_unique_signers: threshold,
            validator_set_hash: governance_finality_validator_set_hash(signer_node_ids.as_slice()),
            stake_root: governance_finality_stake_root(signer_node_ids.as_slice()),
            threshold,
            signer_node_ids,
        })
    }

    fn normalize_governance_finality_epoch_snapshot(
        &self,
        snapshot: &mut GovernanceFinalityEpochSnapshot,
    ) -> Result<(), WorldError> {
        let mut unique = BTreeSet::new();
        for node_id in snapshot.signer_node_ids.iter().map(|id| id.trim()) {
            if node_id.is_empty() {
                return Err(WorldError::GovernancePolicyInvalid {
                    reason: format!(
                        "finality epoch snapshot signer node_id is empty epoch_id={}",
                        snapshot.epoch_id
                    ),
                });
            }
            if self.node_identity_public_key(node_id).is_none() {
                return Err(WorldError::GovernancePolicyInvalid {
                    reason: format!(
                        "finality epoch snapshot signer is untrusted epoch_id={} node_id={}",
                        snapshot.epoch_id, node_id
                    ),
                });
            }
            unique.insert(node_id.to_string());
        }
        let unique_signers: Vec<String> = unique.into_iter().collect();
        let min_unique_signers = if snapshot.min_unique_signers > 0 {
            snapshot.min_unique_signers
        } else {
            snapshot.threshold
        };
        if min_unique_signers == 0 {
            return Err(WorldError::GovernancePolicyInvalid {
                reason: format!(
                    "finality epoch snapshot min_unique_signers must be > 0 epoch_id={}",
                    snapshot.epoch_id
                ),
            });
        }
        if unique_signers.len() < min_unique_signers as usize {
            return Err(WorldError::GovernancePolicyInvalid {
                reason: format!(
                    "finality epoch snapshot signatures below threshold epoch_id={} signers={} min_unique_signers={}",
                    snapshot.epoch_id,
                    unique_signers.len(),
                    min_unique_signers
                ),
            });
        }
        let threshold_bps = if snapshot.threshold_bps > 0 {
            snapshot.threshold_bps
        } else {
            governance_threshold_bps(min_unique_signers, unique_signers.len())
        };
        if threshold_bps == 0 || threshold_bps > 10_000 {
            return Err(WorldError::GovernancePolicyInvalid {
                reason: format!(
                    "finality epoch snapshot threshold_bps must be within 1..=10000 epoch_id={} threshold_bps={}",
                    snapshot.epoch_id, threshold_bps
                ),
            });
        }
        let computed_validator_set_hash =
            governance_finality_validator_set_hash(unique_signers.as_slice());
        let validator_set_hash = if snapshot.validator_set_hash.trim().is_empty() {
            computed_validator_set_hash
        } else {
            let normalized = snapshot.validator_set_hash.trim().to_string();
            if normalized != computed_validator_set_hash {
                return Err(WorldError::GovernancePolicyInvalid {
                    reason: format!(
                        "finality epoch snapshot validator_set_hash mismatch epoch_id={} expected={} found={}",
                        snapshot.epoch_id, computed_validator_set_hash, normalized
                    ),
                });
            }
            normalized
        };
        let computed_stake_root = governance_finality_stake_root(unique_signers.as_slice());
        let stake_root = if snapshot.stake_root.trim().is_empty() {
            computed_stake_root
        } else {
            let normalized = snapshot.stake_root.trim().to_string();
            if normalized != computed_stake_root {
                return Err(WorldError::GovernancePolicyInvalid {
                    reason: format!(
                        "finality epoch snapshot stake_root mismatch epoch_id={} expected={} found={}",
                        snapshot.epoch_id, computed_stake_root, normalized
                    ),
                });
            }
            normalized
        };
        snapshot.threshold = min_unique_signers;
        snapshot.min_unique_signers = min_unique_signers;
        snapshot.threshold_bps = threshold_bps;
        snapshot.validator_set_hash = validator_set_hash;
        snapshot.stake_root = stake_root;
        snapshot.signer_node_ids = unique_signers;
        Ok(())
    }

    fn validate_governance_finality_signer_registry(
        &self,
        mut registry: GovernanceFinalitySignerRegistry,
    ) -> Result<GovernanceFinalitySignerRegistry, WorldError> {
        registry.slot_id = registry.slot_id.trim().to_string();
        if registry.slot_id.is_empty() {
            return Err(WorldError::GovernancePolicyInvalid {
                reason: "finality signer registry slot_id cannot be empty".to_string(),
            });
        }
        if registry.threshold == 0 {
            return Err(WorldError::GovernancePolicyInvalid {
                reason: format!(
                    "finality signer registry threshold must be > 0 slot_id={}",
                    registry.slot_id
                ),
            });
        }
        let mut normalized_bindings = BTreeMap::new();
        let mut unique_public_keys = BTreeSet::new();
        for (node_id, public_key_hex) in registry.signer_bindings {
            let node_id = node_id.trim().to_string();
            if node_id.is_empty() {
                return Err(WorldError::GovernancePolicyInvalid {
                    reason: format!(
                        "finality signer registry node_id cannot be empty slot_id={}",
                        registry.slot_id
                    ),
                });
            }
            let public_key_hex = public_key_hex.trim().to_string();
            decode_hex_array::<32>(
                public_key_hex.as_str(),
                format!(
                    "finality signer public key slot_id={} node_id={node_id}",
                    registry.slot_id
                )
                .as_str(),
            )?;
            if !unique_public_keys.insert(public_key_hex.clone()) {
                return Err(WorldError::GovernancePolicyInvalid {
                    reason: format!(
                        "finality signer registry public key must be unique slot_id={} public_key={}",
                        registry.slot_id, public_key_hex
                    ),
                });
            }
            normalized_bindings.insert(node_id, public_key_hex);
        }
        if normalized_bindings.len() < usize::from(registry.threshold) {
            return Err(WorldError::GovernancePolicyInvalid {
                reason: format!(
                    "finality signer registry threshold exceeds signer count slot_id={} threshold={} signers={}",
                    registry.slot_id,
                    registry.threshold,
                    normalized_bindings.len()
                ),
            });
        }
        if registry.threshold_bps == 0 {
            registry.threshold_bps =
                governance_threshold_bps(registry.threshold, normalized_bindings.len());
        }
        if registry.threshold_bps == 0 || registry.threshold_bps > 10_000 {
            return Err(WorldError::GovernancePolicyInvalid {
                reason: format!(
                    "finality signer registry threshold_bps must be within 1..=10000 slot_id={} threshold_bps={}",
                    registry.slot_id, registry.threshold_bps
                ),
            });
        }
        registry.signer_bindings = normalized_bindings;
        Ok(registry)
    }

    fn validate_governance_threshold_signer_policy(
        account_id: &str,
        mut policy: GovernanceThresholdSignerPolicy,
    ) -> Result<GovernanceThresholdSignerPolicy, WorldError> {
        if policy.threshold == 0 {
            return Err(WorldError::GovernancePolicyInvalid {
                reason: format!(
                    "controller signer policy threshold must be > 0 controller_account_id={account_id}"
                ),
            });
        }
        let mut normalized_public_keys = BTreeSet::new();
        for public_key_hex in policy.allowed_public_keys {
            let public_key_hex = public_key_hex.trim().to_string();
            decode_hex_array::<32>(
                public_key_hex.as_str(),
                format!("controller signer policy public key controller_account_id={account_id}")
                    .as_str(),
            )?;
            normalized_public_keys.insert(public_key_hex);
        }
        if normalized_public_keys.len() < usize::from(policy.threshold) {
            return Err(WorldError::GovernancePolicyInvalid {
                reason: format!(
                    "controller signer policy threshold exceeds signer count controller_account_id={account_id} threshold={} signers={}",
                    policy.threshold,
                    normalized_public_keys.len()
                ),
            });
        }
        policy.allowed_public_keys = normalized_public_keys;
        Ok(policy)
    }

    pub(super) fn validate_governance_main_token_controller_registry(
        mut registry: GovernanceMainTokenControllerRegistry,
    ) -> Result<GovernanceMainTokenControllerRegistry, WorldError> {
        registry.genesis_controller_account_id =
            registry.genesis_controller_account_id.trim().to_string();
        if registry.genesis_controller_account_id.is_empty() {
            return Err(WorldError::GovernancePolicyInvalid {
                reason:
                    "main token controller registry genesis_controller_account_id cannot be empty"
                        .to_string(),
            });
        }
        let mut normalized_policies = BTreeMap::new();
        for (account_id, policy) in registry.controller_signer_policies {
            let account_id = account_id.trim().to_string();
            if account_id.is_empty() {
                return Err(WorldError::GovernancePolicyInvalid {
                    reason: "main token controller registry account_id cannot be empty".to_string(),
                });
            }
            normalized_policies.insert(
                account_id.clone(),
                Self::validate_governance_threshold_signer_policy(account_id.as_str(), policy)?,
            );
        }
        if !normalized_policies.contains_key(registry.genesis_controller_account_id.as_str()) {
            return Err(WorldError::GovernancePolicyInvalid {
                reason: format!(
                    "main token controller registry missing genesis controller policy account_id={}",
                    registry.genesis_controller_account_id
                ),
            });
        }
        let mut normalized_slots = BTreeMap::new();
        for (bucket_id, controller_account_id) in registry.treasury_bucket_controller_slots {
            let bucket_id = bucket_id.trim().to_string();
            let controller_account_id = controller_account_id.trim().to_string();
            if bucket_id.is_empty() || controller_account_id.is_empty() {
                return Err(WorldError::GovernancePolicyInvalid {
                    reason:
                        "main token controller registry treasury bucket binding cannot be empty"
                            .to_string(),
                });
            }
            if !normalized_policies.contains_key(controller_account_id.as_str()) {
                return Err(WorldError::GovernancePolicyInvalid {
                    reason: format!(
                        "main token controller registry missing treasury controller policy bucket_id={} controller_account_id={}",
                        bucket_id, controller_account_id
                    ),
                });
            }
            normalized_slots.insert(bucket_id, controller_account_id);
        }
        let mut normalized_restricted_grant_admins = BTreeSet::new();
        for account_id in registry.restricted_starter_claim_admin_account_ids {
            let account_id = account_id.trim().to_string();
            if account_id.is_empty() {
                return Err(WorldError::GovernancePolicyInvalid {
                    reason:
                        "main token controller registry restricted grant admin account_id cannot be empty"
                            .to_string(),
                });
            }
            if !normalized_policies.contains_key(account_id.as_str()) {
                return Err(WorldError::GovernancePolicyInvalid {
                    reason: format!(
                        "main token controller registry missing restricted grant admin signer policy account_id={account_id}",
                    ),
                });
            }
            normalized_restricted_grant_admins.insert(account_id);
        }
        registry.controller_signer_policies = normalized_policies;
        registry.treasury_bucket_controller_slots = normalized_slots;
        registry.restricted_starter_claim_admin_account_ids = normalized_restricted_grant_admins;
        Ok(registry)
    }

    pub(super) fn current_governance_epoch(&self) -> u64 {
        self.governance_epoch_for_time(self.state.time)
    }

    fn governance_epoch_for_time(&self, time: u64) -> u64 {
        let epoch_len = self.governance_execution_policy.epoch_length_ticks.max(1);
        time / epoch_len
    }

    fn is_governance_emergency_brake_active(&self) -> bool {
        self.governance_emergency_brake_until_tick
            .is_some_and(|until| self.state.time < until)
    }

    pub(super) fn validate_guardian_signers(
        &self,
        signer_node_ids: &[String],
        threshold: u16,
    ) -> Result<Vec<String>, WorldError> {
        if threshold == 0 {
            return Err(WorldError::GovernancePolicyInvalid {
                reason: "guardian threshold must be > 0".to_string(),
            });
        }
        let mut unique = BTreeSet::new();
        for node_id in signer_node_ids {
            if self.node_identity_public_key(node_id.as_str()).is_none() {
                return Err(WorldError::GovernancePolicyInvalid {
                    reason: format!("untrusted guardian signer node_id={node_id}"),
                });
            }
            unique.insert(node_id.clone());
        }
        if unique.len() < threshold as usize {
            return Err(WorldError::GovernancePolicyInvalid {
                reason: format!(
                    "guardian signatures below threshold: signers={} threshold={threshold}",
                    unique.len()
                ),
            });
        }
        Ok(unique.into_iter().collect())
    }

    fn validate_governance_finality_certificate(
        &self,
        proposal_id: ProposalId,
        manifest_hash: &str,
        epoch_id: u64,
        certificate: &GovernanceFinalityCertificate,
    ) -> Result<(), WorldError> {
        let snapshot = self.governance_finality_epoch_snapshot_for_epoch(epoch_id);
        let snapshot_min_unique_signers = snapshot.effective_min_unique_signers();
        let epoch_signers: BTreeSet<String> = snapshot.signer_node_ids.iter().cloned().collect();
        if certificate.proposal_id != proposal_id {
            return Err(WorldError::GovernanceFinalityInvalid {
                reason: format!(
                    "proposal_id mismatch: expected={} found={}",
                    proposal_id, certificate.proposal_id
                ),
            });
        }
        if certificate.manifest_hash != manifest_hash {
            return Err(WorldError::GovernanceFinalityInvalid {
                reason: "manifest_hash mismatch".to_string(),
            });
        }
        if certificate.consensus_height == 0 {
            return Err(WorldError::GovernanceFinalityInvalid {
                reason: "consensus_height must be > 0".to_string(),
            });
        }
        if certificate.epoch_id != epoch_id {
            return Err(WorldError::GovernanceFinalityInvalid {
                reason: format!(
                    "epoch_id mismatch: expected={} found={}",
                    epoch_id, certificate.epoch_id
                ),
            });
        }
        if certificate.validator_set_hash.trim().is_empty() {
            return Err(WorldError::GovernanceFinalityInvalid {
                reason: "validator_set_hash must be non-empty".to_string(),
            });
        }
        if certificate.validator_set_hash != snapshot.validator_set_hash {
            return Err(WorldError::GovernanceFinalityInvalid {
                reason: format!(
                    "validator_set_hash mismatch: epoch_id={} certificate={} snapshot={}",
                    epoch_id, certificate.validator_set_hash, snapshot.validator_set_hash
                ),
            });
        }
        if certificate.stake_root.trim().is_empty() {
            return Err(WorldError::GovernanceFinalityInvalid {
                reason: "stake_root must be non-empty".to_string(),
            });
        }
        if certificate.stake_root != snapshot.stake_root {
            return Err(WorldError::GovernanceFinalityInvalid {
                reason: format!(
                    "stake_root mismatch: epoch_id={} certificate={} snapshot={}",
                    epoch_id, certificate.stake_root, snapshot.stake_root
                ),
            });
        }
        if certificate.threshold_bps == 0 || certificate.threshold_bps > 10_000 {
            return Err(WorldError::GovernanceFinalityInvalid {
                reason: format!(
                    "threshold_bps must be within 1..=10000: {}",
                    certificate.threshold_bps
                ),
            });
        }
        if certificate.threshold_bps != snapshot.threshold_bps {
            return Err(WorldError::GovernanceFinalityInvalid {
                reason: format!(
                    "threshold_bps mismatch: epoch_id={} certificate={} snapshot={}",
                    epoch_id, certificate.threshold_bps, snapshot.threshold_bps
                ),
            });
        }
        let certificate_min_unique_signers = certificate.effective_min_unique_signers();
        if certificate_min_unique_signers == 0 {
            return Err(WorldError::GovernanceFinalityInvalid {
                reason: "min_unique_signers must be > 0".to_string(),
            });
        }
        if certificate_min_unique_signers != snapshot_min_unique_signers {
            return Err(WorldError::GovernanceFinalityInvalid {
                reason: format!(
                    "min_unique_signers mismatch: epoch_id={} certificate={} snapshot={}",
                    epoch_id, certificate_min_unique_signers, snapshot_min_unique_signers
                ),
            });
        }
        if certificate.signatures.len() < certificate_min_unique_signers as usize {
            return Err(WorldError::GovernanceFinalityInvalid {
                reason: format!(
                    "signatures below min_unique_signers: signatures={} min_unique_signers={}",
                    certificate.signatures.len(),
                    certificate_min_unique_signers
                ),
            });
        }
        let signed_stake_bps = governance_finality_signed_stake_bps(
            snapshot.signer_node_ids.len(),
            certificate.signatures.len(),
        );
        if signed_stake_bps < certificate.threshold_bps {
            return Err(WorldError::GovernanceFinalityInvalid {
                reason: format!(
                    "signed stake below threshold_bps: epoch_id={} signed_stake_bps={} threshold_bps={}",
                    epoch_id, signed_stake_bps, certificate.threshold_bps
                ),
            });
        }
        for (node_id, signature_with_prefix) in &certificate.signatures {
            if !epoch_signers.contains(node_id) {
                return Err(WorldError::GovernanceFinalityInvalid {
                    reason: format!(
                        "signer node_id={} is not part of finality epoch snapshot epoch_id={}",
                        node_id, epoch_id
                    ),
                });
            }
            let signer_public_key = self.node_identity_public_key(node_id).ok_or_else(|| {
                WorldError::GovernanceFinalityInvalid {
                    reason: format!("untrusted signer node_id: {node_id}"),
                }
            })?;
            let signature_hex = signature_with_prefix
                .strip_prefix(GovernanceFinalityCertificate::SIGNATURE_PREFIX_ED25519_V1)
                .ok_or_else(|| WorldError::GovernanceFinalityInvalid {
                    reason: format!("signature prefix mismatch for signer {node_id}"),
                })?;
            let payload = GovernanceFinalityCertificate::signing_payload_v1(
                certificate.proposal_id,
                certificate.manifest_hash.as_str(),
                certificate.consensus_height,
                certificate.epoch_id,
                certificate.validator_set_hash.as_str(),
                certificate.stake_root.as_str(),
                certificate.threshold_bps,
                certificate_min_unique_signers,
                node_id.as_str(),
            );
            let public_key_bytes =
                decode_hex_array::<32>(signer_public_key, "governance finality signer public key")?;
            let signature_bytes =
                decode_hex_array::<64>(signature_hex, "governance finality signature")?;
            let verifying_key = VerifyingKey::from_bytes(&public_key_bytes).map_err(|_| {
                WorldError::GovernanceFinalityInvalid {
                    reason: format!("invalid signer public key for {node_id}"),
                }
            })?;
            let signature = Signature::from_bytes(&signature_bytes);
            verifying_key
                .verify(payload.as_slice(), &signature)
                .map_err(|error| WorldError::GovernanceFinalityInvalid {
                    reason: format!("signature verification failed for {node_id}: {error}"),
                })?;
        }
        Ok(())
    }

    pub(super) fn apply_governance_event(
        &mut self,
        event: &GovernanceEvent,
    ) -> Result<(), WorldError> {
        match event {
            GovernanceEvent::Proposed {
                proposal_id,
                author,
                base_manifest_hash,
                manifest,
                patch,
            } => {
                let proposal = Proposal {
                    id: *proposal_id,
                    author: author.clone(),
                    base_manifest_hash: base_manifest_hash.clone(),
                    manifest: manifest.clone(),
                    patch: patch.clone(),
                    queued_at_tick: None,
                    not_before_tick: None,
                    activate_epoch: None,
                    timelock_ticks: 0,
                    status: ProposalStatus::Proposed,
                };
                self.proposals.insert(*proposal_id, proposal);
                self.next_proposal_id = self.next_proposal_id.max(proposal_id.saturating_add(1));
            }
            GovernanceEvent::ShadowReport {
                proposal_id,
                manifest_hash,
            } => {
                let proposal =
                    self.proposals
                        .get_mut(proposal_id)
                        .ok_or(WorldError::ProposalNotFound {
                            proposal_id: *proposal_id,
                        })?;
                proposal.status = ProposalStatus::Shadowed {
                    manifest_hash: manifest_hash.clone(),
                };
            }
            GovernanceEvent::Approved {
                proposal_id,
                approver,
                decision,
            } => {
                let proposal =
                    self.proposals
                        .get_mut(proposal_id)
                        .ok_or(WorldError::ProposalNotFound {
                            proposal_id: *proposal_id,
                        })?;
                match decision {
                    ProposalDecision::Approve => {
                        let ProposalStatus::Shadowed { manifest_hash } = &proposal.status else {
                            return Err(WorldError::ProposalInvalidState {
                                proposal_id: *proposal_id,
                                expected: "shadowed".to_string(),
                                found: proposal.status.label(),
                            });
                        };
                        proposal.status = ProposalStatus::Approved {
                            manifest_hash: manifest_hash.clone(),
                            approver: approver.clone(),
                        };
                    }
                    ProposalDecision::Reject { reason } => {
                        proposal.queued_at_tick = None;
                        proposal.not_before_tick = None;
                        proposal.activate_epoch = None;
                        proposal.timelock_ticks = 0;
                        proposal.status = ProposalStatus::Rejected {
                            reason: reason.clone(),
                        };
                    }
                }
            }
            GovernanceEvent::Queued {
                proposal_id,
                manifest_hash,
                queued_at_tick,
                not_before_tick,
                activate_epoch,
                timelock_ticks,
            } => {
                let proposal =
                    self.proposals
                        .get_mut(proposal_id)
                        .ok_or(WorldError::ProposalNotFound {
                            proposal_id: *proposal_id,
                        })?;
                let ProposalStatus::Approved {
                    manifest_hash: approved_hash,
                    ..
                } = &proposal.status
                else {
                    return Err(WorldError::ProposalInvalidState {
                        proposal_id: *proposal_id,
                        expected: "approved".to_string(),
                        found: proposal.status.label(),
                    });
                };
                if approved_hash != manifest_hash {
                    return Err(WorldError::GovernancePolicyInvalid {
                        reason: format!(
                            "queued manifest hash drift: proposal_id={} approved={} queued={}",
                            proposal_id, approved_hash, manifest_hash
                        ),
                    });
                }
                if not_before_tick < queued_at_tick {
                    return Err(WorldError::GovernancePolicyInvalid {
                        reason: format!(
                            "invalid queued timeline: proposal_id={} queued_at={} not_before={}",
                            proposal_id, queued_at_tick, not_before_tick
                        ),
                    });
                }
                proposal.queued_at_tick = Some(*queued_at_tick);
                proposal.not_before_tick = Some(*not_before_tick);
                proposal.activate_epoch = Some(*activate_epoch);
                proposal.timelock_ticks = *timelock_ticks;
            }
            GovernanceEvent::Applied {
                proposal_id,
                manifest_hash,
                ..
            } => {
                let proposal =
                    self.proposals
                        .get_mut(proposal_id)
                        .ok_or(WorldError::ProposalNotFound {
                            proposal_id: *proposal_id,
                        })?;
                let ProposalStatus::Approved {
                    manifest_hash: approved_hash,
                    ..
                } = &proposal.status
                else {
                    return Err(WorldError::ProposalInvalidState {
                        proposal_id: *proposal_id,
                        expected: "approved".to_string(),
                        found: proposal.status.label(),
                    });
                };
                let applied_hash = manifest_hash
                    .clone()
                    .unwrap_or_else(|| approved_hash.clone());
                proposal.status = ProposalStatus::Applied {
                    manifest_hash: applied_hash,
                };
            }
            GovernanceEvent::EmergencyBrakeActivated {
                active_until_tick,
                threshold,
                signer_node_ids,
                ..
            } => {
                self.validate_guardian_signers(signer_node_ids, *threshold)?;
                let next_until = self
                    .governance_emergency_brake_until_tick
                    .map_or(*active_until_tick, |current| {
                        current.max(*active_until_tick)
                    });
                self.governance_emergency_brake_until_tick = Some(next_until);
            }
            GovernanceEvent::EmergencyBrakeReleased {
                threshold,
                signer_node_ids,
                ..
            } => {
                self.validate_guardian_signers(signer_node_ids, *threshold)?;
                self.governance_emergency_brake_until_tick = None;
            }
            GovernanceEvent::EmergencyVetoed {
                proposal_id,
                reason,
                threshold,
                signer_node_ids,
                ..
            } => {
                self.validate_guardian_signers(signer_node_ids, *threshold)?;
                let proposal =
                    self.proposals
                        .get_mut(proposal_id)
                        .ok_or(WorldError::ProposalNotFound {
                            proposal_id: *proposal_id,
                        })?;
                if !matches!(proposal.status, ProposalStatus::Approved { .. }) {
                    return Err(WorldError::ProposalInvalidState {
                        proposal_id: *proposal_id,
                        expected: "approved".to_string(),
                        found: proposal.status.label(),
                    });
                }
                if proposal.not_before_tick.is_none() || proposal.activate_epoch.is_none() {
                    return Err(WorldError::GovernancePolicyInvalid {
                        reason: format!("proposal_id={} is not queued for activation", proposal_id),
                    });
                }
                proposal.queued_at_tick = None;
                proposal.not_before_tick = None;
                proposal.activate_epoch = None;
                proposal.timelock_ticks = 0;
                proposal.status = ProposalStatus::Rejected {
                    reason: format!("emergency_veto: {reason}"),
                };
            }
            GovernanceEvent::IdentityPenaltyApplied {
                penalty_id,
                target_agent_id,
                evidence_hash,
                initiator,
                reason,
                slash_stake,
                appeal_deadline_tick,
                threshold,
                signer_node_ids,
            } => {
                self.validate_guardian_signers(signer_node_ids, *threshold)?;
                if !self.state.agents.contains_key(target_agent_id.as_str()) {
                    return Err(WorldError::AgentNotFound {
                        agent_id: target_agent_id.clone(),
                    });
                }
                Self::validate_governance_identity_evidence_hash(evidence_hash.as_str())?;
                Self::validate_governance_identity_field(
                    "identity penalty reason",
                    reason.as_str(),
                )?;
                Self::validate_governance_identity_field(
                    "identity penalty initiator",
                    initiator.as_str(),
                )?;
                if self.governance_identity_penalties.contains_key(penalty_id) {
                    return Err(WorldError::GovernancePolicyInvalid {
                        reason: format!("duplicate identity penalty id: penalty_id={penalty_id}"),
                    });
                }
                let detection_incident_id =
                    Self::build_identity_penalty_incident_id(target_agent_id, evidence_hash);
                if self
                    .governance_identity_penalties
                    .values()
                    .any(|record| record.detection_incident_id == detection_incident_id)
                {
                    return Err(WorldError::GovernancePolicyInvalid {
                        reason: format!(
                            "duplicate identity penalty incident: incident_id={detection_incident_id}"
                        ),
                    });
                }
                let detection_risk_score = self
                    .threat_heatmap
                    .get(target_agent_id.as_str())
                    .copied()
                    .unwrap_or_default();
                let evidence_chain_hash = Self::build_identity_penalty_chain_hash(
                    *penalty_id,
                    target_agent_id,
                    evidence_hash,
                    reason,
                    detection_incident_id.as_str(),
                );
                let mut profile = self
                    .state
                    .governance_identity_profiles
                    .get(target_agent_id)
                    .cloned()
                    .unwrap_or_else(|| GovernanceIdentityProfileState {
                        agent_id: target_agent_id.clone(),
                        ..GovernanceIdentityProfileState::default()
                    });
                if *slash_stake > profile.stake_locked {
                    return Err(WorldError::GovernancePolicyInvalid {
                        reason: format!(
                            "identity penalty slash exceeds locked stake: penalty_id={} slash={} stake_locked={}",
                            penalty_id, slash_stake, profile.stake_locked
                        ),
                    });
                }
                let identity_status_before = profile.status;
                profile.stake_locked = profile.stake_locked.saturating_sub(*slash_stake);
                profile.status = GovernanceIdentityStatus::Frozen;
                profile.slash_count = profile.slash_count.saturating_add(1);
                profile.updated_at = self.state.time;
                self.state
                    .governance_identity_profiles
                    .insert(target_agent_id.clone(), profile);
                self.governance_identity_penalties.insert(
                    *penalty_id,
                    GovernanceIdentityPenaltyRecord {
                        penalty_id: *penalty_id,
                        target_agent_id: target_agent_id.clone(),
                        evidence_hash: evidence_hash.clone(),
                        reason: reason.clone(),
                        slash_stake: *slash_stake,
                        appeal_deadline_tick: *appeal_deadline_tick,
                        status: GovernanceIdentityPenaltyStatus::Applied,
                        identity_status_before,
                        detection_source: IDENTITY_PENALTY_DETECTION_SOURCE.to_string(),
                        detection_risk_score,
                        detection_incident_id,
                        evidence_chain_hash,
                        appeal_evidence_hash: None,
                        resolution_evidence_hash: None,
                        appellant: None,
                        appeal_reason: None,
                        resolved_by: None,
                        resolution_reason: None,
                        resolved_at_tick: None,
                    },
                );
                self.next_governance_identity_penalty_id = self
                    .next_governance_identity_penalty_id
                    .max(penalty_id.saturating_add(1));
            }
            GovernanceEvent::IdentityPenaltyAppealed {
                penalty_id,
                appellant,
                reason,
            } => {
                Self::validate_governance_identity_field(
                    "identity penalty appeal appellant",
                    appellant.as_str(),
                )?;
                Self::validate_governance_identity_field(
                    "identity penalty appeal reason",
                    reason.as_str(),
                )?;
                let appeal_evidence_hash =
                    Self::build_identity_penalty_stage_evidence_hash("appeal", appellant, reason);
                let penalty = self
                    .governance_identity_penalties
                    .get_mut(penalty_id)
                    .ok_or(WorldError::GovernancePolicyInvalid {
                        reason: format!("identity penalty not found: penalty_id={penalty_id}"),
                    })?;
                if penalty.status != GovernanceIdentityPenaltyStatus::Applied {
                    return Err(WorldError::GovernancePolicyInvalid {
                        reason: format!(
                            "identity penalty is not appealable: penalty_id={} status={:?}",
                            penalty_id, penalty.status
                        ),
                    });
                }
                if self.state.time > penalty.appeal_deadline_tick {
                    return Err(WorldError::GovernancePolicyInvalid {
                        reason: format!(
                            "identity penalty appeal window closed: penalty_id={} deadline_tick={}",
                            penalty_id, penalty.appeal_deadline_tick
                        ),
                    });
                }
                if penalty.detection_source.trim().is_empty() {
                    penalty.detection_source = IDENTITY_PENALTY_DETECTION_SOURCE.to_string();
                }
                if penalty.detection_incident_id.trim().is_empty() {
                    penalty.detection_incident_id = Self::build_identity_penalty_incident_id(
                        penalty.target_agent_id.as_str(),
                        penalty.evidence_hash.as_str(),
                    );
                }
                if penalty.evidence_chain_hash.trim().is_empty() {
                    penalty.evidence_chain_hash = Self::build_identity_penalty_chain_hash(
                        penalty.penalty_id,
                        penalty.target_agent_id.as_str(),
                        penalty.evidence_hash.as_str(),
                        penalty.reason.as_str(),
                        penalty.detection_incident_id.as_str(),
                    );
                }
                penalty.status = GovernanceIdentityPenaltyStatus::Appealed;
                penalty.appellant = Some(appellant.clone());
                penalty.appeal_reason = Some(reason.clone());
                penalty.appeal_evidence_hash = Some(appeal_evidence_hash.clone());
                penalty.evidence_chain_hash = Self::extend_identity_penalty_chain_hash(
                    penalty.evidence_chain_hash.as_str(),
                    "appeal",
                    appeal_evidence_hash.as_str(),
                );
            }
            GovernanceEvent::IdentityPenaltyResolved {
                penalty_id,
                resolver,
                accepted,
                reason,
            } => {
                Self::validate_governance_identity_field(
                    "identity penalty appeal resolver",
                    resolver.as_str(),
                )?;
                Self::validate_governance_identity_field(
                    "identity penalty appeal resolution",
                    reason.as_str(),
                )?;
                let resolution_evidence_hash = Self::build_identity_penalty_stage_evidence_hash(
                    if *accepted {
                        "resolve_accept"
                    } else {
                        "resolve_reject"
                    },
                    resolver,
                    reason,
                );
                let (target_agent_id, slash_stake, identity_status_before) = {
                    let penalty = self
                        .governance_identity_penalties
                        .get_mut(penalty_id)
                        .ok_or(WorldError::GovernancePolicyInvalid {
                            reason: format!("identity penalty not found: penalty_id={penalty_id}"),
                        })?;
                    if penalty.status != GovernanceIdentityPenaltyStatus::Appealed {
                        return Err(WorldError::GovernancePolicyInvalid {
                            reason: format!(
                                "identity penalty appeal is not pending: penalty_id={} status={:?}",
                                penalty_id, penalty.status
                            ),
                        });
                    }
                    if penalty.detection_source.trim().is_empty() {
                        penalty.detection_source = IDENTITY_PENALTY_DETECTION_SOURCE.to_string();
                    }
                    if penalty.detection_incident_id.trim().is_empty() {
                        penalty.detection_incident_id = Self::build_identity_penalty_incident_id(
                            penalty.target_agent_id.as_str(),
                            penalty.evidence_hash.as_str(),
                        );
                    }
                    if penalty.evidence_chain_hash.trim().is_empty() {
                        penalty.evidence_chain_hash = Self::build_identity_penalty_chain_hash(
                            penalty.penalty_id,
                            penalty.target_agent_id.as_str(),
                            penalty.evidence_hash.as_str(),
                            penalty.reason.as_str(),
                            penalty.detection_incident_id.as_str(),
                        );
                    }
                    penalty.status = if *accepted {
                        GovernanceIdentityPenaltyStatus::AppealAccepted
                    } else {
                        GovernanceIdentityPenaltyStatus::AppealRejected
                    };
                    penalty.resolved_by = Some(resolver.clone());
                    penalty.resolution_reason = Some(reason.clone());
                    penalty.resolved_at_tick = Some(self.state.time);
                    penalty.resolution_evidence_hash = Some(resolution_evidence_hash.clone());
                    penalty.evidence_chain_hash = Self::extend_identity_penalty_chain_hash(
                        penalty.evidence_chain_hash.as_str(),
                        "resolve",
                        resolution_evidence_hash.as_str(),
                    );
                    (
                        penalty.target_agent_id.clone(),
                        penalty.slash_stake,
                        penalty.identity_status_before,
                    )
                };
                let profile = self
                    .state
                    .governance_identity_profiles
                    .get_mut(target_agent_id.as_str())
                    .ok_or(WorldError::AgentNotFound {
                        agent_id: target_agent_id.clone(),
                    })?;
                if *accepted {
                    profile.stake_locked = profile.stake_locked.saturating_add(slash_stake);
                    profile.status = identity_status_before;
                }
                profile.updated_at = self.state.time;
            }
            GovernanceEvent::RestrictedStarterClaimAdminRegistryUpdated {
                controller_account_id,
                previous_admin_account_ids,
                next_admin_account_ids,
            } => {
                let Some(current_registry) =
                    self.state.governance_main_token_controller_registry.clone()
                else {
                    return Err(WorldError::GovernancePolicyInvalid {
                        reason:
                            "restricted claim admin registry update requires controller registry"
                                .to_string(),
                    });
                };
                let expected_controller_account_id =
                    Self::restricted_starter_claim_admin_registry_controller_account_id(
                        &current_registry,
                    )?;
                if controller_account_id != expected_controller_account_id {
                    return Err(WorldError::GovernancePolicyInvalid {
                        reason: format!(
                            "restricted claim admin registry controller slot mismatch: expected={} actual={}",
                            expected_controller_account_id, controller_account_id
                        ),
                    });
                }
                let current_admins = current_registry
                    .restricted_starter_claim_admin_account_ids
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>();
                if current_admins != *previous_admin_account_ids {
                    return Err(WorldError::GovernancePolicyInvalid {
                        reason: format!(
                            "restricted claim admin registry drift before apply: expected_previous={:?} actual_current={:?}",
                            previous_admin_account_ids, current_admins
                        ),
                    });
                }
                let mut next_registry = current_registry;
                next_registry.restricted_starter_claim_admin_account_ids = next_admin_account_ids
                    .iter()
                    .map(|value| value.trim())
                    .filter(|value| !value.is_empty())
                    .map(ToString::to_string)
                    .collect();
                next_registry =
                    Self::validate_governance_main_token_controller_registry(next_registry)?;
                self.state.governance_main_token_controller_registry = Some(next_registry);
            }
        }
        Ok(())
    }
}
