import { render, screen, within } from "@solidjs/testing-library";
import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { createViewerFeedbackModule } from "./viewer_feedback_module.js";
import { buildControlProofAgencyDisplayModel, ControlProofPanel } from "./control_proof_panel.jsx";

const tr = (_locale, _zh, en) => en;
const backendSnapshotPath = process.env.OASIS7_AGENCY_BACKEND_SNAPSHOT;
const staleBackendSnapshotPath = process.env.OASIS7_AGENCY_STALE_BACKEND_SNAPSHOT;

describe.skipIf(!backendSnapshotPath)("actual restored Rust owner snapshot bridge", () => {
  it("preserves durable correction and canonical receipt identity through feedback and the rendered panel", () => {
    const snapshot = JSON.parse(readFileSync(backendSnapshotPath, "utf8"));
    const intent = snapshot.player_gameplay.primary_intent;
    const agency = intent.agency_read_model;
    const receipt = agency.causal_receipt;
    const applied = agency.memory_corrections.find((row) => row.correction_id === "correction-applied");
    const ignored = agency.memory_corrections.find((row) => row.status === "ignored");
    expect(applied.status).toBe("applied");
    expect(ignored).toBeDefined();
    expect(typeof applied.earliest_decision_request_id).toBe("string");
    expect(typeof applied.earliest_request_digest).toBe("string");
    expect(typeof applied.committed_decision_request_id).toBe("string");
    expect(typeof applied.committed_request_digest).toBe("string");
    // The latest context belongs to the failed follow-up; the prior applied
    // correction keeps its own committed identity and receipt.
    expect(agency.referenced_memory_context.used_for_decision).toBe(false);
    expect(ignored.active_decision_request_id).toBe(agency.referenced_memory_context.decision_request_id);
    expect(ignored.active_request_digest).toBe(agency.referenced_memory_context.request_digest);
    expect(applied.committed_decision_request_id).not.toBe(ignored.active_decision_request_id);
    expect(applied.runtime_receipt_id).toBe(receipt.receipt_id);
    expect(applied.action_id).toBe(`action:${receipt.action_id}`);
    expect(receipt.intent_id).toBe(intent.intent_id);
    expect(receipt.disposition).toBe("applied");
    expect(receipt.domain_event_refs.length).toBeGreaterThan(0);
    expect(receipt.correction_refs).toContain(applied.correction_id);
    const projected = createViewerFeedbackModule({
      clone: (value) => value == null ? value : JSON.parse(JSON.stringify(value)),
      feedbackBadgeClass: () => "feedback-badge", hostedActionPolicy: () => null,
      isAgentVisibleToCurrentSession: (agentId) => agentId === intent.agent_id,
      isLocaleZh: (locale) => locale === "zh", localeText: tr,
      state: { uiLocale: "en", snapshot },
    }).buildGameplaySummary().controlProof;
    expect(projected.intentId).toBe(intent.intent_id);
    expect(projected.agency).toEqual(agency);
    expect(projected.agency).not.toBe(agency);
    const model = buildControlProofAgencyDisplayModel(projected);
    expect(model.status).toBe("applied");
    expect(model.receiptId).toBe(receipt.receipt_id);
    expect(model.events).toEqual(receipt.domain_event_refs);
    expect(model.receiptAuthorization).toEqual(receipt.authorization);
    expect(model.authorizations.length).toBeGreaterThan(0);
    const usage = model.authorizations[0];
    expect(usage.grant.resource_kind).toBe("electricity");
    expect(usage.spent_units).toBe(receipt.authorization.spent_units);
    expect(model.corrections.find((row) => row.id === applied.correction_id)).toMatchObject({
      status: "applied", receipt: receipt.receipt_id, earliestDecision: applied.earliest_decision_request_id,
      earliestDigest: applied.earliest_request_digest,
      committedDecision: applied.committed_decision_request_id,
      committedDigest: applied.committed_request_digest,
    });
    expect(model.corrections.find((row) => row.id === ignored.correction_id)?.status).toBe("ignored");
    render(() => <ControlProofPanel proof={projected} locale="en" tr={tr} />);
    const panel = screen.getByTestId("control-proof-panel");
    expect(panel).toHaveAttribute("data-proof-status", "applied");
    expect(panel.textContent).toContain(receipt.receipt_id);
    expect(panel.textContent).toContain(receipt.commit_id);
    expect(panel.textContent).toContain(applied.correction_id);
    expect(panel.textContent).toContain(ignored.correction_id);
    expect(panel.textContent).toContain("Corrected context used in committed decision");
    expect(panel.textContent).toContain("Not used in committed result");
    expect(panel.textContent).toContain("Earliest prepared request");
    expect(panel.textContent).toContain(applied.earliest_decision_request_id);
    expect(panel.textContent).toContain("Committed decision request");
    expect(panel.textContent).toContain(applied.committed_decision_request_id);
    expect(panel.textContent).toContain(`${usage.spent_units} / ${usage.remaining_units} / ${usage.grant.limit_units} electricity units`);
    expect(panel.textContent).not.toContain("Display fixture");
  });
});

describe.skipIf(!staleBackendSnapshotPath)("actual stale-replacement owner snapshot bridge", () => {
  it("keeps the earliest prepared identity separate from the committed replacement and its receipt", () => {
    const snapshot = JSON.parse(readFileSync(staleBackendSnapshotPath, "utf8"));
    const intent = snapshot.player_gameplay.primary_intent;
    const agency = intent.agency_read_model;
    const correction = agency.memory_corrections.find((row) => row.status === "applied"
      && typeof row.earliest_decision_request_id === "string"
      && typeof row.committed_decision_request_id === "string"
      && row.earliest_decision_request_id !== row.committed_decision_request_id);
    const receipt = agency.causal_receipt;
    expect(correction).toBeDefined();
    expect(correction.earliest_request_digest).toBeTruthy();
    expect(correction.committed_request_digest).toBeTruthy();
    expect(correction.earliest_request_digest).not.toBe(correction.committed_request_digest);
    expect(agency.referenced_memory_context.used_for_decision).toBe(true);
    expect(agency.referenced_memory_context.decision_request_id).toBe(correction.committed_decision_request_id);
    expect(agency.referenced_memory_context.request_digest).toBe(correction.committed_request_digest);
    expect(correction.runtime_receipt_id).toBe(receipt.receipt_id);
    expect(correction.action_id).toBe(`action:${receipt.action_id}`);
    expect(receipt.intent_id).toBe(intent.intent_id);
    expect(receipt.disposition).toBe("applied");
    expect(receipt.domain_event_refs.length).toBeGreaterThan(0);
    expect(receipt.correction_refs).toContain(correction.correction_id);

    const projected = createViewerFeedbackModule({
      clone: (value) => value == null ? value : JSON.parse(JSON.stringify(value)),
      feedbackBadgeClass: () => "feedback-badge", hostedActionPolicy: () => null,
      isAgentVisibleToCurrentSession: (agentId) => agentId === intent.agent_id,
      isLocaleZh: (locale) => locale === "zh", localeText: tr,
      state: { uiLocale: "en", snapshot },
    }).buildGameplaySummary().controlProof;
    const model = buildControlProofAgencyDisplayModel(projected);
    expect(model.corrections.find((row) => row.id === correction.correction_id)).toMatchObject({
      status: "applied", earliestDecision: correction.earliest_decision_request_id,
      earliestDigest: correction.earliest_request_digest,
      committedDecision: correction.committed_decision_request_id,
      committedDigest: correction.committed_request_digest,
      receipt: receipt.receipt_id,
    });
    render(() => <ControlProofPanel proof={projected} locale="en" tr={tr} />);
    const panel = screen.getByTestId("control-proof-panel");
    expect(panel.textContent).toContain(correction.earliest_decision_request_id);
    expect(panel.textContent).toContain(correction.committed_decision_request_id);
    expect(correction.earliest_decision_request_id).not.toBe(correction.committed_decision_request_id);
    expect(panel.textContent).toContain(receipt.receipt_id);
  });
});
function proof() {
  return {
    intent: "Deliver feedstock", intentId: "intent-owner-1", agentId: "agent-owner-1",
    state: "accepted", recovery: "Request fresh quote", nextMove: "Read runtime guidance",
    agency: {
      status: "available",
      causal_receipt: {
        receipt_id: "receipt-actual-7", commit_id: "commit-7", intent_id: "intent-owner-1",
        effect_intent_id: "effect-7", action_id: 7, action_kind: "move_agent",
        domain_event_refs: [13], disposition: "applied", primary_reason: "route chosen after quota check",
        next_step: "observe_domain_result",
        actual_consequence: [{ type: "agent_moved", private_prompt: "do-not-show" }],
        expected_consequence: { provenance: "agent_explanation_unverified", prediction: "production will become available" },
        stakes: { provenance: "agent_explanation_unverified", summary: "preserve scarce material" },
        alternative: { provenance: "agent_explanation_unverified", alternatives: ["wait for more material"] },
        evidence_refs: ["observation-capacity"], correction_refs: ["correction-1"], interruption_refs: [], owner_control_refs: ["owner-override-1"],
      },
      delegation_authorizations: [{ grant: {
        agent_id: "agent-owner-1", grant_id: "grant-1", issuer_id: "owner-1", source_id: "owner-source",
        object_id: "factory-1", action_kinds: ["move_agent"], revision: 2, period_id: "period-1",
        valid_from_tick: 1, valid_until_tick: 20, limit_units: 10, resource_kind: "electricity", revoked: false,
      }, cost_units: 2, spent_units: 6, remaining_units: 4 }],
      referenced_memory_context: {
        revision: 3, scope: "session_private", source: "private_memory_retrieval_context", used_for_decision: true,
        stale: false, correction_hint: "correct_by_memory_id_and_revision",
        entries: [{ id: "memory-1", summary: "capacity was previously unavailable" }],
        sources: [{ memory_id: "memory-1", origin_receipt_id: "receipt-memory-source", correction_refs: ["correction-1"] }],
      },
      memory_corrections: [{ correction_id: "correction-1", agent_id: "agent-owner-1", status: "applied",
        reason: "corrected_context_committed_decision", memory_revision: 3,
        earliest_decision_request_id: "decision-after-correction", runtime_receipt_id: "receipt-actual-7", action_id: "7" }],
    },
  };
}

describe("ControlProofPanel", () => {
  it("keeps Agent predictions separate from a canonical committed result and shows grant and correction continuity", () => {
    render(() => <ControlProofPanel proof={proof()} locale="en" tr={tr} />);
    const panel = screen.getByTestId("control-proof-panel");
    expect(panel).toHaveAttribute("data-proof-status", "applied");
    expect(within(panel).getByText("Actual world result")).toBeInTheDocument();
    expect(within(panel).getByText("agent moved")).toBeInTheDocument();
    const prediction = within(panel).getByTestId("control-proof-prediction");
    expect(within(prediction).getByText("Agent prediction (unverified)")).toBeInTheDocument();
    expect(within(prediction).getByText("production will become available")).toBeInTheDocument();
    expect(within(panel).getByText("receipt-actual-7")).toBeInTheDocument();
    expect(within(panel).getByText("6 / 4 / 10 electricity units")).toBeInTheDocument();
    expect(within(panel).getByText(/Corrected context used in committed decision/)).toBeInTheDocument();
    expect(within(panel).getByText("decision-after-correction")).toBeInTheDocument();
    expect(within(panel).getByText("owner-override-1")).toBeInTheDocument();
    expect(panel.textContent).not.toContain("do-not-show");
  });

  it.each(["accepted without receipt", "wrong intent receipt", "applied without event references"])("never proves a world effect for %s", (kind) => {
    const input = proof();
    input.state = "completed";
    input.summary = "Control proved by an Agent prediction";
    if (kind === "accepted without receipt") input.agency.causal_receipt = null;
    if (kind === "wrong intent receipt") input.agency.causal_receipt.intent_id = "other-intent";
    if (kind === "applied without event references") input.agency.causal_receipt.domain_event_refs = [];
    render(() => <ControlProofPanel proof={input} locale="en" tr={tr} />);
    const panel = screen.getByTestId("control-proof-panel");
    expect(panel).toHaveAttribute("data-proof-status", "unavailable");
    expect(within(panel).getByText("Awaiting authoritative receipt")).toBeInTheDocument();
    expect(panel.textContent).not.toContain("World effect committed");
    expect(panel.textContent).not.toContain("Control proved by an Agent prediction");
    if (kind === "wrong intent receipt") expect(panel.textContent).not.toContain("production will become available");
  });

  it("preserves rejection, stale and ignored correction reasons without claiming prediction success", () => {
    const input = proof();
    input.agency.causal_receipt.disposition = "not_applied";
    input.agency.causal_receipt.actual_consequence = [{ type: "action_rejected" }];
    input.agency.causal_receipt.dissent = "Agent objects to this route";
    input.agency.causal_receipt.override_actor = "current-owner-1";
    input.agency.causal_receipt.hard_boundary = "World resource limit reached";
    input.primaryNextStep = "replan_required";
    expect(buildControlProofAgencyDisplayModel(input).nextStep).toBe("replan required");
    input.agency.memory_corrections = ["accepted", "stale", "ignored"].map((status) => ({
      agent_id: "agent-owner-1", correction_id: `correction-${status}`, status,
      reason: "memory_revision_changed", memory_revision: 3,
    }));
    input.agency.memory_corrections.push({ agent_id: "other-agent", correction_id: "private-other-correction", status: "applied" });
    render(() => <ControlProofPanel proof={input} locale="en" tr={tr} />);
    const panel = screen.getByTestId("control-proof-panel");
    expect(panel).toHaveAttribute("data-proof-status", "not_applied");
    expect(within(panel).getAllByTestId("control-proof-correction")).toHaveLength(3);
    expect(panel.textContent).toContain("Revision stale");
    expect(panel.textContent).toContain("memory revision changed");
    expect(panel.textContent).not.toContain("private-other-correction");
    expect(panel.textContent).toContain("Agent objects to this route");
    expect(panel.textContent).toContain("current-owner-1");
    expect(panel.textContent).toContain("World resource limit reached");
    expect(panel.textContent).not.toContain("World effect committed");
  });

  it("renders old snapshots as unavailable and never reads arbitrary private metadata", () => {
    const legacy = {
      intent: "legacy instruction", intentId: "legacy-intent", agentId: "legacy-agent", state: "accepted",
      agency: { memory_corrections: [{ correction_id: "legacy-correction", agent_id: "legacy-agent",
        status: "applied", earliest_decision_request_id: "legacy-prepared-request" }] },
    };
    render(() => <ControlProofPanel proof={legacy} locale="en" tr={tr} />);
    const panel = screen.getByTestId("control-proof-panel");
    expect(panel).toHaveAttribute("data-proof-status", "unavailable");
    expect(within(panel).getByText("Detailed causal proof is unavailable; read the runtime next step.")).toBeInTheDocument();
    expect(panel.textContent).toContain("legacy-prepared-request");
    expect(within(panel).getByText("Committed decision request").parentElement.textContent).toContain("Unavailable");
    expect(buildControlProofAgencyDisplayModel({ intentId: "intent-1", agentId: "agent-1", agency: { private_prompt: "private-data" } }).prediction).toBeNull();
  });

  it("does not reinterpret foreign or unsupported grant budgets as electricity", () => {
    const input = proof();
    input.agency.delegation_authorizations[0].grant.resource_kind = "risk_count";
    input.agency.delegation_authorizations.push({ grant: { agent_id: "other-agent", resource_kind: "electricity", grant_id: "foreign-grant" } });
    const model = buildControlProofAgencyDisplayModel(input);
    expect(model.authorizations).toEqual([]);
    render(() => <ControlProofPanel proof={input} locale="en" tr={tr} fixture />);
    const panel = screen.getByTestId("control-proof-panel");
    expect(panel.textContent).toContain("Display fixture: no runtime execution evidence.");
    expect(panel.textContent).not.toContain("risk_count");
    expect(panel.textContent).not.toContain("foreign-grant");
  });

  it("never rounds unsafe native electricity unit values into a displayed budget", () => {
    const input = proof();
    const authorization = input.agency.delegation_authorizations[0];
    authorization.grant.limit_units = Number.MAX_SAFE_INTEGER + 1;
    authorization.spent_units = Number.MAX_SAFE_INTEGER + 3;
    authorization.remaining_units = Number.MAX_SAFE_INTEGER + 5;
    authorization.cost_units = Number.MAX_SAFE_INTEGER + 7;
    input.agency.causal_receipt.authorization = {
      grant: authorization.grant, cost_units: Number.MAX_SAFE_INTEGER + 7,
      spent_units: Number.MAX_SAFE_INTEGER + 3, remaining_units: Number.MAX_SAFE_INTEGER + 5,
    };
    render(() => <ControlProofPanel proof={input} locale="en" tr={tr} />);
    const panel = screen.getByTestId("control-proof-panel");
    expect(panel.textContent).toContain("Unavailable / Unavailable / Unavailable electricity units");
    expect(panel.textContent).toContain("Unavailable electricity units");
    expect(panel.textContent).not.toContain(String(Number.MAX_SAFE_INTEGER + 1));
    expect(panel.textContent).not.toContain(String(Number.MAX_SAFE_INTEGER + 7));
  });
});
