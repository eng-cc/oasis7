/// Shared base duration authority for the built-in recipe catalog.
///
/// This lives on the simulator surface because quotes are also compiled for
/// the launcher WASM target, where the native runtime module is unavailable.
/// Batch count scales material/resource quantities while the accepted plan
/// keeps the catalog's one-tick base duration.
pub fn canonical_recipe_base_duration_ticks(recipe_id: &str) -> Option<u32> {
    match recipe_id.trim().to_ascii_lowercase().as_str() {
        "recipe.smelter.iron_ingot"
        | "recipe.iron_ingot"
        | "recipe.smelter.copper_wire"
        | "recipe.copper_wire"
        | "recipe.smelter.polymer_resin"
        | "recipe.polymer_resin"
        | "recipe.smelter.alloy_plate"
        | "recipe.alloy_plate"
        | "recipe.assembler.gear"
        | "recipe.gear"
        | "recipe.assembler.control_chip"
        | "recipe.control_chip"
        | "recipe.assembler.motor_mk1"
        | "recipe.motor_mk1"
        | "recipe.assembler.logistics_drone"
        | "recipe.logistics_drone"
        | "recipe.assembler.sensor_pack"
        | "recipe.sensor_pack"
        | "recipe.assembler.module_rack"
        | "recipe.module_rack"
        | "recipe.assembler.factory_core"
        | "recipe.factory_core" => Some(1),
        _ => None,
    }
}
