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

const MODULE_ID: &str = "m5.gameplay.governance.council";
const DIRECTIVE_EMIT_KIND: &str = "gameplay.lifecycle.directives";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProposalSnapshot {
    proposal_key: String,
    closes_at: u64,
    quorum_weight: u64,
    pass_threshold_bps: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VoteBallot {
    option: String,
    weight: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct ProposalVoteState {
    #[serde(default)]
    votes_by_agent: BTreeMap<String, VoteBallot>,
    #[serde(default)]
    tallies: BTreeMap<String, u64>,
    #[serde(default)]
    total_weight: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct GovernanceModuleState {
    #[serde(default)]
    proposals: BTreeMap<String, ProposalSnapshot>,
    #[serde(default)]
    votes: BTreeMap<String, ProposalVoteState>,
}

#[derive(Debug, Clone, Deserialize)]
struct DomainEventEnvelope {
    #[serde(rename = "type")]
    event_type: String,
    #[serde(default)]
    data: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
struct GovernanceProposalOpenedData {
    proposal_key: String,
    closes_at: u64,
    quorum_weight: u64,
    pass_threshold_bps: u16,
}

#[derive(Debug, Clone, Deserialize)]
struct GovernanceVoteCastData {
    voter_agent_id: String,
    proposal_key: String,
    option: String,
    weight: u32,
}

#[derive(Debug, Clone, Deserialize)]
struct GovernanceProposalFinalizedData {
    proposal_key: String,
}

#[derive(Debug, Clone, Serialize)]
struct DirectiveEnvelope {
    directives: Vec<LifecycleDirective>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum LifecycleDirective {
    GovernanceFinalize {
        proposal_key: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        winning_option: Option<String>,
        winning_weight: u64,
        total_weight: u64,
        passed: bool,
    },
}

fn decode_state(input: &ModuleCallInput) -> GovernanceModuleState {
    input
        .state
        .as_deref()
        .and_then(|bytes| serde_cbor::from_slice::<GovernanceModuleState>(bytes).ok())
        .unwrap_or_default()
}

fn encode_state(state: &GovernanceModuleState) -> Option<Vec<u8>> {
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

fn apply_domain_event(state: &mut GovernanceModuleState, event: DomainEventEnvelope) {
    match event.event_type.as_str() {
        "GovernanceProposalOpened" => {
            if let Ok(data) = serde_json::from_value::<GovernanceProposalOpenedData>(event.data) {
                let proposal_key = data.proposal_key.clone();
                state.proposals.insert(
                    proposal_key.clone(),
                    ProposalSnapshot {
                        proposal_key: proposal_key.clone(),
                        closes_at: data.closes_at,
                        quorum_weight: data.quorum_weight,
                        pass_threshold_bps: data.pass_threshold_bps,
                    },
                );
                state.votes.entry(proposal_key).or_default();
            }
        }
        "GovernanceVoteCast" => {
            if let Ok(data) = serde_json::from_value::<GovernanceVoteCastData>(event.data) {
                if !state.proposals.contains_key(&data.proposal_key) {
                    return;
                }
                let vote_state = state.votes.entry(data.proposal_key).or_default();
                if let Some(previous) = vote_state.votes_by_agent.get(&data.voter_agent_id) {
                    let previous_weight = u64::from(previous.weight);
                    vote_state.total_weight =
                        vote_state.total_weight.saturating_sub(previous_weight);
                    if let Some(tally) = vote_state.tallies.get_mut(&previous.option) {
                        *tally = tally.saturating_sub(previous_weight);
                        if *tally == 0 {
                            vote_state.tallies.remove(&previous.option);
                        }
                    }
                }

                vote_state.votes_by_agent.insert(
                    data.voter_agent_id,
                    VoteBallot {
                        option: data.option.clone(),
                        weight: data.weight,
                    },
                );
                let weight_u64 = u64::from(data.weight);
                let tally = vote_state.tallies.get(&data.option).copied().unwrap_or(0);
                vote_state
                    .tallies
                    .insert(data.option, tally.saturating_add(weight_u64));
                vote_state.total_weight = vote_state.total_weight.saturating_add(weight_u64);
            }
        }
        "GovernanceProposalFinalized" => {
            if let Ok(data) = serde_json::from_value::<GovernanceProposalFinalizedData>(event.data)
            {
                state.proposals.remove(&data.proposal_key);
                state.votes.remove(&data.proposal_key);
            }
        }
        _ => {}
    }
}

fn run_tick(state: &mut GovernanceModuleState, now: u64) -> Vec<LifecycleDirective> {
    let mut due_keys = state
        .proposals
        .iter()
        .filter_map(|(proposal_key, proposal)| {
            (proposal.closes_at <= now).then_some(proposal_key.clone())
        })
        .collect::<Vec<_>>();
    due_keys.sort();

    let mut directives = Vec::new();
    for proposal_key in due_keys {
        let Some(proposal) = state.proposals.remove(&proposal_key) else {
            continue;
        };
        let vote_state = state.votes.remove(&proposal_key).unwrap_or_default();

        let mut winning_option = None;
        let mut winning_weight = 0_u64;
        for (option, weight) in &vote_state.tallies {
            let better = *weight > winning_weight
                || (*weight == winning_weight
                    && winning_option
                        .as_ref()
                        .map(|current| option < current)
                        .unwrap_or(true));
            if better {
                winning_option = Some(option.clone());
                winning_weight = *weight;
            }
        }

        let total_weight = vote_state.total_weight;
        let reached_quorum = total_weight >= proposal.quorum_weight;
        let reached_threshold = if total_weight == 0 {
            false
        } else {
            (u128::from(winning_weight) * 10_000_u128)
                >= (u128::from(total_weight) * u128::from(proposal.pass_threshold_bps))
        };
        let passed = reached_quorum && reached_threshold && winning_option.is_some();

        directives.push(LifecycleDirective::GovernanceFinalize {
            proposal_key,
            winning_option,
            winning_weight,
            total_weight,
            passed,
        });
    }

    directives
}

fn build_output(
    state: &GovernanceModuleState,
    directives: Vec<LifecycleDirective>,
) -> ModuleOutput {
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
        run_tick(&mut state, input.ctx.time)
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

export_wasm_module!(BuiltinWasmModule);
