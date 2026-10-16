use super::*;
use crate::{
    connection::{BindingTarget, Clock, Limits},
    preset::{Protocol, ProviderId, ProviderPreset, RequestLimits},
    protocol::{ToolCall, Usage},
};
use std::sync::Arc;
use zeroize::Zeroizing;
struct TestClock;
impl Clock for TestClock {
    fn now_ms(&self) -> u64 {
        1000
    }
}
fn fixture() -> (ConnectionManager, Principal, String) {
    let p = Principal {
        account_id: "a".into(),
        session_id: "s".into(),
        auth_ref: "host-only".into(),
    };
    let principal = p.clone();
    let manager = ConnectionManager::new(
        Arc::new(move |v: &Principal| *v == principal),
        Arc::new(|_: &Principal, _: &BindingTarget| false),
        Arc::new(TestClock),
        Limits::default(),
    )
    .unwrap();
    let snapshot = manager
        .create(
            &p,
            ProviderPreset {
                id: "fixture".into(),
                provider: ProviderId::DeepSeek,
                protocol: Protocol::ChatCompletions,
                model: "fixture".into(),
                routes: vec![],
                revision: 1,
                limits: RequestLimits::default(),
            },
            Zeroizing::new(b"canary-only".to_vec()),
            true,
        )
        .unwrap();
    (manager, p, snapshot.id)
}
fn first() -> ProviderResponse {
    let call = ToolCall {
        id: "original-call-id".into(),
        name: "protocol_echo".into(),
        arguments: serde_json::json!({"value":"ready"}),
    };
    ProviderResponse {
        text: String::new(),
        tool_calls: vec![call.clone()],
        replay: vec![Message::Assistant {
            content: None,
            tool_calls: vec![call],
            reasoning_content: None,
            reasoning_details: None,
        }],
        usage: Usage::default(),
    }
}
#[test]
fn two_controlled_roundtrips_reserve_two_calls_and_repeat_id_never_pays_again() {
    let (manager, p, id) = fixture();
    let mut calls = 0;
    let result=run_with(&manager,&p,&id,1,"same-external-operation",|permit,conversation|{
  permit.before_send().unwrap();calls+=1;
  if calls==1 {assert_eq!(conversation.messages.len(),1);Ok(first())}else{
   assert!(matches!(&conversation.messages[2],Message::ToolResult{call_id,..} if call_id=="original-call-id"));
   Ok(ProviderResponse{text:"ready".into(),tool_calls:vec![],replay:vec![],usage:Usage::default()})
  }
 }).unwrap();
    assert_eq!(result, ProbeState::Ready);
    assert_eq!(calls, 2);
    assert_eq!(manager.snapshot(&p, &id).unwrap().budget.calls, 2);
    assert_eq!(manager.snapshot(&p, &id).unwrap().budget.in_flight, 0);
    assert_eq!(
        run_with(
            &manager,
            &p,
            &id,
            1,
            "same-external-operation",
            |_, _| panic!("duplicate paid probe")
        )
        .unwrap(),
        ProbeState::Ready
    );
}
#[test]
fn unknown_or_failed_probe_and_existing_pending_never_resubmit() {
    let (manager, p, id) = fixture();
    let mut calls = 0;
    assert!(
        run_with(&manager, &p, &id, 1, "unknown", |permit, _| {
            permit.before_send().unwrap();
            calls += 1;
            Err(ProviderFailure::TransportFailed)
        })
        .is_err()
    );
    assert_eq!(calls, 1);
    assert_eq!(
        run_with(&manager, &p, &id, 1, "unknown", |_, _| panic!(
            "unknown replay"
        ))
        .unwrap(),
        ProbeState::Failed
    );
    let (manager, p, id) = fixture();
    let _ = manager.begin_probe(&p, &id, 1, "pending").unwrap();
    assert_eq!(
        run_with(&manager, &p, &id, 1, "pending", |_, _| panic!(
            "pending replay"
        ))
        .unwrap(),
        ProbeState::Pending
    );
}
