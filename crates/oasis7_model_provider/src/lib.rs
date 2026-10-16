//! Session-scoped model inference. This crate cannot sign or mutate world state.
//! Credentials have no environment fallback and are never a game authority.
pub mod connection;
pub mod preset;
pub mod probe;
pub mod protocol;
pub mod transport;

pub mod harness_adapter;
