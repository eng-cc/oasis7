use super::*;

const CONSENSUS_SLASHING_APPEAL_WINDOW_TICKS: u64 = 32;

impl PosNodeEngine {
    pub(super) fn slashing_intent_for_evidence(
        &self,
        evidence: &ConsensusMisbehaviorEvidence,
    ) -> NodeConsensusSlashingIntentSnapshot {
        let evidence_hash = evidence_hash(evidence);
        let target_agent_id = self
            .validator_players
            .get(evidence.validator_id.as_str())
            .cloned()
            .unwrap_or_else(|| evidence.validator_id.clone());
        let reason = format!(
            "consensus {} validator={} node={} height={} stake_root={}",
            evidence.kind,
            evidence.validator_id,
            evidence.node_id,
            evidence.height,
            evidence.validator_stake_root
        );
        NodeConsensusSlashingIntentSnapshot {
            intent_id: slashing_intent_id(evidence, evidence_hash.as_str()),
            evidence_hash,
            kind: evidence.kind.clone(),
            validator_id: evidence.validator_id.clone(),
            target_agent_id,
            reason,
            slash_stake: evidence.slashable_stake,
            appeal_window_ticks: CONSENSUS_SLASHING_APPEAL_WINDOW_TICKS,
            validator_stake_root: evidence.validator_stake_root.clone(),
            governance_method: "apply_identity_penalty".to_string(),
            status: "pending_governance_submission".to_string(),
            enforced: false,
        }
    }
}

pub(super) fn evidence_hash(evidence: &ConsensusMisbehaviorEvidence) -> String {
    let fields = vec![
        evidence.kind.clone(),
        evidence.validator_id.clone(),
        evidence.node_id.clone(),
        evidence.height.to_string(),
        evidence.first_block_hash.clone(),
        evidence.second_block_hash.clone(),
        evidence
            .first_execution_block_hash
            .clone()
            .unwrap_or_default(),
        evidence
            .second_execution_block_hash
            .clone()
            .unwrap_or_default(),
        evidence
            .first_execution_state_root
            .clone()
            .unwrap_or_default(),
        evidence
            .second_execution_state_root
            .clone()
            .unwrap_or_default(),
        evidence.first_action_root.clone(),
        evidence.second_action_root.clone(),
        evidence.first_public_key_hex.clone().unwrap_or_default(),
        evidence.second_public_key_hex.clone().unwrap_or_default(),
        evidence.first_signature_hex.clone().unwrap_or_default(),
        evidence.second_signature_hex.clone().unwrap_or_default(),
        evidence.slashable_stake.to_string(),
        evidence.total_stake.to_string(),
        evidence.validator_stake_root.clone(),
    ];
    hash_cbor(("oasis7.consensus_misbehavior_evidence.v1", fields)).unwrap_or_else(|_| {
        blake3_hex(
            format!(
                "{}:{}:{}:{}:{}",
                evidence.kind,
                evidence.validator_id,
                evidence.node_id,
                evidence.height,
                evidence.validator_stake_root
            )
            .as_bytes(),
        )
    })
}

fn slashing_intent_id(evidence: &ConsensusMisbehaviorEvidence, evidence_hash: &str) -> String {
    hash_cbor((
        "oasis7.consensus_slashing_intent.v1",
        evidence.kind.as_str(),
        evidence.validator_id.as_str(),
        evidence.height,
        evidence_hash,
    ))
    .unwrap_or_else(|_| {
        blake3_hex(
            format!(
                "{}:{}:{}:{}",
                evidence.kind, evidence.validator_id, evidence.height, evidence_hash
            )
            .as_bytes(),
        )
    })
}

pub(super) fn build_validator_stake_proof_chain(
    validators: &BTreeMap<String, u64>,
    validator_players: &BTreeMap<String, String>,
    validator_signers: &BTreeMap<String, String>,
) -> Result<(String, String, Vec<NodeValidatorStakeProofSnapshot>), NodeError> {
    let leaves = validators
        .iter()
        .map(|(validator_id, stake)| {
            let player_id = validator_players
                .get(validator_id.as_str())
                .ok_or_else(|| NodeError::InvalidConfig {
                    reason: format!(
                        "validator stake proof missing player binding for {validator_id}"
                    ),
                })?;
            let signer_public_key_hex = validator_signers.get(validator_id.as_str()).cloned();
            let leaf_hash = validator_stake_leaf_hash(
                validator_id,
                player_id,
                *stake,
                signer_public_key_hex.as_deref(),
            )?;
            Ok(NodeValidatorStakeProofSnapshot {
                validator_id: validator_id.clone(),
                player_id: player_id.clone(),
                stake: *stake,
                signer_public_key_hex,
                leaf_hash,
                proof: Vec::new(),
            })
        })
        .collect::<Result<Vec<_>, NodeError>>()?;
    let validator_set_hash = hash_validator_set(leaves.as_slice())?;
    let validator_stake_root = merkle_root(
        leaves
            .iter()
            .map(|leaf| leaf.leaf_hash.clone())
            .collect::<Vec<_>>(),
    )?;
    let proofs = leaves
        .iter()
        .enumerate()
        .map(|(index, leaf)| {
            let mut proof = leaf.clone();
            proof.proof = merkle_proof_for_index(
                leaves
                    .iter()
                    .map(|candidate| candidate.leaf_hash.clone())
                    .collect::<Vec<_>>(),
                index,
            )?;
            Ok(proof)
        })
        .collect::<Result<Vec<_>, NodeError>>()?;
    Ok((validator_set_hash, validator_stake_root, proofs))
}

fn validator_stake_leaf_hash(
    validator_id: &str,
    player_id: &str,
    stake: u64,
    signer_public_key_hex: Option<&str>,
) -> Result<String, NodeError> {
    hash_cbor((
        "oasis7.validator_stake_leaf.v1",
        validator_id,
        player_id,
        stake,
        signer_public_key_hex,
    ))
}

fn hash_validator_set(leaves: &[NodeValidatorStakeProofSnapshot]) -> Result<String, NodeError> {
    let rows = leaves
        .iter()
        .map(|leaf| {
            (
                leaf.validator_id.as_str(),
                leaf.player_id.as_str(),
                leaf.stake,
                leaf.signer_public_key_hex.as_deref(),
                leaf.leaf_hash.as_str(),
            )
        })
        .collect::<Vec<_>>();
    hash_cbor(("oasis7.validator_set.v1", rows))
}

fn merkle_root(mut level: Vec<String>) -> Result<String, NodeError> {
    if level.is_empty() {
        return hash_cbor(("oasis7.validator_stake_root.v1", Vec::<String>::new()));
    }
    while level.len() > 1 {
        level = merkle_parent_level(level)?;
    }
    Ok(level.remove(0))
}

fn merkle_proof_for_index(
    mut level: Vec<String>,
    mut index: usize,
) -> Result<Vec<NodeValidatorStakeProofStepSnapshot>, NodeError> {
    let mut proof = Vec::new();
    while level.len() > 1 {
        let sibling_index = if index % 2 == 0 {
            if index + 1 < level.len() {
                index + 1
            } else {
                index
            }
        } else {
            index - 1
        };
        proof.push(NodeValidatorStakeProofStepSnapshot {
            side: if index % 2 == 0 {
                "right".to_string()
            } else {
                "left".to_string()
            },
            hash: level[sibling_index].clone(),
        });
        level = merkle_parent_level(level)?;
        index /= 2;
    }
    Ok(proof)
}

fn merkle_parent_level(level: Vec<String>) -> Result<Vec<String>, NodeError> {
    let mut next = Vec::new();
    for pair in level.chunks(2) {
        let left = pair[0].as_str();
        let right = pair.get(1).map(String::as_str).unwrap_or(left);
        next.push(hash_cbor(("oasis7.validator_stake_node.v1", left, right))?);
    }
    Ok(next)
}

fn hash_cbor<T: serde::Serialize>(value: T) -> Result<String, NodeError> {
    let bytes = serde_cbor::to_vec(&value).map_err(|err| NodeError::Consensus {
        reason: format!("encode validator stake proof payload failed: {err}"),
    })?;
    Ok(blake3_hex(bytes.as_slice()))
}
