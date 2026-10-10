//! Host-bound invocation metadata shared by provider transports and runtime.
//!
//! This is deliberately a serialization-only DTO.  It carries the identity
//! that a provider must echo in a typed response, but it does not contain a
//! signature, grant validation result, or any other authority.  The native
//! runtime re-exports the same type for its durable binding and performs the
//! authoritative grant/revocation checks when executing a response.

pub use oasis7_agent_api::CapabilityInvocationContext;
