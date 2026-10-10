//! Authenticated topology-independent service wire and adapters.
pub mod agent_authority;
pub mod agent_chat;
pub mod authority;
pub mod client;
pub mod correlation;
pub mod gameplay;
pub mod projection;
pub mod verified_view;
pub mod wire;
pub use authority::*;
pub use correlation::derive_correlation;
pub use oasis7_client_api::world_service::*;
pub use wire::*;
