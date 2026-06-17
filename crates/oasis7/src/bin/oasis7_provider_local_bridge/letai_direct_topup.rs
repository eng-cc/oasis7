use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};

use super::super::credit_adapter::{LetaiOpenApiAdapter, LetaiUserTopupRequest};
use super::super::{short_sha256, summarize_text};
use super::{value_keys, AutoTopupOutcome, LetaiChatConfig};

const LETAI_QUOTA_UNITS_PER_USD: f64 = 500_000.0;
static LETAI_AUTO_TOPUP_ORDER_COUNTER: AtomicU64 = AtomicU64::new(1);

pub(crate) fn should_auto_topup_letai_error(error: &str) -> bool {
    error
        .to_ascii_lowercase()
        .contains("insufficient_user_quota")
        || error.contains("余额")
}

pub(crate) fn maybe_auto_topup_letai_user(
    config: &LetaiChatConfig,
    error: &str,
) -> Result<AutoTopupOutcome, String> {
    if !should_auto_topup_letai_error(error) {
        return Ok(AutoTopupOutcome {
            triggered: false,
            trace: json!({
                "stage": "auto_topup",
                "status": "not_applicable",
                "reason": "error_not_quota_related",
            }),
        });
    }
    let Some(topup_usd) = config.auto_topup_usd.as_deref() else {
        return Ok(AutoTopupOutcome {
            triggered: false,
            trace: json!({
                "stage": "auto_topup",
                "status": "skipped",
                "reason": "auto_topup_usd_missing",
            }),
        });
    };
    let Some(quota) = quota_from_usd(topup_usd)? else {
        return Ok(AutoTopupOutcome {
            triggered: false,
            trace: json!({
                "stage": "auto_topup",
                "status": "skipped",
                "reason": "auto_topup_usd_disabled",
                "amount_usd": topup_usd.trim(),
            }),
        });
    };
    let Some(platform_key) = config.platform_key.as_deref() else {
        return Ok(AutoTopupOutcome {
            triggered: false,
            trace: json!({
                "stage": "auto_topup",
                "status": "skipped",
                "reason": "platform_key_missing",
                "amount_usd": topup_usd.trim(),
                "quota": quota,
            }),
        });
    };
    let Some(platform_user_id) = config.platform_user_id.as_deref() else {
        return Ok(AutoTopupOutcome {
            triggered: false,
            trace: json!({
                "stage": "auto_topup",
                "status": "skipped",
                "reason": "platform_user_id_missing",
                "amount_usd": topup_usd.trim(),
                "quota": quota,
            }),
        });
    };
    let adapter = LetaiOpenApiAdapter::new(
        config.platform_base_url.as_str(),
        platform_key,
        None,
        30_000,
    )
    .map_err(|err| format!("build LetAI auto topup adapter failed: {err}"))?;
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| format!("system clock before unix epoch: {err}"))?
        .as_millis();
    let order_seq = LETAI_AUTO_TOPUP_ORDER_COUNTER.fetch_add(1, AtomicOrdering::Relaxed);
    let external_order_id = format!(
        "oasis7-local-auto-topup-{now_ms}-{}-{order_seq}",
        std::process::id()
    );
    let topup_receipt = adapter
        .topup_user(
            platform_user_id,
            &LetaiUserTopupRequest {
                external_order_id: external_order_id.clone(),
                quota,
                amount: Some(topup_usd.trim().to_string()),
                currency: Some("USD".to_string()),
            },
        )
        .map_err(|err| format!("auto topup failed: {}: {}", err.code, err.message))?;
    let platform_diagnostics = auto_topup_platform_diagnostics(
        &adapter,
        platform_user_id,
        config.platform_project_id.as_deref(),
        external_order_id.as_str(),
        &topup_receipt,
    );
    eprintln!(
        "{}",
        json!({
            "event": "rust_direct_letai_auto_topup",
            "quota": quota,
            "amount_usd": topup_usd.trim(),
            "external_order_id": external_order_id,
            "platform_diagnostics": platform_diagnostics,
        })
    );
    Ok(AutoTopupOutcome {
        triggered: true,
        trace: json!({
            "stage": "auto_topup",
            "status": "triggered",
            "quota": quota,
            "amount_usd": topup_usd.trim(),
            "platform_user_id": platform_user_id,
            "platform_project_id": config.platform_project_id.as_deref(),
            "external_order_id": external_order_id,
            "platform_diagnostics": platform_diagnostics,
        }),
    })
}

fn auto_topup_platform_diagnostics(
    adapter: &LetaiOpenApiAdapter,
    platform_user_id: &str,
    platform_project_id: Option<&str>,
    external_order_id: &str,
    topup_receipt: &Value,
) -> Value {
    let Some(project_id) = platform_project_id else {
        return json!({
            "topup_receipt": summarize_letai_platform_payload("topup_user", topup_receipt),
            "user_summary": {
                "stage": "fetch_user_summary",
                "status": "skipped",
                "reason": "platform_project_id_missing",
            },
            "project_summary": {
                "stage": "fetch_project_token_summary",
                "status": "skipped",
                "reason": "platform_project_id_missing",
            },
            "project_logs": {
                "stage": "fetch_project_logs",
                "status": "skipped",
                "reason": "platform_project_id_missing",
            },
        });
    };
    let user_summary = summarize_letai_platform_probe(
        adapter.fetch_user_summary(platform_user_id),
        "fetch_user_summary",
    );
    let (project_summary, project_logs) = {
        (
            summarize_letai_platform_probe(
                adapter.fetch_project_token_summary(project_id),
                "fetch_project_token_summary",
            ),
            summarize_letai_platform_probe(
                adapter.fetch_project_logs(project_id, external_order_id),
                "fetch_project_logs",
            ),
        )
    };

    json!({
        "topup_receipt": summarize_letai_platform_payload("topup_user", topup_receipt),
        "user_summary": user_summary,
        "project_summary": project_summary,
        "project_logs": project_logs,
    })
}

fn summarize_letai_platform_probe(
    result: Result<Value, super::super::credit_adapter::LetaiAdapterError>,
    stage: &str,
) -> Value {
    match result {
        Ok(payload) => summarize_letai_platform_payload(stage, &payload),
        Err(err) => json!({
            "stage": stage,
            "status": "error",
            "code": err.code,
            "message": summarize_text(err.message.as_str(), 240),
        }),
    }
}

fn summarize_letai_platform_payload(stage: &str, payload: &Value) -> Value {
    json!({
        "stage": stage,
        "status": "ok",
        "top_level_keys": value_keys(payload),
        "data_keys": payload.get("data").map(value_keys).unwrap_or_default(),
        "ok": payload.get("ok").and_then(Value::as_bool),
        "success": payload.get("success").and_then(Value::as_bool),
        "items_len": payload
            .get("items")
            .and_then(Value::as_array)
            .map(Vec::len)
            .or_else(|| payload.get("data").and_then(Value::as_array).map(Vec::len)),
        "payload_sha256": short_sha256(payload.to_string().as_str()),
    })
}

pub(crate) fn quota_from_usd(raw: &str) -> Result<Option<u64>, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let amount = trimmed
        .parse::<f64>()
        .map_err(|err| format!("invalid auto topup USD amount: {trimmed}: {err}"))?;
    if amount <= 0.0 {
        return Ok(None);
    }
    let quota = (amount * LETAI_QUOTA_UNITS_PER_USD).floor();
    if !quota.is_finite() || quota <= 0.0 || quota > u64::MAX as f64 {
        return Err(format!("invalid auto topup USD amount: {trimmed}"));
    }
    Ok(Some(quota as u64))
}
