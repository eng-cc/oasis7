use super::*;
use crate::preset::{Protocol, ProviderId, ProviderPreset, RequestLimits};
fn preset(provider: ProviderId, protocol: Protocol) -> ProviderPreset {
    ProviderPreset {
        id: "deployment-model".into(),
        provider,
        protocol,
        model: "configured-model".into(),
        routes: if provider == ProviderId::OpenRouter {
            vec!["approved-route".into()]
        } else {
            vec![]
        },
        revision: 1,
        limits: RequestLimits::default(),
    }
}
fn conversation() -> Conversation {
    Conversation {
        system_prompt: "authorized current task only".into(),
        messages: vec![Message::User {
            content: "observe".into(),
        }],
        tools: vec![Tool {
            name: "observe".into(),
            description: "read only".into(),
            parameters: json!({"type":"object","properties":{},"additionalProperties":false}),
        }],
        max_output_tokens: 128,
    }
}
fn decode(p: &ProviderPreset, v: Value) -> Result<ProviderResponse, ProviderFailure> {
    decode_response(
        p,
        &serde_json::to_vec(&v).unwrap(),
        "application/json",
        &conversation().tools,
    )
}
#[test]
fn chat_tool_roundtrip_preserves_ids_reasoning_order_and_route_guards() {
    let p = preset(ProviderId::OpenRouter, Protocol::ChatCompletions);
    let mut c = conversation();
    let reply=decode(&p,json!({"model":"configured-model","choices":[{"finish_reason":"tool_calls","message":{"role":"assistant","content":null,"reasoning_content":"bounded reasoning","reasoning_details":[{"type":"reasoning.encrypted","data":"opaque"}],"tool_calls":[{"id":"call-first","type":"function","function":{"name":"observe","arguments":"{}"}}]}}]})).unwrap();
    assert_eq!(reply.tool_calls[0].id, "call-first");
    assert_eq!(reply.usage, Usage::default());
    c.messages.extend(reply.replay);
    c.messages.push(Message::ToolResult {
        call_id: "call-first".into(),
        content: "read result".into(),
    });
    let body: Value = serde_json::from_slice(&encode_request(&p, &c).unwrap()).unwrap();
    assert_eq!(body["messages"][0]["role"], "system");
    assert_eq!(body["messages"][1]["role"], "user");
    assert_eq!(body["messages"][2]["role"], "assistant");
    assert_eq!(body["messages"][2]["tool_calls"][0]["id"], "call-first");
    assert_eq!(body["messages"][3]["role"], "tool");
    assert_eq!(body["messages"][3]["tool_call_id"], "call-first");
    assert_eq!(
        body["messages"][2]["reasoning_details"][0]["data"],
        "opaque"
    );
    assert_eq!(
        body["provider"],
        json!({"only":["approved-route"],"allow_fallbacks":false,"require_parameters":true})
    );
    assert_eq!(body["stream"], false);
}
#[test]
fn responses_stateless_replay_preserves_reasoning_and_function_ids() {
    let p = preset(ProviderId::DeepSeek, Protocol::Responses);
    let mut c = conversation();
    let output = json!([{"type":"reasoning","id":"reasoning-id","encrypted_content":"opaque","summary":[]},{"type":"function_call","id":"item-id","call_id":"call-1","name":"observe","arguments":"{}"}]);
    let reply=decode(&p,json!({"model":"configured-model","status":"completed","output":output,"usage":{"input_tokens":12,"output_tokens":4,"total_tokens":16}})).unwrap();
    assert_eq!(reply.usage.total_tokens, Some(16));
    c.messages.extend(reply.replay);
    c.messages.push(Message::ToolResult {
        call_id: "call-1".into(),
        content: "result".into(),
    });
    let body: Value = serde_json::from_slice(&encode_request(&p, &c).unwrap()).unwrap();
    assert_eq!(body["store"], false);
    assert!(body.get("previous_response_id").is_none());
    assert_eq!(body["input"][1], output[0]);
    assert_eq!(body["input"][2], output[1]);
    assert_eq!(body["input"][3]["call_id"], "call-1");
}
#[test]
fn malformed_unsupported_truncated_or_missing_terminal_never_become_actions() {
    let p = preset(ProviderId::DeepSeek, Protocol::ChatCompletions);
    for value in [
        json!({"choices":[{"message":{"role":"assistant","content":"wait"}}]}),
        json!({"choices":[{"finish_reason":"length","message":{"role":"assistant","content":"partial"}}]}),
        json!({"choices":[{"finish_reason":"tool_calls","message":{"role":"assistant","tool_calls":[{"id":"c","type":"function","function":{"name":"unsupported","arguments":"{}"}}]}}]}),
        json!({"choices":[{"finish_reason":"tool_calls","message":{"role":"assistant","tool_calls":[{"id":"c","type":"function","function":{"name":"observe","arguments":"broken"}}]}}]}),
    ] {
        assert!(decode(&p, value).is_err());
    }
    assert!(matches!(
        decode_response(&p, b"data: [DONE]", "text/event-stream", &[]),
        Err(ProviderFailure::CapabilityUnsupported)
    ));
    let p = preset(ProviderId::DeepSeek, Protocol::Responses);
    assert!(decode(&p, json!({"status":"incomplete","output":[]})).is_err());
}
#[test]
fn missing_duplicate_or_cross_protocol_tool_results_are_rejected() {
    let p = preset(ProviderId::DeepSeek, Protocol::ChatCompletions);
    let mut c = conversation();
    c.messages.push(Message::ToolResult {
        call_id: "unknown".into(),
        content: "x".into(),
    });
    assert!(encode_request(&p, &c).is_err());
    let mut c = conversation();
    c.messages.push(Message::Assistant {
        content: None,
        tool_calls: vec![
            ToolCall {
                id: "same".into(),
                name: "observe".into(),
                arguments: json!({}),
            },
            ToolCall {
                id: "same".into(),
                name: "observe".into(),
                arguments: json!({}),
            },
        ],
        reasoning_content: None,
        reasoning_details: None,
    });
    assert!(encode_request(&p, &c).is_err());
}
#[test]
fn deployment_limits_and_unsupported_route_combinations_fail_closed() {
    let mut p = preset(ProviderId::OpenRouter, Protocol::ChatCompletions);
    p.routes.clear();
    assert!(p.validate().is_err());
    assert!(
        preset(ProviderId::OpenRouter, Protocol::Responses)
            .validate()
            .is_err()
    );
    let mut p = preset(ProviderId::DeepSeek, Protocol::ChatCompletions);
    p.limits.max_input_bytes = 1;
    assert!(matches!(
        encode_request(&p, &conversation()),
        Err(ProviderFailure::InputLimit)
    ));
    assert_eq!(p.host(), "api.deepseek.com");
    assert_eq!(p.path(), "/chat/completions");
}

#[test]
fn malformed_partial_or_inconsistent_usage_retains_unknown_total() {
    for (input, output) in [
        ("prompt_tokens", "completion_tokens"),
        ("input_tokens", "output_tokens"),
    ] {
        for (i, o, total) in [
            (json!(100), json!(100), json!(0)),
            (json!(-1), json!(2), json!(1)),
            (json!(null), json!(2), json!(2)),
            (json!(u64::MAX), json!(1), json!(0)),
            (json!(1), json!(1.5), json!(2)),
        ] {
            let mut value = json!({"total_tokens":total});
            value[input] = i;
            value[output] = o;
            assert_eq!(usage(&value, input, output).total_tokens, None);
        }
        let mut valid = json!({"total_tokens":5});
        valid[input] = json!(2);
        valid[output] = json!(3);
        assert_eq!(usage(&valid, input, output).total_tokens, Some(5));
    }
}
