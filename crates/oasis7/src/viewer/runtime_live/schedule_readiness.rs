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
    let available_data = resources.get(ResourceKind::Data);
    if available_data < readiness.data_cost {
        return Some(format!(
            "insufficient data: need {}, have {}; replenish data before scheduling this run",
            readiness.data_cost, available_data
        ));
    }
    let factory = state.factories.get(factory_id.as_str())?;
    let consume = recipe_consume_with_maintenance_sinks(state, &plan.consume, &plan.produce);
    let consume_ledger = if ledger_has_materials(state, &factory.input_ledger, &consume) {
        factory.input_ledger.clone()
    } else {
        MaterialLedgerId::world()
    };
    for stack in &consume {
        let available = ledger_material_balance(state, &consume_ledger, stack.kind.as_str());
        if available < stack.amount {
            return Some(format!(
                "insufficient {} in {} ledger: need {}, have {}; replenish {} before scheduling this run",
                stack.kind, consume_ledger, stack.amount, available, stack.kind
            ));
        }
    }
    let available_world_electricity = state
        .resources
        .get(&ResourceKind::Electricity)
        .copied()
        .unwrap_or_default();
    if available_world_electricity < plan.power_required {
        return Some(format!(
            "insufficient world electricity: need {}, have {}; replenish electricity before scheduling this run",
            plan.power_required, available_world_electricity
        ));
    }
    None
}

fn recipe_consume_with_maintenance_sinks(
    state: &WorldState,
    consume: &[MaterialStack],
    produce: &[MaterialStack],
) -> Vec<MaterialStack> {
    let mut merged = BTreeMap::new();
    for stack in consume {
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
            let amount = merged.entry(sink.kind.clone()).or_insert(0_i64);
            *amount = amount.saturating_add(sink.amount.saturating_mul(stack.amount));
        }
    }
    merged
        .into_iter()
        .map(|(kind, amount)| MaterialStack::new(kind, amount))
        .collect()
}

fn ledger_has_materials(
    state: &WorldState,
    ledger_id: &MaterialLedgerId,
    consume: &[MaterialStack],
) -> bool {
    consume
        .iter()
        .all(|stack| ledger_material_balance(state, ledger_id, stack.kind.as_str()) >= stack.amount)
}

fn ledger_material_balance(state: &WorldState, ledger_id: &MaterialLedgerId, kind: &str) -> i64 {
    state
        .material_ledgers
        .get(ledger_id)
        .and_then(|materials| materials.get(kind))
        .copied()
        .unwrap_or(0)
}
