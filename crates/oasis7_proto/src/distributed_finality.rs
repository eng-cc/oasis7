//! Bounded validator-set/finality proof evidence anchored to `WorldHeadProofV1`.

use std::collections::{BTreeMap, BTreeSet};

use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};

use crate::distributed::WorldHeadProofV1;

pub const WORLD_FINALITY_PROOF_V1_SCHEMA: u16 = 2;
pub const WORLD_FINALITY_PROOF_HASH_DOMAIN_V1: &str = "oasis7.world_finality_proof.v2";
pub const WORLD_FINALITY_VALIDATOR_SET_HASH_DOMAIN_V1: &str =
    "oasis7.world_finality_validator_set.v1";
pub const WORLD_FINALITY_GOVERNANCE_SET_HASH_DOMAIN_V1: &str =
    "oasis7.world_finality_governance_set.v1";
pub const WORLD_FINALITY_VOTE_SIGNING_DOMAIN_V1: &str = "oasis7.world_finality_vote.v1";
pub const WORLD_FINALITY_VALIDATOR_SET_TRANSITION_SIGNING_DOMAIN_V1: &str =
    "oasis7.world_finality_validator_set_transition.v1";
pub const WORLD_FINALITY_VALIDATOR_SET_TRANSITION_GOVERNANCE_SIGNING_DOMAIN_V1: &str =
    "oasis7.world_finality_validator_set_transition_governance.v1";
pub const WORLD_FINALITY_PROOF_CLAIM_BOUNDARY_V1: &str =
    "validator_set_finality_evidence_only_not_full_light_client_or_mainnet_readiness";

fn world_finality_proof_v1_schema() -> u16 {
    WORLD_FINALITY_PROOF_V1_SCHEMA
}

fn world_finality_proof_claim_boundary_v1() -> String {
    WORLD_FINALITY_PROOF_CLAIM_BOUNDARY_V1.to_string()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldFinalityValidatorV1 {
    pub validator_id: String,
    pub stake_weight: u64,
    pub finality_signer_public_key: String,
    pub activation_height: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_height: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldFinalityVoteV1 {
    pub validator_id: String,
    pub height: u64,
    pub block_hash: String,
    pub state_root: String,
    pub signature_scheme: String,
    pub signature_evidence_hash: String,
    #[serde(default)]
    pub signature_hex: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldFinalityCommitmentV1 {
    pub height: u64,
    pub round_id: String,
    pub block_hash: String,
    pub state_root: String,
    pub validator_set_hash: String,
    pub quorum_threshold_bps: u64,
    pub votes: Vec<WorldFinalityVoteV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldFinalityMisbehaviorEvidenceV1 {
    pub height: u64,
    pub validator_id: String,
    pub evidence_kind: String,
    pub conflicting_block_hash: String,
    pub conflicting_proof_hash: String,
    pub disposition: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldFinalityValidatorSetTransitionApprovalV1 {
    pub validator_id: String,
    pub from_validator_set_hash: String,
    pub to_validator_set_hash: String,
    pub to_validator_set_activation_height: u64,
    pub transition_height: u64,
    pub transition_block_hash: String,
    pub signature_scheme: String,
    pub signature_hex: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldFinalityGovernanceSignerV1 {
    pub signer_id: String,
    pub stake_weight: u64,
    pub governance_public_key: String,
    pub activation_height: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_height: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldFinalityValidatorSetTransitionGovernanceApprovalV1 {
    pub signer_id: String,
    pub governance_set_hash: String,
    pub from_validator_set_hash: String,
    pub to_validator_set_hash: String,
    pub to_validator_set_activation_height: u64,
    pub transition_height: u64,
    pub transition_block_hash: String,
    pub signature_scheme: String,
    pub signature_hex: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldFinalityValidatorSetTransitionGovernanceCertificateV1 {
    pub governance_set_id: String,
    pub governance_set_activation_height: u64,
    pub governance_threshold_bps: u64,
    pub governance_set_hash: String,
    pub governance_signers: Vec<WorldFinalityGovernanceSignerV1>,
    pub governance_approvals: Vec<WorldFinalityValidatorSetTransitionGovernanceApprovalV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldFinalityValidatorSetTransitionV1 {
    pub from_validator_set_id: String,
    pub from_validator_set_hash: String,
    pub to_validator_set_id: String,
    pub to_validator_set_activation_height: u64,
    pub to_quorum_threshold_bps: u64,
    pub to_validator_set_hash: String,
    pub to_validators: Vec<WorldFinalityValidatorV1>,
    pub transition_height: u64,
    pub transition_block_hash: String,
    pub approvals: Vec<WorldFinalityValidatorSetTransitionApprovalV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub governance_certificate: Option<WorldFinalityValidatorSetTransitionGovernanceCertificateV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldFinalityProofV1 {
    #[serde(default = "world_finality_proof_v1_schema")]
    pub schema_version: u16,
    pub world_id: String,
    pub from_height: u64,
    pub to_height: u64,
    pub trusted_anchor_height: u64,
    pub trusted_anchor_block_hash: String,
    pub validator_set_id: String,
    pub validator_set_activation_height: u64,
    pub validator_set_hash: String,
    pub validators: Vec<WorldFinalityValidatorV1>,
    pub quorum_threshold_bps: u64,
    pub head_proofs: Vec<WorldHeadProofV1>,
    pub finality_commitments: Vec<WorldFinalityCommitmentV1>,
    #[serde(default)]
    pub validator_set_transitions: Vec<WorldFinalityValidatorSetTransitionV1>,
    #[serde(default)]
    pub misbehavior_evidence: Vec<WorldFinalityMisbehaviorEvidenceV1>,
    #[serde(default = "world_finality_proof_claim_boundary_v1")]
    pub claim_boundary: String,
}

impl WorldFinalityValidatorV1 {
    fn is_active_at(&self, height: u64) -> bool {
        self.activation_height <= height && self.exit_height.is_none_or(|exit| height < exit)
    }
}

impl WorldFinalityGovernanceSignerV1 {
    fn is_active_at(&self, height: u64) -> bool {
        self.activation_height <= height && self.exit_height.is_none_or(|exit| height < exit)
    }
}

impl WorldFinalityProofV1 {
    pub fn validate_contract(&self) -> Result<(), String> {
        if self.schema_version != WORLD_FINALITY_PROOF_V1_SCHEMA {
            return Err(format!(
                "unsupported world finality proof schema: {}",
                self.schema_version
            ));
        }
        if self.claim_boundary != WORLD_FINALITY_PROOF_CLAIM_BOUNDARY_V1 {
            return Err(format!(
                "unexpected world finality proof claim boundary: {}",
                self.claim_boundary
            ));
        }
        require_non_empty("world_id", &self.world_id)?;
        if self.from_height == 0 || self.to_height < self.from_height {
            return Err(format!(
                "invalid finality proof height range: from={} to={}",
                self.from_height, self.to_height
            ));
        }
        require_non_empty("trusted_anchor_block_hash", &self.trusted_anchor_block_hash)?;
        if self.trusted_anchor_height + 1 != self.from_height {
            return Err(format!(
                "trusted anchor height must precede from_height: anchor={} from={}",
                self.trusted_anchor_height, self.from_height
            ));
        }
        require_non_empty("validator_set_id", &self.validator_set_id)?;
        if self.validator_set_activation_height > self.from_height {
            return Err(format!(
                "validator set activation height must be <= from_height: activation={} from={}",
                self.validator_set_activation_height, self.from_height
            ));
        }
        if !(5_001..=10_000).contains(&self.quorum_threshold_bps) {
            return Err(format!(
                "quorum_threshold_bps must be in 5001..=10000, got {}",
                self.quorum_threshold_bps
            ));
        }
        validate_validator_set(self.validators.as_slice())?;
        let computed_set_hash = compute_world_finality_validator_set_hash(
            self.validator_set_id.as_str(),
            self.validator_set_activation_height,
            self.quorum_threshold_bps,
            self.validators.as_slice(),
        )?;
        if self.validator_set_hash != computed_set_hash {
            return Err(format!(
                "validator set hash mismatch: proof={} computed={computed_set_hash}",
                self.validator_set_hash
            ));
        }
        let expected_span = self.to_height - self.from_height + 1;
        if self.head_proofs.len() != expected_span as usize {
            return Err(format!(
                "head_proofs length must match height span: expected={} actual={}",
                expected_span,
                self.head_proofs.len()
            ));
        }
        if self.finality_commitments.len() != expected_span as usize {
            return Err(format!(
                "finality_commitments length must match height span: expected={} actual={}",
                expected_span,
                self.finality_commitments.len()
            ));
        }

        let mut previous_block_hash: Option<String> = None;
        let mut previous_timestamp_ms: Option<i64> = None;
        let mut committed_blocks = BTreeMap::new();
        let mut consensus_approvers_by_height = BTreeMap::new();

        for (index, proof) in self.head_proofs.iter().enumerate() {
            proof.validate_contract()?;
            let expected_height = self.from_height + index as u64;
            if proof.height != expected_height {
                return Err(format!(
                    "head proof height gap at index {index}: expected={} actual={}",
                    expected_height, proof.height
                ));
            }
            if proof.world_id != self.world_id {
                return Err(format!(
                    "head proof world_id mismatch at height {}: expected={} actual={}",
                    proof.height, self.world_id, proof.world_id
                ));
            }
            if index == 0 {
                if proof.block.prev_block_hash != self.trusted_anchor_block_hash {
                    return Err(format!(
                        "trusted anchor hash mismatch: anchor={} first_prev={}",
                        self.trusted_anchor_block_hash, proof.block.prev_block_hash
                    ));
                }
            } else if let Some(previous_hash) = previous_block_hash.as_deref() {
                if proof.block.prev_block_hash != previous_hash {
                    return Err(format!(
                        "prev_block_hash mismatch at height {}: expected={} actual={}",
                        proof.height, previous_hash, proof.block.prev_block_hash
                    ));
                }
            }
            if let Some(previous_timestamp) = previous_timestamp_ms {
                if proof.timestamp_ms < previous_timestamp {
                    return Err(format!("timestamp regressed at height {}", proof.height));
                }
            }
            previous_block_hash = Some(proof.head.block_hash.clone());
            previous_timestamp_ms = Some(proof.timestamp_ms);
            committed_blocks.insert(
                proof.height,
                (proof.head.block_hash.clone(), proof.head.state_root.clone()),
            );
            consensus_approvers_by_height
                .insert(proof.height, proof.consensus.approver_ids.clone());
        }

        let active_validator_sets = validate_validator_set_transitions(
            self,
            &committed_blocks,
            &consensus_approvers_by_height,
        )?;
        for (index, commitment) in self.finality_commitments.iter().enumerate() {
            let expected_height = self.from_height + index as u64;
            let active_set =
                active_validator_set_for_height(&active_validator_sets, expected_height)
                    .ok_or_else(|| {
                        format!("missing active validator set at height {expected_height}")
                    })?;
            let validators_by_id = active_set
                .validators
                .iter()
                .map(|validator| (validator.validator_id.as_str(), validator))
                .collect::<BTreeMap<_, _>>();
            let consensus_approvers = consensus_approvers_by_height
                .get(&expected_height)
                .ok_or_else(|| {
                    format!("missing consensus approvers at height {expected_height}")
                })?;
            validate_commitment(
                commitment,
                expected_height,
                active_set.validator_set_hash.as_str(),
                active_set.quorum_threshold_bps,
                &validators_by_id,
                &committed_blocks,
                consensus_approvers.as_slice(),
            )?;
        }
        validate_misbehavior_evidence(
            self.misbehavior_evidence.as_slice(),
            &active_validator_sets,
            &committed_blocks,
        )?;
        Ok(())
    }

    pub fn proof_hash(&self) -> Result<String, String> {
        self.validate_contract()?;
        canonical_blake3_hex(&(WORLD_FINALITY_PROOF_HASH_DOMAIN_V1, self))
            .map_err(|err| format!("encode world finality proof: {err}"))
    }
}

pub fn world_finality_vote_signing_payload(
    validator_id: &str,
    height: u64,
    block_hash: &str,
    state_root: &str,
    validator_set_hash: &str,
    round_id: &str,
) -> Result<Vec<u8>, serde_cbor::Error> {
    serde_cbor::to_vec(&(
        WORLD_FINALITY_VOTE_SIGNING_DOMAIN_V1,
        validator_id,
        height,
        block_hash,
        state_root,
        validator_set_hash,
        round_id,
    ))
}

pub fn world_finality_validator_set_transition_signing_payload(
    validator_id: &str,
    from_validator_set_hash: &str,
    to_validator_set_hash: &str,
    to_validator_set_activation_height: u64,
    transition_height: u64,
    transition_block_hash: &str,
) -> Result<Vec<u8>, serde_cbor::Error> {
    serde_cbor::to_vec(&(
        WORLD_FINALITY_VALIDATOR_SET_TRANSITION_SIGNING_DOMAIN_V1,
        validator_id,
        from_validator_set_hash,
        to_validator_set_hash,
        to_validator_set_activation_height,
        transition_height,
        transition_block_hash,
    ))
}

pub fn world_finality_validator_set_transition_governance_signing_payload(
    signer_id: &str,
    world_id: &str,
    governance_set_hash: &str,
    from_validator_set_hash: &str,
    to_validator_set_hash: &str,
    to_validator_set_activation_height: u64,
    transition_height: u64,
    transition_block_hash: &str,
) -> Result<Vec<u8>, serde_cbor::Error> {
    serde_cbor::to_vec(&(
        WORLD_FINALITY_VALIDATOR_SET_TRANSITION_GOVERNANCE_SIGNING_DOMAIN_V1,
        signer_id,
        world_id,
        governance_set_hash,
        from_validator_set_hash,
        to_validator_set_hash,
        to_validator_set_activation_height,
        transition_height,
        transition_block_hash,
    ))
}

#[derive(Debug, Clone)]
struct ActiveFinalityValidatorSet {
    validator_set_id: String,
    activation_height: u64,
    validator_set_hash: String,
    validators: Vec<WorldFinalityValidatorV1>,
    quorum_threshold_bps: u64,
}

pub fn compute_world_finality_validator_set_hash(
    validator_set_id: &str,
    activation_height: u64,
    quorum_threshold_bps: u64,
    validators: &[WorldFinalityValidatorV1],
) -> Result<String, String> {
    validate_validator_set(validators)?;
    canonical_blake3_hex(&(
        WORLD_FINALITY_VALIDATOR_SET_HASH_DOMAIN_V1,
        validator_set_id,
        activation_height,
        quorum_threshold_bps,
        validators,
    ))
    .map_err(|err| format!("encode world finality validator set: {err}"))
}

pub fn compute_world_finality_governance_set_hash(
    governance_set_id: &str,
    activation_height: u64,
    governance_threshold_bps: u64,
    governance_signers: &[WorldFinalityGovernanceSignerV1],
) -> Result<String, String> {
    validate_governance_signer_set(governance_signers)?;
    canonical_blake3_hex(&(
        WORLD_FINALITY_GOVERNANCE_SET_HASH_DOMAIN_V1,
        governance_set_id,
        activation_height,
        governance_threshold_bps,
        governance_signers,
    ))
    .map_err(|err| format!("encode world finality governance signer set: {err}"))
}

fn validate_validator_set(validators: &[WorldFinalityValidatorV1]) -> Result<(), String> {
    if validators.is_empty() {
        return Err("validators must not be empty".to_string());
    }
    let mut ids = BTreeSet::new();
    let mut signer_keys = BTreeSet::new();
    for (index, validator) in validators.iter().enumerate() {
        require_non_empty(
            format!("validators[{index}].validator_id").as_str(),
            &validator.validator_id,
        )?;
        if !ids.insert(validator.validator_id.as_str()) {
            return Err(format!(
                "validators[{index}].validator_id duplicates an earlier validator"
            ));
        }
        require_non_empty(
            format!("validators[{index}].finality_signer_public_key").as_str(),
            &validator.finality_signer_public_key,
        )?;
        if !signer_keys.insert(validator.finality_signer_public_key.as_str()) {
            return Err(format!(
                "validators[{index}].finality_signer_public_key duplicates an earlier validator"
            ));
        }
        if validator.stake_weight == 0 {
            return Err(format!("validators[{index}].stake_weight must be positive"));
        }
        if let Some(exit_height) = validator.exit_height {
            if exit_height <= validator.activation_height {
                return Err(format!(
                    "validators[{index}].exit_height must be greater than activation_height"
                ));
            }
        }
    }
    Ok(())
}

fn validate_governance_signer_set(
    governance_signers: &[WorldFinalityGovernanceSignerV1],
) -> Result<(), String> {
    if governance_signers.is_empty() {
        return Err("governance_signers must not be empty".to_string());
    }
    let mut ids = BTreeSet::new();
    let mut signer_keys = BTreeSet::new();
    for (index, signer) in governance_signers.iter().enumerate() {
        require_non_empty(
            format!("governance_signers[{index}].signer_id").as_str(),
            &signer.signer_id,
        )?;
        if !ids.insert(signer.signer_id.as_str()) {
            return Err(format!(
                "governance_signers[{index}].signer_id duplicates an earlier signer"
            ));
        }
        require_non_empty(
            format!("governance_signers[{index}].governance_public_key").as_str(),
            &signer.governance_public_key,
        )?;
        if !signer_keys.insert(signer.governance_public_key.as_str()) {
            return Err(format!(
                "governance_signers[{index}].governance_public_key duplicates an earlier signer"
            ));
        }
        if signer.stake_weight == 0 {
            return Err(format!(
                "governance_signers[{index}].stake_weight must be positive"
            ));
        }
        if let Some(exit_height) = signer.exit_height {
            if exit_height <= signer.activation_height {
                return Err(format!(
                    "governance_signers[{index}].exit_height must be greater than activation_height"
                ));
            }
        }
    }
    Ok(())
}

fn active_total_stake(
    validators: &[WorldFinalityValidatorV1],
    height: u64,
) -> Result<u128, String> {
    let total = validators
        .iter()
        .filter(|validator| validator.is_active_at(height))
        .map(|validator| u128::from(validator.stake_weight))
        .sum::<u128>();
    if total == 0 {
        return Err(format!("no active validator stake at height {height}"));
    }
    Ok(total)
}

fn active_validator_set_for_height(
    active_sets: &BTreeMap<u64, ActiveFinalityValidatorSet>,
    height: u64,
) -> Option<&ActiveFinalityValidatorSet> {
    active_sets.range(..=height).next_back().map(|(_, set)| set)
}

fn validate_validator_set_transitions(
    proof: &WorldFinalityProofV1,
    committed_blocks: &BTreeMap<u64, (String, String)>,
    consensus_approvers_by_height: &BTreeMap<u64, Vec<String>>,
) -> Result<BTreeMap<u64, ActiveFinalityValidatorSet>, String> {
    let mut active_sets = BTreeMap::new();
    let mut current = ActiveFinalityValidatorSet {
        validator_set_id: proof.validator_set_id.clone(),
        activation_height: proof.validator_set_activation_height,
        validator_set_hash: proof.validator_set_hash.clone(),
        validators: proof.validators.clone(),
        quorum_threshold_bps: proof.quorum_threshold_bps,
    };
    active_sets.insert(proof.from_height, current.clone());

    let mut previous_activation_height = current.activation_height;
    let mut previous_transition_height = current.activation_height.saturating_sub(1);
    for (index, transition) in proof.validator_set_transitions.iter().enumerate() {
        if transition.to_validator_set_activation_height > proof.to_height {
            return Err(format!(
                "validator_set_transitions[{index}].to_validator_set_activation_height must be inside verified range"
            ));
        }
        if transition.to_validator_set_activation_height <= previous_activation_height
            || transition.transition_height <= previous_transition_height
        {
            return Err(format!(
                "validator_set_transitions[{index}] must be in strictly increasing execution order"
            ));
        }
        previous_activation_height = transition.to_validator_set_activation_height;
        previous_transition_height = transition.transition_height;
    }

    let mut seen_activation_heights = BTreeSet::new();
    for (index, transition) in proof.validator_set_transitions.iter().enumerate() {
        validate_single_validator_set_transition(
            index,
            transition,
            &current,
            proof.world_id.as_str(),
            committed_blocks,
            consensus_approvers_by_height,
        )?;
        if !seen_activation_heights.insert(transition.to_validator_set_activation_height) {
            return Err(format!(
                "validator_set_transitions[{index}] duplicates an activation height"
            ));
        }
        current = ActiveFinalityValidatorSet {
            validator_set_id: transition.to_validator_set_id.clone(),
            activation_height: transition.to_validator_set_activation_height,
            validator_set_hash: transition.to_validator_set_hash.clone(),
            validators: transition.to_validators.clone(),
            quorum_threshold_bps: transition.to_quorum_threshold_bps,
        };
        active_sets.insert(
            transition.to_validator_set_activation_height,
            current.clone(),
        );
    }
    Ok(active_sets)
}

fn validate_single_validator_set_transition(
    index: usize,
    transition: &WorldFinalityValidatorSetTransitionV1,
    current: &ActiveFinalityValidatorSet,
    world_id: &str,
    committed_blocks: &BTreeMap<u64, (String, String)>,
    consensus_approvers_by_height: &BTreeMap<u64, Vec<String>>,
) -> Result<(), String> {
    require_non_empty(
        format!("validator_set_transitions[{index}].from_validator_set_id").as_str(),
        &transition.from_validator_set_id,
    )?;
    require_non_empty(
        format!("validator_set_transitions[{index}].to_validator_set_id").as_str(),
        &transition.to_validator_set_id,
    )?;
    if transition.from_validator_set_id != current.validator_set_id {
        return Err(format!(
            "validator_set_transitions[{index}].from_validator_set_id must match active set"
        ));
    }
    if transition.from_validator_set_hash != current.validator_set_hash {
        return Err(format!(
            "validator_set_transitions[{index}].from_validator_set_hash must match active set"
        ));
    }
    if !(5_001..=10_000).contains(&transition.to_quorum_threshold_bps) {
        return Err(format!(
            "validator_set_transitions[{index}].to_quorum_threshold_bps must be in 5001..=10000"
        ));
    }
    if transition.to_validator_set_activation_height != transition.transition_height + 1 {
        return Err(format!(
            "validator_set_transitions[{index}].to_validator_set_activation_height must be transition_height + 1"
        ));
    }
    if transition.to_validator_set_activation_height <= current.activation_height {
        return Err(format!(
            "validator_set_transitions[{index}].to_validator_set_activation_height must advance active set height"
        ));
    }
    let (transition_block_hash, _) = committed_blocks
        .get(&transition.transition_height)
        .ok_or_else(|| {
            format!("validator_set_transitions[{index}].transition_height outside verified range")
        })?;
    if transition.transition_block_hash != *transition_block_hash {
        return Err(format!(
            "validator_set_transitions[{index}].transition_block_hash mismatch"
        ));
    }
    let computed_to_hash = compute_world_finality_validator_set_hash(
        transition.to_validator_set_id.as_str(),
        transition.to_validator_set_activation_height,
        transition.to_quorum_threshold_bps,
        transition.to_validators.as_slice(),
    )?;
    if transition.to_validator_set_hash != computed_to_hash {
        return Err(format!(
            "validator_set_transitions[{index}] transition to_validator_set_hash mismatch"
        ));
    }

    let current_validators_by_id = current
        .validators
        .iter()
        .map(|validator| (validator.validator_id.as_str(), validator))
        .collect::<BTreeMap<_, _>>();
    let consensus_approvers = consensus_approvers_by_height
        .get(&transition.transition_height)
        .ok_or_else(|| {
            format!(
                "validator_set_transitions[{index}] missing consensus approvers at transition height"
            )
        })?;
    validate_transition_approvals(
        index,
        transition,
        &current_validators_by_id,
        consensus_approvers.as_slice(),
        current.quorum_threshold_bps,
        current.validators.as_slice(),
    )?;
    validate_transition_governance_certificate(index, transition, world_id)
}

fn validate_transition_governance_certificate(
    transition_index: usize,
    transition: &WorldFinalityValidatorSetTransitionV1,
    world_id: &str,
) -> Result<(), String> {
    let certificate = transition.governance_certificate.as_ref().ok_or_else(|| {
        format!(
            "validator_set_transitions[{transition_index}].governance_certificate required for trust-minimized transition governance"
        )
    })?;
    require_non_empty(
        format!(
            "validator_set_transitions[{transition_index}].governance_certificate.governance_set_id"
        )
        .as_str(),
        &certificate.governance_set_id,
    )?;
    if certificate.governance_set_activation_height > transition.transition_height {
        return Err(format!(
            "validator_set_transitions[{transition_index}].governance_certificate.governance_set_activation_height must be <= transition_height"
        ));
    }
    if !(5_001..=10_000).contains(&certificate.governance_threshold_bps) {
        return Err(format!(
            "validator_set_transitions[{transition_index}].governance_certificate.governance_threshold_bps must be in 5001..=10000"
        ));
    }
    let computed_hash = compute_world_finality_governance_set_hash(
        certificate.governance_set_id.as_str(),
        certificate.governance_set_activation_height,
        certificate.governance_threshold_bps,
        certificate.governance_signers.as_slice(),
    )?;
    if certificate.governance_set_hash != computed_hash {
        return Err(format!(
            "validator_set_transitions[{transition_index}].governance_certificate.governance_set_hash mismatch"
        ));
    }
    if certificate.governance_approvals.is_empty() {
        return Err(format!(
            "validator_set_transitions[{transition_index}].governance_certificate.governance_approvals must not be empty"
        ));
    }
    let governance_signers_by_id = certificate
        .governance_signers
        .iter()
        .map(|signer| (signer.signer_id.as_str(), signer))
        .collect::<BTreeMap<_, _>>();
    let mut seen_approvers = BTreeSet::new();
    let mut signed_stake = 0_u128;
    for (index, approval) in certificate.governance_approvals.iter().enumerate() {
        require_non_empty(
            format!(
                "validator_set_transitions[{transition_index}].governance_certificate.governance_approvals[{index}].signer_id"
            )
            .as_str(),
            &approval.signer_id,
        )?;
        if !seen_approvers.insert(approval.signer_id.as_str()) {
            return Err(format!(
                "validator_set_transitions[{transition_index}].governance_certificate.governance_approvals[{index}] duplicate signer"
            ));
        }
        if approval.governance_set_hash != certificate.governance_set_hash
            || approval.from_validator_set_hash != transition.from_validator_set_hash
            || approval.to_validator_set_hash != transition.to_validator_set_hash
            || approval.to_validator_set_activation_height
                != transition.to_validator_set_activation_height
            || approval.transition_height != transition.transition_height
            || approval.transition_block_hash != transition.transition_block_hash
        {
            return Err(format!(
                "validator_set_transitions[{transition_index}].governance_certificate.governance_approvals[{index}] target mismatch"
            ));
        }
        let signer = governance_signers_by_id
            .get(approval.signer_id.as_str())
            .ok_or_else(|| {
                format!(
                    "validator_set_transitions[{transition_index}].governance_certificate.governance_approvals[{index}] signer not in governance set"
                )
            })?;
        if !signer.is_active_at(transition.transition_height) {
            return Err(format!(
                "validator_set_transitions[{transition_index}].governance_certificate.governance_approvals[{index}] signer not active"
            ));
        }
        if approval.signature_scheme != "ed25519" {
            return Err(format!(
                "unsupported transition governance approval signature_scheme: {}",
                approval.signature_scheme
            ));
        }
        let payload = world_finality_validator_set_transition_governance_signing_payload(
            approval.signer_id.as_str(),
            world_id,
            approval.governance_set_hash.as_str(),
            approval.from_validator_set_hash.as_str(),
            approval.to_validator_set_hash.as_str(),
            approval.to_validator_set_activation_height,
            approval.transition_height,
            approval.transition_block_hash.as_str(),
        )
        .map_err(|err| format!("encode transition governance approval signing payload: {err}"))?;
        verify_ed25519_signature(
            signer.governance_public_key.as_str(),
            approval.signature_hex.as_str(),
            payload.as_slice(),
            "transition governance approval signature",
        )?;
        signed_stake += u128::from(signer.stake_weight);
    }
    let total_stake = certificate
        .governance_signers
        .iter()
        .filter(|signer| signer.is_active_at(transition.transition_height))
        .map(|signer| u128::from(signer.stake_weight))
        .sum::<u128>();
    if total_stake == 0 {
        return Err(format!(
            "validator_set_transitions[{transition_index}].governance_certificate has no active governance stake"
        ));
    }
    if signed_stake * 10_000 < total_stake * u128::from(certificate.governance_threshold_bps) {
        return Err(format!(
            "validator_set_transitions[{transition_index}].governance_certificate approval stake below threshold: signed={signed_stake} total={total_stake} threshold_bps={}",
            certificate.governance_threshold_bps
        ));
    }
    Ok(())
}

fn validate_transition_approvals(
    transition_index: usize,
    transition: &WorldFinalityValidatorSetTransitionV1,
    validators_by_id: &BTreeMap<&str, &WorldFinalityValidatorV1>,
    consensus_approvers: &[String],
    quorum_threshold_bps: u64,
    validators: &[WorldFinalityValidatorV1],
) -> Result<(), String> {
    if transition.approvals.is_empty() {
        return Err(format!(
            "validator_set_transitions[{transition_index}].approvals must not be empty"
        ));
    }
    let consensus_approver_set = consensus_approvers
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let mut seen_approvers = BTreeSet::new();
    let mut signed_stake = 0_u128;
    for (index, approval) in transition.approvals.iter().enumerate() {
        require_non_empty(
            format!(
                "validator_set_transitions[{transition_index}].approvals[{index}].validator_id"
            )
            .as_str(),
            &approval.validator_id,
        )?;
        if !seen_approvers.insert(approval.validator_id.as_str()) {
            return Err(format!(
                "validator_set_transitions[{transition_index}].approvals[{index}] duplicate validator"
            ));
        }
        if approval.from_validator_set_hash != transition.from_validator_set_hash
            || approval.to_validator_set_hash != transition.to_validator_set_hash
            || approval.to_validator_set_activation_height
                != transition.to_validator_set_activation_height
            || approval.transition_height != transition.transition_height
            || approval.transition_block_hash != transition.transition_block_hash
        {
            return Err(format!(
                "validator_set_transitions[{transition_index}].approvals[{index}] target mismatch"
            ));
        }
        let validator = validators_by_id
            .get(approval.validator_id.as_str())
            .ok_or_else(|| {
                format!(
                    "validator_set_transitions[{transition_index}].approvals[{index}] validator not in active set"
                )
            })?;
        if !validator.is_active_at(transition.transition_height) {
            return Err(format!(
                "validator_set_transitions[{transition_index}].approvals[{index}] validator not active"
            ));
        }
        if !consensus_approver_set.contains(approval.validator_id.as_str()) {
            return Err(format!(
                "validator_set_transitions[{transition_index}].approvals[{index}] validator was not a consensus approver"
            ));
        }
        if approval.signature_scheme != "ed25519" {
            return Err(format!(
                "unsupported transition approval signature_scheme: {}",
                approval.signature_scheme
            ));
        }
        let payload = world_finality_validator_set_transition_signing_payload(
            approval.validator_id.as_str(),
            approval.from_validator_set_hash.as_str(),
            approval.to_validator_set_hash.as_str(),
            approval.to_validator_set_activation_height,
            approval.transition_height,
            approval.transition_block_hash.as_str(),
        )
        .map_err(|err| format!("encode transition approval signing payload: {err}"))?;
        verify_ed25519_signature(
            validator.finality_signer_public_key.as_str(),
            approval.signature_hex.as_str(),
            payload.as_slice(),
            "transition approval signature",
        )?;
        signed_stake += u128::from(validator.stake_weight);
    }
    let total_stake = active_total_stake(validators, transition.transition_height)?;
    if signed_stake * 10_000 < total_stake * u128::from(quorum_threshold_bps) {
        return Err(format!(
            "validator_set_transitions[{transition_index}] approval stake below threshold: signed={signed_stake} total={total_stake} threshold_bps={quorum_threshold_bps}"
        ));
    }
    Ok(())
}

fn validate_commitment(
    commitment: &WorldFinalityCommitmentV1,
    expected_height: u64,
    validator_set_hash: &str,
    quorum_threshold_bps: u64,
    validators_by_id: &BTreeMap<&str, &WorldFinalityValidatorV1>,
    committed_blocks: &BTreeMap<u64, (String, String)>,
    consensus_approvers: &[String],
) -> Result<(), String> {
    if commitment.height != expected_height {
        return Err(format!(
            "finality commitment height mismatch: expected={} actual={}",
            expected_height, commitment.height
        ));
    }
    require_non_empty("finality commitment round_id", &commitment.round_id)?;
    if commitment.validator_set_hash != validator_set_hash {
        return Err(format!(
            "finality commitment validator_set_hash mismatch: expected={} actual={}",
            validator_set_hash, commitment.validator_set_hash
        ));
    }
    if commitment.quorum_threshold_bps != quorum_threshold_bps {
        return Err(format!(
            "finality commitment quorum_threshold_bps mismatch: expected={} actual={}",
            quorum_threshold_bps, commitment.quorum_threshold_bps
        ));
    }
    let (committed_block_hash, committed_state_root) = committed_blocks
        .get(&commitment.height)
        .ok_or_else(|| format!("missing committed head at height {}", commitment.height))?;
    if &commitment.block_hash != committed_block_hash {
        return Err(format!(
            "finality commitment block_hash mismatch at height {}: commitment={} head={}",
            commitment.height, commitment.block_hash, committed_block_hash
        ));
    }
    if &commitment.state_root != committed_state_root {
        return Err(format!(
            "finality commitment state_root mismatch at height {}: commitment={} head={}",
            commitment.height, commitment.state_root, committed_state_root
        ));
    }
    if commitment.votes.is_empty() {
        return Err(format!(
            "finality commitment at height {} must include votes",
            commitment.height
        ));
    }
    let consensus_approver_set = consensus_approvers
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let mut seen_voters = BTreeSet::new();
    let mut signed_stake = 0_u128;
    for (index, vote) in commitment.votes.iter().enumerate() {
        require_non_empty(
            format!("finality vote[{index}].validator_id").as_str(),
            &vote.validator_id,
        )?;
        if !seen_voters.insert(vote.validator_id.as_str()) {
            return Err(format!(
                "finality commitment height {} duplicate validator vote: {}",
                commitment.height, vote.validator_id
            ));
        }
        let validator = validators_by_id
            .get(vote.validator_id.as_str())
            .ok_or_else(|| format!("finality vote validator not in set: {}", vote.validator_id))?;
        if !validator.is_active_at(commitment.height) {
            return Err(format!(
                "finality vote validator is not active at height {}: {}",
                commitment.height, vote.validator_id
            ));
        }
        if !consensus_approver_set.contains(vote.validator_id.as_str()) {
            return Err(format!(
                "finality vote validator was not a consensus approver at height {}: {}",
                commitment.height, vote.validator_id
            ));
        }
        if vote.height != commitment.height
            || vote.block_hash != commitment.block_hash
            || vote.state_root != commitment.state_root
        {
            return Err(format!(
                "finality vote target mismatch at height {} for {}",
                commitment.height, vote.validator_id
            ));
        }
        if vote.signature_scheme != "ed25519" {
            return Err(format!(
                "unsupported finality vote signature_scheme: {}",
                vote.signature_scheme
            ));
        }
        require_non_empty(
            format!("finality vote[{index}].signature_evidence_hash").as_str(),
            &vote.signature_evidence_hash,
        )?;
        require_non_empty(
            format!("finality vote[{index}].signature_hex").as_str(),
            &vote.signature_hex,
        )?;
        let payload = world_finality_vote_signing_payload(
            vote.validator_id.as_str(),
            vote.height,
            vote.block_hash.as_str(),
            vote.state_root.as_str(),
            commitment.validator_set_hash.as_str(),
            commitment.round_id.as_str(),
        )
        .map_err(|err| format!("encode finality vote signing payload: {err}"))?;
        let payload_hash = canonical_blake3_hex(&payload)
            .map_err(|err| format!("encode finality vote signature evidence hash: {err}"))?;
        if vote.signature_evidence_hash != payload_hash {
            return Err(format!(
                "finality vote signature_evidence_hash mismatch at height {} for {}",
                commitment.height, vote.validator_id
            ));
        }
        verify_ed25519_signature(
            validator.finality_signer_public_key.as_str(),
            vote.signature_hex.as_str(),
            payload.as_slice(),
            "finality vote signature",
        )?;
        signed_stake += u128::from(validator.stake_weight);
    }
    let active_validators = validators_by_id
        .values()
        .copied()
        .cloned()
        .collect::<Vec<_>>();
    let total_stake = active_total_stake(active_validators.as_slice(), commitment.height)?;
    if signed_stake * 10_000 < total_stake * u128::from(quorum_threshold_bps) {
        return Err(format!(
            "finality signed stake below threshold at height {}: signed={} total={} threshold_bps={}",
            commitment.height, signed_stake, total_stake, quorum_threshold_bps
        ));
    }
    Ok(())
}

fn validate_misbehavior_evidence(
    evidence: &[WorldFinalityMisbehaviorEvidenceV1],
    active_validator_sets: &BTreeMap<u64, ActiveFinalityValidatorSet>,
    committed_blocks: &BTreeMap<u64, (String, String)>,
) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    for (index, item) in evidence.iter().enumerate() {
        if !seen.insert((
            item.height,
            item.validator_id.as_str(),
            item.evidence_kind.as_str(),
        )) {
            return Err(format!(
                "misbehavior_evidence[{index}] duplicates an earlier evidence key"
            ));
        }
        let active_set = active_validator_set_for_height(active_validator_sets, item.height)
            .ok_or_else(|| {
                format!("misbehavior_evidence[{index}].height is outside verified range")
            })?;
        let validators_by_id = active_set
            .validators
            .iter()
            .map(|validator| (validator.validator_id.as_str(), validator))
            .collect::<BTreeMap<_, _>>();
        let validator = validators_by_id
            .get(item.validator_id.as_str())
            .ok_or_else(|| {
                format!("misbehavior_evidence[{index}].validator_id not in active validator set")
            })?;
        if !validator.is_active_at(item.height) {
            return Err(format!(
                "misbehavior_evidence[{index}].validator_id not active at evidence height"
            ));
        }
        if !matches!(
            item.evidence_kind.as_str(),
            "conflicting_head" | "double_finality_vote" | "fork_vote"
        ) {
            return Err(format!(
                "misbehavior_evidence[{index}].evidence_kind unsupported: {}",
                item.evidence_kind
            ));
        }
        require_non_empty(
            format!("misbehavior_evidence[{index}].conflicting_block_hash").as_str(),
            &item.conflicting_block_hash,
        )?;
        require_non_empty(
            format!("misbehavior_evidence[{index}].conflicting_proof_hash").as_str(),
            &item.conflicting_proof_hash,
        )?;
        if !matches!(item.disposition.as_str(), "rejected" | "slashing_candidate") {
            return Err(format!(
                "misbehavior_evidence[{index}].disposition must be rejected or slashing_candidate"
            ));
        }
        let (committed_block_hash, _) = committed_blocks.get(&item.height).ok_or_else(|| {
            format!("misbehavior_evidence[{index}].height is outside verified range")
        })?;
        if &item.conflicting_block_hash == committed_block_hash {
            return Err(format!(
                "misbehavior_evidence[{index}].conflicting_block_hash must differ from committed head"
            ));
        }
    }
    Ok(())
}

fn require_non_empty(label: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{label} must not be empty"));
    }
    Ok(())
}

fn verify_ed25519_signature(
    public_key_hex: &str,
    signature_hex: &str,
    payload: &[u8],
    label: &str,
) -> Result<(), String> {
    let public_key_bytes =
        decode_hex_array::<32>(public_key_hex, format!("{label} public key").as_str())?;
    let signature_bytes =
        decode_hex_array::<64>(signature_hex, format!("{label} signature").as_str())?;
    let verifying_key = VerifyingKey::from_bytes(&public_key_bytes)
        .map_err(|err| format!("{label} public key invalid: {err}"))?;
    let signature = Signature::from_bytes(&signature_bytes);
    verifying_key
        .verify(payload, &signature)
        .map_err(|err| format!("{label} verification failed: {err}"))
}

fn decode_hex_array<const N: usize>(raw: &str, label: &str) -> Result<[u8; N], String> {
    let bytes =
        hex::decode(raw.trim()).map_err(|err| format!("{label} hex decode failed: {err}"))?;
    bytes
        .try_into()
        .map_err(|bytes: Vec<u8>| format!("{label} must be {N} bytes, got {}", bytes.len()))
}

fn canonical_blake3_hex<T: Serialize>(value: &T) -> Result<String, serde_cbor::Error> {
    let payload = serde_cbor::to_vec(value)?;
    Ok(blake3::hash(&payload).to_hex().to_string())
}

#[cfg(test)]
#[path = "distributed_finality_tests.rs"]
mod distributed_finality_tests;
