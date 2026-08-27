import { waitFor, within } from "@solidjs/testing-library";
import { afterEach, describe, expect, it } from "vitest";
import { buildTaskGame076ScenarioSnapshot } from "./gameplay_attraction_scenario.js";
import {
  AGENT_ACTIVITY_STATUSES,
  AGENT_INTENT_STATUSES,
  buildAgentIntentFixtureSnapshot,
} from "./agent_intent_visual_fixture.js";
import { renderViewerApp } from "./test_support/viewer_app_fixture.jsx";

const AGENT_ID = "agent-0";
const INTENT_SUMMARY = "Stabilize power before expanding the iron line.";

function acceptedIntent(overrides = {}) {
  return {
    schema_version: 2,
    intent_id: "agent-intent-v2:test",
    agent_id: AGENT_ID,
    world_id: "live-runtime-test",
    reorg_epoch: 0,
    status: "accepted",
    message: INTENT_SUMMARY,
    resume_required: false,
    source_class: "runtime_projection",
    freshness: "current",
    control_state: "controllable",
    logical_time: 7,
    event_seq: "11",
    updated_at: 7,
    ...overrides,
  };
}

function snapshotWithIntent(primaryIntent, activity = { status: "executing", operation: "resource_recovery", target: "factory-activity-target", updated_at: 7 }) {
  const base = buildTaskGame076ScenarioSnapshot();
  return {
    ...base,
    model: {
      ...base.model,
      agent_player_bindings: { [AGENT_ID]: "local-test-player-bound" },
      agent_player_public_key_bindings: { [AGENT_ID]: "abcdef0123456789abcdef0123456789" },
      agents: {
        ...base.model.agents,
        [AGENT_ID]: { ...base.model.agents[AGENT_ID], activity },
      },
    },
    player_gameplay: { ...base.player_gameplay, primary_intent: primaryIntent },
  };
}

let dispose = null;

afterEach(() => {
  dispose?.();
  dispose = null;
  document.body.innerHTML = "";
});

async function commandSurfaceFor(snapshot) {
  const app = await renderViewerApp(snapshot);
  dispose = app.dispose;
  app.core.applySelection({ kind: "agent", id: AGENT_ID });
  const surface = app.container.querySelector("#viewer-details-panel .command-surface");
  await waitFor(() => expect(surface?.querySelector(".agent-intent")).toBeInTheDocument());
  return { app, surface, intentSurface: surface.querySelector(".agent-intent") };
}

describe("Agent Console V2 authoritative intent", () => {
  it("renders every canonical lifecycle status through one connected deterministic surface", async () => {
    for (const status of AGENT_INTENT_STATUSES) {
      const receiptRef = status === "completed"
        ? {
          intent_id: "agent-intent-v2:test",
          world_id: "live-runtime-test",
          reorg_epoch: 0,
          logical_time: 7,
          event_seq: "11",
          receipt_id: "world-event:12",
        }
        : null;
      const { intentSurface } = await commandSurfaceFor(snapshotWithIntent(acceptedIntent({
        status,
        receipt_ref: receiptRef,
        replaced_by: status === "superseded" ? "agent-intent-v2:replacement" : null,
      })));
      expect(intentSurface).toHaveAttribute("data-agent-intent-state", "current");
      const statusLabel = {
        proposed: "Proposed",
        submitted: "Submitted",
        accepted: "Accepted",
        blocked: "Blocked",
        completed: "Completed",
        rejected: "Rejected",
        expired: "Expired",
        cancelled: "Cancelled",
        superseded: "Replaced",
      }[status];
      expect(within(intentSurface).getByText(statusLabel)).toBeInTheDocument();
      expect(intentSurface).toHaveTextContent(INTENT_SUMMARY);
      expect(intentSurface.textContent).not.toMatch(/agent-intent-v2|live-runtime-test|world-event:/);
      dispose?.();
      dispose = null;
    }
  }, 60000);

  it("keeps missing and malformed intent unavailable instead of inferring a plan", async () => {
    const { intentSurface } = await commandSurfaceFor(snapshotWithIntent(null));
    expect(intentSurface).toHaveAttribute("data-agent-intent-state", "unavailable");
    expect(within(intentSurface).getByText("Intent unavailable")).toBeInTheDocument();
    expect(intentSurface).not.toHaveTextContent(/Idle|current plan|last_active|provider/i);
  });

  it("shows stale and conflicting authority as caution states while hiding control boundaries", async () => {
    for (const [freshness, label] of [["stale", "Stale intent"], ["conflict", "Needs confirmation"]]) {
      const { intentSurface } = await commandSurfaceFor(snapshotWithIntent(acceptedIntent({ freshness })));
      expect(intentSurface).toHaveAttribute("data-agent-intent-state", freshness);
      expect(within(intentSurface).getByText(label)).toBeInTheDocument();
      expect(intentSurface).toHaveTextContent(INTENT_SUMMARY);
      dispose?.();
      dispose = null;
    }

    for (const controlState of ["control_lost", "read_only", "unauthorized"]) {
      const { intentSurface } = await commandSurfaceFor(snapshotWithIntent(acceptedIntent({ control_state: controlState })));
      expect(intentSurface).toHaveAttribute("data-agent-intent-state", controlState);
      expect(intentSurface).not.toHaveTextContent(INTENT_SUMMARY);
      expect(intentSurface.textContent).not.toMatch(/runtime_projection|agent-intent-v2|live-runtime-test/);
      dispose?.();
      dispose = null;
    }
  }, 60000);

  it("requires a matching world receipt before exposing completion", async () => {
    const validReceipt = {
      intent_id: "agent-intent-v2:test",
      world_id: "live-runtime-test",
      reorg_epoch: 0,
      logical_time: 7,
      event_seq: "11",
      receipt_id: "world-event:12",
    };
    const valid = await commandSurfaceFor(snapshotWithIntent(acceptedIntent({ status: "completed", receipt_ref: validReceipt })));
    expect(valid.intentSurface).toHaveAttribute("data-agent-intent-receipt-state", "confirmed");
    expect(valid.intentSurface).toHaveTextContent("World receipt confirmed");
    dispose?.();
    dispose = null;

    const missing = await commandSurfaceFor(snapshotWithIntent(acceptedIntent({ status: "completed", receipt_ref: null })));
    expect(missing.intentSurface).toHaveAttribute("data-agent-intent-state", "unavailable");
    expect(missing.intentSurface).toHaveAttribute("data-agent-intent-receipt-state", "missing");
    expect(missing.intentSurface).not.toHaveTextContent("Completed");
  }, 60000);

  it("keeps duplicate and replacement dispositions explicit without rendering internal identities", async () => {
    const duplicate = await commandSurfaceFor(snapshotWithIntent(acceptedIntent({ duplicate: true, reason_code: "duplicate_request" })));
    expect(duplicate.intentSurface).toHaveTextContent("Duplicate request coalesced; no new intent was created.");
    expect(duplicate.intentSurface.textContent).not.toMatch(/agent-intent-v2|intent_id|world_id/);
    dispose?.();
    dispose = null;

    const replacement = await commandSurfaceFor(snapshotWithIntent(acceptedIntent({
      status: "superseded",
      replaced_by: "agent-intent-v2:replacement",
      reason_code: "superseded_by_replacement",
    })));
    expect(replacement.intentSurface).toHaveTextContent("Replaced by a newer intent");
    expect(replacement.intentSurface.textContent).not.toMatch(/agent-intent-v2|replacement/);
  });

  it("covers the activity matrix and reports missing activity as unavailable", async () => {
    for (const status of AGENT_ACTIVITY_STATUSES) {
      const activity = status === "missing" ? null : {
        status,
        operation: status === "idle" ? null : "resource_recovery",
        target: status === "idle" ? null : "factory-activity-target",
        reason: status === "blocked" ? "upstream material is not ready" : null,
        updated_at: 7,
      };
      const app = await renderViewerApp(snapshotWithIntent(acceptedIntent(), activity));
      dispose = app.dispose;
      app.core.applySelection({ kind: "agent", id: AGENT_ID });
      const activitySurfaces = [...app.container.querySelectorAll(".agent-activity")];
      await waitFor(() => expect(activitySurfaces.length).toBeGreaterThanOrEqual(2));
      expect(activitySurfaces[0]).toHaveAttribute("data-agent-activity-state", status === "missing" ? "unavailable" : status);
      expect(activitySurfaces[1]).toHaveAttribute("data-agent-activity-state", status === "missing" ? "unavailable" : status);
      expect(app.container.textContent).not.toMatch(/factory-activity-target|last_active|status_[a-z_]+/i);
      dispose?.();
      dispose = null;
    }
  }, 60000);

  it("keeps the headed fixture connected and deterministic across its query matrix", () => {
    const state = {
      status: "superseded",
      freshness: "conflict",
      controlState: "controllable",
      activityStatus: "blocked",
      receiptState: "missing",
      variant: "replacement",
      connectionStatus: "connected",
    };
    const first = buildAgentIntentFixtureSnapshot(() => buildTaskGame076ScenarioSnapshot(), state);
    const second = buildAgentIntentFixtureSnapshot(() => buildTaskGame076ScenarioSnapshot(), state);
    expect(first).toEqual(second);
    expect(first.player_gameplay.primary_intent.status).toBe("superseded");
    expect(first.model.agents[AGENT_ID].activity.status).toBe("blocked");
    expect(state.connectionStatus).toBe("connected");
    expect(JSON.stringify(first.player_gameplay.primary_intent)).toContain("replaced_by");
  });
});
