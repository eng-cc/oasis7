//! Local durability prerequisite, deliberately disconnected from runtime activation.
//!
//! This module provides no cross-host fencing, independent-fault-domain durability,
//! signer rotation or formal committed receipt. Its proof is offline local evidence.
//! Callers must retain a head anchor outside this directory to detect rollback.
//! Storage must be a stable, operator-controlled local filesystem path supporting
//! exclusive file locks, atomic replacement and directory fsync. The lock excludes
//! cooperative writers on its retained inode; it does not protect against malicious
//! processes with the same filesystem permissions replacing that inode or directory.

mod journal;
mod proof;
mod storage;
pub mod replicated_protocol;

pub use journal::ControlledAuthorityLocalJournal;
pub use proof::verify_local_decision_proof;

use serde::{Deserialize, Serialize};

pub const LOCAL_DECISION_SCOPE: &str = "local_durable_prerequisite";
pub const LOCAL_AUTHORITY_PROFILE: &str = "controlled_single_authority";
pub const LOCAL_DECISION_SCHEMA: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalSourceBinding {
    pub execution_engine_ref: String,
    pub governing_rules_ref: String,
}

/// Operator-supplied trust, never inferred from a stored proof or network topology.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrustedLocalAuthority {
    pub world_id: String,
    pub chain_id: String,
    pub genesis_digest: String,
    pub signer_public_key_hex: String,
    pub authority_epoch: u64,
    pub source_binding: LocalSourceBinding,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Caller must authenticate and authorize the subject before constructing this key.
/// Local authority signatures do not prove player authorization or grant permission.
pub struct LocalRequestIdentity {
    pub verified_subject: String,
    pub operation_domain: String,
    pub nonce_scope: String,
    pub request_id: String,
}

/// Caller must compute the digest over the authenticated payload in its original
/// signing/nonce domain. This module does not authenticate payloads, authorize
/// subjects, execute decisions, or verify referenced blobs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedLocalDecision {
    pub payload_digest: String,
    pub execution_block_hash: String,
    pub state_root: String,
    pub snapshot_ref: String,
    pub journal_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalDecisionBodyV1 {
    pub schema_version: u32,
    pub profile: String,
    pub scope: String,
    pub authority: TrustedLocalAuthority,
    pub position: u64,
    pub parent_hash: String,
    pub request: LocalRequestIdentity,
    pub prepared: PreparedLocalDecision,
}

/// Never accepted by WorldService, CommitRef, the old finality verifier or startup.
/// A valid signature authenticates the local decision, not actual disk durability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlledAuthorityLocalDecisionProofV1 {
    pub decision: LocalDecisionBodyV1,
    pub signature_hex: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalHeadAnchor {
    pub position: u64,
    pub proof_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalAppendOutcome {
    LocallyRecorded(Box<ControlledAuthorityLocalDecisionProofV1>),
    /// Preserve this identity. The writer is poisoned until an explicit reopen.
    Unknown {
        request: LocalRequestIdentity,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalLookupOutcome {
    LocallyRecorded(Box<ControlledAuthorityLocalDecisionProofV1>),
    /// Absence in the recovered local snapshot, not global rejection/non-execution.
    NotLocallyRecorded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalJournalError {
    Invalid(String),
    Locked,
    Io(String),
    Poisoned,
    UnsupportedPlatform,
}

impl std::fmt::Display for LocalJournalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(reason) => write!(f, "invalid local authority journal: {reason}"),
            Self::Locked => f.write_str("local authority journal writer lock is held"),
            Self::Io(reason) => write!(f, "local authority journal I/O: {reason}"),
            Self::Poisoned => {
                f.write_str("uncertain local write; query and explicitly reopen before append")
            }
            Self::UnsupportedPlatform => {
                f.write_str("local authority journal durability is unsupported on this platform")
            }
        }
    }
}
impl std::error::Error for LocalJournalError {}

fn invalid(reason: impl Into<String>) -> LocalJournalError {
    LocalJournalError::Invalid(reason.into())
}

fn required(value: &str, name: &str) -> Result<(), LocalJournalError> {
    if value.trim().is_empty() {
        return Err(invalid(format!("{name} must not be empty")));
    }
    Ok(())
}

fn hash_hex(value: &str, name: &str) -> Result<(), LocalJournalError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(invalid(format!(
            "{name} requires a lowercase 32-byte hex digest"
        )));
    }
    Ok(())
}

impl TrustedLocalAuthority {
    pub fn validate(&self) -> Result<(), LocalJournalError> {
        required(&self.world_id, "world_id")?;
        required(&self.chain_id, "chain_id")?;
        hash_hex(&self.genesis_digest, "genesis_digest")?;
        hash_hex(&self.signer_public_key_hex, "signer_public_key_hex")?;
        required(
            &self.source_binding.execution_engine_ref,
            "execution_engine_ref",
        )?;
        required(
            &self.source_binding.governing_rules_ref,
            "governing_rules_ref",
        )?;
        if self.authority_epoch == 0 {
            return Err(invalid("authority_epoch must be positive and fixed"));
        }
        Ok(())
    }

    /// Explicit genesis trust cannot detect rollback of a previously used journal.
    pub fn genesis_anchor(&self) -> Result<LocalHeadAnchor, LocalJournalError> {
        self.validate()?;
        Ok(LocalHeadAnchor {
            position: 0,
            proof_hash: proof::domain_hash("oasis7.controlled_authority.local_genesis.v1", self)?,
        })
    }
}

impl LocalRequestIdentity {
    fn validate(&self) -> Result<(), LocalJournalError> {
        for (value, name) in [
            (&self.verified_subject, "verified_subject"),
            (&self.operation_domain, "operation_domain"),
            (&self.nonce_scope, "nonce_scope"),
            (&self.request_id, "request_id"),
        ] {
            required(value, name)?;
        }
        Ok(())
    }
}

impl PreparedLocalDecision {
    fn validate(&self) -> Result<(), LocalJournalError> {
        for (value, name) in [
            (&self.payload_digest, "payload_digest"),
            (&self.execution_block_hash, "execution_block_hash"),
            (&self.state_root, "state_root"),
            (&self.snapshot_ref, "snapshot_ref"),
            (&self.journal_ref, "journal_ref"),
        ] {
            hash_hex(value, name)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
