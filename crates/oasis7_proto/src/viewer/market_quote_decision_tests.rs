use super::*;

#[test]
fn viewer_market_quote_decision_request_round_trip() {
    let request = ViewerRequest::QuoteMarketDecision {
        request: MarketQuoteDecisionRequest {
            consume: vec![MarketQuoteMaterialRequest {
                material: "iron_ingot".to_string(),
                amount: 4,
            }],
            player_id: "player-1".to_string(),
            public_key: Some("pk-1".to_string()),
            auth: None,
        },
    };
    let json = serde_json::to_string(&request).expect("serialize request");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&json).expect("request json")["type"],
        "quote_market_decision"
    );
    assert_eq!(
        serde_json::from_str::<ViewerRequest>(&json).expect("deserialize request"),
        request
    );
}

#[test]
fn viewer_response_round_trip_market_quote_preflight() {
    let response = ViewerResponse::<
        serde_json::Value,
        serde_json::Value,
        serde_json::Value,
        serde_json::Value,
        u64,
    >::MarketQuoteDecisionPreflight {
        quote: MarketQuoteDecisionPreflight {
            consuming_agent_id: "agent-0".to_string(),
            contributions: vec![MarketQuoteMaterialContribution {
                material: "Iron ingot".to_string(),
                requested_amount: 4,
                local_available_amount: 1,
                world_available_amount: 3,
                world_cover_amount: 3,
                shortfall_amount: 0,
                transit_loss_bps: 20,
                governance_tax_bps: 100,
                effective_cost_index_ppm: 1_002_000,
            }],
            total_shortfall_amount: 0,
            submission_allowed: true,
            conditional_notice: "This is a conditional preview.".to_string(),
            recommendation: "Submit using available world supply".to_string(),
            rationale: "World supply can cover the local gap.".to_string(),
            next_action: "Submit the recipe when ready".to_string(),
        },
    };
    let json = serde_json::to_string(&response).expect("serialize response");
    assert_eq!(
        serde_json::from_str::<
            ViewerResponse<
                serde_json::Value,
                serde_json::Value,
                serde_json::Value,
                serde_json::Value,
                u64,
            >,
        >(&json)
        .expect("deserialize response"),
        response
    );
}
