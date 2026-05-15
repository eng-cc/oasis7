import { screen, waitFor, within } from "@solidjs/testing-library";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("./pixel_world_host.jsx", () => ({
  PixelWorldHost: (props) => (
    <div data-testid="pixel-world-host">
      {`pixel-world-host:${typeof props.locale === "function" ? props.locale() : props.locale}`}
    </div>
  ),
}));

function viewerUrl() {
  return "/software_safe.html?test_api=1&connect=0&locale=en";
}

let activeCleanup = null;

function sampleSnapshot(overrides = {}) {
  return {
    time: 12,
    config: {
      space: {
        width_cm: 10_000_000,
        depth_cm: 5_000_000,
        height_cm: 1_000_000,
      },
    },
    model: {
      agents: {
        "agent-0": {
          id: "agent-0",
          name: "Agent 0",
          location_id: "loc-0",
          resources: {},
        },
      },
      locations: {
        "loc-0": {
          id: "loc-0",
          name: "Factory Anchor",
          pos: { x_cm: 0, y_cm: 0, z_cm: 0 },
          profile: { radius_cm: 25_000, radiation_emission_per_tick: 0, material: "silicate" },
          resources: {},
        },
      },
      agent_prompt_profiles: {},
      agent_execution_debug_contexts: {},
    },
    player_gameplay: {
      stage_id: "post_onboarding",
      stage_status: "blocked",
      execution_state: "blocked",
      goal_id: "post_onboarding.recover_capability",
      goal_kind: "RecoverCapability",
      goal_title: "Recover sustainable capability",
      objective: "Stabilize the first production line before expanding.",
      progress_detail: "The primary line is blocked by missing material input.",
      progress_percent: 68,
      blocker_kind: "material_shortage",
      blocker_detail: "iron input exhausted at factory-0",
      causality_kind: "world_constraint",
      causality_detail: "iron input exhausted at factory-0",
      blocker_supplemental_detail: null,
      next_step_hint: "Replenish upstream materials, then advance again to confirm the line resumes.",
      branch_hint: null,
      available_actions: [
        {
          action_id: "request_snapshot",
          label: "Request snapshot",
          protocol_action: "world.request_snapshot",
          disabled_reason: null,
        },
      ],
      recent_feedback: null,
      agent_claim: null,
    },
    ...overrides,
  };
}

function sampleHostedPublicJoinAccess(overrides = {}) {
  return {
    deployment_mode: "hosted_public_join",
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
  setupCore = null,
  setupAfterMount = null,
} = {}) {
  activeCleanup?.();
  activeCleanup = null;
  vi.resetModules();
  window.history.replaceState({}, "", viewerUrl());
  window.localStorage.clear();
  document.body.innerHTML = "";

  const core = await import("./legacy_core.js");
  const { mountViewerApp } = await import("./main.jsx");
  const appRoot = document.createElement("div");
  appRoot.id = "app";
  document.body.appendChild(appRoot);

  core.setViewerLocale("en");
  core.injectSnapshot(snapshot);
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

beforeEach(() => {
  window.history.replaceState({}, "", viewerUrl());
  window.localStorage.clear();
  document.body.innerHTML = "";
});

afterEach(() => {
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

    const targetsPanel = container.querySelector("#viewer-targets-panel");
    const stagePanel = container.querySelector("#viewer-stage-panel");
    const detailsPanel = container.querySelector("#viewer-details-panel");

    expect(targetsPanel).toBeTruthy();
    expect(stagePanel).toBeTruthy();
    expect(detailsPanel).toBeTruthy();
    expect(within(targetsPanel).getByText("Targets")).toBeInTheDocument();
    expect(within(stagePanel).getByText("Industrial World Command Desk")).toBeInTheDocument();
    expect(within(stagePanel).getAllByText("Recover sustainable capability").length).toBeGreaterThan(0);
    expect(within(stagePanel).getByText("Goal Execution")).toBeInTheDocument();
    expect(within(stagePanel).getByText("World Constraint")).toBeInTheDocument();
    expect(within(stagePanel).getByText("Current Blocker")).toBeInTheDocument();
    expect(within(stagePanel).getByText("Runtime Diagnostics")).toBeInTheDocument();
    expect(within(stagePanel).getByText("Session Ladder")).toBeInTheDocument();
    expect(within(stagePanel).getByText("Runtime Diagnostics")).toBeInTheDocument();
    expect(screen.getByTestId("pixel-world-host")).toHaveTextContent("pixel-world-host:en");
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
  });

  it("keeps diagnostics visually demoted behind the player path surface", async () => {
    const { container } = await renderViewerApp();

    const summary = screen.getByText("Runtime Diagnostics").closest("summary");
    expect(summary).toBeTruthy();
    expect(summary).toHaveClass("diagnostic-surface__summary");

    const stagePanel = container.querySelector("#viewer-stage-panel");
    expect(stagePanel).toBeTruthy();
    expect(within(stagePanel).getByText("Formal Gameplay Summary")).toBeInTheDocument();
    expect(within(stagePanel).getAllByText("Next Step").length).toBeGreaterThan(0);
    expect(within(stagePanel).getByText("Actions Not Exposed On This Page")).toBeInTheDocument();
  });

  it("forces goal execution blocked when the empty-entity guard trips", async () => {
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
        },
      }),
    });

    const stagePanel = container.querySelector("#viewer-stage-panel");
    expect(stagePanel).toBeTruthy();
    expect(within(stagePanel).getByText("Goal Execution")).toBeInTheDocument();
    expect(within(stagePanel).getAllByText("Blocked").length).toBeGreaterThan(0);
    expect(within(stagePanel).getByText("World Constraint")).toBeInTheDocument();
  });
  it("surfaces hosted recovery and preview strong-auth truth without not-implemented drift", async () => {
    await renderViewerApp({
      setupAfterMount(core) {
        core.state.hostedAccess = sampleHostedPublicJoinAccess();
        core.state.auth.error = "session_revoked";
        core.state.auth.revokeReason = "qa-kick";
        core.state.auth.revokedBy = "qa";
      },
    });

    screen.getByText("Runtime Diagnostics").click();
    expect(screen.getAllByRole("button", { name: "Re-acquire Hosted Player Session" }).length).toBeGreaterThan(0);
    expect(screen.getByText("upgrade_after_player_session")).toBeInTheDocument();
    expect(
      screen.getAllByText(
        "The runtime or operator revoked this browser session by qa. Reason: qa-kick. You need to acquire a fresh hosted player session before gameplay, chat, or prompt actions can continue.",
      ).length,
    ).toBeGreaterThan(0);
    expect(screen.queryByText("not_implemented")).not.toBeInTheDocument();
    expect(screen.queryByText(/not implemented yet/i)).not.toBeInTheDocument();
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
        "hosted preview backend reauth is available after the browser-local player_session has completed runtime registration for prompt_control",
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByText("page reload will reuse the browser-local hosted key and attempt reconnect_sync first"),
    ).toBeInTheDocument();
    const assetLane = screen.getByText("Asset / Governance Lane").closest("section");
    expect(assetLane).toBeTruthy();
    expect(
      within(assetLane).getAllByText(/main_token_transfer remains blocked until a higher-trust hosted strong-auth lane exists/i).length,
    ).toBeGreaterThan(0);
    expect(screen.queryByText("not_implemented")).not.toBeInTheDocument();
  });

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
        "hosted preview backend reauth stays pending until the browser-local player_session finishes runtime registration",
      ),
    ).toBeInTheDocument();
  });
});
