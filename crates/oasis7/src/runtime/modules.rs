//! Module types and registry for WASM runtime integration.

pub use oasis7_wasm_abi::{
    EconomyModuleKind, FactoryBuildDecision, FactoryBuildRequest, FactoryModuleApi,
    FactoryModuleSpec, FactoryProfileV1, GameplayContract, GameplayModuleKind,
    MaterialDefaultPriority, MaterialProfileV1, MaterialStack, MaterialTransportLossClass,
    ModuleAbiContract, ModuleActivation, ModuleArtifact, ModuleArtifactIdentity, ModuleCache,
    ModuleChangeSet, ModuleDeactivation, ModuleEvent, ModuleEventKind, ModuleKind, ModuleLimits,
    ModuleManifest, ModuleRecord, ModuleRegistry, ModuleRole, ModuleSubscription,
    ModuleSubscriptionStage, ModuleUpgrade, ProductModuleApi, ProductModuleSpec, ProductProfileV1,
    ProductValidationDecision, ProductValidationRequest, RecipeExecutionPlan,
    RecipeExecutionRequest, RecipeModuleApi, RecipeModuleSpec, RecipeProfileV1,
};

/// Shared base duration authority for the built-in recipe catalog.
///
/// Batch count scales material/resource quantities, while each accepted batch
/// is processed by the same one-tick recipe execution plan.  Keep this lookup
/// shared by quote construction and runtime action construction so a quote
/// cannot drift from the `RecipeStarted` duration.
pub(crate) fn canonical_recipe_base_duration_ticks(recipe_id: &str) -> Option<u32> {
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
