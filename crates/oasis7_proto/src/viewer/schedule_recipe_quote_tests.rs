use super::*;

#[test]
fn schedule_recipe_quote_protocol_round_trip_preserves_signed_request_and_preflight() {
    let request = ViewerRequest::QuoteScheduleRecipe {
        request: ScheduleRecipeQuoteRequest {
            factory_id: "factory.smelter.mk1".to_string(),
            recipe_id: "recipe.smelter.iron_ingot".to_string(),
            batches: 2,
            player_id: "player-1".to_string(),
            public_key: Some("pk-1".to_string()),
            auth: Some(PlayerAuthProof {
                scheme: PlayerAuthScheme::Ed25519,
                player_id: "player-1".to_string(),
                public_key: "pk-1".to_string(),
                nonce: 7,
                signature: "awviewauth:v1:deadbeef".to_string(),
            }),
        },
    };
    let request_json = serde_json::to_string(&request).expect("serialize request");
    let parsed: ViewerRequest = serde_json::from_str(&request_json).expect("deserialize request");
    assert_eq!(parsed, request);

    let quote = ScheduleRecipeQuotePreflight {
        owner_agent_id: "agent-0".to_string(),
        factory_id: "factory.smelter.mk1".to_string(),
        recipe_id: "recipe.smelter.iron_ingot".to_string(),
        batches: 2,
        base_duration_ticks: 2,
        electricity_cost: 6,
        electricity_after: 26,
        hardware_cost: 2,
        data_output: 1,
        finished_product_id: "iron_ingot".to_string(),
        finished_product_units: 1,
        local_shortage_delay_ticks: 0,
        shortage_reason: "none".to_string(),
        recommended_pre_step: "schedule_now".to_string(),
        runway_before_ticks: 100,
        runway_after_ticks: 100,
        downtime_threshold_ppm: 50_000,
        continue_production_risk: "low".to_string(),
        maintenance_pressure_delta: "unchanged".to_string(),
        recommended_maintenance_action: "continue_production".to_string(),
    };
    let quote_json = serde_json::to_string(&quote).expect("serialize preflight");
    let parsed: ScheduleRecipeQuotePreflight =
        serde_json::from_str(&quote_json).expect("deserialize preflight");
    assert_eq!(parsed, quote);
}
