use super::*;

struct FailingProvider {
    code: &'static str,
    preflight: bool,
    allowed: Option<Arc<AtomicBool>>,
}
impl ModelProvider for FailingProvider {
    fn provider_id(&self) -> &str {
        "diagnostic-fixture"
    }
    fn preflight(&self, _: &ModelProviderRequest) -> Result<(), ModelProviderError> {
        if self.preflight {
            Err(self.error())
        } else {
            Ok(())
        }
    }
    fn complete(
        &mut self,
        _: &ModelProviderRequest,
    ) -> Result<ModelProviderResponse, ModelProviderError> {
        if let Some(allowed) = &self.allowed {
            allowed.store(false, Ordering::SeqCst);
        }
        Err(self.error())
    }
    fn check_execution(&self) -> Result<(), ModelProviderError> {
        if self
            .allowed
            .as_ref()
            .is_some_and(|a| !a.load(Ordering::SeqCst))
        {
            Err(ModelProviderError::new("stopped", "private-stop", false))
        } else {
            Ok(())
        }
    }
}
impl FailingProvider {
    fn error(&self) -> ModelProviderError {
        ModelProviderError::with_upstream_trace(
            self.code,
            "private-key-and-body",
            true,
            Some(json!({"secret":"private-trace"})),
        )
    }
}
#[test]
fn safe_model_failure_is_allowlisted_without_retry_or_private_text() {
    for (code, expected) in [
        ("input_limit", NativeTaskModelFailure::InputLimit),
        ("output_limit", NativeTaskModelFailure::OutputLimit),
        (
            "model_identity_mismatch",
            NativeTaskModelFailure::ModelIdentityMismatch,
        ),
        (
            "unexpected_streaming",
            NativeTaskModelFailure::UnexpectedStreaming,
        ),
        (
            "unsupported_finish_reason",
            NativeTaskModelFailure::UnsupportedFinishReason,
        ),
        ("unsupported_tool", NativeTaskModelFailure::UnsupportedTool),
        (
            "invalid_tool_identity",
            NativeTaskModelFailure::InvalidToolIdentity,
        ),
        (
            "unsupported_content",
            NativeTaskModelFailure::UnsupportedContent,
        ),
        (
            "capability_unsupported",
            NativeTaskModelFailure::CapabilityUnsupported,
        ),
        ("private-key-arbitrary-code", NativeTaskModelFailure::Other),
    ] {
        for preflight in [false, true] {
            let host = Arc::new(FakeHost::new());
            let r = NativeTaskRunner::new(
                host.clone(),
                Box::new(FailingProvider {
                    code,
                    preflight,
                    allowed: None,
                }),
                config(),
            )
            .unwrap()
            .run()
            .unwrap();
            assert_eq!(r.model_failure, Some(expected));
            assert_eq!(r.model_calls_spent, u32::from(!preflight));
            assert_eq!(r.turns_completed, 0);
            assert_eq!(
                r.stop_reason,
                if code == "input_limit" {
                    NativeTaskStopReason::InputLimit
                } else {
                    NativeTaskStopReason::ModelFailed
                }
            );
            assert!(!host.events().iter().any(|e| e.starts_with("tool:")));
            let serialized = serde_json::to_string(&r).unwrap();
            assert!(!serialized.contains("private-"));
        }
    }
}
#[test]
fn stop_discards_late_failure_classification() {
    let allowed = Arc::new(AtomicBool::new(true));
    let host = Arc::new(FakeHost::new());
    let r = NativeTaskRunner::new(
        host,
        Box::new(FailingProvider {
            code: "output_limit",
            preflight: false,
            allowed: Some(allowed),
        }),
        config(),
    )
    .unwrap()
    .run()
    .unwrap();
    assert_eq!(r.stop_reason, NativeTaskStopReason::ControlChanged);
    assert_eq!(r.model_failure, None);
}
#[test]
fn old_report_without_diagnostic_remains_readable() {
    let value = json!({"run_id":"old","model_calls_spent":1,"turns_completed":0,"last_operation":null,"stop_reason":"model_failed"});
    let r: NativeTaskRunReport = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(r.model_failure, None);
    assert_eq!(serde_json::to_value(r).unwrap(), value);
}

struct RecordingFailure;
impl ModelProvider for RecordingFailure {
    fn provider_id(&self) -> &str {
        "record-fixture"
    }
    fn complete(
        &mut self,
        r: &ModelProviderRequest,
    ) -> Result<ModelProviderResponse, ModelProviderError> {
        Ok(response(
            r,
            vec![ModelProviderTurn::ToolCall {
                name: "observe".into(),
                arguments: json!({}),
            }],
        ))
    }
    fn pending_tool_calls(&self) -> Vec<crate::ModelProviderToolCall> {
        vec![crate::ModelProviderToolCall {
            call_id: "supplier-one".into(),
            name: "observe".into(),
            arguments: json!({}),
        }]
    }
    fn record_tool_results(
        &mut self,
        _: &[crate::ModelProviderToolResult],
    ) -> Result<(), ModelProviderError> {
        Err(ModelProviderError::new(
            "invalid_tool_identity",
            "private-record-body",
            false,
        ))
    }
}
#[test]
fn supplier_recording_failure_preserves_safe_classification() {
    let host = Arc::new(FakeHost::new());
    let r = NativeTaskRunner::new(host.clone(), Box::new(RecordingFailure), config())
        .unwrap()
        .run()
        .unwrap();
    assert_eq!(
        r.model_failure,
        Some(NativeTaskModelFailure::InvalidToolIdentity)
    );
    assert_eq!(r.stop_reason, NativeTaskStopReason::ModelFailed);
    assert_eq!(r.model_calls_spent, 1);
    assert_eq!(r.turns_completed, 0);
    assert_eq!(
        host.events()
            .iter()
            .filter(|e| e.starts_with("tool:"))
            .count(),
        1
    );
}
