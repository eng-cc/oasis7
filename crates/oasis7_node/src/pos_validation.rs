use std::collections::BTreeMap;

use oasis7_proto::distributed_pos::required_supermajority_stake;

use crate::{NodeError, NodePosConfig};

type ValidatedPosState = (
    BTreeMap<String, u64>,
    BTreeMap<String, String>,
    BTreeMap<String, String>,
    u64,
    u64,
);

pub(crate) fn validate_pos_config(pos_config: &NodePosConfig) -> Result<(), NodeError> {
    let _ = validated_pos_state(pos_config)?;
    Ok(())
}

pub(crate) fn validated_pos_state(
    pos_config: &NodePosConfig,
) -> Result<ValidatedPosState, NodeError> {
    if pos_config.validators.is_empty() {
        return Err(NodeError::InvalidConfig {
            reason: "pos validators cannot be empty".to_string(),
        });
    }
    if pos_config.epoch_length_slots == 0 {
        return Err(NodeError::InvalidConfig {
            reason: "epoch_length_slots must be positive".to_string(),
        });
    }
    if pos_config.slot_duration_ms == 0 {
        return Err(NodeError::InvalidConfig {
            reason: "slot_duration_ms must be positive".to_string(),
        });
    }
    if pos_config.ticks_per_slot == 0 {
        return Err(NodeError::InvalidConfig {
            reason: "ticks_per_slot must be positive".to_string(),
        });
    }
    if pos_config.proposal_tick_phase >= pos_config.ticks_per_slot {
        return Err(NodeError::InvalidConfig {
            reason: format!(
                "proposal_tick_phase={} must be less than ticks_per_slot={}",
                pos_config.proposal_tick_phase, pos_config.ticks_per_slot
            ),
        });
    }
    if pos_config.supermajority_denominator == 0
        || pos_config.supermajority_numerator == 0
        || pos_config.supermajority_numerator > pos_config.supermajority_denominator
    {
        return Err(NodeError::InvalidConfig {
            reason: format!(
                "invalid supermajority ratio {}/{}",
                pos_config.supermajority_numerator, pos_config.supermajority_denominator
            ),
        });
    }
    if pos_config.supermajority_numerator <= pos_config.supermajority_denominator / 2 {
        return Err(NodeError::InvalidConfig {
            reason: "supermajority ratio must be greater than 1/2".to_string(),
        });
    }

    let mut validators = BTreeMap::new();
    let mut validator_players = BTreeMap::new();
    let mut validator_signers = BTreeMap::new();
    let mut player_to_validator = BTreeMap::new();
    for validator in &pos_config.validators {
        let validator_id = validator.validator_id.trim();
        if validator_id.is_empty() {
            return Err(NodeError::InvalidConfig {
                reason: "validator_id cannot be empty".to_string(),
            });
        }
        if validator.stake == 0 {
            return Err(NodeError::InvalidConfig {
                reason: format!("validator {} stake must be positive", validator_id),
            });
        }
        if validators
            .insert(validator_id.to_string(), validator.stake)
            .is_some()
        {
            return Err(NodeError::InvalidConfig {
                reason: format!("duplicate validator: {}", validator_id),
            });
        }
        let player_id = pos_config
            .validator_player_ids
            .get(validator_id)
            .ok_or_else(|| NodeError::InvalidConfig {
                reason: format!("missing player binding for validator {}", validator_id),
            })?;
        let player_id = player_id.trim();
        if player_id.is_empty() {
            return Err(NodeError::InvalidConfig {
                reason: format!("validator {} has empty player_id binding", validator_id),
            });
        }
        if let Some(existing_validator) =
            player_to_validator.insert(player_id.to_string(), validator_id.to_string())
        {
            return Err(NodeError::InvalidConfig {
                reason: format!(
                    "player {} is bound to multiple validators: {} and {}",
                    player_id, existing_validator, validator_id
                ),
            });
        }
        validator_players.insert(validator_id.to_string(), player_id.to_string());
    }
    for validator_id in pos_config.validator_player_ids.keys() {
        if !validators.contains_key(validator_id.as_str()) {
            return Err(NodeError::InvalidConfig {
                reason: format!(
                    "validator player binding references unknown validator: {}",
                    validator_id
                ),
            });
        }
    }
    if !pos_config.validator_signer_public_keys.is_empty() {
        let mut signer_to_validator = BTreeMap::new();
        for validator_id in validators.keys() {
            let raw_signer = pos_config
                .validator_signer_public_keys
                .get(validator_id.as_str())
                .ok_or_else(|| NodeError::InvalidConfig {
                    reason: format!("missing signer binding for validator {}", validator_id),
                })?;
            let normalized_signer = normalize_consensus_public_key_hex(
                raw_signer,
                format!("validator_signer_public_keys[{validator_id}]").as_str(),
            )?;
            if let Some(existing_validator) =
                signer_to_validator.insert(normalized_signer.clone(), validator_id.to_string())
            {
                return Err(NodeError::InvalidConfig {
                    reason: format!(
                        "signer {} is bound to multiple validators: {} and {}",
                        normalized_signer, existing_validator, validator_id
                    ),
                });
            }
            validator_signers.insert(validator_id.to_string(), normalized_signer);
        }
        for validator_id in pos_config.validator_signer_public_keys.keys() {
            if !validators.contains_key(validator_id.as_str()) {
                return Err(NodeError::InvalidConfig {
                    reason: format!(
                        "validator signer binding references unknown validator: {}",
                        validator_id
                    ),
                });
            }
        }
    }

    let total_stake = validators
        .values()
        .try_fold(0u64, |acc, stake| acc.checked_add(*stake))
        .ok_or_else(|| NodeError::InvalidConfig {
            reason: "total stake overflow".to_string(),
        })?;
    let required_stake = required_supermajority_stake(
        total_stake,
        pos_config.supermajority_numerator,
        pos_config.supermajority_denominator,
    )
    .map_err(|reason| NodeError::InvalidConfig {
        reason: format!("invalid pos supermajority: {}", reason),
    })?;
    Ok((
        validators,
        validator_players,
        validator_signers,
        total_stake,
        required_stake,
    ))
}

pub(crate) fn normalize_consensus_public_key_hex(
    raw: &str,
    field: &str,
) -> Result<String, NodeError> {
    let normalized = raw.trim();
    if normalized.is_empty() {
        return Err(NodeError::InvalidConfig {
            reason: format!("{field} cannot be empty"),
        });
    }
    let bytes = hex::decode(normalized).map_err(|_| NodeError::InvalidConfig {
        reason: format!("{field} must be valid hex"),
    })?;
    let key_bytes: [u8; 32] = bytes.try_into().map_err(|_| NodeError::InvalidConfig {
        reason: format!("{field} must be 32-byte hex"),
    })?;
    Ok(hex::encode(key_bytes))
}
