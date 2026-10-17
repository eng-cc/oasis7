//! HTTP composition reuses hosted authentication and strict JSON parsing.

pub(crate) fn handle_request(
    runtime: &super::super::static_http::StaticHttpRuntimeConfig,
    method: &str,
    target: &str,
    headers: &str,
    body: &str,
) -> super::super::hosted_identity::IdentityReply {
    use super::{disabled, error};
    let capabilities = method == "GET" && target == "/v1/agent-client/capabilities";
    if target.contains('?') || target.contains('#') {
        return error(400, "invalid_request");
    }
    let Some(client) = &runtime.model_client else {
        return if capabilities {
            disabled("deployment_unavailable")
        } else {
            error(503, "deployment_unavailable")
        };
    };
    let principal = {
        let Ok(mut guard) = runtime.identity.lock() else {
            return error(503, "authentication_failed");
        };
        let Some(service) = guard.as_mut() else {
            return disabled("authentication_required");
        };
        match service.model_principal(headers, method != "GET") {
            Ok(principal) => principal,
            Err(error_code) => {
                return if capabilities {
                    disabled("authentication_required")
                } else {
                    super::super::hosted_identity::IdentityReply::error(error_code)
                };
            }
        }
    };
    let payload = if method == "GET" {
        serde_json::json!({})
    } else {
        match super::super::static_http::parse_model_json(body.as_bytes()) {
            Ok(payload) => payload,
            Err(_) => return error(400, "invalid_request"),
        }
    };
    client.handle(&principal, method, target, payload)
}
