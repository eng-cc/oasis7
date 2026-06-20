extern crate self as oasis7;

#[cfg(not(target_arch = "wasm32"))]
pub mod consensus_action_payload;
pub mod env_mut;
pub mod geometry;
pub mod launcher_bootstrap_peers;
pub mod models;
pub mod network_tier_manifest;
#[cfg(not(target_arch = "wasm32"))]
pub mod observability;
#[cfg(not(target_arch = "wasm32"))]
pub mod runtime;
pub mod simulator;
pub mod viewer;

pub use geometry::{
    DEFAULT_CLOUD_DEPTH_CM, DEFAULT_CLOUD_DEPTH_KM, DEFAULT_CLOUD_HEIGHT_CM,
    DEFAULT_CLOUD_HEIGHT_KM, DEFAULT_CLOUD_WIDTH_CM, DEFAULT_CLOUD_WIDTH_KM, GeoPos, SPACE_UNIT_CM,
    space_distance_cm, space_distance_m,
};
pub use models::{AgentState, BodyKernelView, DEFAULT_AGENT_HEIGHT_CM, RobotBodySpec};
