import { within } from "@solidjs/testing-library";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { buildTaskGame076ScenarioSnapshot } from "./gameplay_attraction_scenario.js";

vi.mock("./pixel_world_host.jsx", () => ({
  PixelWorldHost: () => <div data-testid="pixel-world-host" />,
}));

const HEAVY_UI_TEST_TIMEOUT_MS = 60000;
const TEST_ED25519_PKCS8_PREFIX = new Uint8Array([
  0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06,
  0x03, 0x2b, 0x65, 0x70, 0x04, 0x22, 0x04, 0x20,
]);
let activeCleanup = null;

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
        if (format === "raw") return publicBytes.buffer.slice(0);
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

function blockedAndReadySnapshot() {
  const base = buildTaskGame076ScenarioSnapshot();
  return buildTaskGame076ScenarioSnapshot({
    overrides: {
      player_gameplay: {
        ...base.player_gameplay,
        next_step_hint: null,
        available_actions: [
          {
            action_id: "build_alloy_factory",
            label: "Build alloy factory core",
            protocol_action: "gameplay_action.submit",
            target_agent_id: "agent-0",
            disabled_reason: "missing structural frames",
          },
          {
            action_id: "advance_step",
            label: "Advance 1 step",
            protocol_action: "live_control.step",
            disabled_reason: null,
          },
        ],
      },
    },
  });
}

async function renderActionCards() {
  vi.resetModules();
  window.history.replaceState({}, "", "/software_safe.html?test_api=1&connect=0&hosted_bootstrap=0&locale=en");
  const core = await import("./legacy_core.js");
  const main = await import("./main.jsx");
  const snapshot = blockedAndReadySnapshot();
  const appRoot = document.createElement("div");
  appRoot.id = "app";
  document.body.appendChild(appRoot);

  core.initializeSoftwareSafeCore();
  core.setViewerLocale("en");
  core.injectSnapshot(snapshot);
  core.state.auth = {
    ...core.state.auth,
    available: true,
    playerId: "local-test-player-bound",
    publicKey: "abcdef0123456789abcdef0123456789",
    privateKey: "private-key-must-stay-hidden",
    source: "local_test_api_ephemeral",
    registrationStatus: "registered",
    runtimeStatus: "registered",
    boundAgentId: "agent-0",
  };
  main.__markStarterOcOnboardingCompleteForTest("agent-0");
  activeCleanup = main.mountViewerApp(appRoot);
  return appRoot;
}

beforeEach(() => {
  vi.restoreAllMocks();
  window.localStorage.clear();
  Object.defineProperty(window, "crypto", { configurable: true, value: createTestCrypto() });
  document.body.innerHTML = "";
});

afterEach(() => {
  vi.restoreAllMocks();
  vi.useRealTimers();
  activeCleanup?.();
  activeCleanup = null;
  document.body.innerHTML = "";
});

describe("gameplay action card state", () => {
  it("distinguishes blocked gameplay action cards from ready actions without inventing recovery outcomes", async () => {
    const stagePanel = (await renderActionCards()).querySelector("#viewer-stage-panel");
    const blockedCard = within(stagePanel).getByText("Build alloy factory core").closest(".event-card--action");
    expect(blockedCard).toHaveAttribute("data-action-state", "blocked");
    expect(within(blockedCard).getByText("Blocked")).toBeInTheDocument();
    expect(within(blockedCard).getByText("missing structural frames")).toBeInTheDocument();
    expect(within(blockedCard).getByText(/review next move or gameplay details before retrying/i)).toBeInTheDocument();
    expect(within(blockedCard).getByRole("link", { name: /gameplay details/i })).toHaveAttribute("href", "#viewer-details-panel");
    const blockedButton = within(blockedCard).getByRole("button", { name: "Build alloy factory core" });
    expect(blockedButton).toBeDisabled();
    expect(blockedButton).toHaveAttribute("aria-describedby");
    expect(blockedCard.querySelector(`#${blockedButton.getAttribute("aria-describedby")}`)).toHaveTextContent(
      "missing structural frames",
    );
    expect(blockedCard).not.toHaveTextContent(/will (build|create|restore|produce)|structural frames (will|are) available/i);

    const readyCard = [...stagePanel.querySelectorAll('.event-card--action[data-action-state="ready"]')]
      .find((card) => within(card).queryByText("Advance 1 step"));
    expect(readyCard).toBeTruthy();
    expect(readyCard).toHaveAttribute("data-action-state", "ready");
    expect(within(readyCard).queryByText("Blocked")).not.toBeInTheDocument();
    expect(within(readyCard).queryByText(/review next move or gameplay details before retrying/i)).not.toBeInTheDocument();
    expect(within(readyCard).getByRole("button", { name: "Advance 1 step" })).not.toBeDisabled();
  }, HEAVY_UI_TEST_TIMEOUT_MS);
});
