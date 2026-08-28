use serde::{Deserialize, Serialize};

use super::PlayerAuthProof;

fn is_false(value: &bool) -> bool {
    !*value
}

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
    /// Optional explicit legacy route or ordered multi-hop path binding.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub route_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub auto_reroute: bool,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub route_ids: Vec<String>,
    #[serde(default)]
    pub tariff_electricity_total: i64,
    #[serde(default)]
    pub reroute_count: u32,
    pub recommendation: String,
    pub conditional: bool,
}

#[cfg(test)]
mod tests {
    use super::{TransferMaterialPriority, TransferMaterialQuotePreflight};

    #[test]
    fn preflight_serializes_known_zero_route_metrics() {
        let quote = TransferMaterialQuotePreflight {
            requester_agent_id: "agent-0".into(),
            from_ledger: "site:source".into(),
            to_ledger: "site:destination".into(),
            kind: "iron_ingot".into(),
            requested_amount: 1,
            submission_feasible: true,
            max_transferable_amount: 1,
            sent_amount: 1,
            distance_km: 1,
            loss_bps: 0,
            expected_loss_amount: 0,
            expected_received_amount: 1,
            source_amount_before: 1,
            source_amount_after: 0,
            destination_amount_before: 0,
            destination_expected_amount_after: 1,
            ticks_until_arrival: 1,
            ready_at: 1,
            effective_priority: TransferMaterialPriority::Standard,
            priority_reason: "material_default_priority".into(),
            inflight_before: 0,
            inflight_capacity: 8,
            path_id: Some("path:free".into()),
            route_ids: vec!["route:free".into()],
            tariff_electricity_total: 0,
            reroute_count: 0,
            recommendation: "submit_transfer".into(),
            conditional: true,
        };

        let encoded = serde_json::to_value(quote).expect("serialize transfer quote");
        assert_eq!(encoded["tariff_electricity_total"], 0);
        assert_eq!(encoded["reroute_count"], 0);
    }
}
