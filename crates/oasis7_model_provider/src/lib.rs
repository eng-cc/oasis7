//! Session-scoped model inference plus explicit legacy simulator compatibility.
//! BYOK credentials have no environment fallback and are never a game authority.
//! The `legacy` module preserves historical configuration for existing consumers.
pub mod connection;
pub mod preset;
pub mod probe;
pub mod protocol;
pub mod transport;

pub mod harness_adapter;

/// Legacy simulator compatibility; never used by session-scoped BYOK.
pub mod legacy;
// Preserve the existing simulator and explicit legacy test API.
pub use legacy::*;
