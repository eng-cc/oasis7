use super::pos;
use crate::runtime::{
    Action, DomainEvent, FactoryProfileV1, GovernanceProposalStatus, IndustryStage,
    MaterialDefaultPriority, MaterialLedgerId, MaterialProfileV1, MaterialTransitPriority,
    MaterialTransportLossClass, ProductProfileV1, ProposalDecision, RecipeProfileV1, RejectReason,
    World, WorldEventBody,
};
use crate::simulator::ResourceKind;
use oasis7_wasm_abi::{FactoryModuleSpec, MaterialStack, RecipeExecutionPlan};

#[path = "economy_priority_governance_tests.rs"]
mod governance_tests;

fn factory_spec(factory_id: &str, build_time_ticks: u32, recipe_slots: u16) -> FactoryModuleSpec {
    FactoryModuleSpec {
        factory_id: factory_id.to_string(),
        display_name: "Test Factory".to_string(),
        tier: 1,
        tags: vec!["assembly".to_string()],
        build_cost: vec![
            MaterialStack::new("steel_plate", 10),
            MaterialStack::new("circuit_board", 2),
        ],
        build_time_ticks,
        base_power_draw: 5,
        recipe_slots,
        throughput_bps: 10_000,
        maintenance_per_tick: 1,
    }
}

fn authorize_policy_update(world: &mut World, operator_agent_id: &str, proposal_key: &str) {
    world.submit_action(Action::OpenGovernanceProposal {
        proposer_agent_id: operator_agent_id.to_string(),
        proposal_key: proposal_key.to_string(),
        title: format!("title.{proposal_key}"),
        description: "authorize gameplay policy update".to_string(),
        options: vec!["approve".to_string(), "reject".to_string()],
        voting_window_ticks: 1,
        quorum_weight: 3,
        pass_threshold_bps: 5_000,
    });
    world.step().expect("open governance proposal");

    world.submit_action(Action::CastGovernanceVote {
        voter_agent_id: operator_agent_id.to_string(),
        proposal_key: proposal_key.to_string(),
        option: "approve".to_string(),
        weight: 3,
    });
    world.step().expect("cast governance vote");

    for _ in 0..2 {
        let Some(proposal) = world.state().governance_proposals.get(proposal_key) else {
            break;
        };
        if proposal.status != GovernanceProposalStatus::Open {
            break;
        }
        world.step().expect("advance governance proposal");
    }

    let proposal = world
        .state()
        .governance_proposals
        .get(proposal_key)
        .expect("proposal finalized");
    assert_eq!(proposal.status, GovernanceProposalStatus::Passed);
}

fn approved_manifest_proposal(world: &mut World, author: &str) -> u64 {
    let mut manifest = world.manifest().clone();
    manifest.version = manifest.version.saturating_add(1);
    let proposal_id = world
        .propose_manifest_update(manifest, author.to_string())
        .expect("propose manifest update");
    world
        .shadow_proposal(proposal_id)
        .expect("shadow manifest proposal");
    world
        .approve_proposal(proposal_id, author.to_string(), ProposalDecision::Approve)
        .expect("approve manifest proposal");
    proposal_id
}

fn latest_action_rejected_message(world: &World) -> String {
    world
        .journal()
        .events
        .iter()
        .rev()
        .find_map(|event| match &event.body {
            WorldEventBody::Domain(DomainEvent::ActionRejected { reason, .. }) => {
                Some(format!("{reason:?}"))
            }
            _ => None,
        })
        .expect("action rejected")
}

fn latest_factory_production_block(world: &World) -> (String, String) {
    world
        .journal()
        .events
        .iter()
        .rev()
        .find_map(|event| match &event.body {
            WorldEventBody::Domain(DomainEvent::FactoryProductionBlocked {
                blocker_kind,
                blocker_detail,
                ..
            }) => Some((blocker_kind.clone(), blocker_detail.clone())),
            _ => None,
        })
        .expect("factory production blocked")
}

include!("economy_priority_logistics_priority_tests.rs");
include!("economy_priority_logistics_recipe_tests.rs");
include!("economy_priority_logistics_network_tests.rs");
