//! Authorized Agent context DTO; values do not confer admission authority.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldServiceAgentContext {
    pub agent_id: String,
    pub capability_authorization_root: String,
    pub capability_snapshot_hash: String,
    pub authority_context_hash: String,
    pub capability_catalog: oasis7_wasm_abi::CapabilityCatalogSnapshot,
    pub capability_invocation_context: crate::CapabilityInvocationContext,
    pub payer_binding: serde_json::Value,
}
