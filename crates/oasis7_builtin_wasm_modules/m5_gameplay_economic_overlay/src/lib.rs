#![allow(improper_ctypes_definitions)]

use oasis7_wasm_sdk::{
    export_wasm_module,
    wire::{
        decode_input, empty_output, encode_output, ModuleCallInput, ModuleEmit, ModuleOutput,
        ModuleTickLifecycleDirective,
    },
    LifecycleStage, WasmModuleLifecycle,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

const MODULE_ID: &str = "m5.gameplay.economic.overlay";
const DIRECTIVE_EMIT_KIND: &str = "gameplay.lifecycle.directives";
const OVERLAY_OPERATOR_ID: &str = "system.economic.overlay";
const RESILIENCE_TRACK: &str = "resilience";
const ECONOMY_TRACK: &str = "economy";
const RELIABILITY_TRACK: &str = "reliability";
const CONTRACT_SUCCESS_ACHIEVEMENT_PREFIX: &str = "contract.success.streak";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MetaGrantDirective {
    operator_agent_id: String,
    target_agent_id: String,
    track: String,
    points: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    achievement_id: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct EconomicOverlayState {
    #[serde(default)]
    pending_meta_grants: Vec<MetaGrantDirective>,
    #[serde(default)]
    contract_success_streak: BTreeMap<String, u32>,
    #[serde(default)]
    processed_contract_ids: BTreeSet<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct DomainEventEnvelope {
    #[serde(rename = "type")]
    event_type: String,
    #[serde(default)]
    data: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
struct CrisisResolvedData {
    resolver_agent_id: String,
    success: bool,
    impact: i64,
}

#[derive(Debug, Clone, Deserialize)]
struct EconomicContractSettledData {
    operator_agent_id: String,
    contract_id: String,
    success: bool,
    transfer_amount: i64,
    tax_amount: i64,
    creator_reputation_delta: i64,
    counterparty_reputation_delta: i64,
}

#[derive(Debug, Clone, Deserialize)]
struct EconomicContractExpiredData {
    contract_id: String,
    creator_agent_id: String,
    counterparty_agent_id: String,
    creator_reputation_delta: i64,
    counterparty_reputation_delta: i64,
}

#[derive(Debug, Clone, Serialize)]
struct DirectiveEnvelope {
    directives: Vec<LifecycleDirective>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum LifecycleDirective {
    MetaGrant {
        operator_agent_id: String,
        target_agent_id: String,
        track: String,
        points: i64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        achievement_id: Option<String>,
    },
}

fn decode_state(input: &ModuleCallInput) -> EconomicOverlayState {
    input
        .state
        .as_deref()
        .and_then(|bytes| serde_cbor::from_slice::<EconomicOverlayState>(bytes).ok())
        .unwrap_or_default()
}

fn encode_state(state: &EconomicOverlayState) -> Option<Vec<u8>> {
    serde_cbor::to_vec(state).ok()
}

fn parse_domain_event(input: &ModuleCallInput) -> Option<DomainEventEnvelope> {
    let event_bytes = input.event.as_deref()?;
    let event_value = serde_cbor::from_slice::<serde_json::Value>(event_bytes).ok()?;
    if event_value.get("body")?.get("kind")?.as_str()? != "Domain" {
        return None;
    }
    let payload = event_value.get("body")?.get("payload")?.clone();
    serde_json::from_value(payload).ok()
}

fn apply_domain_event(state: &mut EconomicOverlayState, event: DomainEventEnvelope) {
    match event.event_type.as_str() {
        "CrisisResolved" => {
            let Ok(data) = serde_json::from_value::<CrisisResolvedData>(event.data) else {
                return;
            };
            if !data.success || data.impact <= 0 {
                return;
            }
            let points = (data.impact / 4).max(1);
            push_meta_grant(
                &mut state.pending_meta_grants,
                data.resolver_agent_id.as_str(),
                RESILIENCE_TRACK,
                points,
                None,
            );
        }
        "EconomicContractSettled" => {
            let Ok(data) = serde_json::from_value::<EconomicContractSettledData>(event.data) else {
                return;
            };
            if !state
                .processed_contract_ids
                .insert(data.contract_id.clone())
            {
                return;
            }
            if data.success {
                let operator_points = (data.transfer_amount / 12)
                    .saturating_add(data.tax_amount / 4)
                    .max(1);
                push_meta_grant(
                    &mut state.pending_meta_grants,
                    data.operator_agent_id.as_str(),
                    ECONOMY_TRACK,
                    operator_points,
                    None,
                );

                let streak = state
                    .contract_success_streak
                    .entry(data.operator_agent_id.clone())
                    .or_insert(0);
                *streak = streak.saturating_add(1);
                if *streak % 3 == 0 {
                    push_meta_grant(
                        &mut state.pending_meta_grants,
                        data.operator_agent_id.as_str(),
                        RELIABILITY_TRACK,
                        2,
                        Some(format!(
                            "{}.{}",
                            CONTRACT_SUCCESS_ACHIEVEMENT_PREFIX, streak
                        )),
                    );
                }

                if data.creator_reputation_delta > 0 {
                    push_meta_grant(
                        &mut state.pending_meta_grants,
                        data.operator_agent_id.as_str(),
                        RELIABILITY_TRACK,
                        1,
                        None,
                    );
                }
                if data.counterparty_reputation_delta > 0 {
                    push_meta_grant(
                        &mut state.pending_meta_grants,
                        data.operator_agent_id.as_str(),
                        ECONOMY_TRACK,
                        1,
                        None,
                    );
                }
            } else {
                state
                    .contract_success_streak
                    .insert(data.operator_agent_id.clone(), 0);
                push_meta_grant(
                    &mut state.pending_meta_grants,
                    data.operator_agent_id.as_str(),
                    RELIABILITY_TRACK,
                    -2,
                    None,
                );
            }
        }
        "EconomicContractExpired" => {
            let Ok(data) = serde_json::from_value::<EconomicContractExpiredData>(event.data) else {
                return;
            };
            if !state
                .processed_contract_ids
                .insert(data.contract_id.clone())
            {
                return;
            }
            state
                .contract_success_streak
                .insert(data.creator_agent_id.clone(), 0);
            state
                .contract_success_streak
                .insert(data.counterparty_agent_id.clone(), 0);
            let creator_penalty = data.creator_reputation_delta.min(-1);
            let counterparty_penalty = data.counterparty_reputation_delta.min(-1);
            push_meta_grant(
                &mut state.pending_meta_grants,
                data.creator_agent_id.as_str(),
                RELIABILITY_TRACK,
                creator_penalty,
                None,
            );
            push_meta_grant(
                &mut state.pending_meta_grants,
                data.counterparty_agent_id.as_str(),
                RELIABILITY_TRACK,
                counterparty_penalty,
                None,
            );
        }
        _ => {}
    }
}

fn run_tick(state: &mut EconomicOverlayState) -> Vec<LifecycleDirective> {
    if state.pending_meta_grants.is_empty() {
        return Vec::new();
    }
    let pending = std::mem::take(&mut state.pending_meta_grants);
    let mut aggregated: BTreeMap<(String, String, String), MetaGrantDirective> = BTreeMap::new();
    for grant in pending {
        let key = (
            grant.operator_agent_id.clone(),
            grant.target_agent_id.clone(),
            grant.track.clone(),
        );
        let entry = aggregated.entry(key).or_insert(MetaGrantDirective {
            operator_agent_id: grant.operator_agent_id.clone(),
            target_agent_id: grant.target_agent_id.clone(),
            track: grant.track.clone(),
            points: 0,
            achievement_id: grant.achievement_id.clone(),
        });
        entry.points = entry.points.saturating_add(grant.points);
        if entry.achievement_id.is_none() {
            entry.achievement_id = grant.achievement_id;
        }
    }

    aggregated
        .into_values()
        .filter(|grant| grant.points != 0)
        .map(|grant| LifecycleDirective::MetaGrant {
            operator_agent_id: grant.operator_agent_id,
            target_agent_id: grant.target_agent_id,
            track: grant.track,
            points: grant.points,
            achievement_id: grant.achievement_id,
        })
        .collect()
}

fn push_meta_grant(
    pending: &mut Vec<MetaGrantDirective>,
    target_agent_id: &str,
    track: &str,
    points: i64,
    achievement_id: Option<String>,
) {
    if points == 0 || target_agent_id.trim().is_empty() || track.trim().is_empty() {
        return;
    }
    pending.push(MetaGrantDirective {
        operator_agent_id: OVERLAY_OPERATOR_ID.to_string(),
        target_agent_id: target_agent_id.to_string(),
        track: track.to_string(),
        points,
        achievement_id,
    });
}

fn build_output(state: &EconomicOverlayState, directives: Vec<LifecycleDirective>) -> ModuleOutput {
    let emits = if directives.is_empty() {
        Vec::new()
    } else {
        let payload = serde_json::to_value(DirectiveEnvelope { directives })
            .unwrap_or_else(|_| serde_json::json!({ "directives": [] }));
        vec![ModuleEmit {
            kind: DIRECTIVE_EMIT_KIND.to_string(),
            payload,
        }]
    };

    ModuleOutput {
        new_state: encode_state(state),
        effects: Vec::new(),
        emits,
        tick_lifecycle: Some(ModuleTickLifecycleDirective::WakeAfterTicks { ticks: 1 }),
        output_bytes: 2048,
    }
}

fn reduce_output(input: &ModuleCallInput) -> ModuleOutput {
    let mut state = decode_state(input);
    if let Some(event) = parse_domain_event(input) {
        apply_domain_event(&mut state, event);
    }

    let directives = if input.ctx.stage.as_deref() == Some("tick") {
        run_tick(&mut state)
    } else {
        Vec::new()
    };

    build_output(&state, directives)
}

fn read_input_bytes(input_ptr: i32, input_len: i32) -> Vec<u8> {
    if input_ptr > 0 && input_len > 0 {
        let ptr = input_ptr as *const u8;
        let len = input_len as usize;
        // SAFETY: host guarantees valid wasm linear memory pointer/len for the call.
        return unsafe { std::slice::from_raw_parts(ptr, len).to_vec() };
    }
    Vec::new()
}

fn write_bytes_to_memory(bytes: &[u8]) -> (i32, i32) {
    let len = i32::try_from(bytes.len()).unwrap_or(0);
    if len <= 0 {
        return (0, 0);
    }
    let ptr = oasis7_wasm_sdk::default_alloc(len);
    if ptr <= 0 {
        return (0, 0);
    }
    // SAFETY: alloc returns a writable wasm linear memory region with at least len bytes.
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr as *mut u8, len as usize);
    }
    (ptr, len)
}

fn reduce_impl(input_ptr: i32, input_len: i32) -> (i32, i32) {
    let input = read_input_bytes(input_ptr, input_len);
    let Ok(mut decoded) = decode_input(&input) else {
        return write_bytes_to_memory(&encode_output(empty_output()));
    };
    decoded.ctx.module_id = MODULE_ID.to_string();
    let output = reduce_output(&decoded);
    write_bytes_to_memory(&encode_output(output))
}

#[derive(Default)]
struct BuiltinWasmModule;

impl WasmModuleLifecycle for BuiltinWasmModule {
    fn module_id(&self) -> &'static str {
        MODULE_ID
    }

    fn alloc(&mut self, len: i32) -> i32 {
        oasis7_wasm_sdk::default_alloc(len)
    }

    fn on_init(&mut self, _stage: LifecycleStage) {}

    fn on_teardown(&mut self, _stage: LifecycleStage) {}

    fn on_reduce(&mut self, input_ptr: i32, input_len: i32) -> (i32, i32) {
        reduce_impl(input_ptr, input_len)
    }

    fn on_call(&mut self, input_ptr: i32, input_len: i32) -> (i32, i32) {
        reduce_impl(input_ptr, input_len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_settlement_generates_economy_grants() {
        let mut state = EconomicOverlayState::default();
        apply_domain_event(
            &mut state,
            DomainEventEnvelope {
                event_type: "EconomicContractSettled".to_string(),
                data: serde_json::json!({
                    "operator_agent_id": "agent-1",
                    "contract_id": "contract.a",
                    "success": true,
                    "transfer_amount": 24,
                    "tax_amount": 4,
                    "creator_reputation_delta": 2,
                    "counterparty_reputation_delta": 1,
                }),
            },
        );

        let directives = run_tick(&mut state);
        let grant_points: i64 = directives
            .iter()
            .filter_map(|directive| match directive {
                LifecycleDirective::MetaGrant { points, .. } => Some(*points),
            })
            .sum();
        assert!(grant_points >= 3);
    }

    #[test]
    fn contract_expiry_generates_negative_reliability_grants() {
        let mut state = EconomicOverlayState::default();
        apply_domain_event(
            &mut state,
            DomainEventEnvelope {
                event_type: "EconomicContractExpired".to_string(),
                data: serde_json::json!({
                    "contract_id": "contract.expired.1",
                    "creator_agent_id": "agent-1",
                    "counterparty_agent_id": "agent-2",
                    "creator_reputation_delta": -4,
                    "counterparty_reputation_delta": -2,
                }),
            },
        );

        let directives = run_tick(&mut state);
        assert!(directives.iter().any(|directive| {
            matches!(
                directive,
                LifecycleDirective::MetaGrant {
                    target_agent_id,
                    points,
                    ..
                } if target_agent_id == "agent-1" && *points < 0
            )
        }));
    }

    #[test]
    fn duplicate_contract_settlement_is_deduplicated() {
        let mut state = EconomicOverlayState::default();
        let event = DomainEventEnvelope {
            event_type: "EconomicContractSettled".to_string(),
            data: serde_json::json!({
                "operator_agent_id": "agent-1",
                "contract_id": "contract.dedupe.1",
                "success": true,
                "transfer_amount": 24,
                "tax_amount": 4,
                "creator_reputation_delta": 2,
                "counterparty_reputation_delta": 1,
            }),
        };

        apply_domain_event(&mut state, event.clone());
        apply_domain_event(&mut state, event);

        let directives = run_tick(&mut state);
        let grant_points: i64 = directives
            .iter()
            .filter_map(|directive| match directive {
                LifecycleDirective::MetaGrant { points, .. } => Some(*points),
            })
            .sum();

        assert_eq!(grant_points, 5);
        assert_eq!(
            state.contract_success_streak.get("agent-1").copied(),
            Some(1)
        );
    }

    #[test]
    fn duplicate_contract_expiry_is_deduplicated() {
        let mut state = EconomicOverlayState::default();
        let event = DomainEventEnvelope {
            event_type: "EconomicContractExpired".to_string(),
            data: serde_json::json!({
                "contract_id": "contract.dedupe.expired.1",
                "creator_agent_id": "agent-1",
                "counterparty_agent_id": "agent-2",
                "creator_reputation_delta": -4,
                "counterparty_reputation_delta": -2,
            }),
        };

        apply_domain_event(&mut state, event.clone());
        apply_domain_event(&mut state, event);

        let directives = run_tick(&mut state);
        let grant_points: i64 = directives
            .iter()
            .filter_map(|directive| match directive {
                LifecycleDirective::MetaGrant { points, .. } => Some(*points),
            })
            .sum();

        assert_eq!(grant_points, -6);
    }
}

export_wasm_module!(BuiltinWasmModule);
