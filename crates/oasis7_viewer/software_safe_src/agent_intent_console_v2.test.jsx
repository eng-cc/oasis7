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
    status: "accepted",
    message: "Stabilize power before expanding the iron line.",
    resume_required: false,
    source_class: "runtime_projection",
    freshness: "current",
    control_state: "controllable",
    logical_time: 7,
    event_seq: "11",
    ...overrides,
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
});
