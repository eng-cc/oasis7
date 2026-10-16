use super::*;
use crate::{
    connection::{BindingTarget, Limits, MonotonicClock, ProbeStart},
    preset::{Protocol, ProviderId, ProviderPreset, RequestLimits},
    protocol::{ProviderResponse, Usage},
};
use serde_json::json;
use std::sync::Arc;
use zeroize::Zeroizing;
fn fixture() -> (SessionModelProvider, ModelProviderRequest) {
    fixture_limits(Limits::default(), RequestLimits::default())
}
fn fixture_limits(
    limits: Limits,
    request_limits: RequestLimits,
) -> (SessionModelProvider, ModelProviderRequest) {
    let principal = Principal {
        account_id: "account".into(),
        session_id: "session".into(),
        auth_ref: "opaque".into(),
    };
    let expected = principal.clone();
    let manager = ConnectionManager::new(
        Arc::new(move |p: &Principal| *p == expected),
        Arc::new(|_: &Principal, _: &BindingTarget| true),
        Arc::new(MonotonicClock::default()),
        limits,
    )
    .unwrap();
    let initial = manager
        .create(
            &principal,
            ProviderPreset {
                id: "test".into(),
                provider: ProviderId::DeepSeek,
                protocol: Protocol::ChatCompletions,
                model: "fixture-model".into(),
                routes: vec![],
                revision: 1,
                limits: request_limits,
            },
            Zeroizing::new(b"fake-key-not-sent".to_vec()),
            true,
        )
        .unwrap();
    let ProbeStart::Operation(op) = manager
        .begin_probe(&principal, &initial.id, initial.version, "fixture-probe")
        .unwrap()
    else {
        panic!("new probe")
    };
    for phase in [0, 1] {
        let p = op.reserve_phase(phase, 100).unwrap();
        p.before_send().unwrap();
        p.finish(Some(10), true).unwrap();
    }
    let ready = manager.snapshot(&principal, &initial.id).unwrap();
    let active = manager
        .activate(
            &principal,
            &initial.id,
            ready.version,
            BindingTarget {
                full_fence_digest: "fixture-full-fence".into(),
                world_id: "world".into(),
                agent_id: "agent".into(),
                task_id: "task".into(),
                goal_revision: 1,
                executor_id: "executor".into(),
                executor_epoch: 1,
            },
        )
        .unwrap();
    let adapter = SessionModelProvider::new(
        manager,
        principal,
        active.id,
        active.version,
        active.generation,
    )
    .unwrap();
    let request:ModelProviderRequest=serde_json::from_value(json!({"request_digest":"blake3:0000000000000000000000000000000000000000000000000000000000000000","model":"fixture-model","system_prompt":"authorized context only","user_prompt":"inspect current task","tools":[{"name":"inspect","parameters":{"type":"object"}}],"budget":{"max_latency_ms":1000,"max_repair_attempts":0,"max_model_calls":3,"max_tool_calls":1}})).unwrap();
    (adapter, request)
}
fn response() -> ProviderResponse {
    let call = ToolCall {
        id: "supplier-call-1".into(),
        name: "inspect".into(),
        arguments: json!({}),
    };
    ProviderResponse {
        text: String::new(),
        tool_calls: vec![call.clone()],
        replay: vec![Message::Assistant {
            content: None,
            tool_calls: vec![call],
            reasoning_content: Some("required private history".into()),
            reasoning_details: None,
        }],
        usage: Usage {
            input_tokens: Some(10),
            output_tokens: Some(2),
            total_tokens: Some(12),
        },
    }
}
#[test]
fn typed_results_preserve_supplier_identity_and_history_without_prompt_parsing() {
    let (mut adapter, request) = fixture();
    let response = adapter
        .complete_using(&request, |permit, _| {
            permit.before_send().unwrap();
            Ok(response())
        })
        .unwrap();
    assert_eq!(response.turns.len(), 1);
    assert_eq!(adapter.pending_tool_calls()[0].call_id, "supplier-call-1");
    assert_eq!(
        adapter
            .complete_using(&request, |_, _| panic!("must not send without result"))
            .unwrap_err()
            .code(),
        "tool_results_required"
    );
    assert!(
        adapter
            .record_tool_results(&[ModelProviderToolResult {
                call_id: "world-operation-is-not-tool-id".into(),
                content: json!({"ok":true})
            }])
            .is_err()
    );
    adapter
        .record_tool_results(&[ModelProviderToolResult {
            call_id: "supplier-call-1".into(),
            content: json!({"observation":"actual result"}),
        }])
        .unwrap();
    adapter.complete_using(&request,|permit,history|{
        assert!(matches!(&history.messages[1],Message::Assistant{reasoning_content:Some(text),..} if text=="required private history"));
        assert!(matches!(&history.messages[2],Message::ToolResult{call_id,..} if call_id=="supplier-call-1"));
        permit.before_send().unwrap();Ok(super::tests::response())
    }).unwrap();
}
#[test]
fn stop_during_model_call_discards_late_output_and_keeps_its_budget() {
    let (mut adapter, request) = fixture();
    let manager = adapter.manager.clone();
    let principal = adapter.principal.clone();
    let id = adapter.connection_id.clone();
    let version = adapter.version;
    let err = adapter
        .complete_using(&request, |permit, _| {
            permit.before_send().unwrap();
            manager.stop(&principal, &id, version).unwrap();
            Ok(response())
        })
        .unwrap_err();
    assert_eq!(err.code(), "model_result_expired");
    assert!(adapter.pending_tool_calls().is_empty());
    assert_eq!(manager.snapshot(&principal, &id).unwrap().budget.calls, 3);
}

#[test]
fn caller_zero_budgets_block_before_reservation_or_transport() {
    for field in ["calls", "latency", "tools"] {
        let (mut adapter, mut request) = fixture();
        match field {
            "calls" => request.budget.max_model_calls = 0,
            "latency" => request.budget.max_latency_ms = 0,
            _ => request.budget.max_tool_calls = 0,
        }
        assert_eq!(
            adapter
                .complete_using(&request, |_, _| panic!("zero budget must not send"))
                .unwrap_err()
                .code(),
            "budget_exhausted"
        );
        assert_eq!(
            adapter
                .manager
                .snapshot(&adapter.principal, &adapter.connection_id)
                .unwrap()
                .budget
                .calls,
            2
        );
    }
}
#[test]
fn tool_count_over_caller_limit_is_not_returned_for_execution() {
    let (mut adapter, request) = fixture();
    let failure = adapter
        .complete_using(&request, |permit, _| {
            permit.before_send().unwrap();
            let mut reply = response();
            let mut extra = reply.tool_calls[0].clone();
            extra.id = "second".into();
            reply.tool_calls.push(extra);
            Ok(reply)
        })
        .unwrap_err();
    assert_eq!(failure.code(), "tool_budget_exhausted");
    assert!(adapter.pending_tool_calls().is_empty());
}

#[test]
fn local_input_rejection_preserves_budget_and_history() {
    let (mut adapter, mut request) = fixture();
    request.user_prompt = "x".repeat(300_000);
    let before = adapter
        .manager
        .snapshot(&adapter.principal, &adapter.connection_id)
        .unwrap()
        .budget;
    assert_eq!(
        adapter
            .complete_using(&request, |_, _| panic!("local rejection must not send"))
            .unwrap_err()
            .code(),
        "input_limit"
    );
    let after = adapter
        .manager
        .snapshot(&adapter.principal, &adapter.connection_id)
        .unwrap()
        .budget;
    assert_eq!(before.calls, after.calls);
    assert_eq!(before.tokens, after.tokens);
    assert!(adapter.conversation.is_none());
}

fn budget(adapter: &SessionModelProvider) -> crate::connection::BudgetSnapshot {
    adapter
        .manager
        .snapshot(&adapter.principal, &adapter.connection_id)
        .unwrap()
        .budget
}
#[test]
fn full_history_encoding_boundary_is_exact_and_rejection_is_repeatable() {
    let (adapter, request) = fixture();
    let candidate = adapter.candidate(&request).unwrap();
    let preset = adapter
        .manager
        .active_preset(
            &adapter.principal,
            &adapter.connection_id,
            adapter.version,
            adapter.generation,
        )
        .unwrap();
    let size = crate::protocol::encode_request(&preset, &candidate)
        .unwrap()
        .len();
    for oversize in [false, true] {
        let (mut adapter, request) = fixture_limits(
            Limits::default(),
            RequestLimits {
                max_input_bytes: size - usize::from(oversize),
                ..RequestLimits::default()
            },
        );
        if !oversize {
            adapter.preflight(&request).unwrap();
            adapter
                .complete_using(&request, |permit, conversation| {
                    assert_eq!(
                        crate::protocol::encode_request(permit.preset(), conversation)
                            .unwrap()
                            .len(),
                        size
                    );
                    permit.before_send().unwrap();
                    Ok(response())
                })
                .unwrap();
            assert_eq!(budget(&adapter).calls, 3);
        } else {
            let before = budget(&adapter);
            for _ in 0..2 {
                assert_eq!(
                    adapter.preflight(&request).unwrap_err().code(),
                    "input_limit"
                );
                assert_eq!(
                    adapter
                        .complete_using(&request, |_, _| panic!("must not send"))
                        .unwrap_err()
                        .code(),
                    "input_limit"
                );
            }
            assert_eq!(budget(&adapter).calls, before.calls);
            assert_eq!(budget(&adapter).tokens, before.tokens);
            assert!(adapter.conversation.is_none());
        }
    }
}
#[test]
fn conservative_token_boundary_is_checked_before_reservation() {
    let (adapter, request) = fixture();
    let candidate = adapter.candidate(&request).unwrap();
    let preset = adapter
        .manager
        .active_preset(
            &adapter.principal,
            &adapter.connection_id,
            adapter.version,
            adapter.generation,
        )
        .unwrap();
    let size = crate::protocol::encode_request(&preset, &candidate)
        .unwrap()
        .len() as u64;
    for excess in [0, 1] {
        let (mut adapter, request) = fixture_limits(
            Limits {
                max_input_tokens: size + 256 - excess,
                ..Limits::default()
            },
            RequestLimits::default(),
        );
        if excess == 0 {
            adapter.preflight(&request).unwrap();
        } else {
            assert_eq!(
                adapter
                    .complete_using(&request, |_, _| panic!("must not send"))
                    .unwrap_err()
                    .code(),
                "input_limit"
            );
            assert_eq!(budget(&adapter).calls, 2);
        }
    }
}
#[test]
fn retained_reasoning_and_results_count_toward_input_and_are_not_trimmed() {
    let (mut adapter, request) = fixture();
    adapter
        .complete_using(&request, |permit, _| {
            permit.before_send().unwrap();
            Ok(response())
        })
        .unwrap();
    adapter
        .record_tool_results(&[ModelProviderToolResult {
            call_id: "supplier-call-1".into(),
            content: json!({"result_sentinel":"unique"}),
        }])
        .unwrap();
    let before = adapter.conversation.clone().unwrap();
    let mut huge = request.clone();
    huge.user_prompt = "x".repeat(300_000);
    let old_budget = budget(&adapter);
    assert_eq!(
        adapter
            .complete_using(&huge, |_, _| panic!("must not send"))
            .unwrap_err()
            .code(),
        "input_limit"
    );
    assert_eq!(budget(&adapter).calls, old_budget.calls);
    assert!(adapter.conversation.as_ref().unwrap().messages == before.messages);
    adapter
        .complete_using(&request, |permit, history| {
            let encoded = String::from_utf8(
                crate::protocol::encode_request(permit.preset(), history).unwrap(),
            )
            .unwrap();
            assert_eq!(encoded.matches("result_sentinel").count(), 1);
            assert_eq!(encoded.matches("required private history").count(), 1);
            permit.before_send().unwrap();
            Ok(response())
        })
        .unwrap();
}
#[test]
fn preflight_does_not_authorize_after_stop_and_unknown_keeps_reservation() {
    let (mut adapter, request) = fixture();
    adapter.preflight(&request).unwrap();
    adapter
        .manager
        .stop(&adapter.principal, &adapter.connection_id, adapter.version)
        .unwrap();
    assert_eq!(
        adapter
            .complete_using(&request, |_, _| panic!("stopped must not send"))
            .unwrap_err()
            .code(),
        "model_session_unavailable"
    );
    assert_eq!(budget(&adapter).calls, 2);
    let (mut adapter, request) = fixture();
    let before = budget(&adapter);
    adapter
        .complete_using(&request, |permit, _| {
            permit.before_send().unwrap();
            Err(crate::protocol::ProviderFailure::TransportFailed)
        })
        .unwrap_err();
    let after = budget(&adapter);
    assert_eq!(after.calls, before.calls + 1);
    assert_eq!(
        after.tokens,
        before.tokens
            + adapter.manager.limits().max_input_tokens
            + adapter.manager.limits().max_output_tokens
    );
}

#[test]
fn identity_failure_is_local_and_stop_between_reserve_and_send_stays_blocked() {
    let (mut adapter, mut request) = fixture();
    request.model = "another-model".into();
    assert_eq!(
        adapter
            .complete_using(&request, |_, _| panic!("must not send"))
            .unwrap_err()
            .code(),
        "model_identity_mismatch"
    );
    assert_eq!(budget(&adapter).calls, 2);
    assert!(adapter.conversation.is_none());
    let (mut adapter, request) = fixture();
    let manager = adapter.manager.clone();
    let principal = adapter.principal.clone();
    let id = adapter.connection_id.clone();
    let version = adapter.version;
    assert_eq!(
        adapter
            .complete_using(&request, |permit, _| {
                manager.stop(&principal, &id, version).unwrap();
                assert!(permit.before_send().is_err());
                Err(crate::protocol::ProviderFailure::SessionExpired)
            })
            .unwrap_err()
            .code(),
        "session_expired"
    );
    assert_eq!(budget(&adapter).calls, 3);
}

#[path = "harness_history_tests.rs"]
mod history;
