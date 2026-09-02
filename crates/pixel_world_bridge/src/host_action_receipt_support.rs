use serde_json::Value;

use super::{normalize_gameplay_token, obj, str_key, tr};

pub(super) fn action_receipt_title(locale: &str, state: &str, present: bool) -> String {
    if !present {
        return tr(locale, "暂无行动回执", "No action receipt yet");
    }
    match state {
        "accepted" | "submitted" | "queued" | "ack" => tr(locale, "行动已接受", "Action accepted"),
        "blocked" => tr(locale, "行动被阻塞", "Action blocked"),
        "completed" | "completed_advanced" | "committed" => {
            tr(locale, "世界已改变", "World changed")
        }
        "rejected" => tr(locale, "行动被拒绝", "Action rejected"),
        _ => tr(locale, "行动进行中", "Action in progress"),
    }
}

pub(super) fn has_enabled_first_agent_claim(gameplay: &Value) -> bool {
    obj(gameplay, "availableActions")
        .as_array()
        .is_some_and(|actions| {
            actions.iter().any(|action| {
                str_key(action, "actionId") == Some("claim_first_agent")
                    && str_key(action, "disabledReason").is_none()
            })
        })
}

pub(super) fn is_pending_receipt_stage(stage: Option<&str>) -> bool {
    matches!(
        stage,
        Some("accepted" | "submitted" | "queued" | "ack" | "registering" | "signing" | "sent")
    )
}

pub(super) fn is_non_advancing_receipt_stage(stage: Option<&str>) -> bool {
    matches!(stage, Some("completed_no_progress" | "blocked"))
}

pub(super) fn is_rejected_receipt_stage(stage: Option<&str>) -> bool {
    stage == Some("rejected")
}

pub(super) fn is_authoritative_world_delta_stage(stage: Option<&str>) -> bool {
    matches!(stage, Some("completed_advanced" | "committed"))
}

fn has_rejected_reason(value: Option<&str>, tokens: &[&str]) -> bool {
    let normalized = normalize_gameplay_token(value);
    tokens.iter().any(|token| {
        let token = normalize_gameplay_token(Some(token));
        normalized == token || normalized.starts_with(&token)
    })
}

pub(super) fn rejected_receipt_detail(
    locale: &str,
    gameplay: &Value,
    recent_feedback: &Value,
) -> String {
    let reason =
        str_key(recent_feedback, "reason").or_else(|| str_key(gameplay, "executionCauseKind"));
    if has_rejected_reason(
        reason,
        &[
            "unsupported_gameplay_action",
            "unknown_gameplay_action",
            "unsupported_action",
        ],
    ) {
        return tr(
            locale,
            "请选择已发布的玩法动作后重试。",
            "Choose a published gameplay action before retrying.",
        );
    }
    if has_rejected_reason(
        reason,
        &[
            "permission_denied",
            "unauthorized",
            "forbidden",
            "agent_permission_denied",
        ],
    ) {
        return tr(
            locale,
            "请检查当前 Agent 绑定和所需权限后重试。",
            "Check the current Agent binding and required permissions before retrying.",
        );
    }
    if has_rejected_reason(reason, &["unsupported_mode", "mode_denied", "invalid_mode"]) {
        return tr(
            locale,
            "请切换到支持的模式，并重试已发布的动作。",
            "Switch to a supported mode and retry the published action.",
        );
    }
    tr(
        locale,
        "请查看已发布动作和当前前提后重试。",
        "Review the published actions and current prerequisites before retrying.",
    )
}
