//! Bounded validator-set/finality proof evidence anchored to `WorldHeadProofV1`.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::distributed::WorldHeadProofV1;

pub const WORLD_FINALITY_PROOF_V1_SCHEMA: u16 = 1;
pub const WORLD_FINALITY_PROOF_HASH_DOMAIN_V1: &str = "oasis7.world_finality_proof.v1";
pub const WORLD_FINALITY_VALIDATOR_SET_HASH_DOMAIN_V1: &str =
    "oasis7.world_finality_validator_set.v1";
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
    pub misbehavior_evidence: Vec<WorldFinalityMisbehaviorEvidenceV1>,
    #[serde(default = "world_finality_proof_claim_boundary_v1")]
    pub claim_boundary: String,
}

impl WorldFinalityValidatorV1 {
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

        let validators_by_id = self
            .validators
            .iter()
            .map(|validator| (validator.validator_id.as_str(), validator))
            .collect::<BTreeMap<_, _>>();
        let mut previous_block_hash: Option<String> = None;
        let mut previous_timestamp_ms: Option<i64> = None;
        let mut committed_blocks = BTreeMap::new();

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
        }

        let active_total_stake_by_height = active_total_stake_by_height(
            self.validators.as_slice(),
            self.from_height,
            self.to_height,
        )?;
        for (index, commitment) in self.finality_commitments.iter().enumerate() {
            let expected_height = self.from_height + index as u64;
            validate_commitment(
                commitment,
                expected_height,
                self.validator_set_hash.as_str(),
                self.quorum_threshold_bps,
                &validators_by_id,
                &active_total_stake_by_height,
                &committed_blocks,
                self.head_proofs[index].consensus.approver_ids.as_slice(),
            )?;
        }
        validate_misbehavior_evidence(
            self.misbehavior_evidence.as_slice(),
            &validators_by_id,
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

fn active_total_stake_by_height(
    validators: &[WorldFinalityValidatorV1],
    from_height: u64,
    to_height: u64,
) -> Result<BTreeMap<u64, u128>, String> {
    let mut totals = BTreeMap::new();
    for height in from_height..=to_height {
        let total = validators
            .iter()
            .filter(|validator| validator.is_active_at(height))
            .map(|validator| u128::from(validator.stake_weight))
            .sum::<u128>();
        if total == 0 {
            return Err(format!("no active validator stake at height {height}"));
        }
        totals.insert(height, total);
    }
    Ok(totals)
}

fn validate_commitment(
    commitment: &WorldFinalityCommitmentV1,
    expected_height: u64,
    validator_set_hash: &str,
    quorum_threshold_bps: u64,
    validators_by_id: &BTreeMap<&str, &WorldFinalityValidatorV1>,
    active_total_stake_by_height: &BTreeMap<u64, u128>,
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
        if vote.signature_scheme != "ed25519_evidence_hash_v1" {
            return Err(format!(
                "unsupported finality vote signature_scheme: {}",
                vote.signature_scheme
            ));
        }
        require_non_empty(
            format!("finality vote[{index}].signature_evidence_hash").as_str(),
            &vote.signature_evidence_hash,
        )?;
        signed_stake += u128::from(validator.stake_weight);
    }
    let total_stake = active_total_stake_by_height
        .get(&commitment.height)
        .copied()
        .ok_or_else(|| format!("missing active stake at height {}", commitment.height))?;
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
    validators_by_id: &BTreeMap<&str, &WorldFinalityValidatorV1>,
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
        if !validators_by_id.contains_key(item.validator_id.as_str()) {
            return Err(format!(
                "misbehavior_evidence[{index}].validator_id not in validator set"
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

fn canonical_blake3_hex<T: Serialize>(value: &T) -> Result<String, serde_cbor::Error> {
    let payload = serde_cbor::to_vec(value)?;
    Ok(blake3::hash(&payload).to_hex().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::distributed::{
        BlobRef, CheckpointClosureEvidenceV1, ExecutionBindingEvidenceV1, HeadConsensusEvidenceV1,
        WIRE_ENCODING_CBOR, WORLD_HEAD_PROOF_CLAIM_BOUNDARY_V1, WORLD_HEAD_PROOF_V1_SCHEMA,
        WorldBlock, WorldHeadAnnounce,
    };

    fn sample_world_head_proof_at(height: u64, prev_block_hash: &str) -> WorldHeadProofV1 {
        let state_root = format!("state-root-{height}");
        let action_root = format!("action-root-{height}");
        let journal_ref = format!("journal-ref-{height}");
        let snapshot_ref = format!("snapshot-ref-{height}");
        let block = WorldBlock {
            world_id: "world-a".to_string(),
            height,
            prev_block_hash: prev_block_hash.to_string(),
            action_root: action_root.clone(),
            event_root: format!("event-root-{height}"),
            state_root: state_root.clone(),
            journal_ref: journal_ref.clone(),
            snapshot_ref: snapshot_ref.clone(),
            receipts_root: format!("receipts-root-{height}"),
            proposer_id: "validator-a".to_string(),
            timestamp_ms: 1_772_467_200_000 + height as i64,
            signature: "block-signature-evidence-only".to_string(),
        };
        let block_hash = canonical_blake3_hex(&block).expect("block hash");
        WorldHeadProofV1 {
            schema_version: WORLD_HEAD_PROOF_V1_SCHEMA,
            world_id: "world-a".to_string(),
            height,
            timestamp_ms: 1_772_467_200_000 + height as i64,
            head: WorldHeadAnnounce {
                world_id: "world-a".to_string(),
                height,
                block_hash: block_hash.clone(),
                state_root: state_root.clone(),
                timestamp_ms: 1_772_467_200_000 + height as i64,
                signature: "head-signature-evidence-only".to_string(),
            },
            block,
            snapshot_manifest_ref: BlobRef {
                content_hash: snapshot_ref.clone(),
                size_bytes: 120,
                codec: WIRE_ENCODING_CBOR.to_string(),
                links: vec!["snapshot-chunk-1".to_string()],
            },
            journal_segments_ref: BlobRef {
                content_hash: journal_ref.clone(),
                size_bytes: 80,
                codec: WIRE_ENCODING_CBOR.to_string(),
                links: vec!["journal-segment-1".to_string()],
            },
            consensus: HeadConsensusEvidenceV1 {
                consensus_status: "committed".to_string(),
                proposer_id: "validator-a".to_string(),
                quorum_threshold: 2,
                validator_count: 3,
                vote_count: 2,
                approver_ids: vec!["validator-a".to_string(), "validator-b".to_string()],
                evidence_hash: format!("consensus-evidence-{height}"),
            },
            execution: ExecutionBindingEvidenceV1 {
                execution_height: height,
                node_block_hash: block_hash,
                execution_block_hash: format!("execution-block-{height}"),
                execution_state_root: state_root.clone(),
                action_root,
            },
            checkpoint: Some(CheckpointClosureEvidenceV1 {
                checkpoint_height: height,
                execution_block_hash: format!("execution-block-{height}"),
                execution_state_root: state_root.clone(),
                manifest_ref: format!("checkpoint-manifest-{height}"),
                manifest_hash: format!("checkpoint-manifest-hash-{height}"),
                pinned_refs: vec![snapshot_ref, journal_ref, state_root],
            }),
            claim_boundary: WORLD_HEAD_PROOF_CLAIM_BOUNDARY_V1.to_string(),
        }
    }

    fn sample_valid_finality_proof() -> WorldFinalityProofV1 {
        let first = sample_world_head_proof_at(40, "prev-block-39");
        let second = sample_world_head_proof_at(41, first.head.block_hash.as_str());
        let third = sample_world_head_proof_at(42, second.head.block_hash.as_str());
        let validators = vec![
            WorldFinalityValidatorV1 {
                validator_id: "validator-a".to_string(),
                stake_weight: 34,
                finality_signer_public_key: "finality-pk-a".to_string(),
                activation_height: 1,
                exit_height: None,
            },
            WorldFinalityValidatorV1 {
                validator_id: "validator-b".to_string(),
                stake_weight: 33,
                finality_signer_public_key: "finality-pk-b".to_string(),
                activation_height: 1,
                exit_height: None,
            },
            WorldFinalityValidatorV1 {
                validator_id: "validator-c".to_string(),
                stake_weight: 33,
                finality_signer_public_key: "finality-pk-c".to_string(),
                activation_height: 1,
                exit_height: None,
            },
        ];
        let validator_set_hash =
            compute_world_finality_validator_set_hash("sample-set-1", 1, 6_667, &validators)
                .expect("validator set hash");
        let head_proofs = vec![first, second, third];
        let finality_commitments = head_proofs
            .iter()
            .map(|proof| WorldFinalityCommitmentV1 {
                height: proof.height,
                round_id: format!("round-{}", proof.height),
                block_hash: proof.head.block_hash.clone(),
                state_root: proof.head.state_root.clone(),
                validator_set_hash: validator_set_hash.clone(),
                quorum_threshold_bps: 6_667,
                votes: vec![
                    WorldFinalityVoteV1 {
                        validator_id: "validator-a".to_string(),
                        height: proof.height,
                        block_hash: proof.head.block_hash.clone(),
                        state_root: proof.head.state_root.clone(),
                        signature_scheme: "ed25519_evidence_hash_v1".to_string(),
                        signature_evidence_hash: format!("sig-a-{}", proof.height),
                    },
                    WorldFinalityVoteV1 {
                        validator_id: "validator-b".to_string(),
                        height: proof.height,
                        block_hash: proof.head.block_hash.clone(),
                        state_root: proof.head.state_root.clone(),
                        signature_scheme: "ed25519_evidence_hash_v1".to_string(),
                        signature_evidence_hash: format!("sig-b-{}", proof.height),
                    },
                ],
            })
            .collect();
        WorldFinalityProofV1 {
            schema_version: WORLD_FINALITY_PROOF_V1_SCHEMA,
            world_id: "world-a".to_string(),
            from_height: 40,
            to_height: 42,
            trusted_anchor_height: 39,
            trusted_anchor_block_hash: "prev-block-39".to_string(),
            validator_set_id: "sample-set-1".to_string(),
            validator_set_activation_height: 1,
            validator_set_hash,
            validators,
            quorum_threshold_bps: 6_667,
            head_proofs,
            finality_commitments,
            misbehavior_evidence: vec![WorldFinalityMisbehaviorEvidenceV1 {
                height: 42,
                validator_id: "validator-c".to_string(),
                evidence_kind: "conflicting_head".to_string(),
                conflicting_block_hash: "conflicting-block-42".to_string(),
                conflicting_proof_hash: "conflicting-proof-42".to_string(),
                disposition: "rejected".to_string(),
            }],
            claim_boundary: WORLD_FINALITY_PROOF_CLAIM_BOUNDARY_V1.to_string(),
        }
    }

    #[test]
    fn world_finality_proof_v1_validates_contract_and_hash() {
        let proof = sample_valid_finality_proof();
        proof.validate_contract().expect("valid finality proof");
        let proof_hash = proof.proof_hash().expect("proof hash");
        assert!(!proof_hash.is_empty());
    }

    #[test]
    fn world_finality_proof_v1_rejects_below_stake_threshold() {
        let mut proof = sample_valid_finality_proof();
        for commitment in &mut proof.finality_commitments {
            commitment.votes.pop();
        }

        let err = proof
            .validate_contract()
            .expect_err("below threshold rejected");
        assert!(err.contains("signed stake below threshold"), "{err}");
    }

    #[test]
    fn world_finality_proof_v1_rejects_large_stake_below_threshold_without_overflow() {
        let mut proof = sample_valid_finality_proof();
        proof.validators[0].stake_weight = u64::MAX;
        proof.validators[1].stake_weight = u64::MAX;
        proof.validators[2].stake_weight = 1;
        proof.validator_set_hash = compute_world_finality_validator_set_hash(
            proof.validator_set_id.as_str(),
            proof.validator_set_activation_height,
            proof.quorum_threshold_bps,
            proof.validators.as_slice(),
        )
        .expect("large validator set hash");
        for commitment in &mut proof.finality_commitments {
            commitment.validator_set_hash = proof.validator_set_hash.clone();
            commitment.votes.truncate(1);
        }

        let err = proof
            .validate_contract()
            .expect_err("large below-threshold stake rejected");
        assert!(err.contains("signed stake below threshold"), "{err}");
    }

    #[test]
    fn world_finality_proof_v1_rejects_vote_not_in_consensus_approvers() {
        let mut proof = sample_valid_finality_proof();
        proof.finality_commitments[0].votes[1].validator_id = "validator-c".to_string();
        proof.finality_commitments[0].votes[1].signature_evidence_hash = "sig-c-40".to_string();

        let err = proof
            .validate_contract()
            .expect_err("non consensus approver rejected");
        assert!(
            err.contains("was not a consensus approver at height 40"),
            "{err}"
        );
    }

    #[test]
    fn world_finality_proof_v1_rejects_validator_set_hash_tamper() {
        let mut proof = sample_valid_finality_proof();
        proof.validator_set_hash = "wrong-validator-set-hash".to_string();

        let err = proof
            .validate_contract()
            .expect_err("validator set hash tamper rejected");
        assert!(err.contains("validator set hash mismatch"), "{err}");
    }

    #[test]
    fn world_finality_proof_v1_rejects_conflicting_hash_equal_to_committed_head() {
        let mut proof = sample_valid_finality_proof();
        proof.misbehavior_evidence[0].conflicting_block_hash =
            proof.head_proofs[2].head.block_hash.clone();

        let err = proof
            .validate_contract()
            .expect_err("same conflicting hash rejected");
        assert!(
            err.contains("conflicting_block_hash must differ from committed head"),
            "{err}"
        );
    }

    #[test]
    fn world_finality_proof_v1_rejects_misbehavior_height_outside_verified_range() {
        let mut proof = sample_valid_finality_proof();
        proof.misbehavior_evidence[0].height = 99;

        let err = proof
            .validate_contract()
            .expect_err("out-of-window misbehavior evidence rejected");
        assert!(err.contains("height is outside verified range"), "{err}");
    }
}
