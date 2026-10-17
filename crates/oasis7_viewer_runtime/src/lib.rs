//! Viewer TCP, WebSocket and read-only HTTP service hosting.
//!
//! Runtime state and authorization contracts remain owned by `oasis7`; this
//! crate owns network listeners and transport adapters.

#[cfg(not(target_arch = "wasm32"))]
mod game_read_api;
#[cfg(not(target_arch = "wasm32"))]
mod server;
#[cfg(not(target_arch = "wasm32"))]
mod web_bridge;

#[cfg(not(target_arch = "wasm32"))]
pub use server::{ViewerServer, ViewerServerConfig, ViewerServerError};
#[cfg(not(target_arch = "wasm32"))]
pub use web_bridge::{ViewerWebBridge, ViewerWebBridgeConfig, ViewerWebBridgeError};
