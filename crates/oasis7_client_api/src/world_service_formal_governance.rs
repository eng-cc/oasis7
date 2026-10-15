//! Public governance body committed inside a formal projection leaf.
//! No CommitRef is embedded here: canonical record identity depends on this root.
use super::{PublicMembershipCandidate, PublicMembershipRegistry};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormalGovernanceMembershipV1 {
    pub schema_version: u32,
    pub state_root_hash: String,
    pub governance_epoch: u64,
    pub candidates: BTreeMap<String, PublicMembershipCandidate>,
    pub selected_candidates: BTreeMap<String, String>,
    pub effective_registry: Option<PublicMembershipRegistry>,
    pub effective_registry_digest: String,
}
