use std::time::Duration;

use reqwest::blocking::{Client, RequestBuilder};
use serde::Serialize;
use serde_json::{json, Map, Value};

const DEFAULT_LOG_QUERY_LIMIT: usize = 50;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LetaiAdapterError {
    pub(super) code: &'static str,
    pub(super) message: String,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct LetaiUserUpsertRequest {
    pub(super) external_user_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) external_user_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) metadata: Option<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct LetaiUserUpsertResult {
    pub(super) platform_user_id: String,
    pub(super) snapshot: Value,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct LetaiEnsureProjectTokenRequest {
    pub(super) external_project_id: String,
    pub(super) project_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) parent_channel_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) metadata: Option<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct LetaiProjectTokenResult {
    pub(super) platform_project_id: String,
    pub(super) token_key: String,
    pub(super) token_status: Option<String>,
    pub(super) snapshot: Value,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct LetaiUserTopupRequest {
    pub(super) external_order_id: String,
    pub(super) quota: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) amount: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) currency: Option<String>,
}

#[derive(Debug, Clone)]
pub(super) struct LetaiOpenApiAdapter {
    base_url: String,
    platform_key: String,
    parent_channel_id: Option<String>,
    client: Client,
}

impl LetaiOpenApiAdapter {
    pub(super) fn new(
        base_url: &str,
        platform_key: &str,
        parent_channel_id: Option<&str>,
        timeout_ms: u64,
    ) -> Result<Self, String> {
        let base_url = normalize_base_url(base_url)?;
        let platform_key = normalize_required("platform key", platform_key)?;
        let client = Client::builder()
            .timeout(Duration::from_millis(timeout_ms.max(1)))
            .build()
            .map_err(|err| format!("build LetAI OpenAPI client failed: {err}"))?;
        Ok(Self {
            base_url,
            platform_key,
            parent_channel_id: parent_channel_id
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned),
            client,
        })
    }

    pub(super) fn parent_channel_id(&self) -> Option<&str> {
        self.parent_channel_id.as_deref()
    }

    pub(super) fn upsert_user(
        &self,
        request: &LetaiUserUpsertRequest,
    ) -> Result<LetaiUserUpsertResult, LetaiAdapterError> {
        let payload = self.post_json("/api/platform/open/users/upsert", request)?;
        let platform_user_id = extract_required_string(
            &payload,
            &["platform_user_id", "user_id", "id"],
            "platform_user_id_missing",
            "LetAI users/upsert response is missing platform_user_id",
        )?;
        Ok(LetaiUserUpsertResult {
            platform_user_id,
            snapshot: payload,
        })
    }

    pub(super) fn ensure_project_token(
        &self,
        platform_user_id: &str,
        request: &LetaiEnsureProjectTokenRequest,
    ) -> Result<LetaiProjectTokenResult, LetaiAdapterError> {
        let path = format!(
            "/api/platform/open/users/{}/projects/ensure-token",
            platform_user_id.trim()
        );
        let payload = self.post_json(path.as_str(), request)?;
        let platform_project_id = extract_required_string(
            &payload,
            &["platform_project_id", "project_id", "id"],
            "platform_project_id_missing",
            "LetAI ensure-project response is missing platform_project_id",
        )?;
        let token_key = extract_required_string(
            &payload,
            &["token_key"],
            "token_key_missing",
            "LetAI ensure-project response is missing token_key",
        )?;
        let token_status = extract_optional_string(&payload, &["token_status", "status"]);
        Ok(LetaiProjectTokenResult {
            platform_project_id,
            token_key,
            token_status,
            snapshot: payload,
        })
    }

    pub(super) fn topup_user(
        &self,
        platform_user_id: &str,
        request: &LetaiUserTopupRequest,
    ) -> Result<Value, LetaiAdapterError> {
        let path = format!(
            "/api/platform/open/users/{}/topups",
            platform_user_id.trim()
        );
        self.post_json(path.as_str(), request)
    }

    pub(super) fn fetch_user_summary(
        &self,
        platform_user_id: &str,
    ) -> Result<Value, LetaiAdapterError> {
        let path = format!("/api/platform/open/users/{}", platform_user_id.trim());
        self.get_json(path.as_str(), &[])
    }

    pub(super) fn fetch_project_token_summary(
        &self,
        platform_user_id: &str,
    ) -> Result<Value, LetaiAdapterError> {
        let path = format!(
            "/api/platform/open/users/{}/projects/token-summary",
            platform_user_id.trim()
        );
        self.get_json(path.as_str(), &[])
    }

    pub(super) fn fetch_user_logs(
        &self,
        platform_user_id: &str,
        external_order_id: &str,
    ) -> Result<Value, LetaiAdapterError> {
        let path = format!("/api/platform/open/users/{}/logs", platform_user_id.trim());
        self.get_json(
            path.as_str(),
            &[
                ("external_order_id", external_order_id),
                ("limit", &DEFAULT_LOG_QUERY_LIMIT.to_string()),
            ],
        )
    }

    fn post_json(&self, path: &str, payload: &impl Serialize) -> Result<Value, LetaiAdapterError> {
        let url = format!("{}{}", self.base_url, path);
        let response = self
            .authorized(self.client.post(url))
            .json(payload)
            .send()
            .map_err(|err| request_error("letai_request_failed", err.to_string()))?;
        decode_json_response(response, "LetAI POST")
    }

    fn get_json(&self, path: &str, query: &[(&str, &str)]) -> Result<Value, LetaiAdapterError> {
        let url = format!("{}{}", self.base_url, path);
        let response = self
            .authorized(self.client.get(url))
            .query(query)
            .send()
            .map_err(|err| request_error("letai_request_failed", err.to_string()))?;
        decode_json_response(response, "LetAI GET")
    }

    fn authorized(&self, builder: RequestBuilder) -> RequestBuilder {
        builder.bearer_auth(self.platform_key.as_str())
    }
}

fn request_error(code: &'static str, message: String) -> LetaiAdapterError {
    LetaiAdapterError { code, message }
}

fn decode_json_response(
    response: reqwest::blocking::Response,
    label: &str,
) -> Result<Value, LetaiAdapterError> {
    let status = response.status();
    let body = response
        .text()
        .map_err(|err| request_error("letai_response_read_failed", err.to_string()))?;
    if !status.is_success() {
        return Err(request_error(
            "letai_response_not_success",
            format!("{label} returned status {status}: {body}"),
        ));
    }
    if body.trim().is_empty() {
        return Ok(json!({"ok": true}));
    }
    let payload = serde_json::from_str::<Value>(body.as_str()).map_err(|err| {
        request_error(
            "letai_response_decode_failed",
            format!("{label} decode failed: {err}; body={body}"),
        )
    })?;
    if let Some(false) = payload.get("ok").and_then(Value::as_bool) {
        return Err(request_error(
            "letai_response_not_ok",
            format!(
                "{label} returned ok=false{}: {}",
                format_error_code(payload.get("error_code").and_then(Value::as_str)),
                payload
                    .get("error")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown error")
            ),
        ));
    }
    Ok(payload)
}

fn extract_required_string(
    value: &Value,
    keys: &[&str],
    code: &'static str,
    message: &'static str,
) -> Result<String, LetaiAdapterError> {
    extract_optional_string(value, keys).ok_or_else(|| request_error(code, message.to_string()))
}

fn extract_optional_string(value: &Value, keys: &[&str]) -> Option<String> {
    candidate_objects(value)
        .find_map(|object| {
            keys.iter().find_map(|key| {
                object
                    .get(*key)
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(ToOwned::to_owned)
            })
        })
        .or_else(|| extract_string_recursive(value, keys))
}

fn extract_string_recursive(value: &Value, keys: &[&str]) -> Option<String> {
    match value {
        Value::Object(map) => map.iter().find_map(|(key, child)| {
            if keys.iter().any(|candidate| candidate == key) {
                child
                    .as_str()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(ToOwned::to_owned)
                    .or_else(|| extract_string_recursive(child, keys))
            } else {
                extract_string_recursive(child, keys)
            }
        }),
        Value::Array(items) => items
            .iter()
            .find_map(|item| extract_string_recursive(item, keys)),
        _ => None,
    }
}

fn candidate_objects(value: &Value) -> impl Iterator<Item = &Map<String, Value>> {
    let root = value.as_object().into_iter();
    let data = value.get("data").and_then(Value::as_object).into_iter();
    let user = value.get("user").and_then(Value::as_object).into_iter();
    let project = value.get("project").and_then(Value::as_object).into_iter();
    root.chain(data).chain(user).chain(project)
}

fn normalize_base_url(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err("LetAI OpenAPI base URL must not be empty".to_string());
    }
    Ok(trimmed.to_string())
}

fn normalize_required(field: &str, raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(format!("LetAI {field} must not be empty"));
    }
    Ok(trimmed.to_string())
}

fn format_error_code(code: Option<&str>) -> String {
    code.map(|code| format!(" ({code})")).unwrap_or_default()
}
