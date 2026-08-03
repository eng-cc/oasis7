use super::{GameplayActionRequest, build_runtime_action_from_gameplay_request};
use crate::runtime::{Action as RuntimeAction, WorldState};
use crate::simulator::{ResourceKind, default_schedule_recipe_readiness};

pub(super) fn schedule_recipe_disabled_reason(
    state: &WorldState,
    agent_id: &str,
    action_id: &str,
) -> Option<String> {
    let RuntimeAction::ScheduleRecipe { recipe_id, .. } =
        build_runtime_action_from_gameplay_request(&GameplayActionRequest {
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
    None
}
