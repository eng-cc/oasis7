use super::*;

fn failure_disposition(job_id: ActionId) -> FactoryProductionFailureDispositionV1 {
    FactoryProductionFailureDispositionV1 {
        action_id: job_id,
        requester_agent_id: "builder-a".to_string(),
        factory_id: "factory.test".to_string(),
        recipe_id: "recipe.test".to_string(),
        blocker_kind: "product_validation".to_string(),
        blocker_detail: "test rejection".to_string(),
        disposition_kind: "consumed_lost".to_string(),
        consumed_inputs: Vec::new(),
        lost_inputs: Vec::new(),
        consumed_power: 0,
        lost_power: 0,
        next_action: "retry".to_string(),
        next_recheck: None,
    }
}

fn blocked_factory(action_id: ActionId) -> FactoryState {
    let mut production = FactoryProductionState::default();
    production.current_blocker_kind = Some("product_validation".to_string());
    production.current_blocker_detail = Some("test rejection".to_string());
    production.current_blocker_action_id = Some(action_id);
    FactoryState {
        factory_id: "factory.test".to_string(),
        site_id: "site.test".to_string(),
        builder_agent_id: "builder-a".to_string(),
        spec: FactoryModuleSpec {
            factory_id: "factory.test".to_string(),
            display_name: "Test Factory".to_string(),
            tier: 1,
            tags: Vec::new(),
            build_cost: Vec::new(),
            build_time_ticks: 1,
            base_power_draw: 0,
            recipe_slots: 1,
            throughput_bps: 1,
            maintenance_per_tick: 0,
        },
        input_ledger: MaterialLedgerId::world(),
        output_ledger: MaterialLedgerId::world(),
        durability_ppm: 1_000_000,
        production,
        site_authority_revision: None,
        site_location_id: None,
        construction_power_profile_key: None,
        construction_power_profile_revision: None,
        built_at: 0,
    }
}

#[test]
fn legacy_factory_production_state_defaults_blocker_action_identity() {
    let production: FactoryProductionState =
        serde_json::from_str("{}").expect("legacy production state");
    assert_eq!(production.current_blocker_action_id, None);
}

#[test]
fn current_factory_blocker_protects_unresolved_failure_disposition() {
    let mut state = WorldState::default();
    state
        .factories
        .insert("factory.test".to_string(), blocked_factory(1));
    for job_id in 1..=65 {
        state.settled_recipe_job_ids.insert(job_id);
        let mut disposition = failure_disposition(job_id);
        if job_id > 1 {
            disposition.factory_id = format!("factory.other-{job_id}");
        }
        state
            .factory_production_failure_dispositions
            .insert(job_id, disposition);
    }
    state.compact_settled_industry_history();
    assert!(
        state
            .factory_production_failure_dispositions
            .contains_key(&1)
    );
    assert_eq!(state.factory_production_failure_dispositions.len(), 64);

    let production = &mut state.factories.get_mut("factory.test").unwrap().production;
    production.current_blocker_kind = None;
    production.current_blocker_detail = None;
    production.current_blocker_action_id = None;
    state.settled_recipe_job_ids.insert(66);
    let mut disposition = failure_disposition(66);
    disposition.factory_id = "factory.other-66".to_string();
    state
        .factory_production_failure_dispositions
        .insert(66, disposition);
    state.compact_settled_industry_history();
    assert!(
        !state
            .factory_production_failure_dispositions
            .contains_key(&1)
    );
    assert_eq!(state.factory_production_failure_dispositions.len(), 64);

    state
        .factories
        .insert("factory.test".to_string(), blocked_factory(1));
    state.factory_production_failure_dispositions.clear();
    state.settled_recipe_job_ids.clear();
    for job_id in (u64::MAX - 63)..=u64::MAX {
        state.settled_recipe_job_ids.insert(job_id);
        state
            .factory_production_failure_dispositions
            .insert(job_id, failure_disposition(job_id));
    }
    state.settled_recipe_job_ids.insert(1);
    state
        .factory_production_failure_dispositions
        .insert(1, failure_disposition(1));
    state.compact_settled_industry_history();
    assert!(
        state
            .factory_production_failure_dispositions
            .contains_key(&1)
    );
    assert_eq!(state.factory_production_failure_dispositions.len(), 64);
}
