use serde_json::Value;

use super::AgentDecisionTrace;

pub(super) fn append_decision_upstream_trace(reason: String, trace: &AgentDecisionTrace) -> String {
    if reason.contains("upstream_trace=") {
        return reason;
    }
    let Some(output) = trace.llm_output.as_deref() else {
        return reason;
    };
    let Ok(payload) = serde_json::from_str::<Value>(output) else {
        return reason;
    };
    let upstream_trace = payload
        .get("upstream_trace")
        .or_else(|| payload.get("trace_payload")?.get("upstream_trace"));
    let Some(upstream_trace) = upstream_trace else {
        return reason;
    };
    let Ok(mut serialized) = serde_json::to_string(upstream_trace) else {
        return reason;
    };
    const MAX_UPSTREAM_TRACE_REASON_CHARS: usize = 1200;
    if serialized.len() > MAX_UPSTREAM_TRACE_REASON_CHARS {
        let mut end = MAX_UPSTREAM_TRACE_REASON_CHARS;
        while end > 0 && !serialized.is_char_boundary(end) {
            end -= 1;
        }
        serialized.truncate(end);
        serialized.push_str("...");
    }
    format!("{reason}; upstream_trace={serialized}")
}

pub(super) fn decision_trace_provider_error_retryable(trace: &AgentDecisionTrace) -> Option<bool> {
    let output = trace.llm_output.as_deref()?;
    let payload = serde_json::from_str::<Value>(output).ok()?;
    payload
        .get("provider_error")
        .or_else(|| payload.get("trace_payload")?.get("provider_error"))
        .and_then(|error| error.get("retryable"))
        .and_then(Value::as_bool)
}
