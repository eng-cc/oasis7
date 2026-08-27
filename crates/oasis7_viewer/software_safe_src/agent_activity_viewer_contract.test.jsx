import { screen, waitFor, within } from "@solidjs/testing-library";
import { afterEach, describe, expect, it } from "vitest";
import { buildTaskGame076ScenarioSnapshot } from "./gameplay_attraction_scenario.js";
import { renderViewerApp } from "./test_support/viewer_app_fixture.jsx";

const ACTIVE_AGENT = "agent-0";
const ACTIVE_PLAYER = "local-test-player-bound";

function activitySnapshot(activity, lastActive = 12) {
  const base = buildTaskGame076ScenarioSnapshot();
  return {
    ...base,
    model: {
      ...base.model,
      agent_player_bindings: { [ACTIVE_AGENT]: ACTIVE_PLAYER },
      agent_player_public_key_bindings: { [ACTIVE_AGENT]: "abcdef0123456789abcdef0123456789" },
      agents: {
        ...base.model.agents,
        [ACTIVE_AGENT]: {
          ...base.model.agents[ACTIVE_AGENT],
          activity,
          last_active: lastActive,
        },
      },
    },
  };
}

let dispose = null;

afterEach(() => {
  dispose?.();
  dispose = null;
  document.body.innerHTML = "";
});

describe("authoritative Agent Activity viewer contract", () => {
  it("shows explicit executing activity in Targets without inferred labels", async () => {
    const app = await renderViewerApp(activitySnapshot({
      status: "executing",
      operation: "recipe",
      target: "factory-activity",
      reason: null,
      updated_at: 7,
    }));
    dispose = app.dispose;

    const targetsPanel = app.container.querySelector("#viewer-targets-panel");
    await waitFor(() => {
      expect(within(targetsPanel).getByText("Executing Recipe")).toBeInTheDocument();
    });
    expect(targetsPanel).not.toHaveTextContent(/\bIdle\b|last_active|Mining|Travelling|status_[a-z_]+/i);
  });

  it("shows explicit blocked activity in Targets with a human-readable state", async () => {
    const app = await renderViewerApp(activitySnapshot({
      status: "blocked",
      operation: "recipe",
      target: "factory-activity",
      reason: "insufficient electricity",
      updated_at: 8,
    }));
    dispose = app.dispose;

    const targetsPanel = app.container.querySelector("#viewer-targets-panel");
    await waitFor(() => {
      expect(within(targetsPanel).getByText("Blocked")).toBeInTheDocument();
    });
    expect(targetsPanel).toHaveTextContent("Insufficient Electricity");
    expect(targetsPanel).not.toHaveTextContent(/\bIdle\b|last_active|Mining|Travelling|status_[a-z_]+/i);
  });

  it("shows Current Activity with operation and target on the selected-agent Command screen", async () => {
    const app = await renderViewerApp(activitySnapshot({
      status: "executing",
      operation: "recipe",
      target: "factory-activity",
      reason: null,
      updated_at: 7,
    }));
    dispose = app.dispose;
    app.core.applySelection({ kind: "agent", id: ACTIVE_AGENT });

    const commandSurface = app.container.querySelector("#viewer-details-panel .command-surface");
    await waitFor(() => {
      expect(within(commandSurface).getByText("Current Activity")).toBeInTheDocument();
    });
    expect(commandSurface).toHaveTextContent("Executing Recipe");
    expect(commandSurface).toHaveTextContent("Factory Activity");
    expect(commandSurface).not.toHaveTextContent(/\bIdle\b|last_active|Mining|Travelling|status_[a-z_]+/i);
  });

  it("renders missing activity as unavailable and never synthesizes Idle or raw runtime fields", async () => {
    const app = await renderViewerApp(activitySnapshot(null, 99));
    dispose = app.dispose;
    const targetsPanel = app.container.querySelector("#viewer-targets-panel");

    await waitFor(() => {
      expect(within(targetsPanel).getByText("Activity unavailable")).toBeInTheDocument();
    });
    expect(targetsPanel).not.toHaveTextContent(/\bIdle\b|last_active|Mining|Travelling|status_[a-z_]+/i);

    app.core.applySelection({ kind: "agent", id: ACTIVE_AGENT });
    const commandSurface = app.container.querySelector("#viewer-details-panel .command-surface");
    await waitFor(() => {
      expect(within(commandSurface).getByText("Activity unavailable")).toBeInTheDocument();
    });
    expect(commandSurface).not.toHaveTextContent(/\bIdle\b|last_active|Mining|Travelling|status_[a-z_]+/i);
  });
});
