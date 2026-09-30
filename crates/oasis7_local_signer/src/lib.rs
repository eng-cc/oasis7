//! Local, exact-grant signing worker and strict IPC protocol.

pub mod protocol;

pub mod admin;
pub mod authorization;
pub mod control;
pub mod error;
pub mod identity;
pub mod installation;
pub mod job;
pub mod rollback;
pub mod store;
pub mod types;
pub mod worker;

mod local_fs;

pub use local_fs::read_candidate_bytes;
