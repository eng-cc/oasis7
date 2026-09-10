use serde::{Deserialize, Serialize};

/// The normalized request budget used by the native continuous-agent lane.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BudgetContractV1 {
    pub max_latency_ms: u64,
    pub max_repair_attempts: u32,
    /// Maximum number of model-provider invocations admitted for this
    /// logical request. Zero is an explicit deny policy; it is not an
    /// unlimited sentinel. The field is required in the target outer wire
    /// contract; legacy inputs must stay on an explicit compatibility lane.
    pub max_model_calls: u32,
    /// Maximum number of host tool/module invocations admitted for this
    /// logical request. Zero is an explicit deny policy.
    pub max_tool_calls: u32,
}
