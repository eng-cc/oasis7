//! Fixed-epoch, two-file-endpoint durability protocol. No runtime activation,
//! formal CommitRef, automatic takeover or independent-fault-domain assertion.
//! Endpoint signatures are trusted assertions of the configured fsync contract;
//! offline verification cannot establish actual hardware durability/topology.
//! Paths are operator controlled, not protected against malicious same-permission
//! processes replacing directories or lock inodes. Unix durability only.

mod coordinator;
mod crypto;
mod endpoint;
mod package;
mod storage;
#[cfg(test)]
mod tests;

use super::LocalRequestIdentity;
pub use coordinator::ReplicatedCoordinator;
pub use crypto::verify_evidence;
pub use endpoint::FileEndpoint;
pub use package::{ArtifactObject, ArtifactRole, ClosedRecord};
use serde::{Deserialize, Serialize};

pub const EVIDENCE_SCOPE: &str = "replicated_durability_prerequisite";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixedTrust {
    pub world_id: String,
    pub chain_id: String,
    pub genesis_digest: String,
    pub authority_epoch: u64,
    pub writer_key: String,
    pub primary_id: String,
    pub primary_key: String,
    pub replica_id: String,
    pub replica_key: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EndpointRole {
    Primary,
    Replica,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReceiptStage {
    Prepared,
    DecisionDurable,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HeadAnchor {
    pub position: u64,
    pub decision_hash: String,
    /// True demands stored two-receipt evidence, not merely an irreversible decision.
    pub qualified: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProposalBody {
    pub schema_version: u32,
    pub scope: String,
    pub trust: FixedTrust,
    pub position: u64,
    pub parent_hash: String,
    pub request: LocalRequestIdentity,
    pub record: ClosedRecord,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedProposal {
    pub body: ProposalBody,
    pub signature_hex: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiptBody {
    pub schema_version: u32,
    pub scope: String,
    pub trust: FixedTrust,
    pub role: EndpointRole,
    pub endpoint_id: String,
    pub stage: ReceiptStage,
    pub decision_hash: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DurableReceipt {
    pub body: ReceiptBody,
    pub signature_hex: String,
}
/// A mechanical protocol proof, never accepted by existing finality/runtime consumers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DurabilityEvidence {
    pub proposal: SignedProposal,
    pub replica_prepare: DurableReceipt,
    pub primary_receipt: DurableReceipt,
    pub replica_receipt: DurableReceipt,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolOutcome {
    DurabilityQualified(Box<DurabilityEvidence>),
    Pending {
        request: LocalRequestIdentity,
    },
    /// Keep the identity; absence at one endpoint is never global non-execution.
    Unknown {
        request: LocalRequestIdentity,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EndpointStatus {
    NotRecorded,
    Prepared(Box<SignedProposal>),
    DecisionUnqualified(Box<SignedProposal>),
    Qualified(Box<DurabilityEvidence>),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReceiptRepairOutcome {
    Durable(Box<DurableReceipt>),
    /// Local absence only; never a global abort or non-execution assertion.
    NotLocallyDecided,
    Unknown {
        request: LocalRequestIdentity,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolError {
    Invalid(String),
    Io(String),
    Locked,
    Poisoned,
    UnsupportedPlatform,
}
impl std::fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ProtocolError {}
pub(super) fn invalid(s: impl Into<String>) -> ProtocolError {
    ProtocolError::Invalid(s.into())
}
pub(super) fn io(e: std::io::Error) -> ProtocolError {
    ProtocolError::Io(e.to_string())
}
