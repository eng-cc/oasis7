//! Read-only executor diagnostics. Missing fields never imply progress or success.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AgentExecutorStatus {
    pub profile: Option<String>,
    pub execution_location: Option<String>,
    pub phase: String,
    pub reason: Option<String>,
    pub model: Option<String>,
    pub max_model_calls: Option<u32>,
    pub reserved_model_calls: Option<u32>,
    pub committed_turns: Option<u32>,
}
impl AgentExecutorStatus {
    pub fn phase_label(&self, zh: bool) -> &'static str {
        match (self.phase.as_str(), zh) {
            ("running", true) => "运行中",
            ("running", false) => "Running",
            ("starting", true) => "启动中",
            ("starting", false) => "Starting",
            ("blocked", true) => "已阻断",
            ("blocked", false) => "Blocked",
            ("stopped", true) => "已停止",
            ("stopped", false) => "Stopped",
            ("not_selected", true) => "未启用",
            ("not_selected", false) => "Not selected",
            (_, true) => "未知",
            (_, false) => "Unknown",
        }
    }
    pub fn reason_label(&self, zh: bool) -> &'static str {
        match (self.reason.as_deref(), zh) {
            (Some("model_budget_exhausted"), true) => "模型预算耗尽；目标状态未知",
            (Some("model_budget_exhausted"), false) => {
                "Model budget exhausted; goal status unknown"
            }
            (Some("turn_limit_reached"), true) => "已达轮次上限；目标状态未知",
            (Some("turn_limit_reached"), false) => "Turn limit reached; goal status unknown",
            (Some("original_result_pending"), true) => "等待原始提交结果",
            (Some("original_result_pending"), false) => "Awaiting original submission result",
            (Some("original_operation_rejected"), true) => "原始提交已拒绝",
            (Some("original_operation_rejected"), false) => "Original submission rejected",
            (Some("context_expired"), true) => "任务上下文已过期",
            (Some("context_expired"), false) => "Task context expired",
            (Some("task_control_changed"), true) => "任务控制已变更",
            (Some("task_control_changed"), false) => "Task control changed",
            (Some("model_unavailable"), true) => "模型不可用",
            (Some("model_unavailable"), false) => "Model unavailable",
            (Some("model_timeout"), true) => "模型请求超时",
            (Some("model_timeout"), false) => "Model request timed out",
            (Some("unsupported_model_output"), true) => "模型输出不受支持",
            (Some("unsupported_model_output"), false) => "Unsupported model output",
            (Some("executor_aborted"), true) => "执行已中止",
            (Some("executor_aborted"), false) => "Executor aborted",
            (Some("client_or_model_unavailable"), true) => "客户端或模型不可用",
            (Some("client_or_model_unavailable"), false) => "Client or model unavailable",
            (Some("client_unavailable"), true) => "客户端不可用",
            (Some("client_unavailable"), false) => "Client unavailable",
            (_, true) => "原因未知",
            (_, false) => "Reason unknown",
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unknown_and_budget_stops_never_claim_goal_completion() {
        let mut status = AgentExecutorStatus::default();
        assert_eq!(status.phase_label(false), "Unknown");
        assert_eq!(status.committed_turns, None);
        for reason in ["model_budget_exhausted", "turn_limit_reached"] {
            status.reason = Some(reason.into());
            assert!(status.reason_label(false).contains("goal status unknown"));
            assert!(!status.reason_label(false).contains("completed"));
        }
        status.reason = Some("opaque diagnostic that may contain private data".into());
        assert_eq!(status.reason_label(false), "Reason unknown");
    }
}
