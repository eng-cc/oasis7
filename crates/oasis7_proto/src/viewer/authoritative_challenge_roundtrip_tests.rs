use super::*;

#[test]
fn viewer_response_round_trip_authoritative_challenge_ack() {
    let response = ViewerResponse::<
        serde_json::Value,
        serde_json::Value,
        serde_json::Value,
        serde_json::Value,
        u64,
    >::AuthoritativeChallengeAck {
        ack: AuthoritativeChallengeAck {
            challenge_id: "challenge-1".to_string(),
            batch_id: "batch-1".to_string(),
            watcher_id: "watcher-1".to_string(),
            status: AuthoritativeChallengeStatus::ResolvedFraudSlashed,
            submitted_at_tick: 40,
            resolved_at_tick: Some(42),
            slash_applied: true,
            slash_reason: Some("state_root_mismatch".to_string()),
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
