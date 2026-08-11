use serde::{Deserialize, Serialize};

use super::PlayerAuthProof;

/// Player-facing priority override for a material transfer quote.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransferMaterialPriority {
    Urgent,
    Standard,
}

/// A signed, read-only request for the runtime's logistics transfer quote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferMaterialQuoteRequest {
    pub requester_agent_id: String,
    pub from_ledger: String,
    pub to_ledger: String,
    pub kind: String,
    pub amount: i64,
    pub distance_km: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_priority: Option<TransferMaterialPriority>,
    pub player_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<PlayerAuthProof>,
}

/// Exact player-safe projection of the runtime logistics quote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferMaterialQuotePreflight {
    pub requester_agent_id: String,
    pub from_ledger: String,
    pub to_ledger: String,
    pub kind: String,
    pub requested_amount: i64,
    pub submission_feasible: bool,
    pub max_transferable_amount: i64,
    pub sent_amount: i64,
    pub distance_km: i64,
    pub loss_bps: i64,
    pub expected_loss_amount: i64,
    pub expected_received_amount: i64,
    pub source_amount_before: i64,
    pub source_amount_after: i64,
    pub destination_amount_before: i64,
    pub destination_expected_amount_after: i64,
    pub ticks_until_arrival: u64,
    pub ready_at: u64,
    pub effective_priority: TransferMaterialPriority,
    pub priority_reason: String,
    pub inflight_before: usize,
    pub inflight_capacity: usize,
    pub recommendation: String,
    pub conditional: bool,
}
