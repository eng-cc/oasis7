use crate::runtime::IndustryStage;
use crate::simulator::persist::{
    PlayerGameplayAction, PlayerGameplayBranchCommitment, PlayerGameplayStageStatus,
};

pub(super) fn branch_recommendations(
    industry_stage: IndustryStage,
    stage_status: PlayerGameplayStageStatus,
    available_actions: &[PlayerGameplayAction],
) -> Vec<PlayerGameplayBranchCommitment> {
    if stage_status != PlayerGameplayStageStatus::BranchReady {
        return Vec::new();
    }

    let enabled = |action_id: &str| {
        available_actions
            .iter()
            .any(|action| action.action_id == action_id && action.disabled_reason.is_none())
    };
    let commitment =
        |action_id: &str,
         route_label: &str,
         immediate_gain: &str,
         future_beat_changed: &str,
         risk_or_lockin: &str,
         next_session_hook: &str| PlayerGameplayBranchCommitment {
            action_id: action_id.to_string(),
            route_label: route_label.to_string(),
            immediate_gain: immediate_gain.to_string(),
            future_beat_changed: future_beat_changed.to_string(),
            risk_or_lockin: risk_or_lockin.to_string(),
            next_session_hook: next_session_hook.to_string(),
        };

    let scale_out_candidates = [
        commitment(
            "schedule_recipe_smelter_alloy_plate",
            "Deepen the smelter line",
            "Convert the stable line into higher-tier alloy output.",
            "The next beat shifts from basic throughput to advanced inputs.",
            "Consumes line time that could have supplied basic materials.",
            "Return to turn the alloy output into the next production upgrade.",
        ),
        commitment(
            "build_factory_assembler_mk1",
            "Open a second production line",
            "Add the assembler capability and widen the real recipe surface.",
            "The next beat becomes coordinating two complementary factories.",
            "Commits construction materials before the new line produces value.",
            "Return to choose and run the assembler's first recipe.",
        ),
    ];
    let governance_candidates = [
        commitment(
            "schedule_recipe_assembler_module_rack",
            "Produce a module rack",
            "Create a governance-stage finished component.",
            "The next beat shifts toward assembling durable industrial modules.",
            "Uses advanced assembler inputs and occupies the current recipe slot.",
            "Return to connect the module rack to a factory-core production run.",
        ),
        commitment(
            "schedule_recipe_assembler_factory_core",
            "Produce a factory core",
            "Create infrastructure output from the mature assembler line.",
            "The next beat shifts toward deploying another industrial capability.",
            "Consumes a module rack and alloy plate before yielding the core.",
            "Return to decide where the new factory core creates the most leverage.",
        ),
        commitment(
            "schedule_recipe_smelter_alloy_plate",
            "Sustain alloy throughput",
            "Produce another alloy plate through the mature smelter line.",
            "The next beat keeps advanced assembler inputs supplied while other routes unlock.",
            "Uses the smelter queue and iron inputs without automatically advancing governance.",
            "Return to inspect the alloy output and choose the next advanced recipe.",
        ),
    ];

    let candidates = match industry_stage {
        IndustryStage::ScaleOut => scale_out_candidates.as_slice(),
        IndustryStage::Governance => governance_candidates.as_slice(),
        IndustryStage::Bootstrap => return Vec::new(),
    };
    candidates
        .into_iter()
        .filter(|candidate| enabled(candidate.action_id.as_str()))
        .take(3)
        .cloned()
        .collect()
}

pub(super) fn effective_branch_stage_status(
    stage_status: PlayerGameplayStageStatus,
    recommendations: &[PlayerGameplayBranchCommitment],
    available_actions: &[PlayerGameplayAction],
) -> PlayerGameplayStageStatus {
    let published_branch_action = available_actions.iter().any(|action| {
        matches!(
            action.action_id.as_str(),
            "schedule_recipe_smelter_alloy_plate"
                | "build_factory_assembler_mk1"
                | "schedule_recipe_assembler_module_rack"
                | "schedule_recipe_assembler_factory_core"
        )
    });
    if stage_status == PlayerGameplayStageStatus::BranchReady
        && recommendations.is_empty()
        && !published_branch_action
    {
        PlayerGameplayStageStatus::Blocked
    } else {
        stage_status
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn branch_ready_stays_discoverable_when_every_published_candidate_is_disabled() {
        let actions = vec![PlayerGameplayAction {
            action_id: "schedule_recipe_smelter_alloy_plate".to_string(),
            label: "Schedule alloy plate".to_string(),
            protocol_action: "gameplay_action.submit".to_string(),
            target_agent_id: None,
            disabled_reason: Some("missing iron ingot".to_string()),
        }];
        let recommendations = branch_recommendations(
            IndustryStage::ScaleOut,
            PlayerGameplayStageStatus::BranchReady,
            actions.as_slice(),
        );

        assert!(recommendations.is_empty());
        assert_eq!(
            effective_branch_stage_status(
                PlayerGameplayStageStatus::BranchReady,
                recommendations.as_slice(),
                actions.as_slice(),
            ),
            PlayerGameplayStageStatus::BranchReady,
        );
    }
}
