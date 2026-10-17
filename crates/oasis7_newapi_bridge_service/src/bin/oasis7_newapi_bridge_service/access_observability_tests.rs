use super::*;
use crate::dispatch_request_observed;

#[test]
fn dispatch_request_observed_reports_safe_access_metadata() {
    let test_service = test_service("http-access-observed", 900);
    let observed = dispatch_request_observed(
        &test_service.service,
        HttpRequest {
            method: "GET".to_string(),
            path: "/v1/bridge/operator/review/bridge-deposit-secret?token=hidden".to_string(),
            body: Vec::new(),
        },
    )
    .expect("dispatch observed");

    assert_eq!(observed.response.status_code, 404);
    assert_eq!(observed.access.method, "GET");
    assert_eq!(
        observed.access.path,
        "/v1/bridge/operator/review/:bridge_deposit_id"
    );
    assert_eq!(observed.access.status_code, 404);
    assert_eq!(observed.access.error_code.as_deref(), Some("not_found"));
    assert!(observed.access.elapsed_ms.is_some());
}

#[test]
fn dispatch_request_observed_coarsens_unknown_paths_without_leaking_segments() {
    let test_service = test_service("http-access-unknown-path", 900);
    let observed = dispatch_request_observed(
        &test_service.service,
        HttpRequest {
            method: "GET".to_string(),
            path: format!("/v1/bridge/bind/sk_live_secret_{}", "令牌".repeat(120)),
            body: Vec::new(),
        },
    )
    .expect("dispatch observed");

    assert_eq!(observed.response.status_code, 404);
    assert_eq!(observed.access.path, "/<unknown>");
    assert!(!observed.access.path.contains("sk_live"));
    assert!(!observed.access.path.contains("令牌"));
    assert_eq!(observed.access.error_code.as_deref(), Some("not_found"));
}
