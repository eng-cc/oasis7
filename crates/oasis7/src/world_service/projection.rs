//! Authorized observation material, never an execution snapshot.
use crate::runtime::{World, WorldEvent, WorldState};
use crate::simulator::RuntimeBindingV1;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldServiceProjection {
    pub state: WorldState,
    pub events: Vec<WorldEvent>,
    pub runtime_binding: Option<RuntimeBindingV1>,
    pub agent_context: Option<WorldServiceAgentContext>,
    pub scheduler_wakes: Vec<crate::runtime::SchedulerWakeV1>,
    pub continuations: Vec<crate::runtime::AgentContinuation>,
    pub cognition_leases: Vec<crate::runtime::CognitionLeaseV1>,
    pub continuation_contexts: std::collections::BTreeMap<String, WorldServiceContinuationContext>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldServiceContinuationContext {
    pub baseline_observation_digest: String,
    pub goal_digest: String,
    pub policy_digest: String,
    pub policy_revision: u64,
    pub precondition_summary: String,
    pub precondition_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldServiceAgentContext {
    pub agent_id: String,
    pub capability_authorization_root: String,
    pub capability_snapshot_hash: String,
    pub authority_context_hash: String,
    pub capability_catalog: oasis7_wasm_abi::CapabilityCatalogSnapshot,
    pub capability_invocation_context:
        crate::capability_invocation_context::CapabilityInvocationContext,
    pub payer_binding: serde_json::Value,
}

impl WorldServiceProjection {
    /// Called only after the service has authorized the requested audience.
    /// An allowlist prevents newly added persistence fields leaking by default.
    pub fn from_world(world: &World, authorized_agent: Option<&str>) -> Result<Self, String> {
        let mut value = serde_json::to_value(world.state()).map_err(|e| e.to_string())?;
        let fields = [
            "time",
            "agents",
            "resources",
            "materials",
            "material_ledgers",
            "material_profiles",
            "logistics_routes",
            "factories",
            "recipe_profiles",
            "factory_profiles",
            "product_profiles",
            "latest_product_validation",
            "pending_material_transits",
            "industry_progress",
            "alliances",
            "gameplay_policy",
            "agent_claims",
            "starter_oc_claims",
            "reputation_scores",
            "wars",
            "governance_votes",
            "governance_proposals",
            "crises",
            "meta_progress",
            "module_visual_entities",
            "main_token_config",
            "main_token_supply",
            "factory_construction_power_profiles",
            "location_anchors",
        ];
        value
            .as_object_mut()
            .ok_or("invalid world state")?
            .retain(|key, _| fields.contains(&key.as_str()));
        let mut state: WorldState = serde_json::from_value(value).map_err(|e| e.to_string())?;
        for (id, agent) in &mut state.agents {
            agent.mailbox.clear();
            if authorized_agent != Some(id.as_str()) {
                agent.intent = None;
            }
        }
        // Generic journals may contain private cognition/prompt artifacts.
        // Authorized committed changes are delivered by the changes route.
        Ok(Self {
            state,
            events: Vec::new(),
            runtime_binding: world.current_cognition_runtime_binding().ok(),
            agent_context: None,
            scheduler_wakes: Vec::new(),
            continuation_contexts: Default::default(),
            continuations: Vec::new(),
            cognition_leases: Vec::new(),
        })
    }
}
