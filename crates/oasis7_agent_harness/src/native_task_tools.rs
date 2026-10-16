use super::*;
use crate::{ModelProviderTool, ModelProviderTurn};
use oasis7_agent_api::{BudgetContractV1, h_v1};
use serde::{Deserialize, Serialize};
use serde_json::json;
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmptyArguments {}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MoveArguments {
    pub to: [i64; 3],
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaitArguments {
    pub ticks: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildArguments {
    pub site: String,
    pub factory: String,
    pub kind: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScheduleArguments {
    pub factory: String,
    pub recipe: String,
    pub batches: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransferArguments {
    pub from: String,
    pub to: String,
    pub material: String,
    pub amount: i64,
    pub distance_km: i64,
    pub priority: String,
    pub routes: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "tool",
    content = "arguments",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum NativeTaskTool {
    Observe(EmptyArguments),
    Industry(EmptyArguments),
    Capabilities(EmptyArguments),
    QuoteBuild(BuildArguments),
    Build(BuildArguments),
    QuoteSchedule(ScheduleArguments),
    Schedule(ScheduleArguments),
    Move(MoveArguments),
    Transfer(TransferArguments),
    Wait(WaitArguments),
    Abort(EmptyArguments),
}
impl NativeTaskTool {
    pub fn parse(name: &str, arguments: &Value) -> Result<Self, String> {
        let tool: Self = serde_json::from_value(json!({"tool":name,"arguments":arguments}))
            .map_err(|_| "unsupported native tool or arguments")?;
        tool.validate()?;
        Ok(tool)
    }
    pub fn side_effect(&self) -> bool {
        matches!(
            self,
            Self::Build(_) | Self::Schedule(_) | Self::Move(_) | Self::Transfer(_) | Self::Abort(_)
        )
    }
    fn validate(&self) -> Result<(), String> {
        let id =
            |s: &str| !s.trim().is_empty() && s.len() <= 256 && !s.chars().any(char::is_control);
        match self {
            Self::Wait(v) if !(1..=16).contains(&v.ticks) => {
                Err("wait must observe between one and sixteen world ticks".into())
            }
            Self::QuoteBuild(v) | Self::Build(v)
                if !id(&v.site)
                    || !id(&v.factory)
                    || !["factory.smelter.mk1", "factory.assembler.mk1"]
                        .contains(&v.kind.as_str()) =>
            {
                Err("invalid build selection".into())
            }
            Self::QuoteSchedule(v) | Self::Schedule(v)
                if !id(&v.factory) || !id(&v.recipe) || v.batches == 0 =>
            {
                Err("invalid recipe selection".into())
            }
            Self::Transfer(v)
                if !id(&v.from)
                    || !id(&v.to)
                    || v.from == v.to
                    || !id(&v.material)
                    || v.amount <= 0
                    || v.distance_km < 0
                    || !["standard", "urgent"].contains(&v.priority.as_str())
                    || v.routes.len() > 16
                    || v.routes.iter().any(|r| !id(r))
                    || v.routes
                        .iter()
                        .collect::<std::collections::BTreeSet<_>>()
                        .len()
                        != v.routes.len() =>
            {
                Err("invalid transfer selection".into())
            }
            _ => Ok(()),
        }
    }
}
pub(super) fn validate_turns(
    response: &ModelProviderResponse,
    maximum: u32,
) -> Result<Vec<NativeTaskTool>, String> {
    if response.turns.is_empty() || response.turns.len() > maximum as usize {
        return Err("native tool call bound exceeded".into());
    }
    let mut tools = Vec::new();
    for turn in &response.turns {
        let tool = match turn {
            ModelProviderTurn::ToolCall { name, arguments } => {
                NativeTaskTool::parse(name, arguments)?
            }
            ModelProviderTurn::Decision { payload } => {
                let tool: NativeTaskTool = serde_json::from_value(payload.clone())
                    .map_err(|_| "decision must name an explicit native tool")?;
                tool.validate()?;
                tool
            }
        };
        tools.push(tool);
    }
    if tools.iter().filter(|tool| tool.side_effect()).count() > 1 {
        return Err("only one side effect is permitted per task context".into());
    }
    // An effect ends this context. Reject trailing reads too, so every accepted
    // response is executed completely without silently discarded tool calls.
    if tools
        .iter()
        .position(NativeTaskTool::side_effect)
        .is_some_and(|i| i + 1 != tools.len())
    {
        return Err("side effect must be the final tool in a model response".into());
    }
    Ok(tools)
}
pub(super) fn validate_call_ids(
    response: &ModelProviderResponse,
    calls: &[crate::ModelProviderToolCall],
) -> Result<(), String> {
    if calls.is_empty() {
        return Ok(());
    } // Legacy providers have no protocol history.
    if calls.len() != response.turns.len() {
        return Err("supplier call count mismatch".into());
    }
    let mut ids = std::collections::BTreeSet::new();
    for (turn, call) in response.turns.iter().zip(calls) {
        if call.call_id.trim().is_empty()
            || call.call_id.len() > 256
            || call.call_id.chars().any(char::is_control)
            || !ids.insert(&call.call_id)
        {
            return Err("invalid supplier call identity".into());
        }
        match turn {
            ModelProviderTurn::ToolCall { name, arguments }
                if name == &call.name && arguments == &call.arguments => {}
            _ => return Err("supplier call does not match model turn".into()),
        }
    }
    Ok(())
}
pub fn native_task_tools() -> Vec<ModelProviderTool> {
    let empty = json!({"type":"object","properties":{},"additionalProperties":false});
    let string = json!({"type":"string","minLength":1,"maxLength":256});
    let build = json!({"type":"object","properties":{"site":string,"factory":string,"kind":{"type":"string","enum":["factory.smelter.mk1","factory.assembler.mk1"]}},"required":["site","factory","kind"],"additionalProperties":false});
    let schedule = json!({"type":"object","properties":{"factory":string,"recipe":string,"batches":{"type":"integer","minimum":1,"maximum":u32::MAX}},"required":["factory","recipe","batches"],"additionalProperties":false});
    let transfer = json!({"type":"object","properties":{"from":string,"to":string,"material":string,
        "amount":{"type":"integer","minimum":1},"distance_km":{"type":"integer","minimum":0},
        "priority":{"type":"string","enum":["standard","urgent"]},"routes":{"type":"array","items":string,"maxItems":16,"uniqueItems":true}},
        "required":["from","to","material","amount","distance_km","priority","routes"],"additionalProperties":false});
    [("observe",empty.clone()),("industry",empty.clone()),("capabilities",empty.clone()),
        ("quote_build",build.clone()),("build",build),("quote_schedule",schedule.clone()),("schedule",schedule),
        ("move",json!({"type":"object","properties":{"to":{"type":"array","items":{"type":"integer"},"minItems":3,"maxItems":3}},"required":["to"],"additionalProperties":false})),
        ("transfer",transfer),("wait",json!({"type":"object","properties":{"ticks":{"type":"integer","minimum":1,"maximum":16}},"required":["ticks"],"additionalProperties":false})),("abort",empty)].into_iter().map(|(name,parameters)| ModelProviderTool {
            name:name.into(),description:Some(if name == "abort" { "Explicitly close this context without a physical world action" } else { "Native industry profile; remote World Service admission remains authoritative" }.into()), parameters,
        }).collect()
}
pub(super) fn request(
    config: &NativeTaskRunConfig,
    context: &NativeTaskContext,
    transcript: &[Value],
    rejected_feedback: &[Value],
) -> Result<ModelProviderRequest, String> {
    let user_prompt = serde_json::to_string(
        &json!({"operation":context.operation,"authorized_context":context.model_context,
        "tool_results":transcript,"prior_authorized_rejections":rejected_feedback}),
    )
    .map_err(|_| "native prompt encoding failed")?;
    if user_prompt.len() > 8 * 1024 * 1024 {
        return Err("complete native prompt exceeds bound; no content was truncated".into());
    }
    let runtime_rules = "Act on the complete authorized goal and soft constraints using the native industry tools. Read results may inform subsequent calls. Read the native_delegation_authorizations before choosing action objects. A new factory ID must exactly match an authorized BuildFactory grant object_id; factory kind is not object authority. A verified read-only selection rejection has no world effect: consume its reason, inspect current capabilities, and choose a different authorized selection or explicitly abort within the existing budget and context lease. Do not repeat the same denied selection blindly. Authentication, revoked authority, unverified responses, and unknown writes must not be treated as correctable quote rejections. Use at most one physical side effect per context, as the final tool. Use wait to observe bounded independent world progress when a job is still pending; it never advances the world clock. If no valid continuation is appropriate, explicitly abort. Tools never grant authority; the World Service may reject proposals. Unsupported capabilities must not be replaced with Wait.";
    let system_prompt = format!("{}\n\n{runtime_rules}", config.system_prompt);
    let tools = native_task_tools();
    let request_digest = h_v1(
        "oasis7.native-task.model-call.v1",
        &(&config.model, &system_prompt, &user_prompt, &tools),
    );
    Ok(ModelProviderRequest {
        request_digest,
        model: config.model.clone(),
        system_prompt,
        user_prompt,
        tools,
        budget: BudgetContractV1 {
            max_latency_ms: config.model_timeout_ms,
            max_repair_attempts: 0,
            max_model_calls: 1,
            max_tool_calls: config.max_tool_calls_per_call,
        },
    })
}
