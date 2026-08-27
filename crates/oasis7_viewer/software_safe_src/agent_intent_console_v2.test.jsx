import { waitFor, within } from "@solidjs/testing-library";
import { afterEach, describe, expect, it } from "vitest";
import { buildTaskGame076ScenarioSnapshot } from "./gameplay_attraction_scenario.js";
import { renderViewerApp } from "./test_support/viewer_app_fixture.jsx";

const AGENT_ID = "agent-0";

function intentSnapshot(primaryIntent) {
  const base = buildTaskGame076ScenarioSnapshot();
  return {
    ...base,
    model: {
      ...base.model,
      agent_player_bindings: { [AGENT_ID]: "local-test-player-bound" },
      agent_player_public_key_bindings: { [AGENT_ID]: "abcdef0123456789abcdef0123456789" },
    },
    player_gameplay: {
      ...base.player_gameplay,
      primary_intent: primaryIntent,
    },
  };
}

function acceptedIntent(overrides = {}) {
  return {
    schema_version: 2,
    intent_id: "agent-intent-v2:test",
    agent_id: AGENT_ID,
    world_id: "live-runtime-test",
    reorg_epoch: 0,
    status: "accepted",
    message: "Stabilize power before expanding the iron line.",
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

function completedIntent(overrides = {}) {
  return acceptedIntent({
    status: "completed",
    effect_intent_id: "effect-intent-v2:test",
    receipt_ref: {
      intent_id: "agent-intent-v2:test",
      world_id: "live-runtime-test",
      reorg_epoch: 0,
      logical_time: 7,
      event_seq: "11",
      receipt_id: "world-event:12",
    },
    ...overrides,
  });
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
  return app.container.querySelector("#viewer-details-panel .command-surface");
}

describe("Agent Console V2 authoritative intent", () => {
  it("renders accepted current intent separately from activity and receipt", async () => {
    const surface = await commandSurfaceFor(intentSnapshot(acceptedIntent()));
    await waitFor(() => expect(within(surface).getByText("Current Intent")).toBeInTheDocument());
    expect(surface).toHaveTextContent("Accepted");
    expect(surface).toHaveTextContent("Current");
    expect(surface).toHaveTextContent("Stabilize power before expanding the iron line.");
    expect(surface).not.toHaveTextContent(/runtime_projection|agent-intent-v2|last_active|provider|rationale/i);
  }, 60000);

  it("keeps missing intent unavailable instead of inferring idle or plan", async () => {
    const surface = await commandSurfaceFor(intentSnapshot(null));
    await waitFor(() => expect(within(surface).getByText("Intent unavailable")).toBeInTheDocument());
    expect(surface).not.toHaveTextContent(/\bIdle\b|current plan|last_active|accepted_new/i);
  }, 60000);

  it("marks retained intent stale without presenting completion", async () => {
    const surface = await commandSurfaceFor(intentSnapshot(acceptedIntent({ freshness: "stale" })));
    await waitFor(() => expect(within(surface).getByText("Stale intent")).toBeInTheDocument());
    expect(surface).toHaveTextContent("Stabilize power before expanding the iron line.");
    expect(surface).not.toHaveTextContent(/Completed|world changed|freshness_stale/i);
  }, 60000);

  it("redacts authoritative details after control loss", async () => {
    const surface = await commandSurfaceFor(intentSnapshot(acceptedIntent({ control_state: "control_lost" })));
    await waitFor(() => expect(within(surface).getByText("Intent hidden — control lost")).toBeInTheDocument());
    expect(surface).not.toHaveTextContent("Stabilize power before expanding the iron line.");
    expect(surface).not.toHaveTextContent(/control_lost|runtime_projection/i);
  }, 60000);

  it("redacts the intent summary for read-only observers", async () => {
    const surface = await commandSurfaceFor(intentSnapshot(acceptedIntent({ control_state: "read_only" })));
    await waitFor(() => expect(within(surface).getByText("Intent hidden — read-only")).toBeInTheDocument());
    expect(surface).not.toHaveTextContent("Stabilize power before expanding the iron line.");
  }, 60000);

  it.each([
    ["legacy schema", { schema_version: 1 }],
    ["local pending source", { source_class: "local_pending" }],
    ["unknown control state", { control_state: "operator_override" }],
  ])("fails closed for %s intent metadata", async (_label, overrides) => {
    const surface = await commandSurfaceFor(intentSnapshot(acceptedIntent(overrides)));
    const intentSurface = surface.querySelector(".agent-intent");
    await waitFor(() => expect(within(intentSurface).getByText("Intent unavailable")).toBeInTheDocument());
    expect(intentSurface).not.toHaveTextContent(/Accepted|Stabilize power/i);
  }, 60000);

  it("bounds and normalizes the player-safe intent summary", async () => {
    const summary = `${"A".repeat(180)}\nprovider rationale must stay hidden`;
    const surface = await commandSurfaceFor(intentSnapshot(acceptedIntent({ message: summary })));
    await waitFor(() => expect(within(surface).getByText("Current Intent")).toBeInTheDocument());
    const rendered = surface.querySelector(".agent-intent__summary")?.textContent || "";
    expect(rendered.length).toBeLessThanOrEqual(161);
    expect(rendered).toContain("…");
    expect(rendered).not.toContain("provider rationale");
  }, 60000);

  it("renders a bounded blocked reason and supported next step", async () => {
    const surface = await commandSurfaceFor(intentSnapshot(acceptedIntent({
      status: "blocked",
      reason_code: "missing_material",
      reason_summary: "Iron input is unavailable.",
      next_step: "Replenish iron input before resuming.",
      resume_required: true,
    })));
    await waitFor(() => expect(within(surface).getByText("Blocked")).toBeInTheDocument());
    expect(surface).toHaveTextContent("Iron input is unavailable.");
    expect(surface).toHaveTextContent("Replenish iron input before resuming.");
    expect(surface).not.toHaveTextContent(/missing_material|resume_required/i);
  }, 60000);

  it("does not claim completion without an authoritative receipt", async () => {
    const surface = await commandSurfaceFor(intentSnapshot(completedIntent({ receipt_ref: null })));
    await waitFor(() => expect(within(surface).getByText("Intent unavailable")).toBeInTheDocument());
    expect(surface).not.toHaveTextContent("Completed");
  }, 60000);

  it("shows completion only when a receipt identity is present", async () => {
    const surface = await commandSurfaceFor(intentSnapshot(completedIntent()));
    await waitFor(() => expect(within(surface).getByText("Completed")).toBeInTheDocument());
    expect(surface).toHaveTextContent("Stabilize power before expanding the iron line.");
    expect(surface).not.toHaveTextContent(/receipt-intent-v2|runtime_projection/i);
  }, 60000);

  it.each([
    ["accepted_new", { status: "accepted_new" }],
    ["reprioritized", { status: "reprioritized" }],
    ["missing agent position", { agent_id: undefined }],
  ])("fails closed for non-canonical or incomplete %s metadata", async (_label, overrides) => {
    const surface = await commandSurfaceFor(intentSnapshot(acceptedIntent(overrides)));
    const intentSurface = surface.querySelector(".agent-intent");
    await waitFor(() => expect(within(intentSurface).getByText("Intent unavailable")).toBeInTheDocument());
    expect(intentSurface).not.toHaveTextContent(/Accepted|Stabilize power/i);
  }, 60000);

  it.each([
    ["raw receipt string", "world-event:12"],
    ["wrong intent identity", { ...completedIntent().receipt_ref, intent_id: "other-intent" }],
    ["wrong world identity", { ...completedIntent().receipt_ref, world_id: "other-world" }],
    ["wrong event position", { ...completedIntent().receipt_ref, event_seq: "13" }],
  ])("fails closed for %s", async (_label, receiptRef) => {
    const surface = await commandSurfaceFor(intentSnapshot(completedIntent({ receipt_ref: receiptRef })));
    const intentSurface = surface.querySelector(".agent-intent");
    await waitFor(() => expect(within(intentSurface).getByText("Intent unavailable")).toBeInTheDocument());
    expect(intentSurface).not.toHaveTextContent("Completed");
  }, 60000);

  it("redacts the retained intent while the runtime connection is reconnecting or offline", async () => {
    const app = await renderViewerApp(intentSnapshot(acceptedIntent()));
    dispose = app.dispose;
    app.core.applySelection({ kind: "agent", id: AGENT_ID });
    const intentSurface = app.container.querySelector("#viewer-details-panel .agent-intent");
    app.core.state.connectionStatus = "reconnecting";
    await waitFor(() => expect(within(intentSurface).getByText("Reconnecting")).toBeInTheDocument());
    expect(intentSurface).not.toHaveTextContent("Stabilize power before expanding the iron line.");
    expect(intentSurface).not.toHaveTextContent("Accepted");

    app.core.state.connectionStatus = "disconnected";
    await waitFor(() => expect(within(intentSurface).getByText("Intent unavailable — world connection lost")).toBeInTheDocument());
    expect(intentSurface).not.toHaveTextContent("Stabilize power before expanding the iron line.");
  }, 60000);
});
