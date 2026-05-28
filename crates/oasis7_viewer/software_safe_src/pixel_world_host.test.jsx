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
          pos: { x_cm: 5_000_000, y_cm: 2_500_000, z_cm: 0 },
          profile: { radius_cm: 25_000, radiation_emission_per_tick: 0, material: "silicate" },
          fragment_profile: {
            blocks: {
              blocks: [
                {
                  origin_cm: { x_cm: 0, y_cm: 0, z_cm: 0 },
                  size_cm: { x_cm: 12_000, y_cm: 7_500, z_cm: 8_000 },
                  density_kg_per_m3: 3200,
                  compounds: {
                    ppm: {
                      silicate_matrix: 800_000,
                      water_ice: 200_000,
                    },
                  },
                },
                {
                  origin_cm: { x_cm: 20_000, y_cm: 1_000, z_cm: 18_000 },
                  size_cm: { x_cm: 20_000, y_cm: 8_000, z_cm: 10_000 },
                  density_kg_per_m3: 7800,
                  compounds: {
                    ppm: {
                      iron_nickel_alloy: 900_000,
                      sulfide_ore: 100_000,
                    },
                  },
                },
              ],
            },
          },
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

  it("derives fragment terrain from location fragment blocks and de-emphasizes location markers", async () => {
    vi.resetModules();
    window.history.replaceState({}, "", "/software_safe.html?test_api=1&connect=0&locale=en");
    window.localStorage.clear();
    document.body.innerHTML = "";

    const core = await import("./legacy_core.js");
    const { buildPixelWorldRenderState } = await import("./pixel_world_host.jsx");

    core.injectSnapshot(sampleSnapshot());

    const renderState = buildPixelWorldRenderState("en");
    expect(renderState.fragment_terrain).toHaveLength(2);
    expect(renderState.locations[0].marker_role).toBe("logic_anchor");
    expect(renderState.locations[0].marker_alpha).toBeLessThan(0.5);

    const metalPatch = renderState.fragment_terrain.find((patch) => (
      patch.dominant_compound === "iron_nickel_alloy"
    ));
    expect(metalPatch).toMatchObject({
      id: "fragment:loc-0:1",
      location_id: "loc-0",
      footprint_cm: 20_000,
      color: [176, 184, 196],
    });
    expect(metalPatch.pos.x_cm).toBeGreaterThan(renderState.locations[0].pos.x_cm);
  });

  it("derives deterministic agent positions from assigned locations when snapshots omit agent coordinates", async () => {
    vi.resetModules();
    window.history.replaceState({}, "", "/software_safe.html?test_api=1&connect=0&locale=en");
    window.localStorage.clear();
    document.body.innerHTML = "";

    const core = await import("./legacy_core.js");
    const { buildPixelWorldRenderState } = await import("./pixel_world_host.jsx");

    core.injectSnapshot(sampleSnapshot());

    const firstState = buildPixelWorldRenderState("en");
    const secondState = buildPixelWorldRenderState("en");
    const agent = firstState.agents.find((entry) => entry.id === "agent-0");

    expect(agent.position_source).toBe("location_derived");
    expect(agent.pos).toEqual(secondState.agents.find((entry) => entry.id === "agent-0").pos);
    expect(agent.status_badges).toContain("position=location_derived");
    expect(firstState.links).toHaveLength(1);
    expect(firstState.links[0].from).toEqual(agent.pos);
    expect(firstState.links[0].to).toEqual(firstState.locations[0].pos);
  });

  it("shows the explicit fallback surface when the wasm runtime is unavailable", async () => {
    const { core } = await renderPixelWorldHost();

    await waitFor(() => {
      expect(screen.getByText("Renderer Not Attached")).toBeInTheDocument();
    });

    expect(screen.getByText(/falls back explicitly instead of keeping a second JS renderer/i)).toBeInTheDocument();
    expect(screen.getByText(/pixel_world_renderer_runtime_unavailable/i)).toBeInTheDocument();
    expect(document.querySelectorAll(".pixel-world-fragment-terrain")).toHaveLength(2);
    expect(document.querySelector(".pixel-world-entity--location")).toHaveAttribute("data-marker-role", "logic_anchor");
    expect(core.state.lastError).toContain("pixel world wasm runtime is unavailable");
  });

  it("keeps fragment terrain as non-interactive background behind readable agents", async () => {
    await renderPixelWorldHost();

    await waitFor(() => {
      expect(screen.getByText("Renderer Not Attached")).toBeInTheDocument();
    });

    const canvas = document.querySelector(".pixel-world-canvas");
    const fragments = Array.from(canvas.querySelectorAll(".pixel-world-fragment-terrain"));
    const location = canvas.querySelector(".pixel-world-entity--location");
    const agent = canvas.querySelector(".pixel-world-entity--agent");
    const children = Array.from(canvas.children);

    expect(fragments).toHaveLength(2);
    expect(fragments.every((fragment) => fragment.tagName === "DIV")).toBe(true);
    expect(fragments.every((fragment) => fragment.getAttribute("role") === null)).toBe(true);
    expect(children.indexOf(fragments[0])).toBeLessThan(children.indexOf(location));
    expect(children.indexOf(location)).toBeLessThan(children.indexOf(agent));
    expect(parseFloat(fragments[0].style.width)).toBeLessThanOrEqual(26);
    expect(parseFloat(location.style.opacity)).toBeLessThan(0.5);
    expect(location).toHaveAttribute("data-marker-role", "logic_anchor");
    expect(agent).toHaveAttribute("data-position-source", "location_derived");
  });
});
