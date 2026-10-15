use crate::world_action::{MaterialLedgerId, ResourceKind};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentDelegationGrantV1 {
    pub grant_id: String,
    pub source_id: String,
    pub issuer_id: String,
    pub owner_id: String,
    pub organization_id: Option<String>,
    pub agent_id: String,
    pub object_id: String,
    pub action_kinds: Vec<String>,
    pub revision: u64,
    pub period_id: String,
    pub valid_from_tick: u64,
    pub valid_until_tick: u64,
    pub limit_units: u64,
    /// Native domain electricity accounting; never a new currency.
    pub resource_kind: String,
    pub revoked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentDelegationAuthorizationV1 {
    pub grant: AgentDelegationGrantV1,
    pub cost_units: u64,
    pub spent_units: u64,
    pub remaining_units: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum FactoryConstructionPowerMode {
    #[default]
    StartOnlySink,
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
    /// Exact source ledger committed at admission and retained in the settled
    /// construction receipt. This is optional only for legacy replay.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub material_ledger: Option<MaterialLedgerId>,
    /// Admission-time electricity facts.  These are optional so snapshots
    /// containing the pre-receipt obligation continue to decode and replay.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub electricity_before: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub electricity_after: Option<i64>,
    /// Admission-time material facts keyed only by required build-cost kind.
    /// Keeping the maps scoped to the requirement avoids treating unrelated
    /// ledger balances as part of the construction receipt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub material_balances_before: Option<BTreeMap<String, i64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub material_balances_after: Option<BTreeMap<String, i64>>,
}

/// Reasons why an action was rejected.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum RejectReason {
    AgentAlreadyExists {
        agent_id: String,
    },
    AgentNotFound {
        agent_id: String,
    },
    AgentsNotCoLocated {
        agent_id: String,
        other_agent_id: String,
    },
    InvalidAmount {
        amount: i64,
    },
    InsufficientResource {
        agent_id: String,
        kind: ResourceKind,
        requested: i64,
        available: i64,
    },
    InsufficientResources {
        deficits: BTreeMap<ResourceKind, i64>,
    },
    InsufficientMaterial {
        material_kind: String,
        requested: i64,
        available: i64,
    },
    MaterialTransferDistanceExceeded {
        distance_km: i64,
        max_distance_km: i64,
    },
    MaterialTransitCapacityExceeded {
        in_flight: usize,
        max_in_flight: usize,
    },
    FactoryNotFound {
        factory_id: String,
    },
    FactoryBusy {
        factory_id: String,
        active_jobs: usize,
        recipe_slots: u16,
    },
    RuleDenied {
        notes: Vec<String>,
    },
}
