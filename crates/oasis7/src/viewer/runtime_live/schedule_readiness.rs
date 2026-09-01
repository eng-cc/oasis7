use super::{GameplayActionRequest, build_runtime_action_from_gameplay_request};
use crate::runtime::{Action as RuntimeAction, MaterialLedgerId, WorldState};
use crate::simulator::{ResourceKind, default_schedule_recipe_readiness};
use oasis7_wasm_abi::MaterialStack;
use std::collections::BTreeMap;

pub(super) fn schedule_recipe_disabled_reason(
    state: &WorldState,
    agent_id: &str,
    action_id: &str,
) -> Option<String> {
    let RuntimeAction::ScheduleRecipe {
        factory_id,
        recipe_id,
        plan,
        ..
    } = build_runtime_action_from_gameplay_request(&GameplayActionRequest {
        action_id: action_id.to_string(),
        target_agent_id: agent_id.to_string(),
        actor_agent_id: None,
        player_id: String::new(),
        public_key: None,
        auth: None,
    })
    .ok()?
    else {
        return None;
    };
    let readiness = default_schedule_recipe_readiness(recipe_id.as_str(), 1)?;
    let resources = &state.agents.get(agent_id)?.state.resources;
    let available_electricity = resources.get(ResourceKind::Electricity);
    if available_electricity < readiness.electricity_cost {
        return Some(format!(
            "insufficient electricity: need {}, have {}; replenish electricity before scheduling this run",
            readiness.electricity_cost, available_electricity
        ));
    }
    let factory = state.factories.get(factory_id.as_str())?;
    if factory.builder_agent_id != agent_id {
        return Some(format!(
            "factory {} is owned by {}; only the factory owner may schedule this run",
            factory_id, factory.builder_agent_id
        ));
    }
    let consume = recipe_consume_with_maintenance_sinks(state, &plan.consume, &plan.produce);
    let consume_ledger = factory.input_ledger.clone();
    for stack in &consume {
        let available = ledger_material_balance(state, &consume_ledger, stack.kind.as_str());
        if available < stack.amount {
            return Some(format!(
                "insufficient {} in {} ledger: need {}, have {}; replenish {} before scheduling this run",
                stack.kind, consume_ledger, stack.amount, available, stack.kind
            ));
        }
    }
    let available_owner_electricity = state
        .agents
        .get(factory.builder_agent_id.as_str())?
        .state
        .resources
        .get(ResourceKind::Electricity);
    if available_owner_electricity < plan.power_required {
        return Some(format!(
            "insufficient factory-owner electricity: need {}, have {}; replenish electricity before scheduling this run",
            plan.power_required, available_owner_electricity
        ));
    }
    None
}

/// Returns the simulator's Data estimate as display-only context.
///
/// Runtime factory scheduling has no Data admission rule. Keep the estimate
/// visible so players can compare it with simulator quotes, but label it as
/// non-authoritative and never use it to disable the runtime action.
pub(super) fn schedule_recipe_data_advisory(agent_id: &str, action_id: &str) -> Option<String> {
    let RuntimeAction::ScheduleRecipe {
        recipe_id, plan, ..
    } = build_runtime_action_from_gameplay_request(&GameplayActionRequest {
        action_id: action_id.to_string(),
        target_agent_id: agent_id.to_string(),
        actor_agent_id: None,
        player_id: String::new(),
        public_key: None,
        auth: None,
    })
    .ok()?
    else {
        return None;
    };
    let readiness =
        default_schedule_recipe_readiness(recipe_id.as_str(), i64::from(plan.accepted_batches))?;
    Some(format!(
        "Data estimate only (simulator advisory; non-authoritative): {}",
        readiness.data_cost
    ))
}

fn recipe_consume_with_maintenance_sinks(
    state: &WorldState,
    consume: &[MaterialStack],
    produce: &[MaterialStack],
) -> Vec<MaterialStack> {
    let mut merged = BTreeMap::new();
    let mut order = Vec::new();
    for stack in consume {
        if !merged.contains_key(stack.kind.as_str()) {
            order.push(stack.kind.clone());
        }
        let amount = merged.entry(stack.kind.clone()).or_insert(0_i64);
        *amount = amount.saturating_add(stack.amount);
    }
    for stack in produce.iter().filter(|stack| stack.amount > 0) {
        let Some(profile) = state.product_profiles.get(stack.kind.as_str()) else {
            continue;
        };
        for sink in profile
            .maintenance_sink
            .iter()
            .filter(|sink| sink.amount > 0)
        {
            if !merged.contains_key(sink.kind.as_str()) {
                order.push(sink.kind.clone());
            }
            let amount = merged.entry(sink.kind.clone()).or_insert(0_i64);
            *amount = amount.saturating_add(sink.amount.saturating_mul(stack.amount));
        }
    }
    order
        .into_iter()
        .filter_map(|kind| {
            merged
                .remove(&kind)
                .map(|amount| MaterialStack::new(kind, amount))
        })
        .collect()
}

fn ledger_material_balance(state: &WorldState, ledger_id: &MaterialLedgerId, kind: &str) -> i64 {
    state
        .material_ledgers
        .get(ledger_id)
        .and_then(|materials| materials.get(kind))
        .copied()
        .unwrap_or(0)
}
