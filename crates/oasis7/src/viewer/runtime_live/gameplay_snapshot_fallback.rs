use crate::simulator::persist::{PlayerGameplayFallbackTradeoffOption, PlayerGameplaySnapshot};

pub(super) fn player_gameplay_fallback_tradeoff_preview(
    gameplay: &PlayerGameplaySnapshot,
    response_window_class: Option<&str>,
) -> Option<Vec<PlayerGameplayFallbackTradeoffOption>> {
    if !matches!(
        response_window_class,
        Some("stalled_needs_escalation" | "blocked_needs_repair")
    ) {
        return None;
    }

    let action_available = |action_id: &str| {
        gameplay
            .available_actions
            .iter()
            .any(|action| action.action_id == action_id && action.disabled_reason.is_none())
    };
    let repair_available = action_available("request_snapshot");
    let reroute_available = action_available("reprioritize");
    let recommended_value_class = if repair_available {
        "repair_now"
    } else if reroute_available {
        "reroute_now"
    } else {
        "safe_wait"
    };

    Some(vec![
        PlayerGameplayFallbackTradeoffOption {
            value_class: "safe_wait".to_string(),
            available: false,
            cost: "No bounded wait trigger is currently available.".to_string(),
            progress_kept: "Keeps the current intent unchanged.".to_string(),
            opportunity_cost: "Waiting cannot verify or repair the blocker.".to_string(),
            reason: "The runtime has no canonical tick or event trigger that bounds a safe wait."
                .to_string(),
            recommended: recommended_value_class == "safe_wait",
        },
        PlayerGameplayFallbackTradeoffOption {
            value_class: "repair_now".to_string(),
            available: repair_available,
            cost: "Refresh the gameplay snapshot and inspect the current blocker.".to_string(),
            progress_kept: "Keeps the current intent while checking recovery state.".to_string(),
            opportunity_cost: "Uses the next decision on diagnosis instead of a new goal."
                .to_string(),
            reason: if repair_available {
                "A snapshot refresh is available to verify or repair the published blocker."
            } else {
                "No enabled repair-oriented snapshot action is currently available."
            }
            .to_string(),
            recommended: recommended_value_class == "repair_now",
        },
        PlayerGameplayFallbackTradeoffOption {
            value_class: "reroute_now".to_string(),
            available: reroute_available,
            cost: "Replace the current Agent short-term goal.".to_string(),
            progress_kept: "Preserves the recorded intent for comparison, not execution progress."
                .to_string(),
            opportunity_cost: "Moves attention from repairing the current blocked intent."
                .to_string(),
            reason: if reroute_available {
                "An enabled reprioritize action can redirect the current goal."
            } else {
                "No enabled reprioritize action is currently available."
            }
            .to_string(),
            recommended: recommended_value_class == "reroute_now",
        },
    ])
}
