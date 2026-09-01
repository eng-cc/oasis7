use super::*;

/// Runtime-owned identity for a location used by site and agent authority.
///
/// The registry is intentionally keyed by this exact `location_id`; physical
/// coordinates and caller-provided labels are not substitutes for an active
/// anchor.  The serde defaults keep older snapshots readable, while admission
/// still fails closed when a new authority update has no active anchor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct LocationAnchorV1 {
    #[serde(default)]
    pub location_id: String,
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub authority_revision: u64,
    #[serde(default)]
    pub effective_at: WorldTime,
}

/// Canonical location assignment for an agent that can participate in
/// location-bound runtime actions. `AgentState::pos` remains a physical
/// coordinate only and is not a location authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AgentLocationAuthorityV1 {
    #[serde(default)]
    pub agent_id: String,
    #[serde(default)]
    pub location_id: String,
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub authority_revision: u64,
    #[serde(default)]
    pub effective_at: WorldTime,
}

/// Persisted site admission authority. The allowlist is normalized on update
/// so access checks and replay have one canonical order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FactorySiteAuthorityV1 {
    #[serde(default)]
    pub site_id: String,
    #[serde(default)]
    pub location_id: String,
    #[serde(default)]
    pub owner_agent_id: String,
    #[serde(default)]
    pub authorized_agent_ids: Vec<String>,
    #[serde(default)]
    pub chunk_ready: bool,
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub authority_revision: u64,
    #[serde(default)]
    pub registered_at: WorldTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum FactoryConstructionPowerMode {
    #[default]
    StartOnlySink,
}

/// M4-governed construction electricity source. The map key is the exact
/// `factory_id` submitted by BuildFactory; callers cannot provide an amount.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FactoryConstructionPowerProfileV1 {
    #[serde(default)]
    pub factory_id: String,
    #[serde(default)]
    pub factory_kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_module_id: Option<String>,
    #[serde(default)]
    pub electricity_amount: i64,
    #[serde(default)]
    pub mode: FactoryConstructionPowerMode,
    #[serde(default)]
    pub authority_revision: u64,
    #[serde(default)]
    pub active: bool,
}

/// Immutable construction power sink resolved at BuildFactory admission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FactoryBuildPowerObligationV1 {
    #[serde(default)]
    pub payer_agent_id: String,
    #[serde(default)]
    pub profile_key: String,
    #[serde(default)]
    pub profile_revision: u64,
    #[serde(default)]
    pub electricity_amount: i64,
    #[serde(default)]
    pub mode: FactoryConstructionPowerMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FactoryProductionStatus {
    Idle,
    Running,
    Blocked,
    Paused,
}

impl Default for FactoryProductionStatus {
    fn default() -> Self {
        Self::Idle
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FactoryProductionState {
    #[serde(default)]
    pub status: FactoryProductionStatus,
    #[serde(default)]
    pub active_jobs: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_job_id: Option<ActionId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_recipe_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_started_at: Option<WorldTime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_completed_at: Option<WorldTime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_blocked_at: Option<WorldTime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_resumed_at: Option<WorldTime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_blocker_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_blocker_detail: Option<String>,
    #[serde(default)]
    pub completed_jobs: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_completed_recipe_id: Option<String>,
    #[serde(default)]
    pub same_recipe_repeat_count: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_completed_canonical_snapshot: Option<FactoryProductionSnapshot>,
}

impl Default for FactoryProductionState {
    fn default() -> Self {
        Self {
            status: FactoryProductionStatus::Idle,
            active_jobs: 0,
            current_job_id: None,
            current_recipe_id: None,
            last_started_at: None,
            last_completed_at: None,
            last_blocked_at: None,
            last_resumed_at: None,
            current_blocker_kind: None,
            current_blocker_detail: None,
            completed_jobs: 0,
            last_completed_recipe_id: None,
            same_recipe_repeat_count: 0,
            last_completed_canonical_snapshot: None,
        }
    }
}

/// Stable prerequisite facts for the latest completed recipe on a factory.
///
/// This snapshot intentionally excludes transient execution details such as
/// duration/ETA, live balances, and market quotes.  The factory containing it
/// is the partition key for the candidate window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FactoryProductionSnapshot {
    #[serde(default)]
    pub recipe_id: String,
    #[serde(default)]
    pub consume: Vec<MaterialStack>,
    #[serde(default)]
    pub produce: Vec<MaterialStack>,
    #[serde(default)]
    pub byproducts: Vec<MaterialStack>,
    #[serde(default)]
    pub consume_ledger: MaterialLedgerId,
    #[serde(default)]
    pub power_required: i64,
    #[serde(default)]
    pub output_ledger: MaterialLedgerId,
    #[serde(default)]
    pub bottleneck_tags: Vec<String>,
    #[serde(default)]
    pub logistics_route_ids: Vec<String>,
    #[serde(default)]
    pub logistics_path_ids: Vec<String>,
}

impl FactoryProductionSnapshot {
    pub(super) fn from_recipe_job(job: &RecipeJobState) -> Self {
        Self {
            recipe_id: job.recipe_id.clone(),
            consume: normalize_material_stacks(&job.consume),
            produce: normalize_material_stacks(&job.produce),
            byproducts: normalize_material_stacks(&job.byproducts),
            consume_ledger: job.consume_ledger.clone(),
            power_required: job.power_required,
            output_ledger: job.output_ledger.clone(),
            bottleneck_tags: normalize_bottleneck_tags(&job.bottleneck_tags),
            logistics_route_ids: job.logistics_route_ids.clone(),
            logistics_path_ids: job.logistics_path_ids.clone(),
        }
    }
}

fn normalize_material_stacks(stacks: &[MaterialStack]) -> Vec<MaterialStack> {
    let mut merged = BTreeMap::<String, i64>::new();
    for stack in stacks {
        let kind = stack.kind.trim().to_ascii_lowercase();
        let amount = merged.entry(kind).or_default();
        *amount = amount.saturating_add(stack.amount);
    }
    merged
        .into_iter()
        .map(|(kind, amount)| MaterialStack::new(kind, amount))
        .collect()
}

fn normalize_bottleneck_tags(tags: &[String]) -> Vec<String> {
    tags.iter()
        .map(|tag| tag.trim().to_ascii_lowercase())
        .filter(|tag| !tag.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}
