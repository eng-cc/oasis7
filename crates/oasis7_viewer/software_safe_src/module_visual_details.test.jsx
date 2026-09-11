import { within, waitFor } from "@solidjs/testing-library";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  bindFirstSnapshotAgentForTest,
  sampleSnapshot,
} from "./pixel_world_host_test_support.jsx";

vi.mock("./pixel_world_host.jsx", () => ({
  PixelWorldHost: () => <div data-testid="pixel-world-host" />,
}));

let activeCleanup = null;

async function renderModuleDetails() {
  activeCleanup?.();
  vi.resetModules();
  window.history.replaceState({}, "", "/software_safe.html?test_api=1&connect=0&locale=en");
  window.localStorage.clear();
  document.body.innerHTML = "";
  const core = await import("./legacy_core.js");
  const main = await import("./main.jsx");
  const root = document.createElement("div");
  root.id = "app";
  document.body.appendChild(root);
  core.initializeSoftwareSafeCore();
  core.setViewerLocale("en");
  const snapshot = sampleSnapshot();
  core.injectSnapshot(snapshot);
  bindFirstSnapshotAgentForTest(core, snapshot);
  const dispose = main.mountViewerApp(root);
  activeCleanup = () => {
    dispose();
    activeCleanup = null;
  };
  core.state.selectedKind = "module_visual";
  core.state.selectedId = "module-relay";
  core.state.selectedObject = {
    id: "module-relay",
    module_id: "relay-seven",
    kind: "relay",
    label: "Relay Seven",
    anchor: { type: "absolute", data: { pos: { x_cm: 1_530_000, y_cm: 1_010_000, z_cm: 0 } } },
  };
  core.requestRender();
  return { core, root };
}

beforeEach(() => {
  window.history.replaceState({}, "", "/software_safe.html?test_api=1&connect=0&locale=en");
  window.localStorage.clear();
  document.body.innerHTML = "";
});

afterEach(() => {
  activeCleanup?.();
  document.body.innerHTML = "";
});

describe("module visual details", () => {
  it("keeps module selection readable without rendering agent-only controls", async () => {
    const { core, root } = await renderModuleDetails();
    const detailsPanel = root.querySelector("#viewer-details-panel");
    await waitFor(() => expect(detailsPanel.querySelector("[data-viewer-module-details='true']")).toBeTruthy());
    const moduleDetails = detailsPanel.querySelector("[data-viewer-module-details='true']");
    expect(core.state.selectedKind).toBe("module_visual");
    expect(within(moduleDetails).getByText("Module Details")).toBeInTheDocument();
    expect(within(moduleDetails).getByText("Relay Seven")).toBeInTheDocument();
    expect(moduleDetails).toHaveTextContent(/Kind\s*:\s*relay/);
    expect(moduleDetails).toHaveTextContent(/absolute · x=15.3 km · y=10.1 km · z=0 cm/i);
    expect(within(detailsPanel).queryByText("Agent Chat")).not.toBeInTheDocument();
    expect(within(detailsPanel).queryByLabelText("Message")).not.toBeInTheDocument();
    expect(within(detailsPanel).queryByText("The current account has no controllable Agent yet. Claim one or wait for your own Agent binding to sync.")).not.toBeInTheDocument();
  }, 60000);
});
