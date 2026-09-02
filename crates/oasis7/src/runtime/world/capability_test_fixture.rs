//! Explicit Runtime-owned provider capability fixture for in-crate live tests.
//!
//! This is intentionally test-only.  Production worlds stay fail-closed until
//! an external authority installs the same proof-bearing records through the
//! public admission APIs.

use super::super::capability_authorization::{
    CapabilityAuthorityFinalityBinding, CapabilityAuthorityFinalityProof, CapabilityAuthorityRecord,
};
use super::super::governance::GovernanceFinalityEpochSnapshot;
use super::super::{
    CapabilityInvocationContext, GovernanceFinalityCertificate, Manifest, ModuleAbiContract,
    ModuleActivation, ModuleChangeSet, ModuleKind, ModuleLimits, ModuleManifest, ModuleRole,
    ProposalDecision, WorldError,
};
use super::World;
use crate::runtime::capability_authorization::CapabilityAgentIdentity;
use ed25519_dalek::{Signer, SigningKey};
use oasis7_wasm_abi::{
    CapabilityAudience, CapabilityGrantV2, CapabilityIssuer, CapabilityPresenter, CapabilityScope,
    CapabilitySubject, ModuleCommandDeclaration, ModuleSchemaDeclarations,
};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

const ISSUER_ID: &str = "governance.local.finality.signer.1";
const FINALITY_SIGNER_ID: &str = "governance.local.finality.signer.2";
const ISSUER_SEED: &[u8] = b"oasis7-governance-local-finality-signer-1-v1";
const FINALITY_SIGNER_SEED: &[u8] = b"oasis7-governance-local-finality-signer-2-v1";
const MODULE_ID: &str = "module.runtime.provider-fixture";
const MODULE_VERSION: &str = "1.0.0";
const MODULE_SCHEMA_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";

impl World {
    /// Install a complete, proof-bearing provider capability fixture for an
    /// already-bound in-crate live test World.  Viewer only opts into this
    /// seam; it does not construct authority, grant, or invocation fields.
    #[cfg(test)]
    pub fn install_test_provider_capability_fixture(
        &mut self,
        agent_id: &str,
    ) -> Result<CapabilityInvocationContext, WorldError> {
        let mut staged = self.clone();
        let invocation = staged.install_test_provider_capability_fixture_inner(agent_id)?;
        *self = staged;
        Ok(invocation)
    }

    fn install_test_provider_capability_fixture_inner(
        &mut self,
        agent_id: &str,
    ) -> Result<CapabilityInvocationContext, WorldError> {
        self.verify_capability_authorization_root()?;
        let (world_id, branch_id, finality_epoch, finality_block_hash) =
            self.bound_runtime_identity()?;
        if !self.state.agents.contains_key(agent_id) {
            return Err(fixture_error("provider fixture requires a live agent"));
        }
        if let Some(existing) = self
            .capability_invocation_contexts
            .values()
            .find(|context| {
                matches!(
                    &context.subject,
                    CapabilitySubject::Agent { agent_id: subject_id, .. }
                        if subject_id == agent_id
                ) && context.presenter.presenter_kind == "provider"
                    && context.audience.world_id == world_id
                    && context.audience.branch_id == branch_id
                    && context.audience.finality_epoch == finality_epoch
            })
        {
            return Ok(existing.clone());
        }

        let identity = self.install_fixture_agent_identity(agent_id)?;
        self.install_fixture_authority(
            world_id.as_str(),
            branch_id.as_str(),
            finality_epoch,
            finality_block_hash.as_deref(),
        )?;
        self.install_fixture_module()?;

        let grant = self.fixture_command_grant(
            agent_id,
            &identity,
            world_id.as_str(),
            branch_id.as_str(),
            finality_epoch,
        )?;
        self.register_capability_grant_v2(grant)?;
        let presenter = CapabilityPresenter {
            presenter_id: format!("runtime-test-provider:{agent_id}"),
            presenter_kind: "provider".to_string(),
            session_id: Some(format!("runtime-test-session:{agent_id}")),
            attestation_ref: None,
        };
        self.install_capability_invocation_context_for_agent(
            agent_id,
            presenter,
            format!("runtime-test-response:{agent_id}"),
        )
    }

    fn bound_runtime_identity(&self) -> Result<(String, String, u64, Option<String>), WorldError> {
        let binding = self
            .cognition
            .get("runtime_binding")
            .and_then(|value| value.as_object())
            .ok_or_else(|| fixture_error("provider fixture requires a bound Runtime cognition"))?;
        let string_field = |name: &str| {
            binding
                .get(name)
                .and_then(|value| value.as_str())
                .filter(|value| !value.trim().is_empty())
                .map(str::to_string)
                .ok_or_else(|| fixture_error(format!("Runtime binding field {name} is required")))
        };
        let world_id = string_field("world_id")?;
        let branch_id = string_field("branch_id")?;
        let finality_epoch = binding
            .get("finality_epoch")
            .and_then(|value| value.as_u64())
            .ok_or_else(|| fixture_error("Runtime binding finality_epoch is required"))?;
        let finality_block_hash = binding
            .get("finality_block_hash")
            .and_then(|value| value.as_str())
            .filter(|value| !value.trim().is_empty())
            .map(str::to_string);
        Ok((world_id, branch_id, finality_epoch, finality_block_hash))
    }

    fn install_fixture_agent_identity(
        &mut self,
        agent_id: &str,
    ) -> Result<CapabilityAgentIdentity, WorldError> {
        if let Some(identity) = self
            .capability_revocation_state
            .agent_identities
            .get(agent_id)
        {
            return Ok(identity.clone());
        }
        self.install_capability_agent_identity(
            agent_id,
            format!("runtime-test-owner:{agent_id}"),
            1,
        )?;
        self.capability_revocation_state
            .agent_identities
            .get(agent_id)
            .cloned()
            .ok_or_else(|| fixture_error("fixture identity was not persisted"))
    }

    fn install_fixture_authority(
        &mut self,
        world_id: &str,
        branch_id: &str,
        finality_epoch: u64,
        finality_block_hash: Option<&str>,
    ) -> Result<(), WorldError> {
        let issuer_key = fixture_signing_key(ISSUER_SEED);
        let finality_key = fixture_signing_key(FINALITY_SIGNER_SEED);
        self.bind_node_identity(
            ISSUER_ID,
            &hex::encode(issuer_key.verifying_key().to_bytes()),
        )?;
        self.bind_node_identity(
            FINALITY_SIGNER_ID,
            &hex::encode(finality_key.verifying_key().to_bytes()),
        )?;
        self.set_governance_finality_epoch_snapshot(GovernanceFinalityEpochSnapshot {
            epoch_id: finality_epoch,
            threshold_bps: 10_000,
            min_unique_signers: 2,
            threshold: 2,
            signer_node_ids: vec![ISSUER_ID.to_string(), FINALITY_SIGNER_ID.to_string()],
            validator_stakes: BTreeMap::from([
                (ISSUER_ID.to_string(), 100),
                (FINALITY_SIGNER_ID.to_string(), 100),
            ]),
            ..GovernanceFinalityEpochSnapshot::default()
        })?;

        let proposal = self.propose_manifest_update(
            Manifest {
                version: self.manifest.version.saturating_add(1),
                content: json!({"runtime_provider_capability_fixture": true}),
            },
            "runtime-test-fixture",
        )?;
        self.shadow_proposal(proposal)?;
        self.approve_proposal(proposal, "runtime-test-fixture", ProposalDecision::Approve)?;
        let proposal_manifest_hash = match &self
            .proposals
            .get(&proposal)
            .ok_or_else(|| fixture_error("fixture authority proposal disappeared"))?
            .status
        {
            super::super::ProposalStatus::Approved { manifest_hash, .. } => manifest_hash.clone(),
            status => {
                return Err(fixture_error(format!(
                    "fixture authority proposal is not approved: {status:?}"
                )));
            }
        };
        let snapshot = self
            .governance_finality_epoch_snapshots
            .get(&finality_epoch)
            .cloned()
            .ok_or_else(|| fixture_error("fixture finality snapshot was not persisted"))?;
        let consensus_height = self.journal.events.len() as u64 + 1;
        let min_unique_signers = snapshot.effective_min_unique_signers();
        let mut certificate_signatures = BTreeMap::new();
        for (node_id, key) in [
            (ISSUER_ID, issuer_key.clone()),
            (FINALITY_SIGNER_ID, finality_key.clone()),
        ] {
            let payload = GovernanceFinalityCertificate::signing_payload_v1(
                proposal,
                proposal_manifest_hash.as_str(),
                consensus_height,
                snapshot.epoch_id,
                snapshot.validator_set_hash.as_str(),
                snapshot.stake_root.as_str(),
                snapshot.threshold_bps,
                min_unique_signers,
                node_id,
            );
            let signature = key.sign(payload.as_slice());
            certificate_signatures.insert(
                node_id.to_string(),
                format!(
                    "{}{}",
                    GovernanceFinalityCertificate::SIGNATURE_PREFIX_ED25519_V1,
                    hex::encode(signature.to_bytes())
                ),
            );
        }
        let certificate = GovernanceFinalityCertificate {
            proposal_id: proposal,
            manifest_hash: proposal_manifest_hash,
            consensus_height,
            epoch_id: snapshot.epoch_id,
            validator_set_hash: snapshot.validator_set_hash,
            stake_root: snapshot.stake_root,
            threshold_bps: snapshot.threshold_bps,
            min_unique_signers,
            threshold: min_unique_signers,
            signatures: certificate_signatures,
        };
        let authority = CapabilityAuthorityRecord {
            issuer_id: ISSUER_ID.to_string(),
            issuer_kind: "governance".to_string(),
            key_id: "governance-local-finality-key-1".to_string(),
            public_key_hex: hex::encode(issuer_key.verifying_key().to_bytes()),
            issuer_key_epoch: 1,
            governance_epoch: finality_epoch,
            finalized_receipt_id: format!(
                "runtime-test-authority:{world_id}:{branch_id}:{finality_epoch}"
            ),
            authority_rotation_receipt_id: None,
            world_id: world_id.to_string(),
            branch_id: branch_id.to_string(),
            finality_epoch,
            finality_block_hash: finality_block_hash
                .map(str::to_string)
                .unwrap_or_else(|| format!("runtime-test-finality-block:{finality_epoch}")),
            finality_status: "finalized".to_string(),
            revocation_epoch: 0,
            revoked_grant_ids: BTreeSet::new(),
            superseded_by: BTreeMap::new(),
        };
        let binding =
            CapabilityAuthorityFinalityBinding::from_record(&authority).map_err(fixture_error)?;
        let mut proof = CapabilityAuthorityFinalityProof {
            proof_version: CapabilityAuthorityFinalityProof::PROOF_VERSION_V1,
            certificate,
            binding,
            signatures: BTreeMap::new(),
        };
        for (node_id, key) in [(ISSUER_ID, issuer_key), (FINALITY_SIGNER_ID, finality_key)] {
            let payload = proof.signing_payload_v1(node_id).map_err(fixture_error)?;
            let signature = key.sign(payload.as_slice());
            proof.signatures.insert(
                node_id.to_string(),
                format!(
                    "{}{}",
                    CapabilityAuthorityFinalityProof::SIGNATURE_PREFIX_ED25519_V1,
                    hex::encode(signature.to_bytes())
                ),
            );
        }
        self.install_capability_authority_record_with_finality_proof(authority, proof)
    }

    fn install_fixture_module(&mut self) -> Result<(), WorldError> {
        let manifest = ModuleManifest {
            module_id: MODULE_ID.to_string(),
            name: "Runtime provider capability fixture".to_string(),
            version: MODULE_VERSION.to_string(),
            kind: ModuleKind::Pure,
            role: ModuleRole::AgentInternal,
            wasm_hash: "runtime-provider-capability-fixture".to_string(),
            interface_version: "wasm-1".to_string(),
            exports: vec!["call".to_string()],
            subscriptions: Vec::new(),
            required_caps: Vec::new(),
            abi_contract: ModuleAbiContract {
                declarations: ModuleSchemaDeclarations {
                    commands: vec![ModuleCommandDeclaration {
                        namespace: "provider".to_string(),
                        name: "observe".to_string(),
                        schema_version: 1,
                        schema_hash: MODULE_SCHEMA_HASH.to_string(),
                        max_payload_bytes: 1024,
                    }],
                },
                ..ModuleAbiContract::default()
            },
            artifact_identity: None,
            limits: ModuleLimits {
                max_mem_bytes: 64 * 1024,
                max_gas: 100_000,
                max_call_rate: 100,
                max_output_bytes: 4 * 1024,
                max_effects: 4,
                max_emits: 4,
            },
        };
        let changes = ModuleChangeSet {
            register: vec![manifest.clone()],
            activate: vec![ModuleActivation {
                module_id: manifest.module_id,
                version: manifest.version,
            }],
            ..ModuleChangeSet::default()
        };
        self.apply_module_changes_for_test(0, &changes, "runtime-test-fixture")
    }

    fn fixture_command_grant(
        &self,
        agent_id: &str,
        identity: &CapabilityAgentIdentity,
        world_id: &str,
        branch_id: &str,
        finality_epoch: u64,
    ) -> Result<CapabilityGrantV2, WorldError> {
        let issuer_key = fixture_signing_key(ISSUER_SEED);
        let finalized_receipt_id =
            format!("runtime-test-authority:{world_id}:{branch_id}:{finality_epoch}");
        let mut grant = CapabilityGrantV2 {
            grant_id: String::new(),
            grant_version: 2,
            subject: CapabilitySubject::Agent {
                agent_id: agent_id.to_string(),
                owner_binding: identity.owner_binding.clone(),
                generation: identity.generation,
            },
            audience: CapabilityAudience {
                world_id: world_id.to_string(),
                branch_id: branch_id.to_string(),
                finality_epoch,
                target_kind: "world".to_string(),
                target_id: None,
            },
            issuer: CapabilityIssuer {
                issuer_id: ISSUER_ID.to_string(),
                issuer_kind: "governance".to_string(),
                governance_epoch: finality_epoch,
                finalized_receipt_id,
                key_id: "governance-local-finality-key-1".to_string(),
                issuer_key_epoch: 1,
                authority_rotation_receipt_id: None,
                signature: String::new(),
            },
            scope: CapabilityScope {
                module_id: MODULE_ID.to_string(),
                module_version: MODULE_VERSION.to_string(),
                namespace: "provider".to_string(),
                object_kind: "command".to_string(),
                object_name: "observe".to_string(),
                operation: "execute".to_string(),
                entity_selector: None,
                resource_selector: None,
                max_payload_bytes: Some(1024),
                policy_class: Some("read-only".to_string()),
            },
            issued_at_tick: self.state.time,
            expires_at_tick: Some(self.state.time.saturating_add(100)),
            grant_nonce: format!("runtime-test-provider:{agent_id}:{branch_id}:{finality_epoch}"),
            parent_grant_id: None,
            delegation_depth: 0,
            revocation_epoch: 0,
            status: "verified".to_string(),
            canonical_body_hash: String::new(),
            issuance_signature: String::new(),
        };
        let body_hash = grant
            .canonical_body_hash()
            .map_err(|error| fixture_error(error.to_string()))?;
        grant.grant_id = body_hash.clone();
        grant.canonical_body_hash = body_hash;
        let signature = issuer_key.sign(
            grant
                .canonical_body_bytes()
                .map_err(|error| fixture_error(error.to_string()))?
                .as_slice(),
        );
        let signature = format!("ed25519:{}", hex::encode(signature.to_bytes()));
        grant.issuer.signature = signature.clone();
        grant.issuance_signature = signature;
        Ok(grant)
    }
}

fn fixture_signing_key(seed_label: &[u8]) -> SigningKey {
    let seed = crate::runtime::util::sha256_hex(seed_label);
    let bytes = hex::decode(seed).expect("decode runtime fixture signing seed");
    SigningKey::from_bytes(
        &bytes
            .as_slice()
            .try_into()
            .expect("runtime fixture signing seed is 32 bytes"),
    )
}

fn fixture_error(reason: impl Into<String>) -> WorldError {
    WorldError::CapabilityAuthorizationDenied {
        reason: reason.into(),
    }
}
