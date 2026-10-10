//! Local, exact-grant signing worker and strict IPC protocol.

pub mod protocol;
pub mod runtime_gate;

pub mod admin;
pub mod authorization;
pub mod backup;
pub mod control;
pub mod detached;
pub mod error;
pub mod identity;
pub mod installation;
pub mod job;
pub mod key_envelope;
pub mod key_management;
pub mod rollback;
pub mod store;
pub mod types;
pub mod worker;

mod local_fs;

pub use local_fs::read_candidate_bytes;

pub mod manager_cli;

pub mod file_cli;

#[cfg(test)]
mod key_management_tests;
