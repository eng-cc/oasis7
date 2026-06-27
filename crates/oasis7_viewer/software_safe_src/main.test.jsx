import { fireEvent, screen, waitFor, within } from "@solidjs/testing-library";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { buildTaskGame076ScenarioSnapshot } from "./gameplay_attraction_scenario.js";
import { HOSTED_PUBLIC_JOIN_DEPLOYMENT_MODE } from "./software_safe_constants.js";

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

function elementPrecedes(first, second) {
  return Boolean(first?.compareDocumentPosition(second) & Node.DOCUMENT_POSITION_FOLLOWING);
}

function sampleSnapshot(overrides = {}) {
  return buildTaskGame076ScenarioSnapshot({ overrides });
}

function sampleAgentClaimSnapshot() {
  const base = sampleSnapshot();
  return sampleSnapshot({
    model: {
      ...base.model,
      agents: {
        ...base.model.agents,
        "agent-claim-target": {
          id: "agent-claim-target",
          name: "Claim Target",
          location_id: "loc-0",
          resources: {},
        },
      },
    },
    player_gameplay: {
      ...base.player_gameplay,
      agent_claim: {
        claimer_agent_id: "agent-0",
        current_epoch: 0,
        reputation_tier: 0,
        claim_cap: 1,
        owned_claim_count: 0,
        liquid_main_token_balance: 0,
        restricted_starter_claim_balance: 0,
        slot_1_auto_restricted_starter_claim_amount: 325,
        slot_1_eligible_claim_balance: 325,
        next_claim_quote: {
          slot_index: 1,
          reputation_tier: 0,
          claim_cap: 1,
          owned_claim_count: 0,
          activation_fee_amount: 100,
          claim_bond_amount: 200,
          upkeep_per_epoch: 25,
          total_upfront_amount: 325,
          transferable_liquid_balance: 0,
          restricted_starter_claim_balance: 0,
          auto_restricted_starter_claim_amount: 325,
          eligible_claim_balance: 325,
          release_cooldown_epochs: 2,
          grace_epochs: 4,
          idle_warning_epochs: 8,
          forced_idle_reclaim_epochs: 10,
          forced_reclaim_penalty_bps: 2000,
        },
        owned_claims: [],
      },
    },
  });
}

function sampleHostedPublicJoinAccess(overrides = {}) {
  return {
    deployment_mode: HOSTED_PUBLIC_JOIN_DEPLOYMENT_MODE,
    action_matrix: [
      {
        action_id: "prompt_control_apply",
        required_auth: "strong_auth",
        availability: "public_player_plane_with_backend_reauth_preview",
        reason: "prompt_control_apply is available through browser-local player auth plus backend re-authorization",
      },
      {
        action_id: "main_token_transfer",
        required_auth: "strong_auth",
        availability: "blocked_until_strong_auth",
        reason: "main_token_transfer remains blocked until a higher-trust hosted strong-auth lane exists",
      },
    ],
    ...overrides,
  };
}

async function renderViewerApp({
  snapshot = sampleSnapshot(),
  selection = null,
  search = viewerUrl(),
  setupCore = null,
  setupAfterMount = null,
} = {}) {
  activeCleanup?.();
  activeCleanup = null;
  vi.resetModules();
  window.history.replaceState({}, "", search);
  window.localStorage.clear();
  document.body.innerHTML = "";

  const core = await import("./legacy_core.js");
  const { mountViewerApp } = await import("./main.jsx");
  const appRoot = document.createElement("div");
  appRoot.id = "app";
  document.body.appendChild(appRoot);

  core.setViewerLocale("en");
  if (snapshot) {
    core.injectSnapshot(snapshot);
  }
  if (selection) {
    core.applySelection(selection);
  }
  if (setupCore) {
    setupCore(core);
  }

  const dispose = mountViewerApp(appRoot);
  if (setupAfterMount) {
    setupAfterMount(core);
    core.requestRender();
  }
  const cleanup = () => {
    dispose();
    if (activeCleanup === cleanup) {
      activeCleanup = null;
    }
  };
  activeCleanup = cleanup;
  return {
    core,
    cleanup,
    container: appRoot,
  };
}

async function renderViewerAppThroughAutoMount({ snapshot = sampleSnapshot(), search }) {
  activeCleanup?.();
  activeCleanup = null;
  vi.resetModules();
  window.history.replaceState({}, "", search);
  window.localStorage.clear();
  document.body.innerHTML = "";

  const core = await import("./legacy_core.js");
  core.setViewerLocale("en");
  if (snapshot) {
    core.injectSnapshot(snapshot);
  }

  const appRoot = document.createElement("div");
  appRoot.id = "app";
  document.body.appendChild(appRoot);
  await import("./main.jsx");

  const cleanup = () => {
    appRoot.textContent = "";
    if (activeCleanup === cleanup) {
      activeCleanup = null;
    }
  };
  activeCleanup = cleanup;
  return {
    core,
    cleanup,
    container: appRoot,
  };
}

beforeEach(() => {
  vi.restoreAllMocks();
  window.history.replaceState({}, "", viewerUrl());
  window.localStorage.clear();
  Object.defineProperty(window, "crypto", {
    configurable: true,
    value: globalThis.crypto,
  });
  document.body.innerHTML = "";
});

afterEach(() => {
  vi.restoreAllMocks();
  activeCleanup?.();
  activeCleanup = null;
  document.body.innerHTML = "";
});

describe("viewer web ui automation baseline", () => {
  it("renders the world-target-command structure and diagnostics anchors", async () => {
    const { container } = await renderViewerApp();

    const nav = screen.getByRole("navigation", { name: /primary entry section navigation/i });
    expect(within(nav).getByRole("link", { name: "World" })).toHaveAttribute("href", "#viewer-stage-panel");
    expect(within(nav).getByRole("link", { name: "Targets" })).toHaveAttribute("href", "#viewer-targets-panel");
    expect(within(nav).getByRole("link", { name: "Command" })).toHaveAttribute("href", "#viewer-details-panel");
    expect(within(nav).getByRole("link", { name: "Diagnostics" })).toHaveAttribute(
      "href",
      "#viewer-diagnostics-panel",
    );

    const targetsPanel = container.querySelector("#viewer-targets-panel");
    const stagePanel = container.querySelector("#viewer-stage-panel");
    const detailsPanel = container.querySelector("#viewer-details-panel");
    const diagnosticsPanel = container.querySelector("#viewer-diagnostics-panel");

    expect(targetsPanel).toBeTruthy();
    expect(stagePanel).toBeTruthy();
    expect(detailsPanel).toBeTruthy();
    expect(diagnosticsPanel).toBeTruthy();
    expect(targetsPanel).toHaveAttribute("data-viewer-surface", "targets");
    expect(stagePanel).toHaveAttribute("data-viewer-surface", "stage");
    expect(detailsPanel).toHaveAttribute("data-viewer-surface", "command");
    expect(diagnosticsPanel).toHaveAttribute("data-viewer-surface", "diagnostics");
    expect(within(targetsPanel).getByText("Targets")).toBeInTheDocument();
    expect(within(stagePanel).getByText("Industrial World Command Desk")).toBeInTheDocument();
    expect(within(stagePanel).getAllByText("Recover sustainable capability").length).toBeGreaterThan(0);
    expect(within(stagePanel).getByText("Control Proof")).toBeInTheDocument();
    expect(within(stagePanel).getByText("Player Intent")).toBeInTheDocument();
    expect(within(stagePanel).getByText("World Consequence")).toBeInTheDocument();
    expect(within(stagePanel).getByText("Recovery Move")).toBeInTheDocument();
    expect(within(stagePanel).getAllByText("Next Move").length).toBeGreaterThan(0);
    expect(within(stagePanel).getByText("Attraction Proof")).toBeInTheDocument();
    expect(within(stagePanel).getByText("What I caused")).toBeInTheDocument();
    expect(within(stagePanel).getByText("New option")).toBeInTheDocument();
    expect(within(stagePanel).getByText("Why continue")).toBeInTheDocument();
    expect(within(stagePanel).getByText("Waiting cost")).toBeInTheDocument();
    expect(within(stagePanel).getAllByText("Recovery").length).toBeGreaterThan(0);
    expect(within(stagePanel).getByText("Agency Moves")).toBeInTheDocument();
    expect(within(stagePanel).getByText("Interrupt")).toBeInTheDocument();
    expect(within(stagePanel).getByText("Reprioritize")).toBeInTheDocument();
    expect(within(stagePanel).getByText(/Switch from stalled smelter build to material recovery/i)).toBeInTheDocument();
    expect(within(stagePanel).getByText("First Win & Anti-Grind")).toBeInTheDocument();
    expect(within(stagePanel).getByText("First Win")).toBeInTheDocument();
    expect(within(stagePanel).getByText("small_player.first_industrial_win")).toBeInTheDocument();
    expect(within(stagePanel).getAllByText(/repair_elasticity/i).length).toBeGreaterThan(0);
    expect(within(stagePanel).getByText("Mature-World Continuation")).toBeInTheDocument();
    expect(within(stagePanel).getByText("Recovery Options")).toBeInTheDocument();
    expect(within(stagePanel).getByText("repair: available / rebuild: available / pivot: available")).toBeInTheDocument();
    expect(within(stagePanel).getByText("Share Replay")).toBeInTheDocument();
    expect(within(stagePanel).getAllByText(/queued build_factory_smelter_mk1/i).length).toBeGreaterThan(0);
    expect(within(stagePanel).getAllByText("Accepted Intent").length).toBeGreaterThan(0);
    expect(within(stagePanel).getAllByText("Queue build_factory_smelter_mk1 for agent-0").length).toBeGreaterThan(0);
    expect(within(stagePanel).getByText("Goal Execution")).toBeInTheDocument();
    expect(within(stagePanel).getByText("Capability Economics")).toBeInTheDocument();
    expect(within(stagePanel).getByText("What The Next Move Changes")).toBeInTheDocument();
    expect(within(stagePanel).queryByText("Why This Step Is Worth Continuing")).not.toBeInTheDocument();
    expect(within(stagePanel).getByText("Repair Move")).toBeInTheDocument();
    expect(within(stagePanel).getByText(/The gating input is the material chain/i)).toBeInTheDocument();
    expect(within(stagePanel).getByText("World Constraint")).toBeInTheDocument();
    expect(within(stagePanel).getByText("Missing Material")).toBeInTheDocument();
    expect(within(stagePanel).getByText("Recommended Action")).toBeInTheDocument();
    expect(within(stagePanel).getByText("Runtime Diagnostics")).toBeInTheDocument();
    expect(within(stagePanel).getByText("Session Ladder")).toBeInTheDocument();
    expect(within(stagePanel).getByText("Runtime Diagnostics")).toBeInTheDocument();
    expect(screen.getByTestId("pixel-world-host")).toHaveTextContent("pixel-world-host:en");
  }, 60000);

  it("shows target list loading affordances before the first snapshot arrives", async () => {
    const { container } = await renderViewerApp({ snapshot: null });

    const targetsPanel = container.querySelector("#viewer-targets-panel");
    expect(targetsPanel).toBeTruthy();
    expect(within(targetsPanel).getByText("Syncing agents…")).toBeInTheDocument();
    expect(within(targetsPanel).getByText("Syncing locations…")).toBeInTheDocument();
    expect(within(targetsPanel).queryByText("No agents in current snapshot.")).not.toBeInTheDocument();
    expect(within(targetsPanel).queryByText("No locations in current snapshot.")).not.toBeInTheDocument();
  }, HEAVY_UI_TEST_TIMEOUT_MS);

  it("auto-issues local test player auth before claiming the first agent", async () => {
    const { core } = await renderViewerApp({
      snapshot: sampleSnapshot({
        model: {
          agents: {},
          locations: {},
          agent_prompt_profiles: {},
          agent_execution_debug_contexts: {},
        },
        player_gameplay: {
          ...sampleSnapshot().player_gameplay,
          available_actions: [
            {
              action_id: "claim_first_agent",
              label: "Claim first Agent",
              protocol_action: "gameplay_action.submit",
              target_agent_id: "starter-agent-0",
              disabled_reason: null,
            },
          ],
        },
      }),
    });

    expect(core.state.auth.available).toBe(false);

    core.sendGameplayAction({
      actionId: "claim_first_agent",
      protocolAction: "gameplay_action.submit",
      targetAgentId: "starter-agent-0",
      executeKind: "claim_first_agent",
    });

    for (let attempt = 0; attempt < 50; attempt += 1) {
      if (core.state.lastGameplayActionFeedback?.stage !== "queued") {
        break;
      }
      await new Promise((resolve) => setTimeout(resolve, 50));
    }
    expect(core.state.lastGameplayActionFeedback?.stage).toBe("error");
    expect(core.state.auth.available).toBe(true);
    expect(core.state.auth.source).toBe("local_test_api_ephemeral");
    expect(core.state.auth.playerId).toMatch(/^local-test-player-/);
    expect(core.state.lastGameplayActionFeedback?.reason).toMatch(/viewer websocket is not connected/i);
  }, HEAVY_UI_TEST_TIMEOUT_MS);

  it("auto-issues loopback local player auth without test_api before claiming the first agent", async () => {
    activeCleanup?.();
    activeCleanup = null;
    vi.resetModules();
    window.history.replaceState(
      {},
      "",
      "/software_safe.html?connect=0&hosted_bootstrap=0&locale=en&ws=ws://127.0.0.1:5011",
    );
    window.localStorage.clear();
    document.body.innerHTML = "";
    const core = await import("./legacy_core.js");

    expect(core.state.auth.available).toBe(false);

    core.sendGameplayAction({
      actionId: "claim_first_agent",
      protocolAction: "gameplay_action.submit",
      targetAgentId: "starter-agent-0",
      executeKind: "claim_first_agent",
    });

    for (let attempt = 0; attempt < 50; attempt += 1) {
      if (core.state.lastGameplayActionFeedback?.stage !== "queued") {
        break;
      }
      await new Promise((resolve) => setTimeout(resolve, 50));
    }
    expect(core.state.auth.available).toBe(true);
    expect(core.state.auth.source).toBe("local_test_api_ephemeral");
    expect(core.state.auth.playerId).toMatch(/^local-test-player-/);
    expect(core.state.lastGameplayActionFeedback?.stage).toBe("error");
    expect(core.state.lastGameplayActionFeedback?.reason).toMatch(/viewer websocket is not connected/i);
  }, HEAVY_UI_TEST_TIMEOUT_MS);

  it("routes the empty-world Claim First Agent button into gameplay action feedback", async () => {
    const { core, container } = await renderViewerApp({
      snapshot: sampleSnapshot({
        model: {
          agents: {},
          locations: {},
          agent_prompt_profiles: {},
          agent_execution_debug_contexts: {},
        },
        player_gameplay: {
          ...sampleSnapshot().player_gameplay,
          available_actions: [
            {
              action_id: "claim_first_agent",
              label: "Claim first Agent",
              protocol_action: "gameplay_action.submit",
              target_agent_id: "starter-agent-0",
              disabled_reason: null,
            },
          ],
        },
      }),
    });

    const stagePanel = container.querySelector("#viewer-stage-panel");
    const claimButtons = within(stagePanel).getAllByRole("button", { name: "Claim First Agent" });
    expect(claimButtons.length).toBeGreaterThan(0);

    fireEvent.click(claimButtons[0]);

    for (let attempt = 0; attempt < 50; attempt += 1) {
      if (core.state.lastGameplayActionFeedback?.stage !== "queued") {
        break;
      }
      await new Promise((resolve) => setTimeout(resolve, 50));
    }
    expect(core.state.auth.available).toBe(true);
    expect(core.state.lastGameplayActionFeedback?.action).toBe("claim_first_agent");
    expect(core.state.lastGameplayActionFeedback?.stage).toBe("error");
    expect(core.state.lastGameplayActionFeedback?.reason).toMatch(/viewer websocket is not connected/i);
    expect(core.buildGameplaySummary().recentFeedback).toMatchObject({
      action: "claim_first_agent",
      stage: "error",
      source: "local_gameplay_action",
    });
    await waitFor(() => {
      expect(within(stagePanel).getAllByText(/viewer websocket is not connected/i).length)
        .toBeGreaterThan(0);
    });
  }, HEAVY_UI_TEST_TIMEOUT_MS);

  it("disables first-agent claim buttons while committed snapshot sync is pending", async () => {
    const { container, core } = await renderViewerApp({
      snapshot: sampleSnapshot({
        model: {
          agents: {},
          locations: {},
          agent_prompt_profiles: {},
          agent_execution_debug_contexts: {},
        },
        player_gameplay: {
          ...sampleSnapshot().player_gameplay,
          blocker_kind: "runtime_snapshot_empty_entities",
          blocker_detail: "runtime exposed an empty new-user world",
          available_actions: [
            {
              action_id: "claim_first_agent",
              label: "Claim first Agent",
              protocol_action: "gameplay_action.submit",
              target_agent_id: "starter-agent-0",
              disabled_reason: null,
            },
          ],
        },
      }),
      setupCore(core) {
        core.state.lastGameplayActionFeedback = {
          kind: "gameplay_action",
          action: "claim_first_agent",
          stage: "ack",
          accepted: true,
          ok: true,
          effect: "submitted gameplay action claim_first_agent for starter-agent-0",
          reason: null,
          response: null,
        };
      },
    });

    const stagePanel = container.querySelector("#viewer-stage-panel");
    const targetsPanel = container.querySelector("#viewer-targets-panel");
    const claimButtons = [
      ...within(stagePanel).getAllByRole("button", { name: "Claim First Agent" }),
      ...within(targetsPanel).getAllByRole("button", { name: "Claim First Agent" }),
    ];
    expect(claimButtons.length).toBeGreaterThan(1);
    claimButtons.forEach((button) => expect(button).toBeDisabled());
    expect(within(stagePanel).getAllByText(/waiting for the committed chain snapshot/i).length)
      .toBeGreaterThan(0);
    expect(within(targetsPanel).getAllByText(/waiting for the committed chain snapshot/i).length)
      .toBeGreaterThan(0);
  }, HEAVY_UI_TEST_TIMEOUT_MS);

  it("moves presentation notes into the stage help tip instead of the right details rail", async () => {
    const { container } = await renderViewerApp();

    const stagePanel = container.querySelector("#viewer-stage-panel");
    const detailsPanel = container.querySelector("#viewer-details-panel");
    const helpButton = within(stagePanel).getByRole("button", { name: /open presentation scale guidance/i });

    expect(helpButton).toHaveAttribute("aria-expanded", "false");
    expect(within(detailsPanel).queryByText("Do not trust marker size")).not.toBeInTheDocument();

    fireEvent.click(helpButton);

    expect(helpButton).toHaveAttribute("aria-expanded", "true");
    expect(within(stagePanel).getByText("Presentation Notes")).toBeInTheDocument();
    expect(within(stagePanel).getByText(/do not read on-screen diameter as real geometry size/i)).toBeInTheDocument();
    expect(helpButton).toHaveAttribute("aria-describedby", "viewer-stage-scale-tip");
  }, HEAVY_UI_TEST_TIMEOUT_MS);

  it("prefers recovery proof controls over generic gameplay actions when blocked", async () => {
    const { container, core } = await renderViewerApp({
      snapshot: sampleSnapshot({
        player_gameplay: {
          ...sampleSnapshot().player_gameplay,
          next_step_hint: "Advance one committed step to prove the material recovery before queuing another build.",
          available_actions: [
            {
              action_id: "build_factory_smelter_mk1",
              target_agent_id: "agent-0",
              label: "Build smelter mk1",
              protocol_action: "gameplay_action.submit",
              disabled_reason: null,
            },
            {
              action_id: "live_control.step",
              label: "Advance recovery proof",
              protocol_action: "live_control.step",
              disabled_reason: null,
            },
            {
              action_id: "request_snapshot",
              label: "Request snapshot",
              protocol_action: "world.request_snapshot",
              disabled_reason: null,
            },
          ],
        },
      }),
    });

    const stagePanel = container.querySelector("#viewer-stage-panel");
    const recommendedCard = within(stagePanel).getByText("Recommended Action").closest(".callout");
    expect(recommendedCard).toBeTruthy();
    expect(within(recommendedCard).getByText("Advance recovery proof")).toBeInTheDocument();
    expect(within(recommendedCard).getByText(/Advance one committed step to apply or prove recovery/i)).toBeInTheDocument();
    expect(within(recommendedCard).getByRole("button", { name: "Advance One Step" })).toHaveAttribute(
      "data-testid",
      "viewer-playthrough-action-recommended",
    );
    expect(screen.getByTestId("viewer-playthrough-action-step")).toHaveAccessibleName(/Advance One Step toward: Advance recovery proof/);
    expect(screen.getByTestId("viewer-primary-action-preview")).toHaveTextContent(/Recommended context: Advance recovery proof/);
    expect(within(stagePanel).getByText(/Refresh the snapshot to confirm whether the blocker is still present/i)).toBeInTheDocument();
  }, HEAVY_UI_TEST_TIMEOUT_MS);

  it("uses snapshot refresh as the recommended recovery action when the blocker asks for fresh state", async () => {
    const { container } = await renderViewerApp({
      snapshot: sampleSnapshot({
        player_gameplay: {
          ...sampleSnapshot().player_gameplay,
          next_step_hint: "Refresh the snapshot first, then prove whether the blocker cleared.",
          available_actions: [
            {
              action_id: "request_snapshot",
              label: "Request snapshot",
              protocol_action: "world.request_snapshot",
              disabled_reason: null,
            },
          ],
        },
      }),
    });

    const stagePanel = container.querySelector("#viewer-stage-panel");
    const recommendedCard = within(stagePanel).getByText("Recommended Action").closest(".callout");
    expect(recommendedCard).toBeTruthy();
    expect(within(recommendedCard).getByText("Request snapshot")).toBeInTheDocument();
    expect(within(recommendedCard).getByRole("button", { name: "Refresh Snapshot" })).toHaveAttribute(
      "data-testid",
      "viewer-playthrough-action-recommended",
    );
    expect(screen.getByTestId("viewer-playthrough-action-request-snapshot")).toHaveAccessibleName(/Refresh Snapshot to verify: Request snapshot/);
    expect(within(recommendedCard).getByText(/Refresh the snapshot to confirm whether the blocker is still present/i)).toBeInTheDocument();
  }, HEAVY_UI_TEST_TIMEOUT_MS);

  it("publishes stable player-visible locators for actual-click attraction playthrough", async () => {
    await renderViewerApp({
      snapshot: sampleSnapshot({
        player_gameplay: {
          ...sampleSnapshot().player_gameplay,
          next_step_hint: "Advance one committed step, then refresh the snapshot.",
          available_actions: [
            {
              action_id: "live_control.step",
              label: "Advance recovery proof",
              protocol_action: "live_control.step",
              disabled_reason: null,
            },
            {
              action_id: "request_snapshot",
              label: "Request snapshot",
              protocol_action: "world.request_snapshot",
              disabled_reason: null,
            },
          ],
        },
      }),
    });

    expect(screen.getByTestId("viewer-playthrough-select-agent")).toHaveAccessibleName(/agent/i);
    expect(screen.getByTestId("viewer-playthrough-action-recommended")).toHaveAccessibleName("Advance One Step");
    expect(screen.getByTestId("viewer-playthrough-action-step")).toHaveAccessibleName(/Advance One Step toward: Advance recovery proof/);
    expect(screen.getByTestId("viewer-playthrough-action-request-snapshot")).toHaveAccessibleName(/Refresh Snapshot to verify: Advance recovery proof/);
  }, HEAVY_UI_TEST_TIMEOUT_MS);

  it("renders claim-agent quote and owned claim guidance without inventing claim controls", async () => {
    const { container } = await renderViewerApp({
      snapshot: sampleSnapshot({
        player_gameplay: {
          ...sampleSnapshot().player_gameplay,
          agent_claim: {
            next_claim_quote: {
              slot_index: 2,
              reputation_tier: "trusted",
              owned_claim_count: 1,
              claim_cap: 3,
              total_upfront_amount: 120,
              activation_fee_amount: 20,
              claim_bond_amount: 80,
              upkeep_per_epoch: 5,
              eligible_claim_balance: 200,
              transferable_liquid_balance: 140,
              restricted_starter_claim_balance: 60,
              auto_restricted_starter_claim_amount: 25,
              release_cooldown_epochs: 2,
              grace_epochs: 1,
              idle_warning_epochs: 4,
              forced_idle_reclaim_epochs: 6,
              forced_reclaim_penalty_bps: 500,
              blocked_reason: null,
            },
            owned_claims: [
              {
                target_agent_id: "agent-0",
                status: "release_ready",
                upkeep_paid_through_epoch: 17,
                upfront_restricted_spent_amount: 30,
                upfront_liquid_spent_amount: 90,
                claim_bond_locked_restricted_amount: 20,
                claim_bond_locked_liquid_amount: 60,
                release_ready_at_epoch: 18,
                release_ready_in_epochs: 0,
                grace_remaining_epochs: 1,
                idle_warning_in_epochs: 2,
                forced_reclaim_in_epochs: 6,
              },
            ],
          },
        },
      }),
    });

    const stagePanel = container.querySelector("#viewer-stage-panel");
    expect(within(stagePanel).getByText("Claim-Agent Choice")).toBeInTheDocument();
    expect(within(stagePanel).getByText("Slot")).toBeInTheDocument();
    expect(within(stagePanel).getAllByText("2").length).toBeGreaterThan(0);
    expect(within(stagePanel).getByText("Owned / cap")).toBeInTheDocument();
    expect(within(stagePanel).getByText("1 / 3")).toBeInTheDocument();
    expect(within(stagePanel).getByText("Total upfront")).toBeInTheDocument();
    expect(within(stagePanel).getByText("Eligible balance")).toBeInTheDocument();
    expect(within(stagePanel).getByText("Liquid balance")).toBeInTheDocument();
    expect(within(stagePanel).getByText("Restricted starter")).toBeInTheDocument();
    expect(within(stagePanel).getByText("agent-0")).toBeInTheDocument();
    expect(within(stagePanel).getByText(/upkeep paid through epoch 17/i)).toBeInTheDocument();
    expect(within(stagePanel).getByText(/release ready in 0/i)).toBeInTheDocument();
    expect(within(stagePanel).getByText(/bond restricted=20 liquid=60/i)).toBeInTheDocument();
    expect(within(stagePanel).getByText(/Maintain by keeping control and upkeep healthy/i)).toBeInTheDocument();
    expect(within(stagePanel).queryByRole("button", { name: /claim/i })).not.toBeInTheDocument();
  }, HEAVY_UI_TEST_TIMEOUT_MS);

  it("does not describe disabled release claim actions as executable", async () => {
    const { container } = await renderViewerApp({
      snapshot: sampleSnapshot({
        player_gameplay: {
          ...sampleSnapshot().player_gameplay,
          agent_claim: {
            next_claim_quote: {
              slot_index: 2,
              owned_claim_count: 1,
              claim_cap: 3,
              blocked_reason: null,
            },
            owned_claims: [
              {
                target_agent_id: "agent-0",
                status: "release_ready",
                release_ready_in_epochs: 0,
              },
            ],
          },
          available_actions: [
            {
              action_id: "release_agent_claim",
              label: "Release agent claim",
              protocol_action: "gameplay_action.submit",
              target_agent_id: "agent-0",
              disabled_reason: "cooldown active",
            },
          ],
        },
      }),
    });

    const stagePanel = container.querySelector("#viewer-stage-panel");
    expect(within(stagePanel).getByText(/Release is published but currently disabled/i)).toBeInTheDocument();
    expect(within(stagePanel).queryByText(/execute it from the available actions list/i)).not.toBeInTheDocument();
  }, HEAVY_UI_TEST_TIMEOUT_MS);

  it("keeps blocked claim reasons as diagnostics instead of fake claim buttons", async () => {
    const { container } = await renderViewerApp({
      snapshot: sampleSnapshot({
        player_gameplay: {
          ...sampleSnapshot().player_gameplay,
          agent_claim: {
            next_claim_quote: {
              slot_index: 3,
              total_upfront_amount: 180,
              eligible_claim_balance: 90,
              transferable_liquid_balance: 90,
              blocked_reason: "insufficient reputation",
            },
            owned_claims: [],
          },
        },
      }),
    });

    const stagePanel = container.querySelector("#viewer-stage-panel");
    expect(within(stagePanel).getByText("Wait before claiming")).toBeInTheDocument();
    expect(within(stagePanel).getByText(/needs waiting, funding, or eligibility first/i)).toBeInTheDocument();
    expect(within(stagePanel).getByText("Claim blocker diagnostics")).toBeInTheDocument();
    expect(within(stagePanel).queryByText("insufficient reputation")).not.toBeInTheDocument();
    expect(within(stagePanel).queryByRole("button", { name: /claim/i })).not.toBeInTheDocument();
  }, HEAVY_UI_TEST_TIMEOUT_MS);

  it("renders expansion tradeoff cards from branch hint and published action labels", async () => {
    const { container } = await renderViewerApp({
      snapshot: sampleSnapshot({
        player_gameplay: {
          ...sampleSnapshot().player_gameplay,
          stage_status: "branch_ready",
          execution_state: "completed",
          goal_kind: "ChooseFirstExpansionTradeoff",
          goal_title: "Choose the first expansion tradeoff",
          branch_hint: "Choose whether the next branch buys throughput, resilience, or reach.",
          next_step_hint: "Commit to one branch only after checking the matched action.",
          available_actions: [
            {
              action_id: "build_alloy_factory",
              label: "Build alloy factory core",
              protocol_action: "gameplay_action.submit",
              disabled_reason: null,
            },
            {
              action_id: "install_sensor_pack",
              label: "Install sensor pack",
              protocol_action: "gameplay_action.submit",
              disabled_reason: "missing control chip",
            },
            {
              action_id: "launch_logistics_drone",
              label: "Launch logistics drone",
              protocol_action: "gameplay_action.submit",
              disabled_reason: null,
            },
          ],
        },
      }),
    });

    const stagePanel = container.querySelector("#viewer-stage-panel");
    expect(within(stagePanel).getByText("Expansion Tradeoffs")).toBeInTheDocument();
    expect(within(stagePanel).getAllByText("Choose whether the next branch buys throughput, resilience, or reach.").length).toBeGreaterThan(0);
    expect(within(stagePanel).getByText("Capacity / Throughput")).toBeInTheDocument();
    expect(within(stagePanel).getAllByText("Build alloy factory core").length).toBeGreaterThan(0);
    expect(within(stagePanel).getByText("Resilience / Input Protection")).toBeInTheDocument();
    expect(within(stagePanel).getByText("Install sensor pack: missing control chip")).toBeInTheDocument();
    expect(within(stagePanel).getByText("Logistics / Reach")).toBeInTheDocument();
    expect(within(stagePanel).getAllByText("Launch logistics drone").length).toBeGreaterThan(0);
    expect(within(stagePanel).getAllByText("Commit to one branch only after checking the matched action.").length).toBeGreaterThan(0);
  }, HEAVY_UI_TEST_TIMEOUT_MS);

  it("compresses world scale details into a one-line details-rail summary", async () => {
    const { container } = await renderViewerApp();

    const detailsPanel = container.querySelector("#viewer-details-panel");
    expect(within(detailsPanel).getByText("World Scale")).toBeInTheDocument();
    expect(within(detailsPanel).getByText("snapshot.config.space")).toBeInTheDocument();
    expect(within(detailsPanel).getByText(/World Bounds 100 km × 50 km × 10 km/i)).toBeInTheDocument();

    expect(within(detailsPanel).queryByText("Nearest Distance Samples")).not.toBeInTheDocument();
    expect(within(detailsPanel).queryByText("Selected location anchor")).not.toBeInTheDocument();
    expect(
      within(detailsPanel).queryByText(/The main runtime state already lives in World Summary/i),
    ).not.toBeInTheDocument();
  });

  it("dismisses the stage help tip on escape and outside click", async () => {
    const { container } = await renderViewerApp();

    const stagePanel = container.querySelector("#viewer-stage-panel");
    const helpButton = within(stagePanel).getByRole("button", { name: /open presentation scale guidance/i });

    fireEvent.click(helpButton);
    expect(helpButton).toHaveAttribute("aria-expanded", "true");

    fireEvent.keyDown(window, { key: "Escape" });
    await waitFor(() => expect(helpButton).toHaveAttribute("aria-expanded", "false"));

    fireEvent.click(helpButton);
    expect(helpButton).toHaveAttribute("aria-expanded", "true");

    fireEvent.pointerDown(document.body);
    await waitFor(() => expect(helpButton).toHaveAttribute("aria-expanded", "false"));
  });

  it("unlocks agent chat and prompt override surfaces once an agent is selected", async () => {
    const { core } = await renderViewerApp({
      selection: { kind: "agent", id: "agent-0" },
    });

    expect(screen.getByText("Agent Chat")).toBeInTheDocument();
    expect(screen.getByLabelText("Message")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Send Chat" })).toBeInTheDocument();

    expect(screen.getByText("Advanced Prompt Settings")).toBeInTheDocument();
    expect(screen.queryByLabelText("System Prompt Override")).not.toBeInTheDocument();

    core.togglePromptOverridesVisible();

    await waitFor(() => {
      expect(screen.getByLabelText("System Prompt Override")).toBeInTheDocument();
    });
    expect(screen.getByLabelText("Short-Term Goal Override")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Preview Prompt" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Apply Prompt" })).toBeInTheDocument();
  }, HEAVY_UI_TEST_TIMEOUT_MS);

  it("surfaces backend agent chat rejection details without requiring diagnostics", async () => {
    await renderViewerApp({
      selection: { kind: "agent", id: "agent-0" },
      setupAfterMount(core) {
        core.state.lastChatFeedback = {
          id: "chat-error-test",
          kind: "chat",
          action: "agent_chat",
          agentId: "agent-0",
          accepted: false,
          ok: false,
          stage: "error",
          reason: "claim starter OC before using LLM/agent chat for this Agent",
          effect: "insufficient_oc_budget",
          response: {
            code: "insufficient_oc_budget",
            message: "claim starter OC before using LLM/agent chat for this Agent",
            agent_id: "agent-0",
          },
        };
      },
    });

    const agentChatPanel = screen.getByText("Agent Chat").closest("section");
    expect(agentChatPanel).toBeTruthy();
    expect(within(agentChatPanel).getByText("Chat failed")).toBeInTheDocument();
    expect(within(agentChatPanel).getByText("code=insufficient_oc_budget")).toBeInTheDocument();
    expect(within(agentChatPanel).getByText(/Agent chat did not complete.*insufficient_oc_budget.*claim starter OC/i))
      .toBeInTheDocument();
    expect(within(agentChatPanel).getByText("claim starter OC before using LLM/agent chat for this Agent"))
      .toBeInTheDocument();
  }, HEAVY_UI_TEST_TIMEOUT_MS);

  it("persists acknowledged chat messages for the current world", async () => {
    const { core } = await renderViewerApp({
      snapshot: null,
      setupCore(core) {
        core.state.worldId = "world-chat-cache";
        core.state.wsUrl = "ws://127.0.0.1:5011";
        core.injectSnapshot(sampleSnapshot());
        core.applySelection({ kind: "agent", id: "agent-0" });
      },
    });

    core.pushChatHistory({
      id: "chat-cache-1",
      source: "player",
      agentId: "agent-0",
      targetAgentId: "agent-0",
      playerId: "player-one",
      speaker: "player-one",
      message: "Keep this message visible after refresh.",
      tick: 44,
    });

    const stored = JSON.parse(window.localStorage.getItem(core.chatHistoryStorageKey()));
    expect(stored).toEqual([
      expect.objectContaining({
        id: "chat-cache-1",
        source: "player",
        message: "Keep this message visible after refresh.",
      }),
    ]);
  }, HEAVY_UI_TEST_TIMEOUT_MS);

  it("hydrates cached chat history after a viewer refresh", async () => {
    await renderViewerApp({
      snapshot: null,
      selection: null,
      setupCore(core) {
        core.state.worldId = "world-chat-cache";
        core.state.wsUrl = "ws://127.0.0.1:5011";
        window.localStorage.setItem(
          core.chatHistoryStorageKey(),
          JSON.stringify([
            {
              id: "chat-cache-reload-1",
              source: "player",
              agentId: "agent-0",
              targetAgentId: "agent-0",
              playerId: "player-one",
              speaker: "player-one",
              message: "This message survived refresh.",
              tick: 45,
            },
          ]),
        );
        core.injectSnapshot(sampleSnapshot());
        core.applySelection({ kind: "agent", id: "agent-0" });
      },
    });

    expect(screen.getByText("This message survived refresh.")).toBeInTheDocument();
  }, HEAVY_UI_TEST_TIMEOUT_MS);

  it("renders chat history as readable messages with raw diagnostics behind disclosure", async () => {
    await renderViewerApp({
      selection: { kind: "agent", id: "agent-0" },
      setupAfterMount(core) {
        core.state.chatHistory = [
          {
            id: "chat-test-1",
            source: "player",
            agentId: "agent-0",
            targetAgentId: "agent-0",
            playerId: "player-one",
            speaker: "player-one",
            locationId: null,
            message: "Please restore the smelter line before expanding.",
            tick: 42,
            intentSeq: 7,
          },
        ];
      },
    });

    const messageFlow = screen.getByText("Message Flow").closest("div");
    expect(messageFlow).toBeTruthy();
    expect(screen.getByText("Player -> agent-0")).toBeInTheDocument();
    expect(screen.getByText("player-one · unknown location")).toBeInTheDocument();
    expect(screen.getByText("Please restore the smelter line before expanding.")).toBeInTheDocument();
    expect(screen.getByText("Raw diagnostics")).toBeInTheDocument();
    expect(screen.queryByText(/"message": "Please restore the smelter line before expanding."/)).not.toBeInTheDocument();
  }, HEAVY_UI_TEST_TIMEOUT_MS);

  it("keeps mounted dom stable across requestRender", async () => {
    const { core, container } = await renderViewerApp({
      selection: { kind: "agent", id: "agent-0" },
    });

    const searchInput = screen.getByRole("searchbox");
    const selectedBadge = screen.getByText("Current Selection");

    core.state.connectionStatus = "connected";
    core.requestRender();

    expect(screen.getByRole("searchbox")).toBe(searchInput);
    expect(screen.getByText("Current Selection")).toBe(selectedBadge);
    expect(container.querySelector("#viewer-stage-panel")).toBeTruthy();
  });

  it("keeps diagnostics visually demoted behind the player path surface", async () => {
    const { container } = await renderViewerApp();

    const summary = screen.getByText("Runtime Diagnostics").closest("summary");
    expect(summary).toBeTruthy();
    expect(summary).toHaveClass("diagnostic-surface__summary");
    const diagnosticsPanel = container.querySelector("#viewer-diagnostics-panel");
    expect(diagnosticsPanel).toBeTruthy();
    expect(diagnosticsPanel).toHaveClass("diagnostic-surface");
    expect(diagnosticsPanel).not.toHaveAttribute("open");

    const stagePanel = container.querySelector("#viewer-stage-panel");
    expect(stagePanel).toBeTruthy();
    expect(within(stagePanel).getByText("Formal Gameplay Summary")).toBeInTheDocument();
    expect(within(stagePanel).getAllByText("Accepted Intent").length).toBeGreaterThan(0);
    expect(within(stagePanel).getAllByText("Next Step").length).toBeGreaterThan(0);
    expect(within(stagePanel).getByText("Actions Not Exposed On This Page")).toBeInTheDocument();
  }, HEAVY_UI_TEST_TIMEOUT_MS);

  it("shows current player identity on the primary world surface", async () => {
    const { container } = await renderViewerApp({
      setupAfterMount(core) {
        core.state.auth = {
          ...core.state.auth,
          available: true,
          playerId: "local-test-player-visible",
          publicKey: "abcdef0123456789abcdef0123456789",
          privateKey: "private-key-must-stay-hidden",
          source: "local_test_api_ephemeral",
          registrationStatus: "registered",
          runtimeStatus: "registered",
          boundAgentId: "starter-agent-0",
        };
      },
    });

    const stagePanel = container.querySelector("#viewer-stage-panel");
    expect(stagePanel).toBeTruthy();
    const identityCard = within(stagePanel).getByTestId("viewer-identity-card");

    expect(within(identityCard).getByText("Current Identity")).toBeInTheDocument();
    expect(within(identityCard).getByText("Local Test Identity")).toBeInTheDocument();
    expect(within(identityCard).getByText(/player=local-test-player-visible/)).toBeInTheDocument();
    expect(within(identityCard).getByText(/pubkey=abcdef012345/)).toBeInTheDocument();
    expect(within(identityCard).getByText(/not an email login account/)).toBeInTheDocument();
    expect(identityCard).not.toHaveTextContent("private-key-must-stay-hidden");
  }, HEAVY_UI_TEST_TIMEOUT_MS);

  it("shows guest identity before local test auth is generated", async () => {
    const { container } = await renderViewerApp();

    const stagePanel = container.querySelector("#viewer-stage-panel");
    expect(stagePanel).toBeTruthy();
    const identityCard = within(stagePanel).getByTestId("viewer-identity-card");

    expect(within(identityCard).getByText("Current Identity")).toBeInTheDocument();
    expect(within(identityCard).getByText("Guest / Not Signed In")).toBeInTheDocument();
    expect(within(identityCard).getByText(/No player session is active yet/)).toBeInTheDocument();
  }, HEAVY_UI_TEST_TIMEOUT_MS);

  it("surfaces the first agent claim entry from gameplay snapshot", async () => {
    const { container } = await renderViewerApp({
      snapshot: sampleAgentClaimSnapshot(),
    });

    const stagePanel = container.querySelector("#viewer-stage-panel");
    expect(stagePanel).toBeTruthy();
    expect(within(stagePanel).getByText("Agent Claim")).toBeInTheDocument();
    expect(within(stagePanel).getByLabelText("Target Agent")).toHaveValue("agent-claim-target");
    expect(within(stagePanel).getByRole("button", { name: "Claim Agent" })).toBeInTheDocument();
    expect(within(stagePanel).getByText("claimer=agent-0")).toBeInTheDocument();
    expect(within(stagePanel).getByText("eligible=325")).toBeInTheDocument();
  }, HEAVY_UI_TEST_TIMEOUT_MS);

  it("surfaces starter OC claim before first agent chat", async () => {
    let sendGameplayAction;
    const { container, core } = await renderViewerApp({
      snapshot: sampleSnapshot({
        player_gameplay: {
          ...sampleSnapshot().player_gameplay,
          available_actions: [
            {
              action_id: "advance_step",
              label: "Advance 1 step",
              protocol_action: "live_control.step",
              disabled_reason: null,
            },
            {
              action_id: "claim_starter_oc",
              label: "Claim starter OC",
              protocol_action: "gameplay_action.submit",
              target_agent_id: "agent-0",
              disabled_reason: null,
            },
            {
              action_id: "chat_first_agent",
              label: "Send one chat/command to the first available agent",
              protocol_action: "agent_chat",
              target_agent_id: "agent-0",
              disabled_reason: "claim starter OC before using LLM/agent chat for this Agent",
            },
          ],
        },
      }),
      setupAfterMount(core) {
        sendGameplayAction = vi.spyOn(core, "sendGameplayAction").mockReturnValue({ ok: true });
      },
    });

    const stagePanel = container.querySelector("#viewer-stage-panel");
    expect(stagePanel).toBeTruthy();
    expect(within(stagePanel).getAllByText("Claim starter OC").length).toBeGreaterThan(0);
    expect(within(stagePanel).getAllByText(/one-time starter OC/i).length).toBeGreaterThan(0);
    expect(core.buildGameplaySummary().recommendedAction).toMatchObject({
      actionId: "claim_starter_oc",
      executeKind: "claim_starter_oc",
    });
    const claimButton = within(stagePanel).getAllByRole("button", { name: "Claim Starter OC" })[0];
    expect(claimButton).toBeInTheDocument();
    fireEvent.click(claimButton);
    expect(sendGameplayAction).toHaveBeenCalledWith(
      expect.objectContaining({
        actionId: "claim_starter_oc",
        targetAgentId: "agent-0",
        executeKind: "claim_starter_oc",
      }),
    );
    expect(within(stagePanel).getByRole("button", { name: "Use Chat Panel" })).toBeDisabled();
  }, HEAVY_UI_TEST_TIMEOUT_MS);

  it("forces goal execution blocked when the empty-entity guard trips", async () => {
    let sendGameplayAction;
    const { container } = await renderViewerApp({
      snapshot: sampleSnapshot({
        model: {
          agents: {},
          locations: {},
          agent_prompt_profiles: {},
          agent_execution_debug_contexts: {},
        },
        player_gameplay: {
          ...sampleSnapshot().player_gameplay,
          stage_status: "active",
          execution_state: "completed",
          blocker_kind: null,
          blocker_detail: null,
          available_actions: [
            {
              action_id: "request_snapshot",
              label: "Request snapshot",
              protocol_action: "request_snapshot",
              disabled_reason: null,
            },
            {
              action_id: "claim_first_agent",
              label: "Claim first Agent",
              protocol_action: "gameplay_action.submit",
              target_agent_id: "starter-agent-0",
              disabled_reason: null,
            },
          ],
        },
      }),
      setupAfterMount(core) {
        sendGameplayAction = vi.spyOn(core, "sendGameplayAction").mockReturnValue({ ok: true });
      },
    });

    const stagePanel = container.querySelector("#viewer-stage-panel");
    const targetsPanel = container.querySelector("#viewer-targets-panel");
    expect(stagePanel).toBeTruthy();
    expect(targetsPanel).toBeTruthy();
    expect(within(stagePanel).getByText("Goal Execution")).toBeInTheDocument();
    expect(within(stagePanel).getAllByText("Blocked").length).toBeGreaterThan(0);
    expect(within(stagePanel).getByText("World Constraint")).toBeInTheDocument();
    expect(within(stagePanel).getByText("Claim first Agent")).toBeInTheDocument();
    const claimButton = within(stagePanel).getAllByRole("button", { name: "Claim First Agent" })[0];
    expect(claimButton).toBeInTheDocument();
    fireEvent.click(claimButton);
    expect(sendGameplayAction).toHaveBeenCalledWith(
      expect.objectContaining({
        actionId: "claim_first_agent",
        targetAgentId: "starter-agent-0",
        executeKind: "claim_first_agent",
      }),
    );
    const recoveryClaimButton = within(targetsPanel).getByRole("button", { name: "Claim First Agent" });
    expect(recoveryClaimButton).toBeInTheDocument();
    fireEvent.click(recoveryClaimButton);
    expect(sendGameplayAction).toHaveBeenCalledTimes(2);
    expect(sendGameplayAction).toHaveBeenLastCalledWith(
      expect.objectContaining({
        actionId: "claim_first_agent",
        targetAgentId: "starter-agent-0",
        executeKind: "claim_first_agent",
      }),
    );
    expect(within(stagePanel).getByText(/New-user empty-world entry/i)).toBeInTheDocument();
    expect(within(targetsPanel).getByText("No agents in current snapshot.")).toBeInTheDocument();
    expect(within(targetsPanel).getByText("No locations in current snapshot.")).toBeInTheDocument();
    expect(within(targetsPanel).queryByText("Syncing agents…")).not.toBeInTheDocument();
  }, HEAVY_UI_TEST_TIMEOUT_MS);

  it("surfaces hosted recovery and preview strong-auth truth without not-implemented drift", async () => {
    await renderViewerApp({
      setupAfterMount(core) {
        core.state.hostedAccess = sampleHostedPublicJoinAccess();
        core.state.auth.error = "session_revoked";
        core.state.auth.revokeReason = "qa-kick";
        core.state.auth.revokedBy = "qa";
        core.state.hostedLogin.handle = "player@example.com";
      },
    });

    screen.getByText("Runtime Diagnostics").click();
    expect(screen.getByRole("dialog", { name: "Sign In With Email" })).toBeInTheDocument();
    expect(screen.getAllByRole("button", { name: "Request Login Code" }).length).toBeGreaterThan(0);
    expect(screen.getAllByLabelText("Email").length).toBeGreaterThan(0);
    expect(screen.getByText("upgrade_after_player_session")).toBeInTheDocument();
    expect(
      screen.getAllByText(
        "The runtime or operator revoked this browser session by qa. Reason: qa-kick. You need to re-login to the hosted account and acquire a fresh player session before gameplay, chat, or prompt actions can continue.",
      ).length,
    ).toBeGreaterThan(0);
    expect(screen.queryByText("not_implemented")).not.toBeInTheDocument();
    expect(screen.queryByText(/not implemented yet/i)).not.toBeInTheDocument();
  }, HEAVY_UI_TEST_TIMEOUT_MS);

  it("does not show the hosted login gate after player session registration", async () => {
    await renderViewerApp({
      setupAfterMount(core) {
        core.state.hostedAccess = sampleHostedPublicJoinAccess();
        core.state.auth = {
          ...core.state.auth,
          available: true,
          playerId: "hosted-player-1",
          publicKey: "oc:pk:test-player",
          privateKey: "ed25519-secret",
          releaseToken: "hosted-release-1",
          source: "hosted_browser_storage",
          registrationStatus: "registered",
          runtimeStatus: "registered",
        };
      },
    });

    expect(screen.queryByRole("dialog", { name: "Sign In With Email" })).not.toBeInTheDocument();
  });

  it("surfaces hosted login retry-after guidance when OTP resend is throttled", async () => {
    await renderViewerApp({
      setupAfterMount(core) {
        core.state.hostedAccess = sampleHostedPublicJoinAccess();
        core.state.hostedLogin.handle = "player@example.com";
        core.state.hostedLogin.error = "a login code was just sent for this email; retry in 21 seconds (retry in 21s)";
        core.state.hostedLogin.retryAfterSeconds = 21;
      },
    });

    screen.getByText("Runtime Diagnostics").click();
    expect(
      screen.getAllByText("a login code was just sent for this email; retry in 21 seconds (retry in 21s)").length,
    ).toBeGreaterThan(0);
    expect(screen.getAllByText("retry_after=21s").length).toBeGreaterThan(0);
  });

  it("clears hosted login retry guidance immediately when the player edits the email", async () => {
    await renderViewerApp({
      setupAfterMount(core) {
        core.state.hostedAccess = sampleHostedPublicJoinAccess();
        core.state.hostedLogin.handle = "player@example.com";
        core.state.hostedLogin.error = "a login code was just sent for this email; retry in 21 seconds (retry in 21s)";
        core.state.hostedLogin.retryAfterSeconds = 21;
      },
    });

    expect(screen.getAllByText("retry_after=21s").length).toBeGreaterThan(0);

    fireEvent.input(document.getElementById("gate-hosted-login-handle"), {
      target: { value: "next-player@example.com" },
    });

    expect(screen.queryByText("retry_after=21s")).not.toBeInTheDocument();
    expect(screen.queryByText(/retry in 21 seconds/i)).not.toBeInTheDocument();
  });

  it("keeps keyboard focus inside the hosted login gate while it is modal", async () => {
    await renderViewerApp({
      setupAfterMount(core) {
        core.state.hostedAccess = sampleHostedPublicJoinAccess();
        core.state.hostedLogin.handle = "player@example.com";
      },
    });

    const dialog = screen.getByRole("dialog", { name: "Sign In With Email" });
    const emailInput = document.getElementById("gate-hosted-login-handle");
    const requestButton = within(dialog).getByRole("button", { name: "Request Login Code" });

    await waitFor(() => {
      expect(document.activeElement).toBe(emailInput);
    });

    requestButton.focus();
    const tabEvent = new KeyboardEvent("keydown", {
      key: "Tab",
      bubbles: true,
      cancelable: true,
    });
    dialog.dispatchEvent(tabEvent);

    expect(tabEvent.defaultPrevented).toBe(true);
    expect(document.activeElement).toBe(emailInput);
  });

  it("marks hosted backend reauth as available once a browser player session is registered", async () => {
    await renderViewerApp({
      setupAfterMount(core) {
        core.state.hostedAccess = sampleHostedPublicJoinAccess();
        core.state.auth = {
          ...core.state.auth,
          available: true,
          playerId: "hosted-player-1",
          publicKey: "oc:pk:test-player",
          privateKey: "ed25519-secret",
          releaseToken: "hosted-release-1",
          source: "hosted_browser_storage",
          registrationStatus: "registered",
          runtimeStatus: "registered",
        };
      },
    });

    screen.getByText("Runtime Diagnostics").click();
    expect(screen.getAllByRole("button", { name: "Release Player Session" }).length).toBeGreaterThan(0);
    expect(screen.getByText("active_hosted_session")).toBeInTheDocument();
    expect(screen.getByText("preview_backend_reauth_available")).toBeInTheDocument();
    expect(
      screen.getByText(
        "hosted preview backend reauth is available after the browser device-session-backed player_session has completed runtime registration for prompt_control",
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByText("page reload will reuse the hosted device session, mint a fresh browser session key, and attempt reconnect_sync first"),
    ).toBeInTheDocument();
    const assetLane = screen.getByText("Asset / Governance Lane").closest("section");
    expect(assetLane).toBeTruthy();
    expect(
      within(assetLane).getAllByText(/main_token_transfer remains blocked until a higher-trust hosted strong-auth lane exists/i).length,
    ).toBeGreaterThan(0);
    expect(screen.queryByText("not_implemented")).not.toBeInTheDocument();
  }, HEAVY_UI_TEST_TIMEOUT_MS);

  it("keeps hosted backend reauth pending until runtime registration finishes", async () => {
    await renderViewerApp({
      setupAfterMount(core) {
        core.state.hostedAccess = sampleHostedPublicJoinAccess();
        core.state.auth = {
          ...core.state.auth,
          available: true,
          playerId: "hosted-player-2",
          publicKey: "oc:pk:test-player-2",
          privateKey: "ed25519-secret-2",
          releaseToken: "hosted-release-2",
          source: "hosted_browser_storage",
          registrationStatus: "issued",
          runtimeStatus: "issued",
        };
      },
    });

    screen.getByText("Runtime Diagnostics").click();
    expect(screen.getAllByText("issued_pending_register").length).toBeGreaterThan(0);
    expect(screen.queryByText("preview_backend_reauth_available")).not.toBeInTheDocument();
    expect(
      screen.getByText(
        "hosted preview backend reauth stays pending until the browser device-session-backed player_session finishes runtime registration",
      ),
    ).toBeInTheDocument();
  }, HEAVY_UI_TEST_TIMEOUT_MS);

  it("installs visual fixture states for screenshot parity without production semantics", async () => {
    const states = [
      "shell_selected_blocker",
      "agent_chat_history",
      "gameplay_diagnostics_expanded",
      "hosted_login_gate",
      "empty_world_recovery",
    ];

    for (const fixtureName of states) {
      const { cleanup, container } = await renderViewerApp({
        snapshot: null,
        search: `${viewerUrl()}&viewer_visual_fixture=${fixtureName}`,
      });

      expect(window.__OASIS7_VIEWER_VISUAL_FIXTURES__).toBeTruthy();
      expect(container).toHaveAttribute("data-viewer-visual-fixture", fixtureName);
      expect(document.body).toHaveAttribute("data-viewer-visual-fixture", fixtureName);

      cleanup();
    }
  }, HEAVY_UI_TEST_TIMEOUT_MS);

  it("keeps visual fixture query parameters inert without test_api", async () => {
    const { container } = await renderViewerAppThroughAutoMount({
      snapshot: null,
      search: "/software_safe.html?connect=0&hosted_bootstrap=0&locale=en&viewer_visual_fixture=gameplay_diagnostics_expanded",
    });

    expect(window.__OASIS7_VIEWER_VISUAL_FIXTURES__).toBeUndefined();
    expect(container).not.toHaveAttribute("data-viewer-visual-fixture");
    expect(document.body).not.toHaveAttribute("data-viewer-visual-fixture");
    expect(container.querySelector("#viewer-gameplay-details")).toHaveAttribute("open");
    expect(container.querySelector("#viewer-diagnostics-panel")).not.toHaveAttribute("open");
    expect(elementPrecedes(
      container.querySelector(".stage-hero"),
      container.querySelector("#viewer-gameplay-details"),
    )).toBe(true);
  }, HEAVY_UI_TEST_TIMEOUT_MS);

  it("renders the shell selected-blocker fixture as a populated command desk", async () => {
    const { container } = await renderViewerApp({
      snapshot: null,
      search: `${viewerUrl()}&viewer_visual_fixture=shell_selected_blocker`,
    });

    const state = window.__AW_TEST__.getState();
    expect(state.selectedKind).toBe("agent");
    expect(state.selectedId).toBe("agent-0");
    expect(within(container.querySelector("#viewer-targets-panel")).getByText("agent-0")).toBeInTheDocument();
    expect(within(container.querySelector("#viewer-targets-panel")).getByText("Assembly Nexus")).toBeInTheDocument();
    expect(within(container.querySelector("#viewer-stage-panel")).getAllByText("Recover sustainable capability").length).toBeGreaterThan(0);
    expect(within(container.querySelector("#viewer-details-panel")).getByText("Agent Chat")).toBeInTheDocument();
  }, HEAVY_UI_TEST_TIMEOUT_MS);

  it("renders the agent chat fixture with history and collapsed prompt controls", async () => {
    const { container } = await renderViewerApp({
      snapshot: null,
      search: `${viewerUrl()}&viewer_visual_fixture=agent_chat_history`,
    });

    const detailsPanel = container.querySelector("#viewer-details-panel");
    const commandSurface = detailsPanel.querySelector(".command-surface");
    expect(commandSurface).toHaveAttribute("data-command-agent", "agent-0");
    expect(commandSurface).toHaveAttribute("data-command-chat-history", "3");
    expect(commandSurface.querySelector(".command-surface__chat-panel")).toBeTruthy();
    expect(elementPrecedes(
      commandSurface.querySelector(".command-surface__chat-panel"),
      commandSurface.querySelector(".command-surface__advanced-panel"),
    )).toBe(true);
    expect(within(detailsPanel).getByText("Chat Ready")).toBeInTheDocument();
    expect(within(detailsPanel).getByText("Awaiting material recovery before the smelter can proceed.")).toBeInTheDocument();
    expect(within(detailsPanel).getByText("Hold position and confirm the blocker.")).toBeInTheDocument();
    expect(within(detailsPanel).getByText("state=hidden_by_default")).toBeInTheDocument();
    expect(within(detailsPanel).queryByLabelText("System Prompt Override")).not.toBeInTheDocument();
    expect(within(detailsPanel).getAllByText(/no transfer form/i).length).toBeGreaterThan(0);
  }, HEAVY_UI_TEST_TIMEOUT_MS);

  it("opens gameplay details and diagnostics for the diagnostics visual fixture", async () => {
    const { container } = await renderViewerApp({
      snapshot: null,
      search: `${viewerUrl()}&viewer_visual_fixture=gameplay_diagnostics_expanded`,
    });

    await waitFor(() => {
      expect(container.querySelector("#viewer-gameplay-details")).toHaveAttribute("open");
      expect(container.querySelector("#viewer-diagnostics-panel")).toHaveAttribute("open");
    });
    expect(within(container.querySelector("#viewer-gameplay-details")).getByText("Formal Gameplay Summary")).toBeInTheDocument();
    expect(within(container.querySelector("#viewer-gameplay-details")).getByText("Capability Economics")).toBeInTheDocument();
    expect(within(container.querySelector("#viewer-diagnostics-panel")).getByText("Execution Lanes")).toBeInTheDocument();
    expect(within(container.querySelector("#viewer-diagnostics-panel")).getByText("state sync")).toBeInTheDocument();
    expect(elementPrecedes(
      container.querySelector("#viewer-gameplay-details"),
      container.querySelector(".stage-hero"),
    )).toBe(true);
  }, HEAVY_UI_TEST_TIMEOUT_MS);

  it("forces a hosted login gate fixture with reserved retry/error guidance", async () => {
    await renderViewerApp({
      snapshot: null,
      search: `${viewerUrl()}&viewer_visual_fixture=hosted_login_gate`,
    });

    const dialog = screen.getByRole("dialog", { name: "Sign In With Email" });
    expect(dialog).toHaveAttribute("data-viewer-fixture-state", "hosted_login_gate");
    expect(within(dialog).getByDisplayValue("player@example.com")).toBeInTheDocument();
    expect(within(dialog).getByText("challenge=fixture-challenge")).toBeInTheDocument();
    expect(within(dialog).getByText("Enter the latest verification code to continue.")).toBeInTheDocument();
    expect(within(dialog).getByText("retry_after=18s")).toBeInTheDocument();
    expect(screen.queryByText(/wallet/i)).not.toBeInTheDocument();
  }, HEAVY_UI_TEST_TIMEOUT_MS);

  it("renders empty-world recovery as intentional recovery instead of selected-agent acceptance", async () => {
    const { container } = await renderViewerApp({
      snapshot: null,
      search: `${viewerUrl()}&viewer_visual_fixture=empty_world_recovery`,
    });

    const state = window.__AW_TEST__.getState();
    expect(state.selectedKind).toBe(null);
    expect(state.selectedId).toBe(null);
    expect(state.gameplaySummary.blockerKind).toBe("runtime_snapshot_empty_entities");
    expect(within(container.querySelector("#viewer-targets-panel")).getByText("No agents in current snapshot.")).toBeInTheDocument();
    expect(within(container.querySelector("#viewer-stage-panel")).getByText("Recover World Snapshot")).toBeInTheDocument();
    expect(within(container.querySelector("#viewer-stage-panel")).getAllByText("Request snapshot").length).toBeGreaterThan(0);
    expect(within(container.querySelector("#viewer-details-panel")).getByText("Claim Your First Agent")).toBeInTheDocument();
    expect(container.querySelector("[data-callout-kind='empty_world_recovery']")).toBeTruthy();
  }, HEAVY_UI_TEST_TIMEOUT_MS);
});
