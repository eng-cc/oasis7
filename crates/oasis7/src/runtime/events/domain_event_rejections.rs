use super::*;

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

/// The cause of an event, for audit purposes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum CausedBy {
    Action(ActionId),
    Effect { intent_id: String },
}
