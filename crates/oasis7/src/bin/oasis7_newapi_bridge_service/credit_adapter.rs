use std::time::Duration;

use reqwest::blocking::Client;
use serde::Serialize;
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize)]
pub(super) struct CreditApplyRequest {
    pub(super) bridge_deposit_id: String,
    pub(super) beneficiary_ref: String,
    pub(super) pricing_version: Option<String>,
    pub(super) topup_plan_id: Option<String>,
    pub(super) amount_oc: u64,
    pub(super) credit_units: u64,
    pub(super) bonus_units: u64,
    pub(super) total_credit_units: u64,
    pub(super) target_type: String,
    pub(super) chain_tx_id: String,
    pub(super) chain_action_id: Option<u64>,
    pub(super) idempotency_key: String,
}

#[derive(Debug, Clone)]
pub(super) struct NewApiCreditAdapter {
    endpoint_url: String,
    auth_token: Option<String>,
    client: Client,
}

impl NewApiCreditAdapter {
    pub(super) fn new(
        endpoint_url: &str,
        auth_token: Option<&str>,
        timeout_ms: u64,
    ) -> Result<Self, String> {
        let endpoint_url = normalize_endpoint_url(endpoint_url)?;
        let client = Client::builder()
            .timeout(Duration::from_millis(timeout_ms.max(1)))
            .build()
            .map_err(|err| format!("build credit adapter client failed: {err}"))?;
        Ok(Self {
            endpoint_url,
            auth_token: auth_token
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned),
            client,
        })
    }

    pub(super) fn apply_credit(&self, request: &CreditApplyRequest) -> Result<Value, String> {
        let mut builder = self.client.post(self.endpoint_url.as_str()).json(request);
        if let Some(token) = self.auth_token.as_deref() {
            builder = builder.bearer_auth(token);
        }
        let response = builder
            .send()
            .map_err(|err| format!("credit adapter request failed: {err}"))?;
        let status = response.status();
        let body = response
            .text()
            .map_err(|err| format!("read credit adapter response failed: {err}"))?;
        if !status.is_success() {
            return Err(format!("credit adapter returned status {status}: {body}"));
        }
        if body.trim().is_empty() {
            return Ok(json!({"ok": true}));
        }
        match serde_json::from_str::<Value>(body.as_str()) {
            Ok(value) => Ok(value),
            Err(_) => Ok(json!({ "raw_body": body })),
        }
    }
}

fn normalize_endpoint_url(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("credit adapter URL must not be empty".to_string());
    }
    Ok(trimmed.to_string())
}
