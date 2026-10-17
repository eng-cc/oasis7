//! Black-box regression also compilable against the unmodified report API.
use super::*;
struct OutputLimitProvider;
impl ModelProvider for OutputLimitProvider {
    fn provider_id(&self) -> &str {
        "black-box-output-limit"
    }
    fn complete(
        &mut self,
        _: &ModelProviderRequest,
    ) -> Result<ModelProviderResponse, ModelProviderError> {
        Err(ModelProviderError::with_upstream_trace(
            "output_limit",
            "private-response-body",
            false,
            Some(json!({"secret":"private-trace"})),
        ))
    }
}
#[test]
fn report_retains_safe_fixed_failure_without_provider_payload() {
    let host = Arc::new(FakeHost::new());
    let report = NativeTaskRunner::new(host.clone(), Box::new(OutputLimitProvider), config())
        .unwrap()
        .run()
        .unwrap();
    let value = serde_json::to_value(&report).unwrap();
    assert_eq!(value["model_failure"], "output_limit");
    assert_eq!(value["stop_reason"], "model_failed");
    assert_eq!(value["model_calls_spent"], 1);
    assert_eq!(value["turns_completed"], 0);
    assert!(!value.to_string().contains("private-"));
    assert!(!host.events().iter().any(|e| e.starts_with("tool:")));
}
