use super::*;

pub(super) fn unlock_meta_track_tiers(
    track: &str,
    track_points: i64,
    progress: &mut MetaProgressState,
) {
    const META_TIER_THRESHOLDS: [(&str, i64); 3] = [("bronze", 20), ("silver", 50), ("gold", 100)];
    let unlocked_tiers = progress
        .unlocked_tiers
        .entry(track.to_string())
        .or_default();
    for (tier, threshold) in META_TIER_THRESHOLDS {
        if track_points < threshold {
            continue;
        }
        if !unlocked_tiers.iter().any(|value| value == tier) {
            unlocked_tiers.push(tier.to_string());
        }
        let achievement_id = format!("tier.{track}.{tier}");
        if !progress
            .achievements
            .iter()
            .any(|value| value == &achievement_id)
        {
            progress.achievements.push(achievement_id);
        }
    }
    unlocked_tiers.sort();
    unlocked_tiers.dedup();
    progress.achievements.sort();
    progress.achievements.dedup();
}

pub(super) fn touch_agent_last_active_required(
    state: &mut WorldState,
    agent_id: &str,
    now: WorldTime,
) -> Result<(), WorldError> {
    let cell = state
        .agents
        .get_mut(agent_id)
        .ok_or_else(|| WorldError::AgentNotFound {
            agent_id: agent_id.to_string(),
        })?;
    cell.last_active = now;
    Ok(())
}

pub(super) fn apply_node_points_settlement_event(
    state: &mut WorldState,
    report: &EpochSettlementReport,
    signer_node_id: &str,
    settlement_hash: &str,
    minted_records: &[NodeRewardMintRecord],
    main_token_bridge_total_amount: u64,
    main_token_bridge_distributions: &[MainTokenNodePointsBridgeDistribution],
) -> Result<(), WorldError> {
    if signer_node_id.trim().is_empty() {
        return Err(WorldError::ResourceBalanceInvalid {
            reason: "settlement signer_node_id cannot be empty".to_string(),
        });
    }
    let expected_hash = hash_json(report)?;
    if expected_hash != settlement_hash {
        return Err(WorldError::ResourceBalanceInvalid {
            reason: format!(
                "settlement_hash mismatch: expected={} actual={}",
                expected_hash, settlement_hash
            ),
        });
    }
    let points_per_credit = state.reward_asset_config.points_per_credit;
    if points_per_credit == 0 {
        return Err(WorldError::ResourceBalanceInvalid {
            reason: "points_per_credit must be positive".to_string(),
        });
    }
    if !state.node_identity_bindings.contains_key(signer_node_id) {
        return Err(WorldError::ResourceBalanceInvalid {
            reason: format!("node identity is not bound: {signer_node_id}"),
        });
    }

    let mut settlement_points = BTreeMap::new();
    for settlement in &report.settlements {
        if settlement.node_id.trim().is_empty() {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: "report settlement contains empty node_id".to_string(),
            });
        }
        if settlement_points
            .insert(settlement.node_id.clone(), settlement.awarded_points)
            .is_some()
        {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "duplicate settlement node in report: {}",
                    settlement.node_id
                ),
            });
        }
        if !state
            .node_identity_bindings
            .contains_key(settlement.node_id.as_str())
        {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!("node identity is not bound: {}", settlement.node_id),
            });
        }
    }

    let mut budget = state
        .system_order_pool_budgets
        .get(&report.epoch_index)
        .cloned();
    if let Some(item) = budget.as_mut() {
        ensure_system_order_budget_caps_for_epoch(report, item);
    }

    let mut seen_nodes = BTreeMap::<String, ()>::new();
    for record in minted_records {
        if record.epoch_index != report.epoch_index {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "mint record epoch mismatch: report={} record={}",
                    report.epoch_index, record.epoch_index
                ),
            });
        }
        if record.signer_node_id != signer_node_id {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "mint record signer mismatch: event={} record={}",
                    signer_node_id, record.signer_node_id
                ),
            });
        }
        if record.settlement_hash != settlement_hash {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "mint record settlement_hash mismatch for node {}",
                    record.node_id
                ),
            });
        }
        let Some(awarded_points) = settlement_points.get(record.node_id.as_str()) else {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "mint record node is missing in report settlements: {}",
                    record.node_id
                ),
            });
        };
        if record.source_awarded_points != *awarded_points {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "mint record awarded points mismatch for node {}: report={} record={}",
                    record.node_id, awarded_points, record.source_awarded_points
                ),
            });
        }
        if record.minted_power_credits == 0 {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "mint record has zero minted_power_credits for node {}",
                    record.node_id
                ),
            });
        }
        let max_minted = record.source_awarded_points / points_per_credit;
        if record.minted_power_credits > max_minted {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "minted credits exceed settlement cap for node {}: minted={} cap={}",
                    record.node_id, record.minted_power_credits, max_minted
                ),
            });
        }
        if seen_nodes.insert(record.node_id.clone(), ()).is_some() {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "duplicate mint record node in one action: {}",
                    record.node_id
                ),
            });
        }
        if state.reward_mint_records.iter().any(|existing| {
            existing.epoch_index == record.epoch_index && existing.node_id == record.node_id
        }) {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "mint record already exists for epoch={} node={}",
                    record.epoch_index, record.node_id
                ),
            });
        }
        verify_reward_mint_record_signature_with_state(state, record).map_err(|reason| {
            WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "mint record signature invalid (epoch={} node={}): {}",
                    record.epoch_index, record.node_id, reason
                ),
            }
        })?;

        if let Some(item) = budget.as_mut() {
            let node_cap = item
                .node_credit_caps
                .get(record.node_id.as_str())
                .copied()
                .unwrap_or(0);
            let node_allocated = item
                .node_credit_allocated
                .get(record.node_id.as_str())
                .copied()
                .unwrap_or(0);
            let node_remaining = node_cap.saturating_sub(node_allocated);
            if record.minted_power_credits > node_remaining {
                return Err(WorldError::ResourceBalanceInvalid {
                    reason: format!(
                        "minted credits exceed node budget cap for {}: minted={} remaining={}",
                        record.node_id, record.minted_power_credits, node_remaining
                    ),
                });
            }
            if record.minted_power_credits > item.remaining_credit_budget {
                return Err(WorldError::ResourceBalanceInvalid {
                    reason: format!(
                        "minted credits exceed remaining system order budget: minted={} remaining={}",
                        record.minted_power_credits, item.remaining_credit_budget
                    ),
                });
            }
            item.remaining_credit_budget = item
                .remaining_credit_budget
                .saturating_sub(record.minted_power_credits);
            item.node_credit_allocated
                .entry(record.node_id.clone())
                .and_modify(|value| *value = value.saturating_add(record.minted_power_credits))
                .or_insert(record.minted_power_credits);
        }
    }

    for record in minted_records {
        let balance = state
            .node_asset_balances
            .entry(record.node_id.clone())
            .or_insert_with(|| NodeAssetBalance {
                node_id: record.node_id.clone(),
                ..NodeAssetBalance::default()
            });
        balance.power_credit_balance = balance
            .power_credit_balance
            .saturating_add(record.minted_power_credits);
        balance.total_minted_credits = balance
            .total_minted_credits
            .saturating_add(record.minted_power_credits);
        state.reward_mint_records.push(record.clone());
    }
    if let Some(item) = budget {
        state
            .system_order_pool_budgets
            .insert(report.epoch_index, item);
    }
    apply_main_token_bridge_from_settlement_event(
        state,
        report,
        settlement_hash,
        main_token_bridge_total_amount,
        main_token_bridge_distributions,
    )?;
    Ok(())
}

fn apply_main_token_bridge_from_settlement_event(
    state: &mut WorldState,
    report: &EpochSettlementReport,
    settlement_hash: &str,
    total_amount: u64,
    distributions: &[MainTokenNodePointsBridgeDistribution],
) -> Result<(), WorldError> {
    if state
        .main_token_node_points_bridge_records
        .contains_key(&report.epoch_index)
    {
        return Err(WorldError::ResourceBalanceInvalid {
            reason: format!(
                "main token node points bridge already processed for epoch={}",
                report.epoch_index
            ),
        });
    }
    if settlement_hash.trim().is_empty() {
        return Err(WorldError::ResourceBalanceInvalid {
            reason: "main token bridge settlement_hash cannot be empty".to_string(),
        });
    }

    let expected_budget = state
        .main_token_epoch_issuance_records
        .get(&report.epoch_index)
        .map(|record| record.node_service_reward_amount)
        .unwrap_or(0);
    if total_amount > expected_budget {
        return Err(WorldError::ResourceBalanceInvalid {
            reason: format!(
                "main token bridge total exceeds epoch node_service budget: epoch={} total={} budget={}",
                report.epoch_index, total_amount, expected_budget
            ),
        });
    }

    let mut distribution_sum = 0_u64;
    let mut seen_nodes = BTreeSet::new();
    for item in distributions {
        if item.node_id.trim().is_empty() {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: "main token bridge distribution node_id cannot be empty".to_string(),
            });
        }
        if item.account_id.trim().is_empty() {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "main token bridge distribution account_id cannot be empty: node={}",
                    item.node_id
                ),
            });
        }
        if item.amount == 0 {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "main token bridge distribution amount must be > 0: node={}",
                    item.node_id
                ),
            });
        }
        if !seen_nodes.insert(item.node_id.clone()) {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "duplicate main token bridge distribution for node={}",
                    item.node_id
                ),
            });
        }
        distribution_sum = distribution_sum.checked_add(item.amount).ok_or_else(|| {
            WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "main token bridge distribution sum overflow: epoch={}",
                    report.epoch_index
                ),
            }
        })?;
    }
    if distribution_sum != total_amount {
        return Err(WorldError::ResourceBalanceInvalid {
            reason: format!(
                "main token bridge sum mismatch: epoch={} total={} distributions_sum={}",
                report.epoch_index, total_amount, distribution_sum
            ),
        });
    }

    let treasury_balance = state
        .main_token_treasury_balances
        .get(MAIN_TOKEN_TREASURY_BUCKET_NODE_SERVICE_REWARD)
        .copied()
        .unwrap_or(0);
    if treasury_balance < total_amount {
        return Err(WorldError::ResourceBalanceInvalid {
            reason: format!(
                "main token bridge treasury insufficient: epoch={} balance={} total={}",
                report.epoch_index, treasury_balance, total_amount
            ),
        });
    }

    if total_amount > 0 {
        state.main_token_treasury_balances.insert(
            MAIN_TOKEN_TREASURY_BUCKET_NODE_SERVICE_REWARD.to_string(),
            treasury_balance - total_amount,
        );
        for item in distributions {
            let account = state
                .main_token_balances
                .entry(item.account_id.clone())
                .or_insert_with(|| MainTokenAccountBalance {
                    account_id: item.account_id.clone(),
                    ..MainTokenAccountBalance::default()
                });
            account.liquid_balance =
                account
                    .liquid_balance
                    .checked_add(item.amount)
                    .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "main token bridge account overflow: account={} current={} amount={}",
                            item.account_id, account.liquid_balance, item.amount
                        ),
                    })?;
        }
        state.main_token_supply.circulating_supply = state
            .main_token_supply
            .circulating_supply
            .checked_add(total_amount)
            .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "main token bridge circulating overflow: current={} amount={}",
                    state.main_token_supply.circulating_supply, total_amount
                ),
            })?;
        if state.main_token_supply.circulating_supply > state.main_token_supply.total_supply {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "main token bridge circulating exceeds total: circulating={} total={}",
                    state.main_token_supply.circulating_supply,
                    state.main_token_supply.total_supply
                ),
            });
        }
    }

    state.main_token_node_points_bridge_records.insert(
        report.epoch_index,
        MainTokenNodePointsBridgeEpochRecord {
            epoch_index: report.epoch_index,
            settlement_hash: settlement_hash.to_string(),
            total_amount,
            distributions: distributions.to_vec(),
        },
    );
    Ok(())
}

pub(super) fn integer_sqrt_u64(value: u64) -> u64 {
    if value < 2 {
        return value;
    }
    let mut x0 = value;
    let mut x1 = (x0 + value / x0) / 2;
    while x1 < x0 {
        x0 = x1;
        x1 = (x0 + value / x0) / 2;
    }
    x0
}

fn add_material_balance(
    balances: &mut BTreeMap<String, i64>,
    kind: &str,
    amount: i64,
) -> Result<(), String> {
    if amount < 0 {
        return Err(format!("negative material amount not allowed: {amount}"));
    }
    let entry = balances.entry(kind.to_string()).or_insert(0);
    *entry = entry.saturating_add(amount);
    if *entry == 0 {
        balances.remove(kind);
    }
    Ok(())
}

pub(super) fn add_material_balance_for_ledger(
    ledgers: &mut BTreeMap<MaterialLedgerId, BTreeMap<String, i64>>,
    ledger: &MaterialLedgerId,
    kind: &str,
    amount: i64,
) -> Result<(), String> {
    let balances = ledgers.entry(ledger.clone()).or_default();
    add_material_balance(balances, kind, amount)
}

fn remove_material_balance(
    balances: &mut BTreeMap<String, i64>,
    kind: &str,
    amount: i64,
) -> Result<(), String> {
    if amount < 0 {
        return Err(format!("negative material amount not allowed: {amount}"));
    }
    let current = balances.get(kind).copied().unwrap_or(0);
    if current < amount {
        return Err(format!(
            "insufficient material {kind}: requested={amount} available={current}"
        ));
    }
    let next = current - amount;
    if next == 0 {
        balances.remove(kind);
    } else {
        balances.insert(kind.to_string(), next);
    }
    Ok(())
}

pub(super) fn remove_material_balance_for_ledger(
    ledgers: &mut BTreeMap<MaterialLedgerId, BTreeMap<String, i64>>,
    ledger: &MaterialLedgerId,
    kind: &str,
    amount: i64,
) -> Result<(), String> {
    let balances = ledgers.entry(ledger.clone()).or_default();
    remove_material_balance(balances, kind, amount)
}

pub(super) fn sync_compat_world_materials(
    ledgers: &BTreeMap<MaterialLedgerId, BTreeMap<String, i64>>,
    compat_world_materials_cache: &mut BTreeMap<String, i64>,
) {
    let world_materials = ledgers
        .get(&MaterialLedgerId::world())
        .cloned()
        .unwrap_or_default();
    *compat_world_materials_cache = world_materials;
}

pub(super) fn apply_war_participant_outcomes(
    agents: &mut BTreeMap<String, AgentCell>,
    reputation_scores: &mut BTreeMap<String, i64>,
    outcomes: &[WarParticipantOutcome],
    now: WorldTime,
) -> Result<(), WorldError> {
    for outcome in outcomes {
        let Some(cell) = agents.get_mut(outcome.agent_id.as_str()) else {
            return Err(WorldError::AgentNotFound {
                agent_id: outcome.agent_id.clone(),
            });
        };

        apply_agent_resource_delta(
            cell,
            ResourceKind::Electricity,
            outcome.electricity_delta,
            outcome.agent_id.as_str(),
            "war electricity outcome",
        )?;
        apply_agent_resource_delta(
            cell,
            ResourceKind::Data,
            outcome.data_delta,
            outcome.agent_id.as_str(),
            "war data outcome",
        )?;
        cell.last_active = now;

        if outcome.reputation_delta != 0 {
            let score = reputation_scores
                .entry(outcome.agent_id.clone())
                .or_insert(0);
            *score = score.saturating_add(outcome.reputation_delta);
        }
    }
    Ok(())
}

fn apply_agent_resource_delta(
    cell: &mut AgentCell,
    kind: ResourceKind,
    delta: i64,
    agent_id: &str,
    context: &str,
) -> Result<(), WorldError> {
    if delta == 0 {
        return Ok(());
    }
    if delta > 0 {
        return cell.state.resources.add(kind, delta).map_err(|err| {
            WorldError::ResourceBalanceInvalid {
                reason: format!("{context} apply failed for {agent_id}: {err:?}"),
            }
        });
    }
    cell.state
        .resources
        .remove(kind, delta.saturating_abs())
        .map_err(|err| WorldError::ResourceBalanceInvalid {
            reason: format!("{context} apply failed for {agent_id}: {err:?}"),
        })
}

pub(super) fn remove_resource_balance(
    balances: &mut BTreeMap<ResourceKind, i64>,
    kind: ResourceKind,
    amount: i64,
) -> Result<(), String> {
    if amount < 0 {
        return Err(format!("negative resource amount not allowed: {amount}"));
    }
    let current = balances.get(&kind).copied().unwrap_or(0);
    if current < amount {
        return Err(format!(
            "insufficient resource {:?}: requested={amount} available={current}",
            kind
        ));
    }
    let next = current - amount;
    if next == 0 {
        balances.remove(&kind);
    } else {
        balances.insert(kind, next);
    }
    Ok(())
}

fn verify_reward_mint_record_signature_with_state(
    state: &WorldState,
    record: &NodeRewardMintRecord,
) -> Result<(), String> {
    let signer_public_key = state
        .node_identity_bindings
        .get(record.signer_node_id.as_str())
        .map(String::as_str)
        .ok_or_else(|| {
            format!(
                "reward mint signer identity is not bound: {}",
                record.signer_node_id
            )
        })?;
    if record.signature.starts_with(REWARD_MINT_SIGNATURE_V2_PREFIX) {
        return verify_reward_mint_signature_v2(
            record.signature.as_str(),
            record.epoch_index,
            record.node_id.as_str(),
            record.source_awarded_points,
            record.minted_power_credits,
            record.settlement_hash.as_str(),
            record.signer_node_id.as_str(),
            signer_public_key,
        );
    }
    if record.signature.starts_with(REWARD_MINT_SIGNATURE_V1_PREFIX) {
        if !state
            .reward_signature_governance_policy
            .allow_mintsig_v1_fallback
        {
            return Err("mintsig:v1 is disabled by governance policy".to_string());
        }
        let expected_signature = reward_mint_signature_v1(
            record.epoch_index,
            record.node_id.as_str(),
            record.source_awarded_points,
            record.minted_power_credits,
            record.settlement_hash.as_str(),
            record.signer_node_id.as_str(),
            signer_public_key,
        );
        if expected_signature != record.signature {
            return Err(format!(
                "reward mint signature mismatch for node {} at epoch {}",
                record.node_id, record.epoch_index
            ));
        }
        return Ok(());
    }
    Err(format!(
        "unsupported reward mint signature version for node {} at epoch {}",
        record.node_id, record.epoch_index
    ))
}

fn ensure_system_order_budget_caps_for_epoch(
    report: &EpochSettlementReport,
    budget: &mut SystemOrderPoolBudget,
) {
    if !budget.node_credit_caps.is_empty() {
        return;
    }
    if budget.total_credit_budget == 0 || report.settlements.is_empty() {
        return;
    }

    let total_awarded_points = report
        .settlements
        .iter()
        .map(|settlement| settlement.awarded_points)
        .sum::<u64>();
    if total_awarded_points == 0 {
        return;
    }

    let mut distributed = 0_u64;
    for settlement in &report.settlements {
        let cap = budget
            .total_credit_budget
            .saturating_mul(settlement.awarded_points)
            / total_awarded_points;
        distributed = distributed.saturating_add(cap);
        budget
            .node_credit_caps
            .insert(settlement.node_id.clone(), cap);
    }

    let mut remainder = budget.total_credit_budget.saturating_sub(distributed);
    if remainder == 0 {
        return;
    }
    let mut ranked = report
        .settlements
        .iter()
        .map(|settlement| (settlement.node_id.as_str(), settlement.awarded_points))
        .collect::<Vec<_>>();
    ranked.sort_by(|(a_node_id, a_points), (b_node_id, b_points)| {
        b_points
            .cmp(a_points)
            .then_with(|| a_node_id.cmp(b_node_id))
    });
    let mut index = 0_usize;
    while remainder > 0 && !ranked.is_empty() {
        let node_id = ranked[index % ranked.len()].0;
        if let Some(cap) = budget.node_credit_caps.get_mut(node_id) {
            *cap = cap.saturating_add(1);
            remainder -= 1;
        }
        index = index.saturating_add(1);
    }
}
