use crate::runtime::{
    FactoryModuleSpec, FactoryProductionState, FactoryState, IndustryStage, MaterialLedgerId,
    MaterialStack, WorldState,
};
use crate::viewer::FACTORY_SMELTER_MK1;

fn small_player_test_factory_spec(factory_id: &str) -> FactoryModuleSpec {
    FactoryModuleSpec {
        factory_id: factory_id.to_string(),
        display_name: "Test Smelter MK1".to_string(),
        tier: 2,
        tags: vec!["smelter".to_string(), "thermal".to_string()],
        build_cost: vec![MaterialStack::new("structural_frame", 12)],
        build_time_ticks: 1,
        base_power_draw: 20,
        recipe_slots: 2,
        throughput_bps: 10_000,
        maintenance_per_tick: 1,
    }
}

#[test]
fn runtime_gameplay_snapshot_flags_grind_only_after_repeating_same_loop_without_new_leverage() {
    let mut state = WorldState::default();
    state.industry_progress.stage = IndustryStage::Bootstrap;
    state.industry_progress.completed_recipe_jobs = 4;
    state.factories.insert(
        FACTORY_SMELTER_MK1.to_string(),
        FactoryState {
            factory_id: FACTORY_SMELTER_MK1.to_string(),
            site_id: "runtime:10:20:0".to_string(),
            builder_agent_id: "agent-1".to_string(),
            spec: small_player_test_factory_spec(FACTORY_SMELTER_MK1),
            input_ledger: MaterialLedgerId::world(),
            output_ledger: MaterialLedgerId::world(),
            durability_ppm: 1_000_000,
            production: FactoryProductionState {
                completed_jobs: 4,
                last_completed_at: Some(12),
                last_completed_recipe_id: Some("recipe.smelter.iron_ingot".to_string()),
                same_recipe_repeat_count: 3,
                ..FactoryProductionState::default()
            },
            built_at: 1,
        },
    );

    let gameplay = super::super::gameplay_snapshot::build_player_gameplay_snapshot(
        &state, None, true, None, None, None, true, None, false, true, None,
    );

    assert_eq!(
        gameplay.goal_id,
        "post_onboarding.stabilize_first_line_after_output"
    );
    assert_eq!(gameplay.same_loop_repeat_count, 3);
    assert_eq!(gameplay.leverage_class.as_deref(), Some("throughput_only"));
    assert!(gameplay.grind_only_flag);
}
