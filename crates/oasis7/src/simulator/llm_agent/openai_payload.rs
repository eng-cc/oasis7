use super::*;
use async_openai::types::responses::{
    InputContent, InputItem, InputMessage, InputParam, InputRole, InputTextContent, Item,
    MessageItem, ResponseStreamEvent,
};
use std::collections::BTreeMap;

const AGENT_SUBMIT_DECISION_SCHEMA_JSON: &str = r#"{
  "type": "object",
  "properties": {
    "decision": {
      "type": "string",
      "enum": [
        "wait",
        "wait_ticks",
        "move_agent",
        "harvest_radiation",
        "buy_power",
        "sell_power",
        "place_power_order",
        "cancel_power_order",
        "transfer_resource",
        "mine_compound",
        "refine_compound",
        "build_factory",
        "schedule_recipe",
        "compile_module_artifact_from_source",
        "deploy_module_artifact",
        "install_module_from_artifact",
        "install_module_to_target_from_artifact",
        "list_module_artifact_for_sale",
        "buy_module_artifact",
        "delist_module_artifact",
        "destroy_module_artifact",
        "place_module_artifact_bid",
        "cancel_module_artifact_bid",
        "publish_social_fact",
        "challenge_social_fact",
        "adjudicate_social_fact",
        "revoke_social_fact",
        "declare_social_edge",
        "form_alliance",
        "join_alliance",
        "leave_alliance",
        "dissolve_alliance",
        "declare_war",
        "open_governance_proposal",
        "cast_governance_vote",
        "resolve_crisis",
        "grant_meta_progress",
        "open_economic_contract",
        "accept_economic_contract",
        "settle_economic_contract",
        "execute_until"
      ]
    },
    "ticks": { "type": "integer", "minimum": 1 },
    "to": { "type": "string" },
    "from": { "type": "string" },
    "max_amount": { "type": "integer", "minimum": 1 },
    "from_owner": { "type": "string" },
    "to_owner": { "type": "string" },
    "kind": { "type": "string" },
    "amount": { "type": "integer", "minimum": 1 },
    "buyer": { "type": "string" },
    "seller": { "type": "string" },
    "price_per_pu": { "type": "integer", "minimum": 0 },
    "side": { "type": "string", "enum": ["buy", "sell"] },
    "limit_price_per_pu": { "type": "integer", "minimum": 0 },
    "order_id": { "type": "integer", "minimum": 1 },
    "owner": { "type": "string" },
    "compound_mass_g": { "type": "integer", "minimum": 1 },
    "location_id": { "type": "string" },
    "factory_id": { "type": "string" },
    "factory_kind": { "type": "string" },
    "recipe_id": { "type": "string" },
    "batches": { "type": "integer", "minimum": 1 },
    "publisher": { "type": "string" },
    "installer": { "type": "string" },
    "bidder": { "type": "string" },
    "module_id": { "type": "string" },
    "module_version": { "type": "string" },
    "install_target_type": {
      "type": "string",
      "enum": ["self_agent", "location_infrastructure"]
    },
    "install_target_location_id": { "type": "string" },
    "manifest_path": { "type": "string" },
    "source_files": {
      "type": "object",
      "additionalProperties": { "type": "string" }
    },
    "wasm_hash": { "type": "string" },
    "wasm_bytes_hex": { "type": "string" },
    "price_kind": { "type": "string" },
    "price_amount": { "type": "integer", "minimum": 1 },
    "bid_order_id": { "type": "integer", "minimum": 1 },
    "activate": { "type": "boolean" },
    "actor": { "type": "string" },
    "declarer": { "type": "string" },
    "subject": { "type": "string" },
    "object": { "type": "string" },
    "schema_id": { "type": "string" },
    "claim": { "type": "string" },
    "confidence_ppm": { "type": "integer", "minimum": 1, "maximum": 1000000 },
    "evidence_event_ids": {
      "type": "array",
      "items": { "type": "integer", "minimum": 1 }
    },
    "ttl_ticks": { "type": "integer", "minimum": 1 },
    "stake": {
      "type": "object",
      "properties": {
        "kind": { "type": "string" },
        "amount": { "type": "integer", "minimum": 1 }
      },
      "additionalProperties": false
    },
    "challenger": { "type": "string" },
    "fact_id": { "type": "integer", "minimum": 1 },
    "reason": { "type": "string" },
    "adjudicator": { "type": "string" },
    "adjudication": { "type": "string", "enum": ["confirm", "retract"] },
    "notes": { "type": "string" },
    "relation_kind": { "type": "string" },
    "weight_bps": { "type": "integer", "minimum": -10000, "maximum": 10000 },
    "backing_fact_ids": {
      "type": "array",
      "items": { "type": "integer", "minimum": 1 }
    },
    "proposer_agent_id": { "type": "string" },
    "alliance_id": { "type": "string" },
    "members": {
      "type": "array",
      "items": { "type": "string" }
    },
    "member_agent_id": { "type": "string" },
    "charter": { "type": "string" },
    "initiator_agent_id": { "type": "string" },
    "war_id": { "type": "string" },
    "aggressor_alliance_id": { "type": "string" },
    "defender_alliance_id": { "type": "string" },
    "objective": { "type": "string" },
    "intensity": { "type": "integer", "minimum": 1, "maximum": 10 },
    "proposal_key": { "type": "string" },
    "title": { "type": "string" },
    "description": { "type": "string" },
    "options": {
      "type": "array",
      "items": { "type": "string" }
    },
    "voting_window_ticks": { "type": "integer", "minimum": 1, "maximum": 1440 },
    "quorum_weight": { "type": "integer", "minimum": 1 },
    "pass_threshold_bps": { "type": "integer", "minimum": 5000, "maximum": 10000 },
    "voter_agent_id": { "type": "string" },
    "option": { "type": "string" },
    "weight": { "type": "integer", "minimum": 1 },
    "resolver_agent_id": { "type": "string" },
    "crisis_id": { "type": "string" },
    "strategy": { "type": "string" },
    "success": { "type": "boolean" },
    "operator_agent_id": { "type": "string" },
    "target_agent_id": { "type": "string" },
    "track": { "type": "string" },
    "points": { "type": "integer" },
    "achievement_id": { "type": "string" },
    "creator_agent_id": { "type": "string" },
    "contract_id": { "type": "string" },
    "counterparty_agent_id": { "type": "string" },
    "settlement_kind": { "type": "string" },
    "settlement_amount": { "type": "integer", "minimum": 1 },
    "reputation_stake": { "type": "integer", "minimum": 1 },
    "expires_at": { "type": "integer", "minimum": 1 },
    "accepter_agent_id": { "type": "string" },
    "action": {
      "type": "object",
      "additionalProperties": true
    },
    "until": {
      "type": "object",
      "properties": {
        "event": { "type": "string" },
        "event_any_of": {
          "type": "array",
          "items": { "type": "string" }
        },
        "value_lte": { "type": "integer", "minimum": 0 }
      },
      "additionalProperties": false
    },
    "max_ticks": { "type": "integer", "minimum": 1 },
    "message_to_user": { "type": "string" }
  },
  "required": ["decision"],
  "additionalProperties": false
}"#;

fn decision_tool_parameters() -> serde_json::Value {
    serde_json::from_str(AGENT_SUBMIT_DECISION_SCHEMA_JSON)
        .expect("agent_submit_decision schema JSON should be valid")
}

#[cfg(test)]
pub(super) fn responses_tools() -> Vec<Tool> {
    responses_tools_with_debug_mode(false)
}

pub(super) fn responses_tools_with_debug_mode(debug_mode: bool) -> Vec<Tool> {
    let mut tools = vec![
        Tool::Function(FunctionTool {
            name: OPENAI_TOOL_AGENT_MODULES_LIST.to_string(),
            description: Some("列出 Agent 可调用的模块能力与参数。".to_string()),
            parameters: Some(serde_json::json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false,
            })),
            strict: None,
        }),
        Tool::Function(FunctionTool {
            name: OPENAI_TOOL_ENVIRONMENT_CURRENT_OBSERVATION.to_string(),
            description: Some("读取当前 tick 的环境观测。".to_string()),
            parameters: Some(serde_json::json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false,
            })),
            strict: None,
        }),
        Tool::Function(FunctionTool {
            name: OPENAI_TOOL_MEMORY_SHORT_TERM_RECENT.to_string(),
            description: Some("读取最近短期记忆。".to_string()),
            parameters: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "limit": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 32
                    }
                },
                "additionalProperties": false,
            })),
            strict: None,
        }),
        Tool::Function(FunctionTool {
            name: OPENAI_TOOL_MEMORY_LONG_TERM_SEARCH.to_string(),
            description: Some("按关键词检索长期记忆（query 为空时按重要度返回）。".to_string()),
            parameters: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string"
                    },
                    "limit": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 32
                    }
                },
                "additionalProperties": false,
            })),
            strict: None,
        }),
        Tool::Function(FunctionTool {
            name: OPENAI_TOOL_WORLD_RULES_GUIDE.to_string(),
            description: Some("读取世界玩法规则与阶段推进建议。".to_string()),
            parameters: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "topic": {
                        "type": "string",
                        "enum": [
                            "quickstart",
                            "resources",
                            "industry",
                            "governance",
                            "economic",
                            "social",
                            "recovery",
                            "all"
                        ]
                    }
                },
                "additionalProperties": false,
            })),
            strict: None,
        }),
        Tool::Function(FunctionTool {
            name: OPENAI_TOOL_MODULE_LIFECYCLE_STATUS.to_string(),
            description: Some("读取模块生命周期状态（artifact 与 installed）。".to_string()),
            parameters: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "module_id": {
                        "type": "string"
                    },
                    "limit_artifacts": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 256
                    },
                    "limit_installed": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 256
                    }
                },
                "additionalProperties": false,
            })),
            strict: None,
        }),
        Tool::Function(FunctionTool {
            name: OPENAI_TOOL_POWER_ORDER_BOOK_STATUS.to_string(),
            description: Some("读取电力订单簿快照。".to_string()),
            parameters: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "limit_orders": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 256
                    }
                },
                "additionalProperties": false,
            })),
            strict: None,
        }),
        Tool::Function(FunctionTool {
            name: OPENAI_TOOL_MODULE_MARKET_STATUS.to_string(),
            description: Some("读取模块市场挂牌/竞价状态。".to_string()),
            parameters: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "wasm_hash": { "type": "string" },
                    "limit_listings": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 256
                    },
                    "limit_bids": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 256
                    }
                },
                "additionalProperties": false,
            })),
            strict: None,
        }),
        Tool::Function(FunctionTool {
            name: OPENAI_TOOL_SOCIAL_STATE_STATUS.to_string(),
            description: Some("读取社会事实与关系边状态。".to_string()),
            parameters: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "include_inactive": { "type": "boolean" },
                    "limit_facts": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 256
                    },
                    "limit_edges": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 256
                    }
                },
                "additionalProperties": false,
            })),
            strict: None,
        }),
        Tool::Function(FunctionTool {
            name: OPENAI_TOOL_AGENT_SUBMIT_DECISION.to_string(),
            description: Some(
                "提交最终决策；所有世界动作必须通过该 tool call，而不是输出文本 JSON。".to_string(),
            ),
            parameters: Some(decision_tool_parameters()),
            strict: None,
        }),
    ];

    if debug_mode {
        tools.push(Tool::Function(FunctionTool {
            name: OPENAI_TOOL_AGENT_DEBUG_GRANT_RESOURCE.to_string(),
            description: Some(
                "仅 debug 模式可用：向 owner 追加任意资源数量用于调试闭环。".to_string(),
            ),
            parameters: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "owner": { "type": "string" },
                    "kind": { "type": "string" },
                    "amount": { "type": "integer", "minimum": 1 }
                },
                "required": ["kind", "amount"],
                "additionalProperties": false
            })),
            strict: None,
        }));
    }

    tools
}

pub(super) fn module_name_from_tool_name(name: &str) -> &str {
    match name {
        OPENAI_TOOL_AGENT_MODULES_LIST => "agent.modules.list",
        OPENAI_TOOL_ENVIRONMENT_CURRENT_OBSERVATION => "environment.current_observation",
        OPENAI_TOOL_MEMORY_SHORT_TERM_RECENT => "memory.short_term.recent",
        OPENAI_TOOL_MEMORY_LONG_TERM_SEARCH => "memory.long_term.search",
        OPENAI_TOOL_WORLD_RULES_GUIDE => "world.rules.guide",
        OPENAI_TOOL_MODULE_LIFECYCLE_STATUS => "module.lifecycle.status",
        OPENAI_TOOL_POWER_ORDER_BOOK_STATUS => "power.order_book.status",
        OPENAI_TOOL_MODULE_MARKET_STATUS => "module.market.status",
        OPENAI_TOOL_SOCIAL_STATE_STATUS => "social.state.status",
        other => other,
    }
}

pub(super) fn decode_tool_arguments(arguments: &str) -> serde_json::Value {
    let trimmed = arguments.trim();
    if trimmed.is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str(trimmed).unwrap_or_else(|_| {
            serde_json::json!({
                "_raw": trimmed,
            })
        })
    }
}

pub(super) fn function_call_to_completion_turn(name: &str, arguments: &str) -> LlmCompletionTurn {
    if name == OPENAI_TOOL_AGENT_SUBMIT_DECISION {
        return LlmCompletionTurn::Decision {
            payload: decode_tool_arguments(arguments),
        };
    }
    if name == OPENAI_TOOL_AGENT_DEBUG_GRANT_RESOURCE {
        let mut payload = decode_tool_arguments(arguments);
        if let Some(map) = payload.as_object_mut() {
            map.insert(
                "decision".to_string(),
                serde_json::Value::String("debug_grant_resource".to_string()),
            );
        } else {
            payload = serde_json::json!({
                "decision": "debug_grant_resource",
                "raw_args": payload,
            });
        }
        return LlmCompletionTurn::Decision { payload };
    }
    LlmCompletionTurn::ModuleCall {
        module: module_name_from_tool_name(name).to_string(),
        args: decode_tool_arguments(arguments),
    }
}

pub(super) fn output_item_to_completion_turn(item: &OutputItem) -> Option<LlmCompletionTurn> {
    match item {
        OutputItem::FunctionCall(function_call) => Some(function_call_to_completion_turn(
            function_call.name.as_str(),
            function_call.arguments.as_str(),
        )),
        _ => None,
    }
}

pub(super) fn completion_turn_to_trace_json(turn: &LlmCompletionTurn) -> String {
    match turn {
        LlmCompletionTurn::Decision { payload } => payload.to_string(),
        LlmCompletionTurn::ModuleCall { module, args } => serde_json::json!({
            "type": "module_call",
            "module": module,
            "args": args,
        })
        .to_string(),
    }
}

pub(super) fn completion_result_from_sdk_response(
    response: Response,
) -> Result<LlmCompletionResult, LlmClientError> {
    let turns = response
        .output
        .iter()
        .filter_map(output_item_to_completion_turn)
        .collect::<Vec<_>>();
    let output = turns
        .iter()
        .map(completion_turn_to_trace_json)
        .collect::<Vec<_>>()
        .join("\n");

    if output.trim().is_empty() {
        return Err(LlmClientError::EmptyChoice);
    }

    let usage = response.usage.as_ref();
    Ok(LlmCompletionResult {
        turns,
        output,
        model: Some(response.model),
        prompt_tokens: usage.map(|usage| usage.input_tokens as u64),
        completion_tokens: usage.map(|usage| usage.output_tokens as u64),
        total_tokens: usage.map(|usage| usage.total_tokens as u64),
    })
}

fn summarize_response_error(value: Option<&impl Serialize>) -> Option<String> {
    value.and_then(|value| serde_json::to_string(value).ok())
}

fn stream_terminal_response_error(status: &str, response: &Response) -> LlmClientError {
    let detail = summarize_response_error(response.error.as_ref())
        .or_else(|| summarize_response_error(response.incomplete_details.as_ref()))
        .unwrap_or_else(|| "no additional detail".to_string());
    LlmClientError::Http {
        message: format!(
            "responses stream terminated with status {status} for {}: {}",
            response.id, detail
        ),
    }
}

pub(super) fn completion_result_from_sdk_stream_events<I>(
    events: I,
) -> Result<LlmCompletionResult, LlmClientError>
where
    I: IntoIterator<Item = ResponseStreamEvent>,
{
    let mut completed_response = None;
    let mut completed_output_items = BTreeMap::<u32, OutputItem>::new();

    for event in events {
        match event {
            ResponseStreamEvent::ResponseOutputItemDone(event) => {
                completed_output_items.insert(event.output_index, event.item);
            }
            ResponseStreamEvent::ResponseCompleted(event) => {
                completed_response = Some(event.response);
            }
            ResponseStreamEvent::ResponseFailed(event) => {
                return Err(stream_terminal_response_error("failed", &event.response));
            }
            ResponseStreamEvent::ResponseIncomplete(event) => {
                return Err(stream_terminal_response_error(
                    "incomplete",
                    &event.response,
                ));
            }
            ResponseStreamEvent::ResponseError(event) => {
                let code = event
                    .code
                    .as_deref()
                    .map(|code| format!(" ({code})"))
                    .unwrap_or_default();
                return Err(LlmClientError::Http {
                    message: format!("responses stream error{code}: {}", event.message),
                });
            }
            _ => {}
        }
    }

    let mut response = completed_response.ok_or_else(|| LlmClientError::DecodeResponse {
        message: "responses stream ended without response.completed".to_string(),
    })?;
    if response.output.is_empty() && !completed_output_items.is_empty() {
        response.output = completed_output_items.into_values().collect();
    }
    completion_result_from_sdk_response(response)
}

pub(super) fn text_output_from_sdk_stream_events<I>(events: I) -> Result<String, LlmClientError>
where
    I: IntoIterator<Item = ResponseStreamEvent>,
{
    let mut completed_response = None;
    let mut completed_text_parts = BTreeMap::<(u32, u32), String>::new();

    for event in events {
        match event {
            ResponseStreamEvent::ResponseCompleted(event) => {
                completed_response = Some(event.response);
            }
            ResponseStreamEvent::ResponseOutputTextDone(event) => {
                completed_text_parts.insert((event.output_index, event.content_index), event.text);
            }
            ResponseStreamEvent::ResponseFailed(event) => {
                return Err(stream_terminal_response_error("failed", &event.response));
            }
            ResponseStreamEvent::ResponseIncomplete(event) => {
                return Err(stream_terminal_response_error(
                    "incomplete",
                    &event.response,
                ));
            }
            ResponseStreamEvent::ResponseError(event) => {
                let code = event
                    .code
                    .as_deref()
                    .map(|code| format!(" ({code})"))
                    .unwrap_or_default();
                return Err(LlmClientError::Http {
                    message: format!("responses stream error{code}: {}", event.message),
                });
            }
            _ => {}
        }
    }

    let response = completed_response.ok_or_else(|| LlmClientError::DecodeResponse {
        message: "responses stream ended without response.completed".to_string(),
    })?;
    if let Some(text) = response
        .output_text()
        .filter(|text| !text.trim().is_empty())
    {
        return Ok(text);
    }

    let text = completed_text_parts.into_values().collect::<String>();
    if text.trim().is_empty() {
        Err(LlmClientError::EmptyChoice)
    } else {
        Ok(text)
    }
}

pub(super) fn normalize_openai_api_base_url(base_url: &str) -> String {
    let normalized = base_url.trim().trim_end_matches('/');
    if let Some(stripped) = normalized.strip_suffix("/chat/completions") {
        stripped.to_string()
    } else if let Some(stripped) = normalized.strip_suffix("/responses") {
        stripped.to_string()
    } else {
        normalized.to_string()
    }
}

pub(super) fn build_responses_request_payload(
    request: &LlmCompletionRequest,
) -> Result<CreateResponse, LlmClientError> {
    CreateResponseArgs::default()
        .model(request.model.clone())
        .instructions(request.system_prompt.clone())
        .input(InputParam::Items(vec![InputItem::Item(Item::Message(
            MessageItem::Input(InputMessage {
                content: vec![InputContent::InputText(InputTextContent {
                    text: request.user_prompt.clone(),
                })],
                role: InputRole::User,
                status: None,
            }),
        ))]))
        .tools(responses_tools_with_debug_mode(request.debug_mode))
        .tool_choice(ToolChoiceParam::Mode(ToolChoiceOptions::Required))
        .parallel_tool_calls(false)
        .build()
        .map_err(|err| LlmClientError::DecodeResponse {
            message: err.to_string(),
        })
}

pub(super) fn build_text_probe_request_payload(
    model: &str,
    user_prompt: &str,
) -> Result<CreateResponse, LlmClientError> {
    CreateResponseArgs::default()
        .model(model.to_string())
        .input(InputParam::Items(vec![InputItem::Item(Item::Message(
            MessageItem::Input(InputMessage {
                content: vec![InputContent::InputText(InputTextContent {
                    text: user_prompt.to_string(),
                })],
                role: InputRole::User,
                status: None,
            }),
        ))]))
        .parallel_tool_calls(false)
        .build()
        .map_err(|err| LlmClientError::DecodeResponse {
            message: err.to_string(),
        })
}

pub(super) fn build_tool_probe_request_payload(
    model: &str,
) -> Result<CreateResponse, LlmClientError> {
    CreateResponseArgs::default()
        .model(model.to_string())
        .instructions("You are a connectivity probe. Call exactly one available tool with minimal valid arguments to confirm tool-call support. Do not answer with plain text.".to_string())
        .input(InputParam::Items(vec![InputItem::Item(Item::Message(
            MessageItem::Input(InputMessage {
                content: vec![InputContent::InputText(InputTextContent {
                    text: "Call one tool now with the smallest valid argument object.".to_string(),
                })],
                role: InputRole::User,
                status: None,
            }),
        ))]))
        .tools(responses_tools_with_debug_mode(false))
        .tool_choice(ToolChoiceParam::Mode(ToolChoiceOptions::Required))
        .parallel_tool_calls(false)
        .build()
        .map_err(|err| LlmClientError::DecodeResponse {
            message: err.to_string(),
        })
}
