import { fireEvent, screen, waitFor, within } from "@solidjs/testing-library";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { buildTaskGame076ScenarioSnapshot } from "./gameplay_attraction_scenario.js";

vi.mock("./pixel_world_host.jsx", () => ({
  PixelWorldHost: (props) => (
    <div data-testid="pixel-world-host">
      {`pixel-world-host:${typeof props.locale === "function" ? props.locale() : props.locale}`}
    </div>
  ),
}));

function viewerUrl() {
  return "/software_safe.html?test_api=1&connect=0&hosted_bootstrap=0&locale=en";
}

let activeCleanup = null;
const HEAVY_UI_TEST_TIMEOUT_MS = 60000;
const TEST_ED25519_PKCS8_PREFIX = new Uint8Array([
  0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06,
  0x03, 0x2b, 0x65, 0x70, 0x04, 0x22, 0x04, 0x20,
]);

function createTestCrypto() {
  const privateBytes = new Uint8Array(32).fill(7);
  const publicBytes = new Uint8Array(32).fill(9);
  return {
    subtle: {
      async generateKey() {
        return {
          privateKey: { kind: "test-ed25519-private" },
          publicKey: { kind: "test-ed25519-public" },
        };
      },
      async exportKey(format) {
        if (format === "pkcs8") {
          const out = new Uint8Array(TEST_ED25519_PKCS8_PREFIX.length + privateBytes.length);
          out.set(TEST_ED25519_PKCS8_PREFIX, 0);
          out.set(privateBytes, TEST_ED25519_PKCS8_PREFIX.length);
          return out.buffer;
        }
        if (format === "raw") {
          return publicBytes.buffer.slice(0);
        }
        throw new Error(`unsupported test key export: ${format}`);
      },
      async importKey() {
        return { kind: "test-ed25519-imported" };
      },
      async sign() {
        return new Uint8Array(64).fill(11).buffer;
      },
    },
  };
}

function sampleSnapshot(overrides = {}) {
  const base = buildTaskGame076ScenarioSnapshot();
  return {
    ...base,
    ...overrides,
    config: {
      ...base.config,
      ...(overrides.config || {}),
    },
    model: {
      ...base.model,
      agent_player_bindings: {
        "agent-0": "local-test-player-bound",
      },
      agent_player_public_key_bindings: {
        "agent-0": "abcdef0123456789abcdef0123456789",
      },
      ...(overrides.model || {}),
    },
    player_gameplay: {
      ...base.player_gameplay,
      ...(overrides.player_gameplay || {}),
    },
  };
}

function bindLocalTestAgent(core, agentId = "agent-0", playerId = "local-test-player-bound") {
  core.state.auth = {
    ...core.state.auth,
    available: true,
    playerId,
    publicKey: "abcdef0123456789abcdef0123456789",
    privateKey: "private-key-must-stay-hidden",
    source: "local_test_api_ephemeral",
    registrationStatus: "registered",
    runtimeStatus: "registered",
    boundAgentId: agentId,
  };
}

function bindFirstSnapshotAgentForTest(core, snapshot) {
  const agentId = Object.keys(snapshot?.model?.agents || {})[0];
  const playerId = snapshot?.model?.agent_player_bindings?.[agentId];
  if (!agentId || !playerId) {
    return;
  }
  bindLocalTestAgent(core, agentId, playerId);
}

async function renderViewerApp({
  snapshot = sampleSnapshot(),
  selection = null,
  setupAfterMount = null,
} = {}) {
  activeCleanup?.();
  activeCleanup = null;
  vi.resetModules();
  window.history.replaceState({}, "", viewerUrl());
  window.localStorage.clear();
  document.body.innerHTML = "";

  const core = await import("./legacy_core.js");
  const main = await import("./main.jsx");
  const appRoot = document.createElement("div");
  appRoot.id = "app";
  document.body.appendChild(appRoot);

  core.initializeSoftwareSafeCore();
  core.setViewerLocale("en");
  if (snapshot) {
    core.injectSnapshot(snapshot);
    bindFirstSnapshotAgentForTest(core, snapshot);
  }
  if (selection) {
    core.applySelection(selection);
  }
  if (snapshot) {
    bindFirstSnapshotAgentForTest(core, core.state.snapshot);
  }
  if (setupAfterMount) {
    setupAfterMount(core);
  }
  main.__markStarterOcOnboardingCompleteForTest(core.state.auth.boundAgentId);

  const dispose = main.mountViewerApp(appRoot);
  const cleanup = () => {
    dispose();
    if (activeCleanup === cleanup) {
      activeCleanup = null;
    }
  };
  activeCleanup = cleanup;
  return { core, container: appRoot };
}

beforeEach(() => {
  vi.restoreAllMocks();
  window.history.replaceState({}, "", viewerUrl());
  window.localStorage.clear();
  Object.defineProperty(window, "crypto", {
    configurable: true,
    value: createTestCrypto(),
  });
  document.body.innerHTML = "";
});

afterEach(() => {
  vi.restoreAllMocks();
  vi.useRealTimers();
  activeCleanup?.();
  activeCleanup = null;
  document.body.innerHTML = "";
});

describe("focused viewer UI contracts", () => {
  it("keeps visible Prompt Overrides contained inside the Advanced disclosure", async () => {
    const { core } = await renderViewerApp({
      selection: { kind: "agent", id: "agent-0" },
      setupAfterMount(core) {
        bindLocalTestAgent(core, "agent-0");
      },
    });

    core.togglePromptOverridesVisible();
    await waitFor(() => {
      expect(screen.getByLabelText("System Prompt Override")).toBeInTheDocument();
    });

    const advancedDetails = screen.getByText("Advanced Prompt Settings").closest("details");
    expect(advancedDetails).toBeTruthy();
    expect(advancedDetails).toContainElement(screen.getByLabelText("System Prompt Override"));
    expect(advancedDetails).toContainElement(screen.getByRole("button", { name: "Preview Prompt" }));
  }, HEAVY_UI_TEST_TIMEOUT_MS);

  it("uses readable entity identity and semantic selection state in Targets", async () => {
    const base = sampleSnapshot();
    const snapshot = sampleSnapshot({
      model: {
        ...base.model,
        agents: {
          ...base.model.agents,
          "agent-0": {
            ...base.model.agents["agent-0"],
            name: "Surveyor Seven",
          },
        },
        locations: {
          ...base.model.locations,
          "loc-0": {
            ...base.model.locations["loc-0"],
            name: "Assembly Nexus",
          },
        },
      },
    });
    const { container } = await renderViewerApp({
      snapshot,
      selection: { kind: "agent", id: "agent-0" },
    });

    const targetsPanel = container.querySelector("#viewer-targets-panel");
    const agentButton = within(targetsPanel).getByTestId("viewer-playthrough-select-agent");
    const locationButton = within(targetsPanel).getByTestId("viewer-select-location-loc-0");
    expect(within(agentButton).getByText("Surveyor Seven")).toBeInTheDocument();
    expect(within(locationButton).getByText("Assembly Nexus")).toBeInTheDocument();
    expect(agentButton).toHaveAttribute("aria-pressed", "true");
    expect(locationButton).toHaveAttribute("aria-pressed", "false");

    fireEvent.click(locationButton);
    await waitFor(() => {
      expect(locationButton).toHaveAttribute("aria-pressed", "true");
      expect(agentButton).toHaveAttribute("aria-pressed", "false");
    });
  }, HEAVY_UI_TEST_TIMEOUT_MS);

  it("keeps the selected Agent identity in the narrow Command context", async () => {
    const base = sampleSnapshot();
    const snapshot = sampleSnapshot({
      model: {
        ...base.model,
        agents: {
          ...base.model.agents,
          "agent-0": { ...base.model.agents["agent-0"], name: "Surveyor Seven" },
        },
      },
    });
    const { container } = await renderViewerApp({
      snapshot,
      selection: { kind: "agent", id: "agent-0" },
    });

    const commandSurface = container.querySelector("#viewer-details-panel .command-surface");
    const context = commandSurface.querySelector(".command-surface__target-row");
    expect(context).toHaveTextContent("Surveyor Seven");
    expect(context).toHaveTextContent(/Chat Ready|Chat Limited/);
  }, HEAVY_UI_TEST_TIMEOUT_MS);
});
