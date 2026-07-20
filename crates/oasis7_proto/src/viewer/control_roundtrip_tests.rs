use super::*;

#[test]
fn viewer_request_round_trip() {
    let request = ViewerRequest::Control {
        mode: ViewerControl::Step { count: 2 },
        request_id: Some(7),
    };
    let json = serde_json::to_string(&request).expect("serialize request");
    let parsed: ViewerRequest = serde_json::from_str(&json).expect("deserialize request");
    assert_eq!(parsed, request);
}
