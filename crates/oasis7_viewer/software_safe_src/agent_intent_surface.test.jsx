import { cleanup, render, screen } from "@solidjs/testing-library";
import { afterEach, describe, expect, it } from "vitest";
import {
  AgentIntentSurface,
  PLAYER_SAFE_COPY_SCHEMA_VERSION,
  describeAgentIntent,
} from "./agent_intent_surface.jsx";

const BASE_INTENT = {
  schema_version: 2,
  copy_schema_version: PLAYER_SAFE_COPY_SCHEMA_VERSION,
  intent_id: "intent-internal-test",
  agent_id: "agent-internal-test",
  world_id: "world-internal-test",
  reorg_epoch: 0,
  status: "accepted",
  message: "Agent guidance accepted; the Agent will evaluate its next world action.",
  source_class: "runtime_projection",
  freshness: "current",
  control_state: "controllable",
  logical_time: 7,
  event_seq: "11",
  updated_at: 7,
};

function intent(overrides = {}) {
  return { ...BASE_INTENT, ...overrides };
}

function renderIntent(value, connectionStatus = "connected") {
  const view = render(() => <AgentIntentSurface intent={value} connectionStatus={connectionStatus} locale="en" />);
  return view.container.querySelector(".agent-intent");
}

afterEach(() => cleanup());

describe("AgentIntentSurface fail-closed player copy boundary", () => {
  it("accepts only the versioned canonical summary for every lifecycle status", () => {
    const expected = {
      proposed: "Agent guidance is proposed and not yet accepted.",
      submitted: "Agent guidance was submitted and awaits runtime acceptance.",
      accepted: "Agent guidance accepted; the Agent will evaluate its next world action.",
      blocked: "Agent guidance is blocked pending a runtime recheck.",
      completed: "Agent guidance completed with a confirmed world receipt.",
      rejected: "Agent guidance was rejected by runtime authority.",
      expired: "Agent guidance expired before execution.",
      cancelled: "Agent guidance was cancelled before completion.",
      superseded: "Agent guidance was replaced by newer guidance.",
    };
    for (const [status, message] of Object.entries(expected)) {
      const model = describeAgentIntent(intent({
        status,
        message,
        ...(status === "completed" ? {
          receipt_ref: {
            intent_id: "intent-internal-test",
            world_id: "world-internal-test",
            reorg_epoch: 0,
            logical_time: 7,
            event_seq: "11",
            receipt_id: "world-event:12",
          },
        } : {}),
      }));
      expect(model.message).toBe(message);
    }
  });

  it.each([
    ["unknown summary", { message: "provider rationale: do whatever the model says" }],
    ["missing summary", { message: "" }],
    ["unknown copy version", { copy_schema_version: 99 }],
    ["unknown reason", { status: "blocked", reason_code: "provider_rationale" }],
    ["unknown reason copy", { status: "accepted", reason_code: "agent_unavailable", reason_summary: "provider rationale: hidden internal detail" }],
    ["unknown next-step key", { next_step_key: "call_provider" }],
    ["unknown next-step copy", { status: "blocked", next_step: "Ask the provider for a hidden trace." }],
  ])("fails closed for %s and supplies a safe recovery step", (_label, overrides) => {
    const model = describeAgentIntent(intent(overrides));
    expect(model.kind).toBe("unavailable");
    expect(model.message).toBeUndefined();
    expect(model.nextStep).toBe("Stop and refresh the world snapshot before retrying.");
  });

  it.each([
    ["blocked", "insufficient_power", "Restore power to continue", "Recheck runtime state before resuming."],
    ["rejected", "policy_denied", "This instruction is not permitted", "Review the latest world state before retrying."],
    ["blocked", "provider_unavailable", "Agent service is temporarily unavailable", "Recheck runtime state before resuming."],
    ["rejected", "provider_rejected", "Agent service rejected this instruction", "Review the latest world state before retrying."],
  ])("renders canonical %s reason and next step for %s", (status, reasonCode, reasonSummary, nextStep) => {
    const value = intent({
      status,
      message: status === "blocked"
        ? "Agent guidance is blocked pending a runtime recheck."
        : "Agent guidance was rejected by runtime authority.",
      reason_code: reasonCode,
      reason_summary: reasonSummary,
    });
    const model = describeAgentIntent(value);
    expect(model.kind).toBe("current");
    expect(model.statusLabel).toBe(status === "blocked" ? "Blocked" : "Rejected");
    expect(model.reasonSummary).toBe(reasonSummary);
    expect(model.nextStep).toBe(nextStep);
    const surface = renderIntent(value);
    expect(surface).toHaveTextContent(model.statusLabel);
    expect(surface).toHaveTextContent(reasonSummary);
    expect(surface).toHaveTextContent(nextStep);
    expect(surface).not.toHaveTextContent("Intent unavailable");
  });

  it.each([
    ["missing intent", null, "Stop and refresh the world snapshot before retrying."],
    ["legacy schema", intent({ schema_version: 1 }), "Stop and refresh the world snapshot before retrying."],
    ["missing receipt", intent({ status: "completed", message: "Agent guidance completed with a confirmed world receipt.", receipt_ref: null }), "Wait for a committed world receipt, then refresh."],
    ["offline connection", intent(), "Stop and refresh the world snapshot before retrying."],
  ])("renders actionable next step for %s", (_label, value, nextStep) => {
    const model = _label === "offline connection"
      ? describeAgentIntent(value, "en", "disconnected")
      : describeAgentIntent(value);
    expect(model.kind).toBe("unavailable");
    expect(model.nextStep).toBe(nextStep);
    const surface = renderIntent(value, _label === "offline connection" ? "disconnected" : "connected");
    expect(surface).toHaveTextContent(nextStep);
  });

  it.each([
    ["stale", "Refresh the world state before acting."],
    ["conflict", "Review the latest world state and reselect an intent."],
    ["reconnecting", "Wait for the runtime connection to recover."],
  ])("keeps %s as a caution state with an allowlisted next step", (freshness, nextStep) => {
    const model = describeAgentIntent(intent({ freshness }));
    expect(model.kind).toBe(freshness);
    expect(model.nextStep).toBe(nextStep);
    const surface = renderIntent(intent({ freshness }));
    expect(surface).toHaveTextContent(nextStep);
  });

  it.each([
    ["control_lost", "Intent hidden — control lost", "Reselect the Agent after control is restored."],
    ["read_only", "Intent hidden — read-only", "Reselect the Agent in a controllable session."],
    ["unauthorized", "Intent hidden — unauthorized", "Request access before viewing this intent."],
  ])("hides summary for %s while preserving safe recovery", (controlState, label, nextStep) => {
    const surface = renderIntent(intent({ control_state: controlState }));
    expect(surface).toHaveTextContent(label);
    expect(surface).toHaveTextContent(nextStep);
    expect(surface).not.toHaveTextContent(BASE_INTENT.message);
    expect(surface.textContent).not.toMatch(/intent-internal-test|world-internal-test|runtime_projection/);
  });

  it("renders blocked reason from the allowlist, never from arbitrary runtime text", () => {
    const surface = renderIntent(intent({
      status: "blocked",
      message: "Agent guidance is blocked pending a runtime recheck.",
      reason_code: "missing_material",
      reason_summary: "World prerequisites changed before execution.",
      next_step: "Recheck runtime state before resuming.",
    }));
    expect(surface).toHaveTextContent("World prerequisites changed before execution.");
    expect(surface).toHaveTextContent("Recheck runtime state before resuming.");
    expect(surface).not.toHaveTextContent("provider rationale");
  });

  it("keeps receipt confirmation semantic and never exposes receipt identity", () => {
    const value = intent({
      status: "completed",
      message: "Agent guidance completed with a confirmed world receipt.",
      receipt_ref: {
        intent_id: "intent-internal-test",
        world_id: "world-internal-test",
        reorg_epoch: 0,
        logical_time: 7,
        event_seq: "11",
        receipt_id: "world-event:12",
      },
    });
    const surface = renderIntent(value);
    expect(surface).toHaveTextContent("World receipt confirmed");
    expect(surface.textContent).not.toMatch(/intent-internal-test|world-internal-test|world-event:12/);
    expect(screen.getByText("World receipt confirmed")).toBeInTheDocument();
  });
});
