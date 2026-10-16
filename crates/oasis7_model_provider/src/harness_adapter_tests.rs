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
        Limits::default(),
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
                limits: RequestLimits::default(),
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
