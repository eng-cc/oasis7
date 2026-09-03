use oasis7::capability_invocation_context::CapabilityInvocationContext;
use oasis7::simulator::{
    BudgetContractV1, COGNITION_CAPABILITY_CATALOG_DOMAIN,
    COGNITION_CAPABILITY_INVOCATION_CONTEXT_DOMAIN, CONTINUOUS_AGENT_CONTEXT_DISCRIMINATOR,
    CONTINUOUS_AGENT_CONTEXT_VERSION, ContinuousAgentRequestContextV1,
    ContinuousAgentTurnContextV1, DecisionRequest, Digest32, GoalSnapshotInputV1,
    GoalSnapshotProjector, MemoryContextSnapshotV1, Observation, RuntimeBindingV1, h_v1,
};
use oasis7_wasm_abi::CapabilityCatalogSnapshot;
use serde_json::json;

use super::DEFAULT_PROTOCOL_VERSION;

pub(super) fn build_target_context(
    mut base_decision_request: DecisionRequest,
    observation: &Observation,
    fixture_id: &str,
    session_id: &str,
    turn: u64,
) -> (
    ContinuousAgentTurnContextV1,
    ContinuousAgentRequestContextV1,
) {
    let world_id = format!("parity-world-{fixture_id}");
    let branch_id = "main".to_string();
    let logical_tick = observation.time;
    let agent_id = observation.agent_id.clone();
    let turn_id = format!("{fixture_id}-turn-{turn}");
    let request_id = format!("{fixture_id}-request-{turn}");
    let runtime_binding = RuntimeBindingV1 {
        world_id: world_id.clone(),
        branch_id: branch_id.clone(),
        finality_epoch: 0,
        finality_block_hash: None,
        finality_status: "pending".to_string(),
        base_tick: logical_tick,
        base_world_hash: h_v1(
            "oasis7.parity.world-binding.v1",
            &base_decision_request.observation,
        ),
        reorg_epoch: 0,
        runtime_manifest_hash: h_v1(
            "oasis7.parity.runtime-manifest.v1",
            &json!({"fixture_id": fixture_id, "branch_id": branch_id}),
        ),
    };
    let subject = json!({
        "kind": "agent",
        "agent_id": agent_id.clone(),
        "owner_binding": "parity-benchmark",
        "generation": 1
    });
    let presenter = json!({
        "presenter_id": "oasis7_provider_parity_bench",
        "presenter_kind": "provider",
        "session_id": session_id
    });
    let audience = json!({
        "world_id": world_id,
        "branch_id": branch_id,
        "finality_epoch": 0,
        "target_kind": "world",
        "target_id": null
    });
    let catalog: CapabilityCatalogSnapshot = serde_json::from_value(json!({
        "snapshot_id": format!("{fixture_id}-catalog-{turn}"),
        "world_id": runtime_binding.world_id.clone(),
        "world_head": logical_tick,
        "branch_id": runtime_binding.branch_id.clone(),
        "finality_epoch": runtime_binding.finality_epoch,
        "logical_tick": logical_tick,
        "module_registry_hash": h_v1("oasis7.parity.module-registry.v1", &fixture_id),
        "policy_hash": h_v1("oasis7.parity.policy.v1", &fixture_id),
        "revocation_epoch": 0,
        "subject": subject,
        "presenter": presenter,
        "audience": audience,
        "entries": [],
        "valid_until_tick": logical_tick
    }))
    .expect("parity capability catalog");
    let invocation = CapabilityInvocationContext {
        grant_id: format!("{fixture_id}-grant-{turn}"),
        subject: catalog.subject.clone(),
        presenter: catalog.presenter.clone(),
        audience: catalog.audience.clone(),
        catalog_snapshot_id: catalog.snapshot_id.clone(),
        module_id: String::new(),
        module_version: String::new(),
        response_nonce: format!("{fixture_id}-nonce-{turn}"),
    };
    base_decision_request.capability_catalog = Some(catalog.clone());
    base_decision_request.capability_invocation_context = Some(invocation.clone());
    let mut memory_snapshot = MemoryContextSnapshotV1 {
        revision: turn,
        entries: Vec::new(),
        scope: format!("agent:{agent_id}"),
        digest: String::new(),
    };
    memory_snapshot.digest = memory_snapshot.computed_digest();
    let goal_snapshot = GoalSnapshotProjector::project(
        Some(GoalSnapshotInputV1 {
            revision: turn,
            short_term_summary: "preserve deterministic forward progress".to_string(),
            long_term_summary: format!("complete parity fixture {fixture_id}"),
            blocked_reason: None,
            provenance: "harness_projection".to_string(),
        }),
        None,
    )
    .expect("parity goal snapshot");
    let mut request_context = ContinuousAgentRequestContextV1 {
        base_decision_request,
        context_discriminator: CONTINUOUS_AGENT_CONTEXT_DISCRIMINATOR.to_string(),
        context_version: CONTINUOUS_AGENT_CONTEXT_VERSION,
        protocol_version: "world-simulator-provider-loopback-http-v1".to_string(),
        agent_session_id: session_id.to_string(),
        agent_turn_id: turn_id.clone(),
        decision_request_id: request_id.clone(),
        retry_seq: 1,
        transport_attempt: turn.saturating_add(1),
        agent_subject: observation.agent_id.clone(),
        runtime_binding,
        observation_digest: Digest32::default(),
        capability_catalog_digest: h_v1(COGNITION_CAPABILITY_CATALOG_DOMAIN, &catalog),
        capability_invocation_context_digest: h_v1(
            COGNITION_CAPABILITY_INVOCATION_CONTEXT_DOMAIN,
            &invocation,
        ),
        memory_snapshot_digest: Digest32::default(),
        goal_snapshot_digest: goal_snapshot.digest.clone().into(),
        continuation_digest: h_v1("oasis7.parity.continuation.v1", &Option::<()>::None),
        adapter_protocol_version: DEFAULT_PROTOCOL_VERSION.to_string(),
        budget_contract: BudgetContractV1 {
            max_latency_ms: 15_000,
            max_repair_attempts: 1,
        },
        request_digest: Digest32::default(),
    };
    request_context.observation_digest = h_v1(
        "oasis7.parity.observation.v1",
        &request_context.base_decision_request.observation,
    );
    request_context.memory_snapshot_digest = memory_snapshot.digest.clone().into();
    request_context.request_digest = request_context.request_digest();
    let turn_context = ContinuousAgentTurnContextV1 {
        agent_id: observation.agent_id.clone(),
        agent_session_id: session_id.to_string(),
        agent_turn_id: turn_id,
        decision_request_id: request_id,
        request_digest: request_context.request_digest.clone(),
        memory_snapshot,
        goal_snapshot,
        continuation: None,
    };
    (turn_context, request_context)
}
