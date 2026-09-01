import { fireEvent, waitFor, within } from "@solidjs/testing-library";
import { afterEach, describe, expect, it } from "vitest";
import { buildTaskGame076ScenarioSnapshot } from "./gameplay_attraction_scenario.js";
import { renderViewerApp } from "./test_support/viewer_app_fixture.jsx";

let dispose = null;
const HEAVY_UI_TEST_TIMEOUT_MS = 60000;

afterEach(() => {
  dispose?.();
  dispose = null;
  document.body.innerHTML = "";
});

describe("Viewer visual fixture integration", () => {
  it("renders the shell selected-blocker fixture as a populated command desk", async () => {
    const app = await renderViewerApp(
      buildTaskGame076ScenarioSnapshot(),
      {},
      "en",
      "/software_safe.html?test_api=1&connect=0&hosted_bootstrap=0&locale=en&viewer_visual_fixture=shell_selected_blocker",
    );
    dispose = app.dispose;

    const state = window.__AW_TEST__.getState();
    expect(state.selectedKind).toBe("agent");
    expect(state.selectedId).toBe("agent-0");
    const targetsPanel = app.container.querySelector("#viewer-targets-panel");
    const agentButton = within(targetsPanel).getByTestId("viewer-playthrough-select-agent");
    const locationButton = within(targetsPanel).getByTestId("viewer-select-location-loc-1");
    expect(within(targetsPanel).getByText("agent-0")).toBeInTheDocument();
    expect(within(agentButton).getByText("Selected")).toBeInTheDocument();
    expect(within(targetsPanel).getByText("Assembly Nexus")).toBeInTheDocument();
    expect(within(app.container.querySelector("#viewer-details-panel")).getByText("Agent Chat")).toBeInTheDocument();
    const agentContext = within(app.container.querySelector("#viewer-details-panel"))
      .getByRole("region", { name: "Agent Context" });
    expect(agentContext).toHaveAttribute("data-agent-context-kind", "agent");
    expect(within(agentContext).getByText("Agent 0")).toBeInTheDocument();
    expect(agentContext.closest(".command-surface")).toHaveAttribute("data-command-agent", "agent-0");
    fireEvent.click(locationButton);
    await waitFor(() => {
      expect(locationButton).toHaveAttribute("data-selected", "true");
    });
    expect(within(locationButton).getByText("Selected")).toBeInTheDocument();
    expect(within(agentButton).queryByText("Selected")).not.toBeInTheDocument();
    expect(within(app.container.querySelector("#viewer-stage-panel")).getAllByText("Recover sustainable capability").length).toBeGreaterThan(0);
  }, HEAVY_UI_TEST_TIMEOUT_MS);

  it("renders the authorized Crisis fixture as ambient World Feed context only", async () => {
    const app = await renderViewerApp(
      buildTaskGame076ScenarioSnapshot(),
      {},
      "en",
      "/software_safe.html?test_api=1&connect=0&hosted_bootstrap=0&locale=en&viewer_visual_fixture=major_world_event_crisis",
    );
    dispose = app.dispose;

    const feed = app.container.querySelector("#viewer-world-feed");
    expect(within(feed).getByText("Crisis active · severity 4")).toBeInTheDocument();
    expect(feed.querySelector("[data-major-event-category='crisis']")).toBeTruthy();
    expect(app.container.querySelector("[data-major-world-event-marker]")).toBeNull();
    expect(app.container.querySelector("[data-world-feed-receipt-ref]")).toBeNull();
  }, HEAVY_UI_TEST_TIMEOUT_MS);
});
