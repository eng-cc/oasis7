use serde::{Deserialize, Serialize};

use super::PlayerAuthProof;

/// A signed, read-only request for the simulator's exact `SellPower` preflight.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PowerSaleQuoteRequest {
    pub buyer_agent_id: String,
    pub amount: i64,
    pub requested_price_per_pu: i64,
    pub player_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<PlayerAuthProof>,
}

/// The non-mutating, authoritative facts a player needs before submitting `SellPower`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PowerSaleQuotePreflight {
    pub seller_agent_id: String,
    pub buyer_agent_id: String,
    pub current_power_level: i64,
    pub power_state_before: String,
    pub sale_amount: i64,
    pub price_per_pu: i64,
    pub expected_revenue: i64,
    pub power_state_after_sale: String,
    pub remaining_runway_ticks: i64,
    pub next_action_affordability_after_sale: String,
    pub production_interrupt_risk: bool,
    pub recommended_sale_action: String,
    pub why_sale_is_safe_or_risky: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::viewer::{PlayerAuthScheme, ViewerRequest, ViewerResponse};

    #[test]
    fn viewer_power_sale_quote_protocol_round_trip_preserves_seller_bound_request_and_risk_fields()
    {
        let request = ViewerRequest::QuotePowerSale {
            request: PowerSaleQuoteRequest {
                buyer_agent_id: "agent-buyer".to_string(),
                amount: 10,
                requested_price_per_pu: 3,
                player_id: "seller-player".to_string(),
                public_key: Some("pk-seller".to_string()),
                auth: Some(PlayerAuthProof {
                    scheme: PlayerAuthScheme::Ed25519,
                    player_id: "seller-player".to_string(),
                    public_key: "pk-seller".to_string(),
                    nonce: 91,
                    signature: "awviewauth:v1:deadbeef".to_string(),
                }),
            },
        };
        let request_json = serde_json::to_string(&request).expect("serialize request");
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&request_json).expect("request json")["type"],
            "quote_power_sale"
        );
        assert_eq!(
            serde_json::from_str::<ViewerRequest>(&request_json).expect("deserialize request"),
            request
        );

        let response = ViewerResponse::<
            serde_json::Value,
            serde_json::Value,
            serde_json::Value,
            serde_json::Value,
            u64,
        >::PowerSaleQuotePreflight {
            quote: PowerSaleQuotePreflight {
                seller_agent_id: "agent-seller".to_string(),
                buyer_agent_id: "agent-buyer".to_string(),
                current_power_level: 15,
                power_state_before: "low_power".to_string(),
                sale_amount: 10,
                price_per_pu: 3,
                expected_revenue: 30,
                power_state_after_sale: "critical".to_string(),
                remaining_runway_ticks: 5,
                next_action_affordability_after_sale: "limited".to_string(),
                production_interrupt_risk: true,
                recommended_sale_action: "defer_sale".to_string(),
                why_sale_is_safe_or_risky: "critical power runway".to_string(),
            },
        };
        let response_json = serde_json::to_string(&response).expect("serialize response");
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&response_json).expect("response json")["type"],
            "power_sale_quote_preflight"
        );
        assert_eq!(
            serde_json::from_str::<
                ViewerResponse<
                    serde_json::Value,
                    serde_json::Value,
                    serde_json::Value,
                    serde_json::Value,
                    u64,
                >,
            >(&response_json)
            .expect("deserialize response"),
            response
        );
    }
}
