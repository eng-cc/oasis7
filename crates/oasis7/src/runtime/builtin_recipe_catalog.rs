//! Canonical native plans for the eleven built-in production recipes.
//! External callers select an exact recipe ID and batch count, never a plan.
//! Trusted and module-backed custom plans retain their separate execution paths.
use super::RecipeExecutionPlan;
use oasis7_wasm_abi::MaterialStack;

fn scale_material_stacks(stacks: &[MaterialStack], scale: i64) -> Option<Vec<MaterialStack>> {
    stacks
        .iter()
        .map(|stack| {
            Some(MaterialStack::new(
                stack.kind.clone(),
                stack.amount.checked_mul(scale)?,
            ))
        })
        .collect()
}

/// Returns the complete built-in plan for an exact canonical ID and positive batches.
pub fn builtin_recipe_plan(recipe_id: &str, accepted_batches: u32) -> Option<RecipeExecutionPlan> {
    if accepted_batches == 0 {
        return None;
    }
    let accepted_batches_i64 = i64::from(accepted_batches);
    let (consume, produce, byproducts, power_per_batch): (
        Vec<MaterialStack>,
        Vec<MaterialStack>,
        Vec<MaterialStack>,
        i64,
    ) = match recipe_id {
        "recipe.smelter.iron_ingot" => (
            vec![
                MaterialStack::new("iron_ore", 4),
                MaterialStack::new("carbon_fuel", 1),
            ],
            vec![MaterialStack::new("iron_ingot", 3)],
            vec![MaterialStack::new("slag", 1)],
            8,
        ),
        "recipe.smelter.copper_wire" => (
            vec![MaterialStack::new("copper_ore", 3)],
            vec![MaterialStack::new("copper_wire", 4)],
            Vec::new(),
            6,
        ),
        "recipe.smelter.polymer_resin" => (
            vec![
                MaterialStack::new("carbon_fuel", 2),
                MaterialStack::new("silicate_ore", 2),
            ],
            vec![MaterialStack::new("polymer_resin", 2)],
            vec![MaterialStack::new("waste_resin", 1)],
            7,
        ),
        "recipe.smelter.alloy_plate" => (
            vec![
                MaterialStack::new("iron_ingot", 2),
                MaterialStack::new("copper_wire", 2),
            ],
            vec![MaterialStack::new("alloy_plate", 2)],
            vec![MaterialStack::new("slag", 1)],
            9,
        ),
        "recipe.assembler.gear" => (
            vec![MaterialStack::new("iron_ingot", 2)],
            vec![MaterialStack::new("gear", 1)],
            Vec::new(),
            4,
        ),
        "recipe.assembler.control_chip" => (
            vec![
                MaterialStack::new("copper_wire", 4),
                MaterialStack::new("polymer_resin", 2),
            ],
            vec![MaterialStack::new("control_chip", 1)],
            vec![MaterialStack::new("waste_resin", 1)],
            6,
        ),
        "recipe.assembler.motor_mk1" => (
            vec![
                MaterialStack::new("gear", 2),
                MaterialStack::new("copper_wire", 3),
            ],
            vec![MaterialStack::new("motor_mk1", 1)],
            Vec::new(),
            7,
        ),
        "recipe.assembler.logistics_drone" => (
            vec![
                MaterialStack::new("motor_mk1", 2),
                MaterialStack::new("control_chip", 1),
                MaterialStack::new("iron_ingot", 2),
            ],
            vec![MaterialStack::new("logistics_drone", 1)],
            vec![MaterialStack::new("assembly_scrap", 1)],
            12,
        ),
        "recipe.assembler.sensor_pack" => (
            vec![
                MaterialStack::new("control_chip", 1),
                MaterialStack::new("copper_wire", 2),
            ],
            vec![MaterialStack::new("sensor_pack", 1)],
            vec![MaterialStack::new("calibration_scrap", 1)],
            8,
        ),
        "recipe.assembler.module_rack" => (
            vec![
                MaterialStack::new("sensor_pack", 2),
                MaterialStack::new("control_chip", 1),
            ],
            vec![MaterialStack::new("module_rack", 1)],
            vec![MaterialStack::new("precision_scrap", 1)],
            10,
        ),
        "recipe.assembler.factory_core" => (
            vec![
                MaterialStack::new("module_rack", 1),
                MaterialStack::new("alloy_plate", 3),
            ],
            vec![MaterialStack::new("factory_core", 1)],
            vec![MaterialStack::new("structural_waste", 1)],
            14,
        ),
        _ => return None,
    };

    Some(RecipeExecutionPlan::accepted(
        accepted_batches,
        scale_material_stacks(&consume, accepted_batches_i64)?,
        scale_material_stacks(&produce, accepted_batches_i64)?,
        scale_material_stacks(&byproducts, accepted_batches_i64)?,
        power_per_batch.checked_mul(accepted_batches_i64)?,
        crate::simulator::canonical_recipe_base_duration_ticks(recipe_id)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Compare with the separately compiled built-in WASM recipe definitions.
    // These are evidence inputs only, not executable module code.
    const MODULE_SOURCES: [&str; 11] = [
        include_str!(
            "../../../oasis7_builtin_wasm_modules/m4_recipe_assembler_control_chip/src/lib.rs"
        ),
        include_str!(
            "../../../oasis7_builtin_wasm_modules/m4_recipe_assembler_factory_core/src/lib.rs"
        ),
        include_str!("../../../oasis7_builtin_wasm_modules/m4_recipe_assembler_gear/src/lib.rs"),
        include_str!(
            "../../../oasis7_builtin_wasm_modules/m4_recipe_assembler_logistics_drone/src/lib.rs"
        ),
        include_str!(
            "../../../oasis7_builtin_wasm_modules/m4_recipe_assembler_module_rack/src/lib.rs"
        ),
        include_str!(
            "../../../oasis7_builtin_wasm_modules/m4_recipe_assembler_motor_mk1/src/lib.rs"
        ),
        include_str!(
            "../../../oasis7_builtin_wasm_modules/m4_recipe_assembler_sensor_pack/src/lib.rs"
        ),
        include_str!(
            "../../../oasis7_builtin_wasm_modules/m4_recipe_smelter_alloy_plate/src/lib.rs"
        ),
        include_str!(
            "../../../oasis7_builtin_wasm_modules/m4_recipe_smelter_copper_wire/src/lib.rs"
        ),
        include_str!(
            "../../../oasis7_builtin_wasm_modules/m4_recipe_smelter_iron_ingot/src/lib.rs"
        ),
        include_str!(
            "../../../oasis7_builtin_wasm_modules/m4_recipe_smelter_polymer_resin/src/lib.rs"
        ),
    ];

    fn constant<'a>(source: &'a str, name: &str) -> &'a str {
        source
            .split_once(&format!("const {name}:"))
            .expect("built-in module constant")
            .1
            .split_once('=')
            .unwrap()
            .1
            .split_once(';')
            .unwrap()
            .0
            .trim()
    }
    fn module_stacks(source: &str, name: &str, batches: u32) -> Vec<MaterialStack> {
        let json = constant(source, name)
            .trim_start_matches('&')
            .replace('(', "[")
            .replace(')', "]");
        let entries: Vec<(String, i64)> = serde_json::from_str(&json).unwrap();
        entries
            .into_iter()
            .map(|(kind, amount)| MaterialStack::new(kind, amount * i64::from(batches)))
            .collect()
    }

    #[test]
    fn builtin_recipe_plans_match_all_eleven_wasm_definitions() {
        let mut unique = std::collections::BTreeSet::new();
        for source in MODULE_SOURCES {
            let id = constant(source, "RECIPE_ID").trim_matches('"');
            assert!(unique.insert(id), "duplicate built-in module recipe");
            for batches in [1, 7, u32::MAX] {
                let expected = RecipeExecutionPlan::accepted(
                    batches,
                    module_stacks(source, "CONSUME_PER_BATCH", batches),
                    module_stacks(source, "PRODUCE_PER_BATCH", batches),
                    module_stacks(source, "BYPRODUCTS_PER_BATCH", batches),
                    constant(source, "POWER_PER_BATCH").parse::<i64>().unwrap()
                        * i64::from(batches),
                    constant(source, "DURATION_TICKS").parse::<u32>().unwrap(),
                );
                let actual = builtin_recipe_plan(id, batches).expect("canonical built-in ID");
                assert_eq!(
                    actual, expected,
                    "complete canonical plan {id}, batches={batches}"
                );
            }
            assert!(builtin_recipe_plan(id, 0).is_none());
        }
        assert_eq!(unique.len(), 11);
    }

    #[test]
    fn builtin_recipe_catalog_rejects_aliases_and_noncanonical_ids() {
        for id in [
            "recipe.iron_ingot",
            "RECIPE.SMELTER.IRON_INGOT",
            " recipe.smelter.iron_ingot",
            "recipe.smelter.iron_ingot ",
            "recipe.custom",
            "",
        ] {
            assert!(
                builtin_recipe_plan(id, 1).is_none(),
                "unsupported ID {id:?}"
            );
        }
    }
}
