use serde::{Deserialize, Serialize};

use crate::cognition::CognitionError;

/// Terminal state labels for a Runtime-owned cognition reservation.
///
/// The API deliberately exposes labels only. It provides no operation for
/// reserving, settling, releasing, refunding, or compensating a lease.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CognitionLeaseStatusV1 {
    Reserved,
    Settled,
    Released,
    Expired,
    Refunded,
}

/// Read-only correlation and consumption view of a Runtime-validated lease.
///
/// Quote accounting and settlement fields are intentionally omitted. This
/// value is not a lease, admission token, or proof of a Runtime reservation;
/// only the Runtime adapter may create it from authoritative state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CognitionLeaseConsumptionViewV1 {
    pub schema_version: String,
    pub lease_id: String,
    pub idempotency_key: String,
    pub agent_id: String,
    pub agent_session_id: String,
    pub agent_turn_id: String,
    pub decision_request_id: String,
    pub request_digest: String,
    pub status: CognitionLeaseStatusV1,
    pub reserved_amount: u64,
    pub reserved_at_tick: u64,
}

impl CognitionLeaseConsumptionViewV1 {
    pub const SCHEMA_VERSION: &'static str = "cognition-lease-consumption-view.v1";
}

/// Correlation projection for a Runtime-issued receipt. A serialized value is
/// not evidence that a durable world readback succeeded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeReceiptLineageV1 {
    pub schema_version: String,
    pub status: String,
    pub receipt_id: String,
    pub receipt_digest: String,
    pub envelope_digest: String,
    pub action_id: String,
    pub agent_id: String,
    pub agent_session_id: String,
    pub agent_turn_id: String,
    pub decision_request_id: String,
    pub request_digest: String,
    pub feedback_id: String,
}

impl RuntimeReceiptLineageV1 {
    pub const SCHEMA_VERSION: &'static str = "runtime-receipt-lineage.v1";

    /// Structural validation only. Runtime remains responsible for deriving
    /// this projection from a durable committed marker and for readback proof.
    pub fn validate(&self) -> Result<(), CognitionError> {
        if self.schema_version != Self::SCHEMA_VERSION
            || self.status != "committed"
            || self.receipt_id.trim().is_empty()
            || self.receipt_digest.trim().is_empty()
            || self.envelope_digest.trim().is_empty()
            || self.action_id.trim().is_empty()
            || self.agent_id.trim().is_empty()
            || self.agent_session_id.trim().is_empty()
            || self.agent_turn_id.trim().is_empty()
            || self.decision_request_id.trim().is_empty()
            || self.request_digest.trim().is_empty()
            || self.feedback_id.trim().is_empty()
        {
            return Err(CognitionError::new(
                "runtime_receipt_lineage_invalid",
                "Runtime receipt lineage is incomplete or not committed",
            ));
        }
        Ok(())
    }
}
