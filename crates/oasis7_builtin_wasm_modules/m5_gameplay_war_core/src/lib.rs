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
use std::collections::BTreeMap;

const MODULE_ID: &str = "m5.gameplay.war.core";
const DIRECTIVE_EMIT_KIND: &str = "gameplay.lifecycle.directives";
const WAR_SCORE_PER_MEMBER: i64 = 10;
const WAR_SCORE_REPUTATION_DIVISOR: i64 = 10;
const WAR_FATIGUE_SCORE_DIVISOR: i64 = 4;
const WAR_WINNER_REPUTATION_PER_INTENSITY: i64 = 2;
const WAR_LOSER_REPUTATION_PER_INTENSITY: i64 = 3;
const WAR_LOSER_ELECTRICITY_PENALTY_PER_INTENSITY: i64 = 6;
const WAR_LOSER_DATA_PENALTY_PER_INTENSITY: i64 = 4;
const BASE_WAR_DURATION_TICKS: u64 = 6;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct AllianceSnapshot {
    #[serde(default)]
    members: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WarSnapshot {
    war_id: String,
    aggressor_alliance_id: String,
    defender_alliance_id: String,
    intensity: u32,
    declared_at: u64,
    max_duration_ticks: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct WarModuleState {
    #[serde(default)]
    alliances: BTreeMap<String, AllianceSnapshot>,
    #[serde(default)]
    active_wars: BTreeMap<String, WarSnapshot>,
    #[serde(default)]
    reputation_scores: BTreeMap<String, i64>,
    #[serde(default)]
    alliance_fatigue: BTreeMap<String, i64>,
}

#[derive(Debug, Clone, Deserialize)]
struct DomainEventEnvelope {
    #[serde(rename = "type")]
    event_type: String,
    #[serde(default)]
    data: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
struct AllianceFormedData {
    alliance_id: String,
    #[serde(default)]
    members: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct AllianceJoinedData {
    alliance_id: String,
    member_agent_id: String,
}

#[derive(Debug, Clone, Deserialize)]
struct AllianceLeftData {
    alliance_id: String,
    member_agent_id: String,
}

#[derive(Debug, Clone, Deserialize)]
struct AllianceDissolvedData {
    alliance_id: String,
}

#[derive(Debug, Clone, Deserialize)]
struct WarDeclaredData {
    war_id: String,
    aggressor_alliance_id: String,
    defender_alliance_id: String,
    intensity: u32,
}

#[derive(Debug, Clone, Deserialize)]
struct WarConcludedData {
    war_id: String,
}

#[derive(Debug, Clone, Deserialize)]
struct EconomicContractSettledData {
    creator_agent_id: String,
    counterparty_agent_id: String,
    creator_reputation_delta: i64,
    counterparty_reputation_delta: i64,
}

#[derive(Debug, Clone, Deserialize)]
struct EconomicContractExpiredData {
    creator_agent_id: String,
    counterparty_agent_id: String,
    creator_reputation_delta: i64,
    counterparty_reputation_delta: i64,
}

#[derive(Debug, Clone, Deserialize)]
struct CrisisResolvedData {
    resolver_agent_id: String,
    success: bool,
    impact: i64,
}

#[derive(Debug, Clone, Serialize)]
struct WarParticipantOutcome {
    agent_id: String,
    electricity_delta: i64,
    data_delta: i64,
    reputation_delta: i64,
}

#[derive(Debug, Clone, Serialize)]
struct DirectiveEnvelope {
    directives: Vec<LifecycleDirective>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum LifecycleDirective {
    WarConclude {
        war_id: String,
        winner_alliance_id: String,
        loser_alliance_id: String,
        aggressor_score: i64,
        defender_score: i64,
        summary: String,
        participant_outcomes: Vec<WarParticipantOutcome>,
    },
}

fn decode_state(input: &ModuleCallInput) -> WarModuleState {
    input
        .state
        .as_deref()
        .and_then(|bytes| serde_cbor::from_slice::<WarModuleState>(bytes).ok())
        .unwrap_or_default()
}

fn encode_state(state: &WarModuleState) -> Option<Vec<u8>> {
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

fn apply_domain_event(state: &mut WarModuleState, event: DomainEventEnvelope, now: u64) {
    match event.event_type.as_str() {
        "AllianceFormed" => {
            if let Ok(data) = serde_json::from_value::<AllianceFormedData>(event.data) {
                let mut members = data.members;
                members.sort();
                members.dedup();
                state
                    .alliances
                    .insert(data.alliance_id, AllianceSnapshot { members });
            }
        }
        "AllianceJoined" => {
            if let Ok(data) = serde_json::from_value::<AllianceJoinedData>(event.data) {
                let entry = state
                    .alliances
                    .entry(data.alliance_id)
                    .or_insert_with(AllianceSnapshot::default);
                if !entry
                    .members
                    .iter()
                    .any(|member| member == &data.member_agent_id)
                {
                    entry.members.push(data.member_agent_id);
                    entry.members.sort();
                }
            }
        }
        "AllianceLeft" => {
            if let Ok(data) = serde_json::from_value::<AllianceLeftData>(event.data) {
                if let Some(alliance) = state.alliances.get_mut(&data.alliance_id) {
                    alliance
                        .members
                        .retain(|member| member != &data.member_agent_id);
                }
            }
        }
        "AllianceDissolved" => {
            if let Ok(data) = serde_json::from_value::<AllianceDissolvedData>(event.data) {
                state.alliances.remove(&data.alliance_id);
                state.alliance_fatigue.remove(&data.alliance_id);
            }
        }
        "WarDeclared" => {
            if let Ok(data) = serde_json::from_value::<WarDeclaredData>(event.data) {
                let max_duration_ticks = BASE_WAR_DURATION_TICKS
                    .saturating_add(u64::from(data.intensity.max(1)).saturating_mul(2));
                state.active_wars.insert(
                    data.war_id.clone(),
                    WarSnapshot {
                        war_id: data.war_id,
                        aggressor_alliance_id: data.aggressor_alliance_id,
                        defender_alliance_id: data.defender_alliance_id,
                        intensity: data.intensity.max(1),
                        declared_at: now,
                        max_duration_ticks,
                    },
                );
            }
        }
        "WarConcluded" => {
            if let Ok(data) = serde_json::from_value::<WarConcludedData>(event.data) {
                state.active_wars.remove(&data.war_id);
            }
        }
        "EconomicContractSettled" => {
            if let Ok(data) = serde_json::from_value::<EconomicContractSettledData>(event.data) {
                apply_reputation_delta(
                    &mut state.reputation_scores,
                    data.creator_agent_id.as_str(),
                    data.creator_reputation_delta,
                );
                apply_reputation_delta(
                    &mut state.reputation_scores,
                    data.counterparty_agent_id.as_str(),
                    data.counterparty_reputation_delta,
                );
            }
        }
        "EconomicContractExpired" => {
            if let Ok(data) = serde_json::from_value::<EconomicContractExpiredData>(event.data) {
                apply_reputation_delta(
                    &mut state.reputation_scores,
                    data.creator_agent_id.as_str(),
                    data.creator_reputation_delta,
                );
                apply_reputation_delta(
                    &mut state.reputation_scores,
                    data.counterparty_agent_id.as_str(),
                    data.counterparty_reputation_delta,
                );
            }
        }
        "CrisisResolved" => {
            if let Ok(data) = serde_json::from_value::<CrisisResolvedData>(event.data) {
                let delta = if data.success {
                    (data.impact.max(1) / 4).max(1)
                } else {
                    -data.impact.abs().max(1)
                };
                apply_reputation_delta(
                    &mut state.reputation_scores,
                    data.resolver_agent_id.as_str(),
                    delta,
                );
            }
        }
        _ => {}
    }
}

fn run_tick(state: &mut WarModuleState, now: u64) -> Vec<LifecycleDirective> {
    decay_fatigue(&mut state.alliance_fatigue);

    let mut due_ids = state
        .active_wars
        .iter()
        .filter_map(|(war_id, war)| {
            let due_at = war
                .declared_at
                .saturating_add(war.max_duration_ticks.max(1));
            (now >= due_at).then_some(war_id.clone())
        })
        .collect::<Vec<_>>();
    due_ids.sort();

    let mut directives = Vec::new();
    for war_id in due_ids {
        let Some(war) = state.active_wars.remove(&war_id) else {
            continue;
        };
        let aggressor_members = state
            .alliances
            .get(&war.aggressor_alliance_id)
            .map(|alliance| alliance.members.len() as i64)
            .unwrap_or(0);
        let defender_members = state
            .alliances
            .get(&war.defender_alliance_id)
            .map(|alliance| alliance.members.len() as i64)
            .unwrap_or(0);
        let aggressor_reputation =
            alliance_reputation_total(state, war.aggressor_alliance_id.as_str());
        let defender_reputation =
            alliance_reputation_total(state, war.defender_alliance_id.as_str());
        let aggressor_fatigue = state
            .alliance_fatigue
            .get(war.aggressor_alliance_id.as_str())
            .copied()
            .unwrap_or(0)
            .max(0);
        let defender_fatigue = state
            .alliance_fatigue
            .get(war.defender_alliance_id.as_str())
            .copied()
            .unwrap_or(0)
            .max(0);
        let aggressor_score = aggressor_members
            .saturating_mul(WAR_SCORE_PER_MEMBER)
            .saturating_add(i64::from(war.intensity))
            .saturating_add(aggressor_reputation.saturating_div(WAR_SCORE_REPUTATION_DIVISOR))
            .saturating_sub(aggressor_fatigue.saturating_div(WAR_FATIGUE_SCORE_DIVISOR));
        let defender_score = defender_members
            .saturating_mul(WAR_SCORE_PER_MEMBER)
            .saturating_add(defender_reputation.saturating_div(WAR_SCORE_REPUTATION_DIVISOR))
            .saturating_sub(defender_fatigue.saturating_div(WAR_FATIGUE_SCORE_DIVISOR));
        let (winner_alliance_id, loser_alliance_id) = if aggressor_score >= defender_score {
            (
                war.aggressor_alliance_id.clone(),
                war.defender_alliance_id.clone(),
            )
        } else {
            (
                war.defender_alliance_id.clone(),
                war.aggressor_alliance_id.clone(),
            )
        };
        let participant_outcomes = build_war_participant_outcomes(
            state,
            winner_alliance_id.as_str(),
            loser_alliance_id.as_str(),
            war.intensity,
        );
        state.alliance_fatigue.insert(
            war.aggressor_alliance_id.clone(),
            aggressor_fatigue.saturating_add(i64::from(war.intensity).saturating_mul(2)),
        );
        state.alliance_fatigue.insert(
            war.defender_alliance_id.clone(),
            defender_fatigue.saturating_add(i64::from(war.intensity).saturating_mul(2)),
        );
        let summary = format!(
            "module settlement: aggressor_score={} defender_score={} aggressor_reputation={} defender_reputation={} aggressor_fatigue={} defender_fatigue={} outcome_count={}",
            aggressor_score,
            defender_score,
            aggressor_reputation,
            defender_reputation,
            aggressor_fatigue,
            defender_fatigue,
            participant_outcomes.len()
        );
        directives.push(LifecycleDirective::WarConclude {
            war_id,
            winner_alliance_id,
            loser_alliance_id,
            aggressor_score,
            defender_score,
            summary,
            participant_outcomes,
        });
    }
    directives
}

fn apply_reputation_delta(scores: &mut BTreeMap<String, i64>, agent_id: &str, delta: i64) {
    if delta == 0 || agent_id.trim().is_empty() {
        return;
    }
    let entry = scores.entry(agent_id.to_string()).or_insert(0);
    *entry = entry.saturating_add(delta);
}

fn alliance_reputation_total(state: &WarModuleState, alliance_id: &str) -> i64 {
    state
        .alliances
        .get(alliance_id)
        .map(|alliance| {
            alliance
                .members
                .iter()
                .map(|member| state.reputation_scores.get(member).copied().unwrap_or(0))
                .sum()
        })
        .unwrap_or(0)
}

fn alliance_members_sorted(state: &WarModuleState, alliance_id: &str) -> Vec<String> {
    let mut members = state
        .alliances
        .get(alliance_id)
        .map(|alliance| alliance.members.clone())
        .unwrap_or_default();
    members.sort();
    members
}

fn build_war_participant_outcomes(
    state: &WarModuleState,
    winner_alliance_id: &str,
    loser_alliance_id: &str,
    intensity: u32,
) -> Vec<WarParticipantOutcome> {
    let intensity = i64::from(intensity.max(1));
    let loser_electricity_penalty = intensity
        .saturating_mul(WAR_LOSER_ELECTRICITY_PENALTY_PER_INTENSITY)
        .max(1);
    let loser_data_penalty = intensity
        .saturating_mul(WAR_LOSER_DATA_PENALTY_PER_INTENSITY)
        .max(1);
    let loser_reputation_delta = intensity
        .saturating_mul(WAR_LOSER_REPUTATION_PER_INTENSITY)
        .saturating_neg();
    let winner_reputation_delta = intensity.saturating_mul(WAR_WINNER_REPUTATION_PER_INTENSITY);

    let loser_members = alliance_members_sorted(state, loser_alliance_id);
    let winner_members = alliance_members_sorted(state, winner_alliance_id);
    let mut outcomes = Vec::new();
    let mut total_electricity_spoils = 0_i64;
    let mut total_data_spoils = 0_i64;

    for member in loser_members {
        total_electricity_spoils =
            total_electricity_spoils.saturating_add(loser_electricity_penalty);
        total_data_spoils = total_data_spoils.saturating_add(loser_data_penalty);
        outcomes.push(WarParticipantOutcome {
            agent_id: member,
            electricity_delta: loser_electricity_penalty.saturating_neg(),
            data_delta: loser_data_penalty.saturating_neg(),
            reputation_delta: loser_reputation_delta,
        });
    }

    if !winner_members.is_empty() {
        let winner_count = i64::try_from(winner_members.len()).unwrap_or(1).max(1);
        let base_electricity_gain = total_electricity_spoils.saturating_div(winner_count);
        let base_data_gain = total_data_spoils.saturating_div(winner_count);
        let mut electricity_remainder = total_electricity_spoils
            .saturating_sub(base_electricity_gain.saturating_mul(winner_count));
        let mut data_remainder =
            total_data_spoils.saturating_sub(base_data_gain.saturating_mul(winner_count));

        for member in winner_members {
            let mut electricity_gain = base_electricity_gain;
            let mut data_gain = base_data_gain;
            if electricity_remainder > 0 {
                electricity_gain = electricity_gain.saturating_add(1);
                electricity_remainder = electricity_remainder.saturating_sub(1);
            }
            if data_remainder > 0 {
                data_gain = data_gain.saturating_add(1);
                data_remainder = data_remainder.saturating_sub(1);
            }
            outcomes.push(WarParticipantOutcome {
                agent_id: member,
                electricity_delta: electricity_gain,
                data_delta: data_gain,
                reputation_delta: winner_reputation_delta,
            });
        }
    }

    outcomes.sort_by(|left, right| left.agent_id.cmp(&right.agent_id));
    outcomes
}

fn decay_fatigue(fatigue: &mut BTreeMap<String, i64>) {
    for value in fatigue.values_mut() {
        if *value > 0 {
            *value = value.saturating_sub(1);
        }
    }
}

fn build_output(state: &WarModuleState, directives: Vec<LifecycleDirective>) -> ModuleOutput {
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
    let now = input.ctx.time;
    if let Some(event) = parse_domain_event(input) {
        apply_domain_event(&mut state, event, now);
    }

    let directives = if input.ctx.stage.as_deref() == Some("tick") {
        run_tick(&mut state, now)
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

    fn formed_event(alliance_id: &str, members: &[&str]) -> DomainEventEnvelope {
        DomainEventEnvelope {
            event_type: "AllianceFormed".to_string(),
            data: serde_json::json!({
                "alliance_id": alliance_id,
                "members": members,
            }),
        }
    }

    fn declared_event(
        war_id: &str,
        aggressor: &str,
        defender: &str,
        intensity: u32,
    ) -> DomainEventEnvelope {
        DomainEventEnvelope {
            event_type: "WarDeclared".to_string(),
            data: serde_json::json!({
                "war_id": war_id,
                "aggressor_alliance_id": aggressor,
                "defender_alliance_id": defender,
                "intensity": intensity,
            }),
        }
    }

    #[test]
    fn run_tick_emits_participant_outcomes_and_loser() {
        let mut state = WarModuleState::default();
        apply_domain_event(&mut state, formed_event("red", &["a", "b"]), 1);
        apply_domain_event(&mut state, formed_event("blue", &["c"]), 1);
        apply_domain_event(&mut state, declared_event("war.1", "red", "blue", 2), 2);

        let directives = run_tick(&mut state, 20);
        assert_eq!(directives.len(), 1);
        let LifecycleDirective::WarConclude {
            winner_alliance_id,
            loser_alliance_id,
            participant_outcomes,
            ..
        } = &directives[0];
        assert_eq!(winner_alliance_id, "red");
        assert_eq!(loser_alliance_id, "blue");
        assert!(participant_outcomes.iter().any(|item| item.agent_id == "c"));
        assert!(participant_outcomes.iter().any(|item| item.agent_id == "a"));
        assert!(participant_outcomes.iter().any(|item| item.agent_id == "b"));
    }

    #[test]
    fn alliance_membership_events_adjust_snapshots() {
        let mut state = WarModuleState::default();
        apply_domain_event(&mut state, formed_event("red", &["a"]), 1);
        apply_domain_event(
            &mut state,
            DomainEventEnvelope {
                event_type: "AllianceJoined".to_string(),
                data: serde_json::json!({
                    "alliance_id": "red",
                    "member_agent_id": "b",
                }),
            },
            2,
        );
        apply_domain_event(
            &mut state,
            DomainEventEnvelope {
                event_type: "AllianceLeft".to_string(),
                data: serde_json::json!({
                    "alliance_id": "red",
                    "member_agent_id": "a",
                }),
            },
            3,
        );

        let members = &state.alliances.get("red").expect("alliance").members;
        assert_eq!(members, &vec!["b".to_string()]);
    }
}

export_wasm_module!(BuiltinWasmModule);
