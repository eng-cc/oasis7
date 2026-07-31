use super::pos;
use crate::runtime::{Action, World};
use crate::simulator::ResourceKind;

fn register_agents(world: &mut World, agent_ids: &[&str]) {
    for (index, agent_id) in agent_ids.iter().enumerate() {
        world.submit_action(Action::RegisterAgent {
            agent_id: (*agent_id).to_string(),
            pos: pos(index as i64, 0),
        });
    }
    world.step().expect("register agents");
}

fn form_war_alliances(world: &mut World) {
    world.submit_action(Action::FormAlliance {
        proposer_agent_id: "a".to_string(),
        alliance_id: "alliance.red".to_string(),
        members: vec!["b".to_string()],
        charter: "charter.red".to_string(),
    });
    world.step().expect("form aggressor alliance");
    world.submit_action(Action::FormAlliance {
        proposer_agent_id: "c".to_string(),
        alliance_id: "alliance.blue".to_string(),
        members: vec!["d".to_string(), "e".to_string()],
        charter: "charter.blue".to_string(),
    });
    world.step().expect("form defender alliance");
}

fn seed_declaration_resources(world: &mut World) {
    world
        .set_agent_resource_balance("a", ResourceKind::Electricity, 120)
        .expect("seed aggressor electricity");
    world
        .set_agent_resource_balance("a", ResourceKind::Data, 120)
        .expect("seed aggressor data");
}

#[test]
fn war_declaration_quote_is_deterministic_non_mutating_and_exposes_projected_core_outcome() {
    let mut world = World::new();
    register_agents(&mut world, &["a", "b", "c", "d", "e"]);
    form_war_alliances(&mut world);

    let snapshot_before = world.snapshot();
    let journal_before = world.journal().clone();
    let quote = world
        .war_declaration_quote("a", "alliance.red", "alliance.blue", 3)
        .expect("declaring alliance can inspect a war quote before submission");
    let repeated_quote = world
        .war_declaration_quote("a", "alliance.red", "alliance.blue", 3)
        .expect("war quote remains available without submitting an action");

    assert_eq!(quote, repeated_quote);
    assert_eq!(world.snapshot(), snapshot_before);
    assert_eq!(world.journal(), &journal_before);
    assert_eq!(quote.actor_alliance_id, "alliance.red");
    assert_eq!(quote.target_alliance_id, "alliance.blue");
    assert_eq!(quote.action_kind, "declare_war");
    assert_eq!(quote.intensity, 3);
    assert_eq!(quote.settlement_path, "core_fallback");
    assert_eq!(quote.war_duration_ticks, 12);
    assert_eq!(quote.aggressor_score_estimate, 23);
    assert_eq!(quote.defender_score_estimate, 30);
    assert_eq!(quote.likely_winner_before_action, "alliance.blue");
    assert_eq!(quote.victory_margin_estimate, -7);
    assert!(quote.conflict_window_blocked_until > world.state().time);
    assert!(quote.settlement_risk.contains("loss"));
    assert!(!quote.expected_narrative_or_module_reward.is_empty());
}

#[test]
fn war_declaration_quote_reports_minimum_winning_intensity_from_the_active_settlement_path() {
    let mut world = World::new();
    register_agents(&mut world, &["a", "b", "c", "d", "e"]);
    form_war_alliances(&mut world);

    let quote = world
        .war_declaration_quote("a", "alliance.red", "alliance.blue", 3)
        .expect("war quote for a reachable threshold");

    // Core score: red members 2 * 10 + intensity, blue members 3 * 10. A tie wins.
    assert_eq!(quote.minimum_winning_intensity, Some(10));
    assert_eq!(quote.recommended_war_action, "recruit");
    assert_eq!(quote.alternative_action, "recruit");
    assert!(quote.why_this_war_is_worth_or_risky.contains("minimum"));
}

#[test]
fn war_declaration_quote_keeps_an_active_conflict_blocker_and_wait_alternative_visible() {
    let mut world = World::new();
    register_agents(&mut world, &["a", "b", "c", "d", "e"]);
    form_war_alliances(&mut world);
    seed_declaration_resources(&mut world);
    world.submit_action(Action::DeclareWar {
        initiator_agent_id: "a".to_string(),
        war_id: "war.quote.active".to_string(),
        aggressor_alliance_id: "alliance.red".to_string(),
        defender_alliance_id: "alliance.blue".to_string(),
        objective: "hold belt".to_string(),
        intensity: 3,
    });
    world.step().expect("declare active war");

    let snapshot_before = world.snapshot();
    let journal_before = world.journal().clone();
    let quote = world
        .war_declaration_quote("a", "alliance.red", "alliance.blue", 3)
        .expect("active-conflict quote must explain why another declaration is blocked");

    assert_eq!(world.snapshot(), snapshot_before);
    assert_eq!(world.journal(), &journal_before);
    assert!(
        quote
            .reentry_cooldown_or_active_conflict_blocker
            .contains("active war")
    );
    assert_eq!(quote.alternative_action, "wait");
    assert_eq!(quote.recommended_war_action, "wait");
    assert!(quote.why_this_war_is_worth_or_risky.contains("active"));
}
