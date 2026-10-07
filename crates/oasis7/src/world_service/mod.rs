//! Authenticated topology-independent service wire and adapters.
pub mod authority;
pub mod agent_authority;
pub mod client;
pub mod projection;
pub mod verified_view;
pub mod correlation;
pub mod gameplay;
pub mod wire;
pub use authority::*;
pub use wire::*;
pub use correlation::derive_correlation;
pub use oasis7_client_api::world_service::*;
