//! Durable state used by the versioned trusted-module authorization lane.
//!
//! The legacy [`CapabilityGrant`](super::effect::CapabilityGrant) map is kept
//! deliberately separate.  Values in this module are audit state, not a
//! process-local cache: they are copied into and out of `Snapshot` and are
//! included in the staged world transaction used by the v2 executor.

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::{BTreeMap, BTreeSet};

pub use crate::capability_invocation_context::CapabilityInvocationContext;
use oasis7_wasm_abi::CapabilitySubject;

/// A finalized governance record installed by the host before a grant can be
/// used.  The executor never treats a process-local key/epoch pair as an
/// authority source: every issuer field is checked against this record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityAuthorityRecord {
    pub issuer_id: String,
    pub issuer_kind: String,
    pub key_id: String,
    pub public_key_hex: String,
    pub issuer_key_epoch: u64,
    pub governance_epoch: u64,
    pub finalized_receipt_id: String,
    #[serde(default)]
    pub authority_rotation_receipt_id: Option<String>,
    pub world_id: String,
    pub branch_id: String,
    pub finality_epoch: u64,
    pub finality_block_hash: String,
    pub finality_status: String,
    pub revocation_epoch: u64,
    #[serde(default)]
    pub revoked_grant_ids: BTreeSet<String>,
    #[serde(default)]
    pub superseded_by: BTreeMap<String, String>,
}

/// Durable logical execution budget for one subject/grant pair.  Runtime
/// resource fees remain separately metered; this account prevents a grant
/// from being replayed indefinitely even when those resources are unbounded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityBudgetAccount {
    pub subject: CapabilitySubject,
    pub grant_id: String,
    pub remaining_units: i64,
    #[serde(default)]
    pub reserved_units: i64,
    #[serde(default)]
    pub spent_units: i64,
}

/// The live trust and revocation view used for v2 authorization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CapabilityRevocationState {
    #[serde(default)]
    pub epoch: u64,
    #[serde(default)]
    pub revoked_grant_ids: BTreeSet<String>,
    #[serde(default)]
    pub superseded_by: BTreeMap<String, String>,
    /// Issuer id -> finalized governance authority record.
    #[serde(default)]
    pub authority_records: BTreeMap<String, CapabilityAuthorityRecord>,
    #[serde(default)]
    pub finalized_receipt_id: Option<String>,
}

/// A durable record for the complete authorization nonce tuple.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityAuthorizationNonceRecord {
    pub request_hash: String,
    pub outcome_hash: String,
    #[serde(default)]
    pub committed_receipt_id: Option<String>,
    /// `reserved`, `committed`, or `aborted`.
    pub state: String,
}

/// Runtime audit receipt for one presented v2 command.
///
/// Subject, presenter and audience are retained as canonical JSON values so
/// the runtime can persist ABI additions without another migration while
/// keeping their exact signed/canonical representation for audit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapabilityAuthorizationAuditReceipt {
    pub receipt_id: String,
    #[serde(default)]
    pub root_receipt_id: Option<String>,
    #[serde(default)]
    pub grant_id: Option<String>,
    pub subject: JsonValue,
    #[serde(default)]
    pub presenter: Option<JsonValue>,
    pub audience: JsonValue,
    pub scope_hash: String,
    #[serde(default)]
    pub module_id: Option<String>,
    #[serde(default)]
    pub module_version: Option<String>,
    #[serde(default)]
    pub manifest_hash: Option<String>,
    #[serde(default)]
    pub catalog_snapshot_id: Option<String>,
    #[serde(default)]
    pub response_nonce: Option<String>,
    #[serde(default)]
    pub authorization_nonce_key_hash: Option<String>,
    /// `accepted`, `denied`, `idempotent`, or `pending`.
    pub decision: String,
    #[serde(default)]
    pub denial_code: Option<String>,
    #[serde(default)]
    pub budget_before: i64,
    #[serde(default)]
    pub budget_after: Option<i64>,
    pub world_head_before: u64,
    #[serde(default)]
    pub world_head_after: Option<u64>,
    pub branch_id: String,
    pub finality_epoch: u64,
    #[serde(default)]
    pub finality_block_hash: Option<String>,
    pub finality_status: String,
    pub state_hash_before: String,
    #[serde(default)]
    pub state_hash_after: Option<String>,
    #[serde(default)]
    pub committed_effect_receipt_id: Option<String>,
    pub canonical_request_hash: String,
    pub canonical_result_hash: String,
}

/// Durable association between a queued module effect and its authorization
/// audit receipt.  The association is removed only when the external effect
/// receipt is ingested, at which point the audit receipt records that actual
/// receipt's intent id.  A command never invents an effect receipt locally.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityEffectReceiptLink {
    pub authorization_receipt_id: String,
}
