//! World Simulator module - provides the simulation kernel, agent interface, and world model.
//!
//! This module is organized into submodules:
//! - `types`: Core type definitions (IDs, constants, resources)
//! - `world_model`: World entities (Agent, Location, Asset, WorldModel)
//! - `kernel`: WorldKernel implementation (time, events, actions)
//! - `persist`: Snapshot, Journal, and persistence utilities
//! - `agent`: Agent interface trait and decision types
//! - `memory`: Agent memory system (short-term, long-term)
//! - `runner`: AgentRunner, quota, rate limiting, metrics
//! - `power`: Power system (M4 social system)

mod agent;
mod asteroid_fragment;
mod chunking;
mod decision_provider;
mod frag_spawn;
mod fragment_physics;
mod init;
mod init_module_visual;
mod kernel;
#[cfg(not(target_arch = "wasm32"))]
mod llm_agent;
mod llm_defaults;
mod memory;
mod module_visual;
mod native_resolution;
pub(crate) mod persist;
mod power;
#[cfg(not(target_arch = "wasm32"))]
mod provider_loopback_adapter;
#[cfg(not(target_arch = "wasm32"))]
mod provider_loopback_http;
mod runner;
pub(crate) mod runtime_perf;
mod scenario;
mod social;
mod types;
mod world_model;

#[cfg(test)]
mod tests;

// Re-export all public types
pub use agent::{
    ActionResult, AgentBehavior, AgentDecision, AgentDecisionTrace, LlmChatMessageTrace,
    LlmChatRole, LlmDecisionDiagnostics, LlmEffectIntentTrace, LlmEffectReceiptTrace,
    LlmPromptSectionTrace, LlmStepTrace,
};
pub use asteroid_fragment::generate_fragments;
pub use chunking::{
    CHUNK_SIZE_X_CM, CHUNK_SIZE_Y_CM, CHUNK_SIZE_Z_CM, ChunkBounds, ChunkCoord, chunk_bounds,
    chunk_coord_of, chunk_coords, chunk_grid_dims, chunk_seed,
};
pub use decision_provider::{
    ActionCatalogEntry, DEFAULT_PROVIDER_ACTION_SCHEMA_VERSION,
    DEFAULT_PROVIDER_OBSERVATION_SCHEMA_VERSION, DecisionProvider, DecisionProviderError,
    DecisionRequest, DecisionRequestContractError, DecisionResponse, FeedbackEnvelope,
    GoldenDecisionFixture, MemoryWriteIntent, MockDecisionProvider, MockDecisionProviderState,
    ObservationEnvelope, ProviderBackedAgentBehavior, ProviderDecision, ProviderDiagnostics,
    ProviderErrorEnvelope, ProviderExecutionMode, ProviderInteractionTarget,
    ProviderMissionContext, ProviderNavigationNode, ProviderNearbyEntity, ProviderObservation,
    ProviderRecentEvent, ProviderSelfState, ProviderTokenUsage, ProviderTraceEnvelope,
    ProviderTranscriptEntry, golden_decision_provider_fixtures,
};
pub use fragment_physics::{
    CM3_PER_M3, CompoundComposition, CuboidSizeCm, FragmentBlock, FragmentBlockField,
    FragmentCompoundKind, FragmentPhysicalProfile, GridPosCm, MIN_BLOCK_EDGE_CM, infer_element_ppm,
    mass_grams_from_volume_density, synthesize_fragment_budget, synthesize_fragment_profile,
};
pub use init::{
    AgentSpawnConfig, AsteroidFragmentInitConfig, LocationSeedConfig, OriginLocationConfig,
    PowerPlantSeedConfig, WorldInitConfig, WorldInitError, WorldInitReport, build_world_model,
    initialize_kernel,
};
pub use kernel::ChunkRuntimeConfig;
pub use kernel::{
    Observation, ObservedAgent, ObservedLocation, ObservedModuleArtifactRecord,
    ObservedModuleLifecycleState, ObservedModuleMarketState, ObservedPowerMarketState,
    ObservedSocialState, WorldKernel,
};
#[cfg(not(target_arch = "wasm32"))]
pub use llm_agent::{
    LlmAgentBehavior, LlmAgentBuildError, LlmAgentConfig, LlmClientError,
    OpenAiChatCompletionClient,
};
pub use llm_defaults::{
    DEFAULT_CONFIG_FILE_NAME, DEFAULT_LLM_FORCE_REPLAN_AFTER_SAME_ACTION,
    DEFAULT_LLM_LONG_TERM_GOAL, DEFAULT_LLM_MAX_DECISION_STEPS, DEFAULT_LLM_MAX_MODULE_CALLS,
    DEFAULT_LLM_MAX_REPAIR_ROUNDS, DEFAULT_LLM_MODEL, DEFAULT_LLM_PROMPT_MAX_HISTORY_ITEMS,
    DEFAULT_LLM_SHORT_TERM_GOAL, DEFAULT_LLM_SYSTEM_PROMPT, ENV_LLM_API_KEY, ENV_LLM_BASE_URL,
    ENV_LLM_FORCE_REPLAN_AFTER_SAME_ACTION, ENV_LLM_LONG_TERM_GOAL, ENV_LLM_MAX_DECISION_STEPS,
    ENV_LLM_MAX_MODULE_CALLS, ENV_LLM_MAX_REPAIR_ROUNDS, ENV_LLM_MODEL,
    ENV_LLM_PROMPT_MAX_HISTORY_ITEMS, ENV_LLM_PROMPT_PROFILE, ENV_LLM_SHORT_TERM_GOAL,
    ENV_LLM_SYSTEM_PROMPT, ENV_LLM_TIMEOUT_MS,
};
pub use memory::{
    AgentMemory, LongTermMemory, LongTermMemoryEntry, MemoryEntry, MemoryEntryKind, ShortTermMemory,
};
pub use module_visual::{ModuleVisualAnchor, ModuleVisualEntity};
pub use native_resolution::{
    CmMappingRule, NativeResolutionDeclaration, NativeResolutionKind, NativeResolutionValue,
    RUNTIME_NATIVE_RESOLUTIONS, RoundingRule, fragment_block_native_resolution,
    native_resolution_by_subsystem, runtime_native_resolutions,
};
pub use persist::{
    PersistError, PlayerAgentClaimOwnedSnapshot, PlayerAgentClaimQuoteSnapshot,
    PlayerAgentClaimSnapshot, PlayerGameplayAction, PlayerGameplayCausalityKind,
    PlayerGameplayExecutionState, PlayerGameplayGoalKind, PlayerGameplayRecentFeedback,
    PlayerGameplaySnapshot, PlayerGameplayStageId, PlayerGameplayStageStatus, WorldJournal,
    WorldSnapshot,
};
#[cfg(not(target_arch = "wasm32"))]
pub use provider_loopback_adapter::ProviderLoopbackAdapter;
#[cfg(not(target_arch = "wasm32"))]
pub use provider_loopback_http::{
    LOOPBACK_HTTP_PROVIDER_TRANSPORT, PROVIDER_PHASE1_ACTION_SET_ALIAS, ProviderAgentChatRequest,
    ProviderAgentChatResponse, ProviderCompatibilityReport, ProviderCompatibilityStatus,
    ProviderFeedbackAck, ProviderHealth, ProviderInfo, ProviderLoopbackHttpClient,
    ProviderLoopbackHttpError, REMOTE_HTTPS_PROVIDER_TRANSPORT, evaluate_provider_compatibility,
    provider_agent_chat_log_key, provider_phase1_required_actions,
    provider_phase1_required_capabilities, validate_provider_http_base_url,
    validate_provider_loopback_http_base_url,
};
pub use runner::{
    AgentQuota, AgentRunner, AgentStats, AgentTickResult, RateLimitPolicy, RateLimitState,
    RegisteredAgent, RunnerLogEntry, RunnerLogKind, RunnerMetrics, SkippedReason,
};
pub use scenario::{ScenarioSpecError, WorldScenario, WorldScenarioSpec};
pub use social::{
    SocialAdjudicationDecision, SocialChallengeState, SocialEdgeLifecycleState, SocialEdgeState,
    SocialFactImpactQuote, SocialFactLifecycleState, SocialFactState, SocialStake,
};
pub use types::{
    Action, ActionEnvelope, ActionId, ActionSubmitter, AgentId, AssetId,
    CHUNK_GENERATION_SCHEMA_VERSION, CM_PER_KM, ChunkResourceBudget,
    DEFAULT_ELEMENT_RECOVERABILITY_PPM, DEFAULT_MOVE_COST_PER_KM_ELECTRICITY,
    DEFAULT_VISIBILITY_RANGE_CM, ElementBudgetError, ElementComposition, FacilityId,
    FragmentElementKind, FragmentResourceBudget, JOURNAL_VERSION, LocationId, LocationProfile,
    MaterialKind, ModuleInstallTarget, PPM_BASE, PowerOrderSide, RefineQuote, ResourceKind,
    ResourceOwner, ResourceStock, SNAPSHOT_VERSION, ScheduleQuote, StockError, WorldEventId,
    WorldTime,
};
pub use world_model::{
    Agent, AgentExecutionDebugContext, AgentKinematics, AgentPromptProfile, Asset, AssetKind,
    AsteroidFragmentConfig, BoundaryReservation, ChunkState, EconomyConfig, Factory,
    FragmentResourceError, InstalledModuleState, Location, MaterialDistributionStrategy,
    MaterialRadiationFactors, MaterialWeights, ModuleArtifactBidState, ModuleArtifactListingState,
    ModuleArtifactState, PhysicsConfig, PhysicsParameterSpec, PowerOrderBookState, PowerOrderState,
    RegionalInfrastructure, SpaceConfig, ThermalStatus, WorldConfig, WorldModel,
    physics_parameter_specs,
};

// Re-export power system types
pub use power::{
    AgentPowerState, AgentPowerStatus, ConsumeReason, PlantStatus, PowerConfig, PowerEvent,
    PowerPlant, PowerSaleQuote, PowerSurvivalQuote,
};
pub use runtime_perf::{
    RuntimePerfBottleneck, RuntimePerfHealth, RuntimePerfSeriesSnapshot, RuntimePerfSnapshot,
};

// Re-export event types from kernel
pub use kernel::{
    ChunkGenerationCause, KernelRuleCost, KernelRuleDecision, KernelRuleDecisionMergeError,
    KernelRuleModuleContext, KernelRuleModuleInput, KernelRuleModuleOutput, KernelRuleVerdict,
    MICRO_DEPOT_DEBIT_AMOUNT_CRITICAL, MICRO_DEPOT_DEBIT_AMOUNT_HIGH, MICRO_DEPOT_DEBIT_AMOUNT_LOW,
    MICRO_DEPOT_DEBIT_AMOUNT_MEDIUM, MICRO_DEPOT_DEBIT_AMOUNT_NONE, MICRO_DEPOT_INSTALL_DATA_COST,
    MICRO_DEPOT_PROPOSAL_EMIT_KIND, MICRO_DEPOT_UPKEEP_DATA_COST, MicroDepotActionContext,
    MicroDepotActionKind, MicroDepotConsumedResourceClass, MicroDepotDecision,
    MicroDepotDeltaClass, MicroDepotEffectPreview, MicroDepotEvalInput, MicroDepotFacilityContext,
    MicroDepotPlayerContext, MicroDepotPlayerFacilitySnapshot, MicroDepotPressureClass,
    MicroDepotProposal, MicroDepotQuotePreview, MicroDepotRegionContext, MicroDepotResourceDebit,
    MicroDepotStatus, PowerOrderFill, PromptUpdateOperation, RejectReason, WorldEvent,
    WorldEventKind, compute_micro_depot_proposal_hash, evaluate_micro_depot_quote_with_module,
    merge_kernel_rule_decisions,
};
