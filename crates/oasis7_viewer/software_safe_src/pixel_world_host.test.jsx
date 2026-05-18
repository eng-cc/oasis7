import { render, screen, waitFor } from "@solidjs/testing-library";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("./pixel_world_runtime_loader.js", () => ({
  createPixelWorldRuntimeBridge: async ({ onFatal }) => ({
    source: "wasm_import_failed",
    moduleUrl: "http://127.0.0.1:4173/pixel-world-bridge/pixel_world_bridge.js",
    bridge: {
      mount() {
        const fatal = {
          code: "pixel_world_renderer_runtime_unavailable",
          message: "pixel world wasm runtime is unavailable: missing wasm bridge",
        };
        onFatal?.(fatal);
        return {
          status: "fallback",
          fatal,
        };
      },
      update() {
        return {
          status: "fallback",
        };
      },
      unmount() {
        return {
          status: "detached",
        };
      },
    },
  }),
}));

let activeCleanup = null;

function sampleSnapshot() {
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
      goal_id: "post_onboarding.recover_capability",
      goal_kind: "RecoverCapability",
      goal_title: "Recover sustainable capability",
      objective: "Stabilize the first production line before expanding.",
      progress_detail: "The primary line is blocked by missing material input.",
      progress_percent: 68,
      blocker_kind: "material_shortage",
      blocker_detail: "iron input exhausted at factory-0",
      blocker_supplemental_detail: null,
      next_step_hint: "Replenish upstream materials, then advance again to confirm the line resumes.",
      branch_hint: null,
      available_actions: [],
      recent_feedback: null,
      agent_claim: null,
    },
  };
}

async function renderPixelWorldHost() {
  activeCleanup?.();
  activeCleanup = null;
  vi.resetModules();
  window.history.replaceState({}, "", "/software_safe.html?test_api=1&connect=0&locale=en");
  window.localStorage.clear();
  document.body.innerHTML = "";

  const core = await import("./legacy_core.js");
  const { PixelWorldHost } = await import("./pixel_world_host.jsx");

  core.setViewerLocale("en");
  core.injectSnapshot(sampleSnapshot());

  const view = render(() => <PixelWorldHost locale="en" />);
  activeCleanup = view.unmount;
  return {
    core,
    ...view,
  };
}

beforeEach(() => {
  window.history.replaceState({}, "", "/software_safe.html?test_api=1&connect=0&locale=en");
  window.localStorage.clear();
  document.body.innerHTML = "";
});

afterEach(() => {
  activeCleanup?.();
  activeCleanup = null;
  document.body.innerHTML = "";
});

describe("pixel world host", () => {
  it("builds richer visual DTO layers from the existing snapshot contract", async () => {
    vi.resetModules();
    window.history.replaceState({}, "", "/software_safe.html?test_api=1&connect=0&locale=en");
    window.localStorage.clear();
    document.body.innerHTML = "";

    const core = await import("./legacy_core.js");
    const { buildPixelWorldRenderState } = await import("./pixel_world_host.jsx");

    const snapshot = sampleSnapshot();
    snapshot.model.agents["agent-0"].pos = { x_cm: 25_000, y_cm: 25_000, z_cm: 0 };
    core.injectSnapshot(snapshot);
    core.state.recentEvents = [
      { eventId: "evt-1", title: "Transfer spike", kind: "resource_transfer" },
      { eventId: "evt-2", title: "Queue update", kind: "build_queue" },
    ];

    const renderState = buildPixelWorldRenderState("en");
    expect(renderState.links).toHaveLength(1);
    expect(renderState.visual_hotspots.length).toBeGreaterThanOrEqual(4);
    expect(renderState.visual_hotspots.some((entry) => entry.kind === "goal")).toBe(true);
    expect(renderState.visual_hotspots.some((entry) => entry.kind === "blocker")).toBe(true);
  });

  it("shows the explicit fallback surface when the wasm runtime is unavailable", async () => {
    const { core } = await renderPixelWorldHost();

    await waitFor(() => {
      expect(screen.getByText("Renderer Not Attached")).toBeInTheDocument();
    });

    expect(screen.getByText(/falls back explicitly instead of keeping a second JS renderer/i)).toBeInTheDocument();
    expect(screen.getByText(/pixel_world_renderer_runtime_unavailable/i)).toBeInTheDocument();
    expect(core.state.lastError).toContain("pixel world wasm runtime is unavailable");
  });
});
