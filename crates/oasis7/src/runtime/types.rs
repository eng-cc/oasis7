//! Type aliases and basic type definitions for the runtime module.

pub use crate::industry_types::MaterialLedgerId;

pub type WorldTime = u64;
pub type WorldEventId = u64;
pub type ActionId = u64;
pub type IntentSeq = u64;
pub type ProposalId = u64;
pub type PatchPath = Vec<String>;
