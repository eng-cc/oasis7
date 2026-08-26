//! Host-bound invocation metadata shared by provider transports and runtime.
//!
//! This is deliberately a serialization-only DTO.  It carries the identity
//! that a provider must echo in a typed response, but it does not contain a
//! signature, grant validation result, or any other authority.  The native
//! runtime re-exports the same type for its durable binding and performs the
//! authoritative grant/revocation checks when executing a response.

use oasis7_wasm_abi::{CapabilityAudience, CapabilityPresenter, CapabilitySubject};
use serde::{Deserialize, Serialize};

/// Context bound by the host to one provider decision turn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityInvocationContext {
    pub grant_id: String,
    pub subject: CapabilitySubject,
    pub presenter: CapabilityPresenter,
    pub audience: CapabilityAudience,
    pub catalog_snapshot_id: String,
    pub module_id: String,
    pub module_version: String,
    pub response_nonce: String,
}
