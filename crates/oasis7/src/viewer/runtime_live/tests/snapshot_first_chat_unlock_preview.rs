use super::snapshot_progress::{SNAPSHOT_PLAYER_ID, bind_agent_for_snapshot};
use super::*;

#[test]
fn compat_snapshot_projects_first_chat_unlock_preview_only_for_zero_liquid_without_starter_oc() {
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");
    let primary_agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("primary agent");
    bind_agent_for_snapshot(&mut server, primary_agent_id.as_str());

    let snapshot = server.compat_snapshot(Some(SNAPSHOT_PLAYER_ID));
    let preview = snapshot
        .player_gameplay
        .as_ref()
        .and_then(|gameplay| gameplay.agent_claim.as_ref())
        .and_then(|claim| claim.first_chat_unlock_preview.as_ref())
        .expect("zero-liquid starter gate publishes first-chat unlock preview");
    assert_eq!(
        preview.chat_purpose,
        "Start a first conversation with your claimed Agent."
    );
    assert_eq!(
        preview.immediate_playable_help,
        "Ask what the Agent can do next for the current gameplay goal."
    );
    assert_eq!(
        preview.first_question_or_action_hint,
        "Ask: What should we do first?"
    );
    assert_eq!(
        preview.resource_boundary,
        "Starter OC unlocks first chat and initial liquid OC; it is separate from slot-1 claim and upkeep funding."
    );
    assert_eq!(
        preview.defer_effect,
        "Deferring keeps the completed claim and its upkeep responsibility, but first chat stays locked while liquid OC is zero and no starter OC claim exists."
    );
    assert_eq!(preview.recommended_unlock_action, "claim_starter_oc");

    let mut nonzero_liquid_state = server.world.state().clone();
    nonzero_liquid_state
        .main_token_balances
        .entry(primary_agent_id.clone())
        .or_default()
        .liquid_balance = 1;
    assert!(
        super::super::claim_snapshot::build_player_agent_claim_snapshot(
            &nonzero_liquid_state,
            primary_agent_id.as_str(),
            server
                .world
                .governance_execution_policy()
                .epoch_length_ticks,
        )
        .expect("primary agent claim snapshot")
        .first_chat_unlock_preview
        .is_none()
    );

    let mut claimed_starter_oc_state = server.world.state().clone();
    claimed_starter_oc_state
        .starter_oc_claims
        .insert(primary_agent_id.clone(), Default::default());
    assert!(
        super::super::claim_snapshot::build_player_agent_claim_snapshot(
            &claimed_starter_oc_state,
            primary_agent_id.as_str(),
            server
                .world
                .governance_execution_policy()
                .epoch_length_ticks,
        )
        .expect("primary agent claim snapshot")
        .first_chat_unlock_preview
        .is_none()
    );

    assert!(
        super::super::claim_snapshot::build_player_agent_claim_snapshot(
            server.world.state(),
            "missing-primary-agent",
            server
                .world
                .governance_execution_policy()
                .epoch_length_ticks,
        )
        .is_none()
    );
}
