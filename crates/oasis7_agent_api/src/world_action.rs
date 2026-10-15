//! Canonical typed world action contract shared by clients and execution adapters.
mod action;
mod geometry;
mod ledger;
mod main_token;
mod support;
pub use action::*;
pub use geometry::GeoPos;
pub use ledger::MaterialLedgerId;
pub use main_token::*;
pub use support::*;
type ProposalId = u64;
type WorldTime = u64;
