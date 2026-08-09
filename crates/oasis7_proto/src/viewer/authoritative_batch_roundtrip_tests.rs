use super::*;

#[test]
fn viewer_response_round_trip_authoritative_batch() {
    let response = ViewerResponse::<
        serde_json::Value,
        serde_json::Value,
        serde_json::Value,
        serde_json::Value,
        u64,
    >::AuthoritativeBatch {
        batch: AuthoritativeBatchFinality {
            batch_id: "batch-7".to_string(),
            tx_hash: "tx-hash-7".to_string(),
            commit_tick: 70,
            confirm_height: 72,
            final_height: 75,
            state_root: "state-root-7".to_string(),
            data_root: "data-root-7".to_string(),
            finality_state: AuthoritativeFinalityState::Confirmed,
            event_seq_start: Some(101),
            event_seq_end: Some(110),
            settlement_ready: false,
            ranking_ready: false,
            challenge_open: true,
            slashed: false,
            active_challenge_id: Some("challenge-9".to_string()),
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
