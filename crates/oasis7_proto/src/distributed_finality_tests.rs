use super::*;
use crate::distributed::{
    BlobRef, CheckpointClosureEvidenceV1, ExecutionBindingEvidenceV1, HeadConsensusEvidenceV1,
    WIRE_ENCODING_CBOR, WORLD_HEAD_PROOF_CLAIM_BOUNDARY_V1, WORLD_HEAD_PROOF_V1_SCHEMA, WorldBlock,
    WorldHeadAnnounce,
};
use ed25519_dalek::{Signer, SigningKey};

fn validator_keys() -> BTreeMap<&'static str, SigningKey> {
    [
        ("validator-a", SigningKey::from_bytes(&[1_u8; 32])),
        ("validator-b", SigningKey::from_bytes(&[2_u8; 32])),
        ("validator-c", SigningKey::from_bytes(&[3_u8; 32])),
        ("validator-d", SigningKey::from_bytes(&[4_u8; 32])),
    ]
    .into_iter()
    .collect()
}

fn governance_keys() -> BTreeMap<&'static str, SigningKey> {
    [
        ("governance-a", SigningKey::from_bytes(&[11_u8; 32])),
        ("governance-b", SigningKey::from_bytes(&[12_u8; 32])),
        ("governance-c", SigningKey::from_bytes(&[13_u8; 32])),
    ]
    .into_iter()
    .collect()
}

fn public_key_hex(key: &SigningKey) -> String {
    hex::encode(key.verifying_key().to_bytes())
}

fn signed_finality_vote(
    key: &SigningKey,
    validator_id: &str,
    commitment: &WorldFinalityCommitmentV1,
) -> WorldFinalityVoteV1 {
    let payload = world_finality_vote_signing_payload(
        validator_id,
        commitment.height,
        commitment.block_hash.as_str(),
        commitment.state_root.as_str(),
        commitment.validator_set_hash.as_str(),
        commitment.round_id.as_str(),
    )
    .expect("vote signing payload");
    WorldFinalityVoteV1 {
        validator_id: validator_id.to_string(),
        height: commitment.height,
        block_hash: commitment.block_hash.clone(),
        state_root: commitment.state_root.clone(),
        signature_scheme: "ed25519".to_string(),
        signature_evidence_hash: canonical_blake3_hex(&payload).expect("signature evidence hash"),
        signature_hex: hex::encode(key.sign(payload.as_slice()).to_bytes()),
    }
}

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
    let keys = validator_keys();
    let validators = vec![
        WorldFinalityValidatorV1 {
            validator_id: "validator-a".to_string(),
            stake_weight: 34,
            finality_signer_public_key: public_key_hex(&keys["validator-a"]),
            activation_height: 1,
            exit_height: None,
        },
        WorldFinalityValidatorV1 {
            validator_id: "validator-b".to_string(),
            stake_weight: 33,
            finality_signer_public_key: public_key_hex(&keys["validator-b"]),
            activation_height: 1,
            exit_height: None,
        },
        WorldFinalityValidatorV1 {
            validator_id: "validator-c".to_string(),
            stake_weight: 33,
            finality_signer_public_key: public_key_hex(&keys["validator-c"]),
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
        .map(|proof| {
            let mut commitment = WorldFinalityCommitmentV1 {
                height: proof.height,
                round_id: format!("round-{}", proof.height),
                block_hash: proof.head.block_hash.clone(),
                state_root: proof.head.state_root.clone(),
                validator_set_hash: validator_set_hash.clone(),
                quorum_threshold_bps: 6_667,
                votes: vec![],
            };
            commitment.votes = vec![
                signed_finality_vote(&keys["validator-a"], "validator-a", &commitment),
                signed_finality_vote(&keys["validator-b"], "validator-b", &commitment),
            ];
            commitment
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
        validator_set_transitions: vec![],
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
fn world_finality_proof_v1_rejects_legacy_schema_after_governance_requirement() {
    let mut proof = sample_valid_finality_proof();
    proof.schema_version = 1;

    let err = proof
        .validate_contract()
        .expect_err("legacy schema rejected after governance requirement");
    assert!(
        err.contains("unsupported world finality proof schema: 1"),
        "{err}"
    );
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
    let keys = validator_keys();
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
        commitment.votes = vec![signed_finality_vote(
            &keys["validator-a"],
            "validator-a",
            commitment,
        )];
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

#[test]
fn world_finality_proof_v1_rejects_invalid_ed25519_vote_signature() {
    let mut proof = sample_valid_finality_proof();
    proof.finality_commitments[0].votes[0].signature_hex =
        proof.finality_commitments[0].votes[1].signature_hex.clone();

    let err = proof
        .validate_contract()
        .expect_err("invalid ed25519 finality vote signature rejected");
    assert!(
        err.contains("finality vote signature verification failed"),
        "{err}"
    );
}

fn add_sample_validator_set_transition(proof: &mut WorldFinalityProofV1) {
    let keys = validator_keys();
    let to_validators = vec![
        WorldFinalityValidatorV1 {
            validator_id: "validator-a".to_string(),
            stake_weight: 34,
            finality_signer_public_key: public_key_hex(&keys["validator-a"]),
            activation_height: 42,
            exit_height: None,
        },
        WorldFinalityValidatorV1 {
            validator_id: "validator-b".to_string(),
            stake_weight: 33,
            finality_signer_public_key: public_key_hex(&keys["validator-b"]),
            activation_height: 42,
            exit_height: None,
        },
        WorldFinalityValidatorV1 {
            validator_id: "validator-d".to_string(),
            stake_weight: 33,
            finality_signer_public_key: public_key_hex(&keys["validator-d"]),
            activation_height: 42,
            exit_height: None,
        },
    ];
    let to_validator_set_hash =
        compute_world_finality_validator_set_hash("sample-set-2", 42, 6_667, &to_validators)
            .expect("to validator set hash");
    let mut transition = WorldFinalityValidatorSetTransitionV1 {
        from_validator_set_id: proof.validator_set_id.clone(),
        from_validator_set_hash: proof.validator_set_hash.clone(),
        to_validator_set_id: "sample-set-2".to_string(),
        to_validator_set_activation_height: 42,
        to_quorum_threshold_bps: 6_667,
        to_validator_set_hash: to_validator_set_hash.clone(),
        to_validators,
        transition_height: 41,
        transition_block_hash: proof.head_proofs[1].head.block_hash.clone(),
        approvals: vec![],
        governance_certificate: None,
    };
    transition.approvals = ["validator-a", "validator-b"]
        .into_iter()
        .map(|validator_id| {
            let payload = world_finality_validator_set_transition_signing_payload(
                validator_id,
                transition.from_validator_set_hash.as_str(),
                transition.to_validator_set_hash.as_str(),
                transition.to_validator_set_activation_height,
                transition.transition_height,
                transition.transition_block_hash.as_str(),
            )
            .expect("transition signing payload");
            WorldFinalityValidatorSetTransitionApprovalV1 {
                validator_id: validator_id.to_string(),
                from_validator_set_hash: transition.from_validator_set_hash.clone(),
                to_validator_set_hash: transition.to_validator_set_hash.clone(),
                to_validator_set_activation_height: transition.to_validator_set_activation_height,
                transition_height: transition.transition_height,
                transition_block_hash: transition.transition_block_hash.clone(),
                signature_scheme: "ed25519".to_string(),
                signature_hex: hex::encode(keys[validator_id].sign(payload.as_slice()).to_bytes()),
            }
        })
        .collect();
    add_sample_governance_certificate(&mut transition);
    proof.validator_set_transitions = vec![transition];
    proof.finality_commitments[2].validator_set_hash = to_validator_set_hash;
    proof.finality_commitments[2].votes = vec![
        signed_finality_vote(
            &keys["validator-a"],
            "validator-a",
            &proof.finality_commitments[2],
        ),
        signed_finality_vote(
            &keys["validator-b"],
            "validator-b",
            &proof.finality_commitments[2],
        ),
    ];
}

fn add_sample_governance_certificate(transition: &mut WorldFinalityValidatorSetTransitionV1) {
    let keys = governance_keys();
    let governance_signers = vec![
        WorldFinalityGovernanceSignerV1 {
            signer_id: "governance-a".to_string(),
            stake_weight: 34,
            governance_public_key: public_key_hex(&keys["governance-a"]),
            activation_height: 1,
            exit_height: None,
        },
        WorldFinalityGovernanceSignerV1 {
            signer_id: "governance-b".to_string(),
            stake_weight: 33,
            governance_public_key: public_key_hex(&keys["governance-b"]),
            activation_height: 1,
            exit_height: None,
        },
        WorldFinalityGovernanceSignerV1 {
            signer_id: "governance-c".to_string(),
            stake_weight: 33,
            governance_public_key: public_key_hex(&keys["governance-c"]),
            activation_height: 1,
            exit_height: None,
        },
    ];
    let governance_set_hash = compute_world_finality_governance_set_hash(
        "sample-governance-set-1",
        1,
        6_667,
        &governance_signers,
    )
    .expect("governance set hash");
    let governance_approvals = ["governance-a", "governance-b"]
        .into_iter()
        .map(|signer_id| {
            let payload = world_finality_validator_set_transition_governance_signing_payload(
                signer_id,
                "world-a",
                governance_set_hash.as_str(),
                transition.from_validator_set_hash.as_str(),
                transition.to_validator_set_hash.as_str(),
                transition.to_validator_set_activation_height,
                transition.transition_height,
                transition.transition_block_hash.as_str(),
            )
            .expect("transition governance signing payload");
            WorldFinalityValidatorSetTransitionGovernanceApprovalV1 {
                signer_id: signer_id.to_string(),
                governance_set_hash: governance_set_hash.clone(),
                from_validator_set_hash: transition.from_validator_set_hash.clone(),
                to_validator_set_hash: transition.to_validator_set_hash.clone(),
                to_validator_set_activation_height: transition.to_validator_set_activation_height,
                transition_height: transition.transition_height,
                transition_block_hash: transition.transition_block_hash.clone(),
                signature_scheme: "ed25519".to_string(),
                signature_hex: hex::encode(keys[signer_id].sign(payload.as_slice()).to_bytes()),
            }
        })
        .collect();
    transition.governance_certificate =
        Some(WorldFinalityValidatorSetTransitionGovernanceCertificateV1 {
            governance_set_id: "sample-governance-set-1".to_string(),
            governance_set_activation_height: 1,
            governance_threshold_bps: 6_667,
            governance_set_hash,
            governance_signers,
            governance_approvals,
        });
}

fn resign_governance_approvals(transition: &mut WorldFinalityValidatorSetTransitionV1) {
    let keys = governance_keys();
    let certificate = transition
        .governance_certificate
        .as_mut()
        .expect("governance certificate");
    certificate.governance_set_hash = compute_world_finality_governance_set_hash(
        certificate.governance_set_id.as_str(),
        certificate.governance_set_activation_height,
        certificate.governance_threshold_bps,
        certificate.governance_signers.as_slice(),
    )
    .expect("governance set hash");
    for approval in &mut certificate.governance_approvals {
        approval.governance_set_hash = certificate.governance_set_hash.clone();
        approval.from_validator_set_hash = transition.from_validator_set_hash.clone();
        approval.to_validator_set_hash = transition.to_validator_set_hash.clone();
        approval.to_validator_set_activation_height = transition.to_validator_set_activation_height;
        approval.transition_height = transition.transition_height;
        approval.transition_block_hash = transition.transition_block_hash.clone();
        let payload = world_finality_validator_set_transition_governance_signing_payload(
            approval.signer_id.as_str(),
            "world-a",
            approval.governance_set_hash.as_str(),
            approval.from_validator_set_hash.as_str(),
            approval.to_validator_set_hash.as_str(),
            approval.to_validator_set_activation_height,
            approval.transition_height,
            approval.transition_block_hash.as_str(),
        )
        .expect("transition governance signing payload");
        approval.signature_hex = hex::encode(
            keys[approval.signer_id.as_str()]
                .sign(payload.as_slice())
                .to_bytes(),
        );
    }
}

#[test]
fn world_finality_proof_v1_executes_validator_set_transition() {
    let mut proof = sample_valid_finality_proof();
    add_sample_validator_set_transition(&mut proof);
    proof.misbehavior_evidence[0].height = 40;
    proof.misbehavior_evidence[0].conflicting_block_hash =
        "conflicting-old-set-block-40".to_string();
    proof.misbehavior_evidence[0].conflicting_proof_hash =
        "conflicting-old-set-proof-40".to_string();

    proof
        .validate_contract()
        .expect("transition proof should validate");
}

#[test]
fn world_finality_proof_v1_rejects_transition_without_governance_certificate() {
    let mut proof = sample_valid_finality_proof();
    add_sample_validator_set_transition(&mut proof);
    proof.misbehavior_evidence[0].height = 40;
    proof.validator_set_transitions[0].governance_certificate = None;

    let err = proof
        .validate_contract()
        .expect_err("missing transition governance certificate rejected");
    assert!(err.contains("governance_certificate required"), "{err}");
}

#[test]
fn world_finality_proof_v1_rejects_transition_governance_signature_tamper() {
    let mut proof = sample_valid_finality_proof();
    add_sample_validator_set_transition(&mut proof);
    proof.misbehavior_evidence[0].height = 40;
    let certificate = proof.validator_set_transitions[0]
        .governance_certificate
        .as_mut()
        .expect("governance certificate");
    certificate.governance_approvals[0].signature_hex =
        certificate.governance_approvals[1].signature_hex.clone();

    let err = proof
        .validate_contract()
        .expect_err("tampered transition governance approval signature rejected");
    assert!(
        err.contains("transition governance approval signature verification failed"),
        "{err}"
    );
}

#[test]
fn world_finality_proof_v1_rejects_transition_governance_below_threshold() {
    let mut proof = sample_valid_finality_proof();
    add_sample_validator_set_transition(&mut proof);
    proof.misbehavior_evidence[0].height = 40;
    let certificate = proof.validator_set_transitions[0]
        .governance_certificate
        .as_mut()
        .expect("governance certificate");
    certificate.governance_approvals.pop();

    let err = proof
        .validate_contract()
        .expect_err("below-threshold transition governance approval rejected");
    assert!(
        err.contains("governance_certificate approval stake below threshold"),
        "{err}"
    );
}

#[test]
fn world_finality_proof_v1_rejects_duplicate_transition_governance_approval() {
    let mut proof = sample_valid_finality_proof();
    add_sample_validator_set_transition(&mut proof);
    proof.misbehavior_evidence[0].height = 40;
    let certificate = proof.validator_set_transitions[0]
        .governance_certificate
        .as_mut()
        .expect("governance certificate");
    certificate
        .governance_approvals
        .push(certificate.governance_approvals[0].clone());

    let err = proof
        .validate_contract()
        .expect_err("duplicate transition governance approval rejected");
    assert!(err.contains("duplicate signer"), "{err}");
}

#[test]
fn world_finality_proof_v1_rejects_transition_governance_target_mismatch() {
    let mut proof = sample_valid_finality_proof();
    add_sample_validator_set_transition(&mut proof);
    proof.misbehavior_evidence[0].height = 40;
    let certificate = proof.validator_set_transitions[0]
        .governance_certificate
        .as_mut()
        .expect("governance certificate");
    certificate.governance_approvals[0].to_validator_set_hash =
        "wrong-transition-target".to_string();

    let err = proof
        .validate_contract()
        .expect_err("transition governance target mismatch rejected");
    assert!(err.contains("target mismatch"), "{err}");
}

#[test]
fn world_finality_proof_v1_rejects_transition_governance_signer_not_in_set() {
    let mut proof = sample_valid_finality_proof();
    add_sample_validator_set_transition(&mut proof);
    proof.misbehavior_evidence[0].height = 40;
    let certificate = proof.validator_set_transitions[0]
        .governance_certificate
        .as_mut()
        .expect("governance certificate");
    certificate.governance_approvals[0].signer_id = "governance-x".to_string();

    let err = proof
        .validate_contract()
        .expect_err("transition governance signer outside set rejected");
    assert!(err.contains("signer not in governance set"), "{err}");
}

#[test]
fn world_finality_proof_v1_rejects_inactive_transition_governance_signer() {
    let mut proof = sample_valid_finality_proof();
    add_sample_validator_set_transition(&mut proof);
    proof.misbehavior_evidence[0].height = 40;
    let transition = &mut proof.validator_set_transitions[0];
    let certificate = transition
        .governance_certificate
        .as_mut()
        .expect("governance certificate");
    certificate.governance_signers[0].exit_height = Some(transition.transition_height);
    resign_governance_approvals(transition);

    let err = proof
        .validate_contract()
        .expect_err("inactive transition governance signer rejected");
    assert!(err.contains("signer not active"), "{err}");
}

#[test]
fn world_finality_proof_v1_rejects_transition_approval_signature_tamper() {
    let mut proof = sample_valid_finality_proof();
    add_sample_validator_set_transition(&mut proof);
    proof.misbehavior_evidence[0].height = 40;
    proof.validator_set_transitions[0].approvals[0].signature_hex =
        proof.validator_set_transitions[0].approvals[1]
            .signature_hex
            .clone();

    let err = proof
        .validate_contract()
        .expect_err("tampered transition approval signature rejected");
    assert!(
        err.contains("transition approval signature verification failed"),
        "{err}"
    );
}

#[test]
fn world_finality_proof_v1_rejects_old_validator_misbehavior_after_transition() {
    let mut proof = sample_valid_finality_proof();
    add_sample_validator_set_transition(&mut proof);

    let err = proof
        .validate_contract()
        .expect_err("old validator misbehavior at new-set height rejected");
    assert!(
        err.contains("validator_id not in active validator set"),
        "{err}"
    );
}

#[test]
fn world_finality_proof_v1_rejects_out_of_order_validator_set_transitions() {
    let keys = validator_keys();
    let mut proof = sample_valid_finality_proof();
    add_sample_validator_set_transition(&mut proof);
    let later_transition = proof.validator_set_transitions[0].clone();
    let earlier_validators = vec![
        WorldFinalityValidatorV1 {
            validator_id: "validator-a".to_string(),
            stake_weight: 34,
            finality_signer_public_key: public_key_hex(&keys["validator-a"]),
            activation_height: 41,
            exit_height: None,
        },
        WorldFinalityValidatorV1 {
            validator_id: "validator-b".to_string(),
            stake_weight: 33,
            finality_signer_public_key: public_key_hex(&keys["validator-b"]),
            activation_height: 41,
            exit_height: None,
        },
        WorldFinalityValidatorV1 {
            validator_id: "validator-d".to_string(),
            stake_weight: 33,
            finality_signer_public_key: public_key_hex(&keys["validator-d"]),
            activation_height: 41,
            exit_height: None,
        },
    ];
    let earlier_hash = compute_world_finality_validator_set_hash(
        "sample-set-early",
        41,
        6_667,
        &earlier_validators,
    )
    .expect("earlier transition set hash");
    let mut earlier_transition = WorldFinalityValidatorSetTransitionV1 {
        from_validator_set_id: proof.validator_set_id.clone(),
        from_validator_set_hash: proof.validator_set_hash.clone(),
        to_validator_set_id: "sample-set-early".to_string(),
        to_validator_set_activation_height: 41,
        to_quorum_threshold_bps: 6_667,
        to_validator_set_hash: earlier_hash,
        to_validators: earlier_validators,
        transition_height: 40,
        transition_block_hash: proof.head_proofs[0].head.block_hash.clone(),
        approvals: vec![],
        governance_certificate: None,
    };
    earlier_transition.approvals = ["validator-a", "validator-b"]
        .into_iter()
        .map(|validator_id| {
            let payload = world_finality_validator_set_transition_signing_payload(
                validator_id,
                earlier_transition.from_validator_set_hash.as_str(),
                earlier_transition.to_validator_set_hash.as_str(),
                earlier_transition.to_validator_set_activation_height,
                earlier_transition.transition_height,
                earlier_transition.transition_block_hash.as_str(),
            )
            .expect("earlier transition signing payload");
            WorldFinalityValidatorSetTransitionApprovalV1 {
                validator_id: validator_id.to_string(),
                from_validator_set_hash: earlier_transition.from_validator_set_hash.clone(),
                to_validator_set_hash: earlier_transition.to_validator_set_hash.clone(),
                to_validator_set_activation_height: earlier_transition
                    .to_validator_set_activation_height,
                transition_height: earlier_transition.transition_height,
                transition_block_hash: earlier_transition.transition_block_hash.clone(),
                signature_scheme: "ed25519".to_string(),
                signature_hex: hex::encode(keys[validator_id].sign(payload.as_slice()).to_bytes()),
            }
        })
        .collect();
    add_sample_governance_certificate(&mut earlier_transition);
    proof.validator_set_transitions = vec![later_transition, earlier_transition];

    let err = proof
        .validate_contract()
        .expect_err("out-of-order transitions rejected");
    assert!(
        err.contains("must be in strictly increasing execution order"),
        "{err}"
    );
}

#[test]
fn world_finality_proof_v1_rejects_transition_activation_after_verified_range() {
    let keys = validator_keys();
    let mut proof = sample_valid_finality_proof();
    add_sample_validator_set_transition(&mut proof);
    let transition = &mut proof.validator_set_transitions[0];
    transition.to_validator_set_activation_height = 43;
    transition.transition_height = 42;
    transition.transition_block_hash = proof.head_proofs[2].head.block_hash.clone();
    for validator in &mut transition.to_validators {
        validator.activation_height = 43;
    }
    transition.to_validator_set_hash = compute_world_finality_validator_set_hash(
        transition.to_validator_set_id.as_str(),
        transition.to_validator_set_activation_height,
        transition.to_quorum_threshold_bps,
        transition.to_validators.as_slice(),
    )
    .expect("late transition set hash");
    transition.approvals = ["validator-a", "validator-b"]
        .into_iter()
        .map(|validator_id| {
            let payload = world_finality_validator_set_transition_signing_payload(
                validator_id,
                transition.from_validator_set_hash.as_str(),
                transition.to_validator_set_hash.as_str(),
                transition.to_validator_set_activation_height,
                transition.transition_height,
                transition.transition_block_hash.as_str(),
            )
            .expect("late transition signing payload");
            WorldFinalityValidatorSetTransitionApprovalV1 {
                validator_id: validator_id.to_string(),
                from_validator_set_hash: transition.from_validator_set_hash.clone(),
                to_validator_set_hash: transition.to_validator_set_hash.clone(),
                to_validator_set_activation_height: transition.to_validator_set_activation_height,
                transition_height: transition.transition_height,
                transition_block_hash: transition.transition_block_hash.clone(),
                signature_scheme: "ed25519".to_string(),
                signature_hex: hex::encode(keys[validator_id].sign(payload.as_slice()).to_bytes()),
            }
        })
        .collect();

    let err = proof
        .validate_contract()
        .expect_err("transition activation outside verified range rejected");
    assert!(
        err.contains("to_validator_set_activation_height must be inside verified range"),
        "{err}"
    );
}

#[test]
fn world_finality_proof_v1_rejects_transition_hash_tamper() {
    let mut proof = sample_valid_finality_proof();
    proof.validator_set_transitions = vec![WorldFinalityValidatorSetTransitionV1 {
        from_validator_set_id: proof.validator_set_id.clone(),
        from_validator_set_hash: proof.validator_set_hash.clone(),
        to_validator_set_id: "sample-set-2".to_string(),
        to_validator_set_activation_height: 42,
        to_quorum_threshold_bps: 6_667,
        to_validator_set_hash: "wrong-transition-set-hash".to_string(),
        to_validators: proof.validators.clone(),
        transition_height: 41,
        transition_block_hash: proof.head_proofs[1].head.block_hash.clone(),
        approvals: vec![],
        governance_certificate: None,
    }];

    let err = proof
        .validate_contract()
        .expect_err("tampered transition set hash rejected");
    assert!(
        err.contains("transition to_validator_set_hash mismatch"),
        "{err}"
    );
}
