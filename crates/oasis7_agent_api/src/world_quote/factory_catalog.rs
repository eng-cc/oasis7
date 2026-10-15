//! Canonical native factory specifications shared by all host entrypoints.
use oasis7_wasm_abi::{FactoryModuleSpec, MaterialStack};

pub fn builtin_factory_spec(factory_kind: &str, factory_id: &str) -> Option<FactoryModuleSpec> {
    let mut spec = match factory_kind {
        "factory.smelter.mk1" => FactoryModuleSpec {
            factory_id: "factory.smelter.mk1".to_string(),
            display_name: "Smelter MK1".to_string(),
            tier: 2,
            tags: vec!["smelter".to_string(), "thermal".to_string()],
            build_cost: vec![
                MaterialStack::new("structural_frame", 12),
                MaterialStack::new("heat_coil", 4),
                MaterialStack::new("refractory_brick", 6),
            ],
            build_time_ticks: 1,
            base_power_draw: 20,
            recipe_slots: 2,
            throughput_bps: 10_000,
            maintenance_per_tick: 1,
        },
        "factory.assembler.mk1" => FactoryModuleSpec {
            factory_id: "factory.assembler.mk1".to_string(),
            display_name: "Assembler MK1".to_string(),
            tier: 3,
            tags: vec!["assembler".to_string(), "precision".to_string()],
            build_cost: vec![
                MaterialStack::new("structural_frame", 8),
                MaterialStack::new("iron_ingot", 10),
                MaterialStack::new("copper_wire", 8),
            ],
            build_time_ticks: 1,
            base_power_draw: 20,
            recipe_slots: 2,
            throughput_bps: 10_000,
            maintenance_per_tick: 1,
        },
        _ => return None,
    };
    spec.factory_id = factory_id.to_string();
    Some(spec)
}
