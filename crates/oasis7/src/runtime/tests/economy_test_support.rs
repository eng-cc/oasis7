use crate::runtime::{
    AgentLocationAuthorityV1, FactoryConstructionPowerMode, FactoryConstructionPowerProfileV1,
    FactorySiteAuthorityV1, World,
};
use crate::simulator::ResourceKind;
use oasis7_wasm_abi::{FactoryModuleSpec, MaterialStack};

pub(super) fn factory_spec(
    factory_id: &str,
    build_time_ticks: u32,
    recipe_slots: u16,
) -> FactoryModuleSpec {
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

pub(super) fn authorize_factory_build(
    world: &mut World,
    builder_agent_id: &str,
    site_id: &str,
    factory_id: &str,
) {
    let location_id = format!("location-{site_id}");
    world
        .set_agent_location_authority(AgentLocationAuthorityV1 {
            agent_id: builder_agent_id.to_string(),
            location_id: location_id.clone(),
            active: true,
            authority_revision: 1,
            effective_at: 0,
        })
        .expect("install builder location authority");
    world
        .set_factory_site_authority(FactorySiteAuthorityV1 {
            site_id: site_id.to_string(),
            location_id,
            owner_agent_id: builder_agent_id.to_string(),
            authorized_agent_ids: Vec::new(),
            chunk_ready: true,
            active: true,
            authority_revision: 1,
            registered_at: 0,
        })
        .expect("install factory site authority");
    world
        .set_factory_construction_power_profile(FactoryConstructionPowerProfileV1 {
            factory_id: factory_id.to_string(),
            factory_kind: "test".to_string(),
            source_module_id: None,
            electricity_amount: 10,
            mode: FactoryConstructionPowerMode::StartOnlySink,
            authority_revision: 1,
            active: true,
        })
        .expect("install construction power profile");
    world
        .set_agent_resource_balance(builder_agent_id, ResourceKind::Electricity, 10)
        .expect("seed construction power");
}

pub(super) fn bind_factory_build_module(world: &mut World, factory_id: &str, module_id: &str) {
    world
        .set_factory_construction_power_profile(FactoryConstructionPowerProfileV1 {
            factory_id: factory_id.to_string(),
            factory_kind: "test".to_string(),
            source_module_id: Some(module_id.to_string()),
            electricity_amount: 10,
            mode: FactoryConstructionPowerMode::StartOnlySink,
            authority_revision: 2,
            active: true,
        })
        .expect("bind construction power profile to module");
}
