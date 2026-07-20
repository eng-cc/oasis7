use super::*;

#[test]
fn viewer_collect_data_submit_request_round_trip() {
    let request = ViewerRequest::CollectData {
        command: CollectDataCommand::Submit {
            request: CollectDataRequest {
                electricity_cost: 7,
                data_amount: 11,
                player_id: "player-collect".to_string(),
                public_key: Some("collect-key".to_string()),
                auth: Some(PlayerAuthProof {
                    scheme: PlayerAuthScheme::Ed25519,
                    player_id: "player-collect".to_string(),
                    public_key: "collect-key".to_string(),
                    nonce: 9,
                    signature: "collect-signature".to_string(),
                }),
            },
        },
    };
    let json = serde_json::to_string(&request).expect("serialize request");
    let parsed: ViewerRequest = serde_json::from_str(&json).expect("deserialize request");
    assert_eq!(parsed, request);
}

#[test]
fn viewer_collect_data_preflight_response_round_trip() {
    let response = ViewerResponse::<
        serde_json::Value,
        serde_json::Value,
        serde_json::Value,
        serde_json::Value,
        u64,
    >::CollectDataPreflight {
        quote: CollectDataPreflight {
            collector_agent_id: "collector".to_string(),
            data_owner_agent_id: "collector".to_string(),
            data_recipient_agent_id: "collector".to_string(),
            data_use: "self_collection".to_string(),
            permission_status: "self_owned_no_grant_required".to_string(),
            electricity_cost: 7,
            data_amount: 11,
            available_electricity: 20,
            electricity_after: 13,
            can_execute: true,
            blocked_reason: None,
            recovery_guidance: None,
            alternative_action: None,
        },
    };
    let json = serde_json::to_string(&response).expect("serialize response");
    let parsed: ViewerResponse<
        serde_json::Value,
        serde_json::Value,
        serde_json::Value,
        serde_json::Value,
        u64,
    > = serde_json::from_str(&json).expect("deserialize response");
    assert_eq!(parsed, response);
}
