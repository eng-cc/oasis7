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

function clone(value) {
  return JSON.parse(JSON.stringify(value));
}

function numericInlineStyle(element, property) {
  const value = Number.parseFloat(element.style[property]);
  expect(Number.isFinite(value)).toBe(true);
  return value;
}

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
      execution_state: "blocked",
      accepted_intent_id: "gameplay_action:build_factory_smelter_mk1",
      intent_summary: "Queue build_factory_smelter_mk1 for agent-0",
      intent_scope: "gameplay_action",
      intent_target: "agent-0",
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
      last_world_change: "Smelter build request reached factory-0; iron shortage blocks construction.",
      blocker_supplemental_detail: null,
      next_step_hint: "Replenish upstream materials, then advance again to confirm the line resumes.",
      branch_hint: null,
      available_actions: [
        {
          action_id: "build_factory_smelter_mk1",
          target_agent_id: "agent-0",
          label: "Build smelter mk1",
          protocol_action: "gameplay_action.submit",
          disabled_reason: null,
        },
      ],
      recent_feedback: {
        action: "build_factory_smelter_mk1",
        stage: "completed_no_progress",
        effect: "Smelter build request reached factory-0; iron shortage blocks construction.",
        reason: "iron input exhausted at factory-0",
        hint: "Replenish upstream materials, then advance again.",
        delta_logical_time: 1,
        delta_event_seq: 2,
      },
      agent_claim: null,
    },
  };
}

function acceptedOnlySnapshot() {
  const snapshot = clone(sampleSnapshot());
  const gameplay = snapshot.player_gameplay;
  gameplay.stage_status = "executing";
  gameplay.execution_state = "accepted";
  gameplay.blocker_kind = null;
  gameplay.blocker_detail = null;
  gameplay.causality_kind = null;
  gameplay.causality_detail = null;
  gameplay.last_world_change = null;
  gameplay.recent_feedback = {
    action: "build_factory_smelter_mk1",
    stage: "accepted",
    effect: null,
    reason: null,
    hint: "Build request queued for agent-0.",
    delta_logical_time: 0,
    delta_event_seq: 1,
  };
  return snapshot;
}

function noReceiptSnapshot() {
  const snapshot = clone(sampleSnapshot());
  const gameplay = snapshot.player_gameplay;
  gameplay.stage_status = "running";
  delete gameplay.execution_state;
  delete gameplay.accepted_intent_id;
  delete gameplay.intent_summary;
  delete gameplay.intent_scope;
  delete gameplay.intent_target;
  delete gameplay.blocker_kind;
  delete gameplay.blocker_detail;
  delete gameplay.causality_kind;
  delete gameplay.causality_detail;
  delete gameplay.last_world_change;
  gameplay.progress_detail = "The first production line is waiting for a player command.";
  gameplay.recent_feedback = null;
  return snapshot;
}

async function renderPixelWorldHost(snapshot = sampleSnapshot()) {
  activeCleanup?.();
  activeCleanup = null;
  vi.resetModules();
  window.history.replaceState({}, "", "/software_safe.html?test_api=1&connect=0&locale=en");
  window.localStorage.clear();
  document.body.innerHTML = "";

  const core = await import("./legacy_core.js");
  const { PixelWorldHost } = await import("./pixel_world_host.jsx");

  core.setViewerLocale("en");
  core.injectSnapshot(snapshot);

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
    expect(renderState.visual_hotspots).toHaveLength(4);
    expect(renderState.visual_hotspots.map((entry) => entry.kind)).toEqual([
      "goal",
      "blocker",
      "resource_transfer",
      "build_queue",
    ]);
    expect(renderState.visual_hotspots.every((entry) => entry.pos)).toBe(true);
    expect(renderState.commercial_surface).toMatchObject({
      active_agent_id: "agent-0",
      objective: {
        title: "Recover sustainable capability",
      },
      next_action: {
        label: "Build smelter mk1",
        target_agent_id: "agent-0",
        execute_kind: "gameplay_action",
      },
      player_leverage: {
        state: "blocked",
        summary: "Queue build_factory_smelter_mk1 for agent-0",
      },
      action_receipt: {
        present: true,
        state: "blocked",
        confidence: "world_delta",
        title: "Action blocked",
        summary: "Smelter build request reached factory-0; iron shortage blocks construction.",
        target_agent_id: "agent-0",
        effect_kind: "world_constraint",
        delta_logical_time: 1,
        delta_event_seq: 2,
      },
      world_read: {
        agents: 1,
        routes: 1,
        fragments: 2,
        hotspots: 4,
      },
    });
  });

  it("classifies action receipt confidence without treating default intent copy as player action", async () => {
    vi.resetModules();
    window.history.replaceState({}, "", "/software_safe.html?test_api=1&connect=0&locale=en");
    window.localStorage.clear();
    document.body.innerHTML = "";

    const core = await import("./legacy_core.js");
    const { buildPixelWorldRenderState } = await import("./pixel_world_host.jsx");

    core.injectSnapshot(acceptedOnlySnapshot());
    let receipt = buildPixelWorldRenderState("en").commercial_surface.action_receipt;
    expect(receipt).toMatchObject({
      present: true,
      state: "accepted",
      confidence: "accepted_intent",
      title: "Action accepted",
      summary: "Queue build_factory_smelter_mk1 for agent-0",
      detail: "Build request queued for agent-0.",
      target_agent_id: "agent-0",
      effect_kind: "queued_for_execution",
      delta_logical_time: 0,
      delta_event_seq: 1,
    });

    core.injectSnapshot(noReceiptSnapshot());
    core.state.recentEvents = [
      { eventId: "ambient-1", title: "Ambient transfer spike", kind: "resource_transfer" },
    ];
    receipt = buildPixelWorldRenderState("en").commercial_surface.action_receipt;
    expect(receipt).toMatchObject({
      present: false,
      state: "waiting_for_intent",
      confidence: "none",
      title: "No action receipt yet",
      target_agent_id: null,
      effect_kind: null,
      delta_logical_time: null,
      delta_event_seq: null,
    });
    expect(receipt.summary).toBe("No player-caused world change has been confirmed yet.");
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
    expect(renderState.locations[0]).toMatchObject({
      marker_role: "logic_anchor",
      marker_alpha: 0.32,
      size_hint_px: 10,
      fragment_terrain_count: 2,
    });
    expect(renderState.fragment_terrain.map((patch) => ({
      id: patch.id,
      dominant_compound: patch.dominant_compound,
      footprint_cm: patch.footprint_cm,
      color: patch.color,
      emphasis: patch.emphasis,
    }))).toEqual([
      {
        id: "fragment:loc-0:0",
        dominant_compound: "silicate_matrix",
        footprint_cm: 12_000,
        color: [126, 144, 99],
        emphasis: 0.58,
      },
      {
        id: "fragment:loc-0:1",
        dominant_compound: "iron_nickel_alloy",
        footprint_cm: 20_000,
        color: [176, 184, 196],
        emphasis: 0.58,
      },
    ]);

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

    expect(screen.getByText("World Command Board")).toBeInTheDocument();
    expect(screen.getByText("Recover sustainable capability")).toBeInTheDocument();
    expect(screen.getByText("Build smelter mk1")).toBeInTheDocument();
    expect(screen.getByText("Queue build_factory_smelter_mk1 for agent-0")).toBeInTheDocument();
    expect(screen.getByText("Action Receipt")).toBeInTheDocument();
    expect(screen.getByText("Action blocked")).toBeInTheDocument();
    expect(screen.getByText("Smelter build request reached factory-0; iron shortage blocks construction.")).toBeInTheDocument();
    const receipt = document.querySelector(".pixel-world-action-receipt");
    expect(receipt).toHaveAttribute("data-receipt-present", "true");
    expect(receipt).toHaveAttribute("data-receipt-state", "blocked");
    expect(receipt).toHaveAttribute("data-receipt-confidence", "world_delta");
    expect(receipt.textContent).toContain("agent=agent-0");
    const diagnostics = screen.getByText("Renderer Diagnostics").closest("details");
    expect(diagnostics.open).toBe(false);
    expect(screen.getByText(/falls back explicitly instead of keeping a second JS renderer/i)).toBeInTheDocument();
    expect(screen.getByText(/pixel_world_renderer_runtime_unavailable/i)).toBeInTheDocument();
    expect(document.querySelectorAll(".pixel-world-fragment-terrain")).toHaveLength(2);
    expect(document.querySelector(".pixel-world-entity--location")).toHaveAttribute("data-marker-role", "logic_anchor");
    expect(core.state.lastError).toContain("pixel world wasm runtime is unavailable");
  });

  it("renders the no-receipt fallback without implying an active agent caused progress", async () => {
    await renderPixelWorldHost(noReceiptSnapshot());

    await waitFor(() => {
      expect(screen.getByText("Renderer Not Attached")).toBeInTheDocument();
    });

    const receipt = document.querySelector(".pixel-world-action-receipt");
    expect(screen.getByText("Action Receipt")).toBeInTheDocument();
    expect(screen.getByText("No action receipt yet")).toBeInTheDocument();
    expect(screen.getByText("No player-caused world change has been confirmed yet.")).toBeInTheDocument();
    expect(receipt).toHaveAttribute("data-receipt-present", "false");
    expect(receipt).toHaveAttribute("data-receipt-state", "waiting_for_intent");
    expect(receipt).toHaveAttribute("data-receipt-confidence", "none");
    expect(receipt.textContent).not.toContain("agent=agent-0");
  });

  it("keeps fragment terrain as non-interactive background behind readable agents", async () => {
    await renderPixelWorldHost();

    await waitFor(() => {
      expect(screen.getByText("Renderer Not Attached")).toBeInTheDocument();
    });

    const canvas = document.querySelector(".pixel-world-canvas");
    const fragments = Array.from(canvas.querySelectorAll(".pixel-world-fragment-terrain"));
    const route = canvas.querySelector(".pixel-world-route");
    const location = canvas.querySelector(".pixel-world-entity--location");
    const agent = canvas.querySelector(".pixel-world-entity--agent");
    const children = Array.from(canvas.children);

    expect(fragments).toHaveLength(2);
    expect(fragments.map((fragment) => fragment.getAttribute("data-compound"))).toEqual([
      "silicate_matrix",
      "iron_nickel_alloy",
    ]);
    expect(numericInlineStyle(fragments[0], "width")).toBeCloseTo(10, 1);
    expect(numericInlineStyle(fragments[0], "height")).toBeCloseTo(10, 1);
    expect(numericInlineStyle(fragments[1], "width")).toBeCloseTo(16.7, 1);
    expect(numericInlineStyle(fragments[1], "height")).toBeCloseTo(16.7, 1);
    expect(route).toHaveAttribute("data-route-kind", "agent_assignment");
    expect(numericInlineStyle(route, "opacity")).toBeCloseTo(0.5936, 4);
    expect(fragments.every((fragment) => fragment.tagName === "DIV")).toBe(true);
    expect(fragments.every((fragment) => fragment.getAttribute("role") === null)).toBe(true);
    expect(children.indexOf(fragments[0])).toBeLessThan(children.indexOf(location));
    expect(children.indexOf(fragments[1])).toBeLessThan(children.indexOf(location));
    expect(children.indexOf(route)).toBeLessThan(children.indexOf(location));
    expect(children.indexOf(fragments[0])).toBeLessThan(children.indexOf(route));
    expect(children.indexOf(fragments[1])).toBeLessThan(children.indexOf(route));
    expect(children.indexOf(location)).toBeLessThan(children.indexOf(agent));
    expect(numericInlineStyle(location, "opacity")).toBeCloseTo(0.32, 2);
    expect(numericInlineStyle(location, "left")).toBeCloseTo(50, 1);
    expect(numericInlineStyle(location, "top")).toBeCloseTo(49, 1);
    expect(location).toHaveAttribute("data-marker-role", "logic_anchor");
    expect(agent).toHaveAttribute("data-position-source", "location_derived");
  });
});
