//! Compatibility exports for Runtime-owned agent and body state.
#[cfg(not(target_arch = "wasm32"))]
pub use crate::runtime::agent_state::*;
// WASM has no authoritative Runtime. Preserve its existing value-only model
// surface while native ownership remains with Runtime.
#[cfg(target_arch = "wasm32")]
#[path = "runtime/agent_state.rs"]
mod wasm_values;
#[cfg(target_arch = "wasm32")]
pub use wasm_values::*;
