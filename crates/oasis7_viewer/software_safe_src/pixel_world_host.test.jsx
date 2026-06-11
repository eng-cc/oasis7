import { render, screen, waitFor } from "@solidjs/testing-library";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const runtimeMock = vi.hoisted(() => ({
  deriveRenderState: null,
  mountCalls: 0,
}));

vi.mock("./pixel_world_runtime_loader.js", () => ({
  createPixelWorldRuntimeBridge: async ({ onFatal }) => ({
    source: runtimeMock.deriveRenderState ? "test_rust_runtime" : "wasm_import_failed",
    moduleUrl: "http://127.0.0.1:4173/pixel-world-bridge/pixel_world_bridge.js",
    deriveRenderState: runtimeMock.deriveRenderState,
    bridge: {
      mount() {
        runtimeMock.mountCalls += 1;
        if (runtimeMock.deriveRenderState) {
          return {
            status: "ready",
            fatal: null,
          };
        }
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
        if (runtimeMock.deriveRenderState) {
          return {
            status: "ready",
            fatal: null,
          };
        }
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
const HEAVY_UI_TEST_TIMEOUT_MS = 60000;

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

function emptyWorldSnapshot() {
  const snapshot = clone(noReceiptSnapshot());
  snapshot.model.agents = {};
  snapshot.model.locations = {};
  snapshot.model.agent_prompt_profiles = {};
  snapshot.model.agent_execution_debug_contexts = {};
  return snapshot;
}

async function renderPixelWorldHost(snapshot = sampleSnapshot(), search = "?test_api=1&connect=0&locale=en") {
  activeCleanup?.();
  activeCleanup = null;
  vi.resetModules();
  window.history.replaceState({}, "", `/software_safe.html${search}`);
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
  runtimeMock.deriveRenderState = null;
  runtimeMock.mountCalls = 0;
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
  }, HEAVY_UI_TEST_TIMEOUT_MS);

  it("localizes command board goal and next-action copy for the selected UI language", async () => {
    vi.resetModules();
    window.history.replaceState({}, "", "/software_safe.html?test_api=1&connect=0&locale=zh");
    window.localStorage.clear();
    document.body.innerHTML = "";

    const core = await import("./legacy_core.js");
    const { buildPixelWorldRenderState } = await import("./pixel_world_host.jsx");

    core.setViewerLocale("zh-CN");
    core.injectSnapshot(sampleSnapshot());

    const surface = buildPixelWorldRenderState("zh-CN").commercial_surface;
    expect(surface.objective.title).toBe("恢复可持续能力");
    expect(surface.objective.detail).toBe("先恢复阻塞点，再确认生产线重新具备可经营能力。");
    expect(surface.next_action.label).toBe("排队建造一型冶炼炉");
    expect(surface.next_action.detail).toBe("查看当前回执和阻塞原因，再决定下一步。");
    expect(surface.objective.title).not.toMatch(/[A-Za-z]/);
    expect(surface.next_action.label).not.toMatch(/[A-Za-z]/);

    const liveSnapshot = sampleSnapshot();
    liveSnapshot.player_gameplay.goal_kind = "CreateFirstWorldFeedback";
    liveSnapshot.player_gameplay.goal_title = "Create the first visible world feedback";
    liveSnapshot.player_gameplay.objective = "Advance the world once and confirm that your action produces a visible state or event delta.";
    liveSnapshot.player_gameplay.next_step_hint = "Request a snapshot, advance 1 step, then inspect the new delta and events.";
    liveSnapshot.player_gameplay.available_actions[0].label = "Queue Smelter MK1 construction";
    core.injectSnapshot(liveSnapshot);

    const liveRenderState = buildPixelWorldRenderState("zh-CN");
    const liveSurface = liveRenderState.commercial_surface;
    expect(liveRenderState.goal_highlight.title).toBe("确认第一条世界反馈");
    expect(liveRenderState.goal_highlight.title).not.toMatch(/[A-Za-z]/);
    expect(liveSurface.objective.title).toBe("确认第一条世界反馈");
    expect(liveSurface.objective.detail).toBe("先拿到一条明确世界反馈，再继续后续工业选择。");
    expect(liveSurface.next_action.label).toBe("排队建造一型冶炼炉");
    expect(liveSurface.next_action.detail).toBe("先请求一次快照，推进 1 步，再检查新的世界变化和事件。");
    expect(liveSurface.objective.title).not.toMatch(/[A-Za-z]/);
    expect(liveSurface.objective.detail).not.toMatch(/[A-Za-z]/);
    expect(liveSurface.next_action.label).not.toMatch(/[A-Za-z]/);
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

  it("shows the explicit fallback surface when renderer deferral is requested", async () => {
    const { core } = await renderPixelWorldHost(
      sampleSnapshot(),
      "?test_api=1&connect=0&locale=en&pixel_world_renderer=defer",
    );

    await waitFor(() => {
      expect(screen.getByText("Renderer Not Attached")).toBeInTheDocument();
    });

    expect(runtimeMock.mountCalls).toBe(0);
    expect(screen.getByText("World Command Board")).toBeInTheDocument();
    expect(screen.getByText("Recover sustainable capability")).toBeInTheDocument();
    expect(screen.getByText("Build smelter mk1")).toBeInTheDocument();
    expect(screen.getByText("Queue build_factory_smelter_mk1 for agent-0")).toBeInTheDocument();
    expect(screen.getByText("Action Receipt")).toBeInTheDocument();
    expect(screen.getByText("Action blocked")).toBeInTheDocument();
    expect(screen.getByText("Smelter build request reached factory-0; iron shortage blocks construction.")).toBeInTheDocument();
    expect(document.querySelector('[data-renderer-state="fallback"]')).toHaveTextContent("Renderer Not Attached");
    const receipt = document.querySelector(".pixel-world-action-receipt");
    expect(receipt).toHaveAttribute("data-receipt-present", "true");
    expect(receipt).toHaveAttribute("data-receipt-state", "blocked");
    expect(receipt).toHaveAttribute("data-receipt-confidence", "world_delta");
    expect(receipt.textContent).toContain("agent=agent-0");
    const diagnostics = screen.getByText("Renderer Diagnostics").closest("details");
    expect(diagnostics.open).toBe(false);
    expect(screen.getByText(/using host fallback first/i)).toBeInTheDocument();

    screen.getByRole("button", { name: "Reattach Embedded Renderer" }).click();

    await waitFor(() => {
      expect(screen.getByText(/pixel_world_renderer_runtime_unavailable/i)).toBeInTheDocument();
    });
    expect(runtimeMock.mountCalls).toBeGreaterThan(0);
    expect(screen.getByText(/pixel_world_renderer_runtime_unavailable/i)).toBeInTheDocument();
    expect(document.querySelectorAll(".pixel-world-fragment-terrain")).toHaveLength(2);
    expect(document.querySelector(".pixel-world-entity--location")).toHaveAttribute("data-marker-role", "logic_anchor");
    expect(core.state.lastError).toContain("pixel world wasm runtime is unavailable");
  }, HEAVY_UI_TEST_TIMEOUT_MS);

  it("auto-attaches the embedded renderer for test_api pages unless deferral is explicit", async () => {
    const { core } = await renderPixelWorldHost();

    await waitFor(() => {
      expect(screen.getByText(/pixel_world_renderer_runtime_unavailable/i)).toBeInTheDocument();
    });

    expect(runtimeMock.mountCalls).toBe(1);
    expect(core.state.lastError).toContain("pixel world wasm runtime is unavailable");
  }, HEAVY_UI_TEST_TIMEOUT_MS);

  it("uses Rust-derived render state from the runtime module when available", async () => {
    runtimeMock.deriveRenderState = vi.fn((input) => ({
      locale: input.locale,
      worldBounds: { width_cm: 10_000_000, depth_cm: 5_000_000, height_cm: 1_000_000 },
      locations: [{
        id: "loc-0",
        label: "Factory Anchor",
        pos: { x_cm: 5_000_000, y_cm: 2_500_000, z_cm: 0 },
        markerRole: "logic_anchor",
        markerAlpha: 0.32,
      }],
      fragmentTerrain: [{
        id: "fragment:loc-0:0",
        locationId: "loc-0",
        pos: { x_cm: 5_000_000, y_cm: 2_500_000, z_cm: 0 },
        dominantCompound: "silicate_matrix",
        footprintCm: 12_000,
        color: [126, 144, 99],
      }],
      agents: [{
        id: "agent-0",
        label: "Agent 0",
        pos: { x_cm: 5_020_000, y_cm: 2_510_000, z_cm: 0 },
        positionSource: "location_derived",
      }],
      links: [{
        id: "link:agent-0:loc-0",
        kind: "agent_assignment",
        from: { x_cm: 5_020_000, y_cm: 2_510_000, z_cm: 0 },
        to: { x_cm: 5_000_000, y_cm: 2_500_000, z_cm: 0 },
        emphasis: 0.72,
      }],
      selection: { kind: "agent", id: "agent-0" },
      goalHighlight: {
        title: "Rust derived goal",
        objective: "Rust objective detail",
      },
      blockerHighlight: null,
      recentEventHotspots: [],
      visualHotspots: [],
      commercial_surface: {
        objective: {
          title: "Rust derived goal",
          detail: "Rust objective detail",
          progress_percent: null,
        },
        next_action: {
          label: "Rust next move",
          detail: null,
          target_agent_id: null,
          execute_kind: null,
        },
        active_agent_id: null,
        player_leverage: {
          state: "waiting_for_intent",
          label: "Waiting for Intent",
          summary: "Rust leverage summary",
          detail: null,
        },
        action_receipt: {
          present: false,
          state: "waiting_for_intent",
          confidence: "none",
          title: "No action receipt yet",
          summary: "Rust receipt summary",
          detail: null,
          target_agent_id: null,
          effect_kind: null,
          delta_logical_time: null,
          delta_event_seq: null,
        },
        blocker: {
          label: null,
          detail: null,
        },
        world_read: {
          agents: 0,
          routes: 0,
          fragments: 0,
          hotspots: 0,
        },
      },
      presentation: {
        world_bounds_label: "rust bounds",
        marker_truth_note: "rust truth",
      },
    }));

    await renderPixelWorldHost();
    screen.getByRole("button", { name: "Reattach Embedded Renderer" }).click();

    await waitFor(() => {
      expect(screen.getByText("Rust derived goal")).toBeInTheDocument();
    });
    expect(screen.getByText("Rust next move")).toBeInTheDocument();
    expect(screen.getByText("Rust leverage summary")).toBeInTheDocument();
    await waitFor(() => {
      expect(document.querySelector(".pixel-world-canvas--rendered")).toBeInTheDocument();
    });
    const canvas = document.querySelector(".pixel-world-canvas--rendered");
    expect(canvas.querySelectorAll(".pixel-world-fragment-terrain")).toHaveLength(0);
    expect(canvas.querySelector(".pixel-world-entity--location")).toBeNull();
    expect(canvas.querySelector(".pixel-world-entity--agent")).toBeNull();
    expect(canvas.querySelector(".pixel-world-route")).toBeNull();
    expect(canvas.querySelector(".pixel-world-canvas__selection")).toHaveTextContent("Selected: agent/agent-0");
    expect(runtimeMock.deriveRenderState).toHaveBeenCalled();
  }, HEAVY_UI_TEST_TIMEOUT_MS);

  it("makes the rendered canvas focusable with an accessible world description", async () => {
    runtimeMock.deriveRenderState = vi.fn((input) => ({
      locale: input.locale,
      worldBounds: { width_cm: 10_000_000, depth_cm: 5_000_000, height_cm: 1_000_000 },
      locations: [],
      fragmentTerrain: [],
      agents: [],
      links: [],
      selection: null,
      goalHighlight: null,
      blockerHighlight: null,
      recentEventHotspots: [],
      visualHotspots: [],
      commercial_surface: {
        objective: { title: "Accessible canvas goal", detail: "Readable adjacent HUD", progress_percent: null },
        next_action: { label: "Inspect world", detail: null, target_agent_id: null, execute_kind: null },
        active_agent_id: null,
        player_leverage: { state: "waiting_for_intent", label: "Waiting for Intent", summary: "Waiting", detail: null },
        action_receipt: {
          present: false,
          state: "waiting_for_intent",
          confidence: "none",
          title: "No action receipt yet",
          summary: "No receipt",
          detail: null,
          target_agent_id: null,
          effect_kind: null,
          delta_logical_time: null,
          delta_event_seq: null,
        },
        blocker: { label: null, detail: null },
        world_read: { agents: 0, routes: 0, fragments: 0, hotspots: 0 },
      },
      presentation: { world_bounds_label: "bounds", marker_truth_note: "truth" },
    }));

    await renderPixelWorldHost();
    screen.getByRole("button", { name: "Reattach Embedded Renderer" }).click();

    const canvas = await screen.findByRole("application", { name: "Interactive world canvas" });
    expect(canvas).toHaveAttribute("tabindex", "0");
    expect(canvas).toHaveAttribute("aria-describedby", "pixel-world-canvas-accessible-summary");
    expect(document.getElementById("pixel-world-canvas-accessible-summary")).toHaveTextContent(/adjacent HUD/i);
    canvas.focus();
    expect(document.activeElement).toBe(canvas);
  }, HEAVY_UI_TEST_TIMEOUT_MS);

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

  it("offers an app-level world focus mode with command and diagnostics drawers", async () => {
    await renderPixelWorldHost(
      sampleSnapshot(),
      "?test_api=1&connect=0&locale=en&pixel_world_renderer=defer",
    );

    await waitFor(() => {
      expect(screen.getByText("Renderer Not Attached")).toBeInTheDocument();
    });

    const host = document.querySelector(".pixel-world-host");
    expect(host).toHaveAttribute("data-world-focus", "false");
    expect(screen.queryByText("World Focus")).not.toBeInTheDocument();

    screen.getByRole("button", { name: "Enter World Focus" }).click();

    expect(host).toHaveAttribute("data-world-focus", "true");
    expect(document.body).toHaveClass("pixel-world-focus-active");
    expect(screen.getByText("World Focus")).toBeInTheDocument();
    expect(screen.queryByText("No blocker")).not.toBeInTheDocument();
    expect(document.querySelector(".pixel-world-focus-hud")).toHaveTextContent("Current Prompt");
    expect(document.querySelector(".pixel-world-focus-hud")).toHaveTextContent("Recover sustainable capability");
    expect(document.querySelector(".pixel-world-focus-hud")).toHaveTextContent("Build smelter mk1");
    expect(document.querySelector(".pixel-world-focus-hud")).toHaveTextContent("Missing Material");
    expect(document.querySelector(".pixel-world-focus-hud")).toHaveTextContent("Mission Progress");
    expect(document.querySelector(".pixel-world-focus-hud")).toHaveTextContent("Receipt");
    expect(document.querySelector(".pixel-world-focus-hud")).toHaveTextContent("Action blocked");
    expect(document.querySelector(".pixel-world-focus-hud")).toHaveTextContent("68%");
    expect(document.querySelector(".pixel-world-focus-hud")).toHaveTextContent("Replenish upstream materials, then advance again to confirm the line resumes.");
    expect(document.querySelector(".pixel-world-focus-hud")).not.toHaveTextContent("Objective");
    expect(document.querySelector(".pixel-world-focus-hud")).not.toHaveTextContent("Next Move");
    expect(document.querySelector(".pixel-world-focus-rail")).toHaveTextContent("agent-0");
    expect(document.querySelector(".pixel-world-focus-rail")).toHaveTextContent("Routes");
    expect(document.querySelector(".pixel-world-focus-rail")).toHaveTextContent("Missing Material");
    expect(document.querySelector(".pixel-world-focus-receipt")).toHaveTextContent("Action blocked");
    expect(document.querySelector(".pixel-world-focus-receipt .pixel-world-action-receipt")).toHaveClass("pixel-world-action-receipt--focus-compact");
    expect(document.querySelector(".pixel-world-focus-receipt .pixel-world-action-receipt")).toHaveAttribute("data-receipt-confidence", "world_delta");
    expect(document.querySelector('[data-focus-cinematic="true"]')).toHaveTextContent("Industrial World Command Board");
    expect(document.querySelector('[data-focus-cinematic="true"]')).toHaveTextContent("Stabilize the first production line before expanding.");
    expect(document.querySelector('[data-focus-cinematic="true"]')).toHaveTextContent("Recover sustainable capability");
    expect(document.querySelector('[data-renderer-state="fallback"]')).toHaveTextContent("Renderer Not Attached");
    expect(document.querySelector('[data-renderer-state="fallback"]')).toHaveTextContent(/formal gameplay summary/i);
    expect(document.querySelector('[data-focus-minimap="true"]')).toHaveTextContent("Mission Map");
    expect(document.querySelector('[data-focus-minimap="true"]')).toHaveTextContent("Factory Anchor");
    expect(document.querySelector('[data-focus-minimap="true"]')).toHaveTextContent("Build smelter mk1");
    expect(document.querySelector('[data-focus-minimap="true"]')).toHaveTextContent("agent-0");
    expect(document.querySelector('[data-focus-fallback-map="true"]')).toHaveTextContent("agents=1");
    expect(document.querySelector('[data-focus-fallback-map="true"]')).toHaveTextContent("targets=1");
    expect(document.querySelector('[data-focus-fallback-map="true"]')).toHaveTextContent("routes=1");
    expect(document.querySelector('[data-focus-fallback-map="true"]')).toHaveTextContent("fragments=2");

    const commandDrawer = document.querySelector(".pixel-world-focus-drawer--command");
    expect(commandDrawer.open).toBe(true);
    expect(commandDrawer).toHaveTextContent("Agent Chat");
    expect(commandDrawer).toHaveTextContent("Command Surface");
    expect(commandDrawer).toHaveTextContent("Current Target");
    expect(commandDrawer).toHaveTextContent("agent=agent-0");
    expect(commandDrawer).toHaveTextContent("Message");
    expect(commandDrawer).toHaveTextContent("Send Chat");
    expect(commandDrawer).toHaveTextContent("No chat feedback yet.");
    expect(commandDrawer).toHaveTextContent("No chat history for this agent yet.");
    expect(commandDrawer.querySelector("#agent-chat-message")).not.toBeNull();
    expect(commandDrawer.querySelector("[data-chat-send='1']")).not.toBeNull();

    expect(screen.getByRole("button", { name: "Command" })).toHaveClass("pixel-world-focus-control--primary");
    expect(screen.getByRole("button", { name: "Diagnostics" })).toHaveClass("pixel-world-focus-control--secondary");
    expect(screen.getByRole("button", { name: "Maximize" })).toHaveClass("pixel-world-focus-control--secondary");
    expect(screen.getByRole("button", { name: "Exit" })).toHaveClass("pixel-world-focus-control--quiet");

    screen.getByRole("button", { name: "Command" }).click();
    expect(commandDrawer.open).toBe(true);
    expect(commandDrawer).toHaveTextContent("Agent Chat");

    screen.getByRole("button", { name: "Maximize" }).click();
    expect(host).toHaveAttribute("data-world-focus-maximized", "true");
    expect(document.body).toHaveClass("pixel-world-focus-maximized");
    expect(document.querySelector(".pixel-world-host__summary")).toBeNull();
    expect(document.querySelector(".pixel-world-focus-rail")).toBeNull();
    expect(document.querySelector('[data-focus-cinematic="true"]')).toBeNull();
    expect(document.querySelector('[data-focus-minimap="true"]')).toBeNull();
    expect(document.querySelector(".pixel-world-focus-receipt")).toBeNull();
    expect(document.querySelector(".pixel-world-render-diagnostics")).toBeNull();
    expect(document.querySelector(".pixel-world-focus-hud")).toHaveTextContent("Minimize");
    expect(document.querySelector(".pixel-world-focus-hud")).toHaveTextContent("Action blocked");
    expect(document.querySelector(".pixel-world-focus-drawer--command")?.open).toBe(true);

    screen.getByRole("button", { name: "Minimize" }).click();
    expect(host).toHaveAttribute("data-world-focus-maximized", "false");
    expect(document.body).not.toHaveClass("pixel-world-focus-maximized");
    expect(document.querySelector(".pixel-world-host__summary")).not.toBeNull();
    expect(document.querySelector('[data-focus-cinematic="true"]')).not.toBeNull();

    screen.getByRole("button", { name: "Diagnostics" }).click();
    const diagnosticsDrawer = document.querySelector(".pixel-world-focus-drawer--diagnostics");
    expect(commandDrawer.open).toBe(false);
    expect(diagnosticsDrawer.open).toBe(true);
    expect(diagnosticsDrawer).toHaveTextContent("renderer=fallback");
    expect(diagnosticsDrawer).toHaveTextContent("runtime=deferred");

    window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    expect(host).toHaveAttribute("data-world-focus", "false");
    expect(document.body).not.toHaveClass("pixel-world-focus-active");
    expect(document.querySelector(".pixel-world-focus-drawer--diagnostics")).toBeNull();
  }, HEAVY_UI_TEST_TIMEOUT_MS);

  it("demotes raw focus command feedback and chat history behind diagnostics", async () => {
    const { core } = await renderPixelWorldHost(
      sampleSnapshot(),
      "?test_api=1&connect=0&locale=en&pixel_world_renderer=defer",
    );
    core.applySelection({ kind: "agent", id: "agent-0" });
    core.state.lastChatFeedback = {
      id: "feedback-1",
      kind: "agent_chat",
      action: "agent_chat",
      agentId: "agent-0",
      accepted: true,
      stage: "accepted",
      ok: true,
      reason: null,
      effect: "Message accepted by agent-0.",
      response: { message: "Agent acknowledged the recovery plan.", code: "chat_ok" },
    };
    core.state.chatHistory = [
      {
        id: "chat-1",
        source: "player",
        agentId: "agent-0",
        targetAgentId: "agent-0",
        playerId: "player-one",
        speaker: "player-one",
        locationId: "loc-0",
        message: "Restore the smelter before expanding.",
        tick: 44,
      },
    ];
    core.requestRender();

    await waitFor(() => {
      expect(screen.getByText("Renderer Not Attached")).toBeInTheDocument();
    });
    screen.getByRole("button", { name: "Enter World Focus" }).click();

    const commandDrawer = document.querySelector(".pixel-world-focus-drawer--command");
    expect(commandDrawer).toHaveTextContent("Message accepted by agent-0.");
    expect(commandDrawer).toHaveTextContent("Restore the smelter before expanding.");
    expect(commandDrawer).toHaveTextContent("Player -> agent-0");
    expect(commandDrawer).toHaveTextContent("player-one · loc-0 · tick=44");
    expect(commandDrawer).toHaveTextContent("Raw diagnostics");
    expect(commandDrawer).not.toHaveTextContent('"message": "Restore the smelter before expanding."');
    expect(commandDrawer).not.toHaveTextContent('"code": "chat_ok"');

    commandDrawer.querySelector("summary").click();
    commandDrawer.querySelectorAll("details.diagnostic summary")[0].click();
    await waitFor(() => {
      expect(commandDrawer).toHaveTextContent('"code": "chat_ok"');
    });
  }, HEAVY_UI_TEST_TIMEOUT_MS);

  it("keeps empty focus rail collapsed while preserving fallback world summary", async () => {
    await renderPixelWorldHost(
      emptyWorldSnapshot(),
      "?test_api=1&connect=0&locale=en&pixel_world_renderer=defer",
    );

    await waitFor(() => {
      expect(screen.getByText("Renderer Not Attached")).toBeInTheDocument();
    });

    expect(document.querySelector('[data-focus-fallback-map="true"]')).toBeNull();

    screen.getByRole("button", { name: "Enter World Focus" }).click();

    expect(document.querySelector(".pixel-world-focus-rail")).toBeNull();
    const fallbackMap = document.querySelector('[data-focus-fallback-map="true"]');
    expect(fallbackMap).not.toBeNull();
    expect(fallbackMap).toHaveTextContent("agents=0");
    expect(fallbackMap).toHaveTextContent("targets=0");
    expect(fallbackMap).toHaveTextContent("routes=0");
    expect(fallbackMap).toHaveTextContent("fragments=0");
    expect(fallbackMap).toHaveTextContent("Unassigned");
    expect(fallbackMap).not.toHaveTextContent("Selected");
  });

  it("preserves world focus UI state across host remounts", async () => {
    activeCleanup?.();
    activeCleanup = null;
    vi.resetModules();
    window.history.replaceState({}, "", "/software_safe.html?test_api=1&connect=0&locale=en&pixel_world_renderer=defer");
    window.localStorage.clear();
    document.body.innerHTML = "";

    const core = await import("./legacy_core.js");
    const { PixelWorldHost } = await import("./pixel_world_host.jsx");
    core.setViewerLocale("en");
    core.injectSnapshot(sampleSnapshot());

    const firstView = render(() => <PixelWorldHost locale="en" />);
    activeCleanup = firstView.unmount;

    await waitFor(() => {
      expect(screen.getByText("Renderer Not Attached")).toBeInTheDocument();
    });

    screen.getByRole("button", { name: "Enter World Focus" }).click();
    screen.getByRole("button", { name: "Diagnostics" }).click();

    const firstHost = document.querySelector(".pixel-world-host");
    expect(firstHost).toHaveAttribute("data-world-focus", "true");
    expect(document.body).toHaveClass("pixel-world-focus-active");
    expect(document.querySelector(".pixel-world-focus-drawer--diagnostics")?.open).toBe(true);

    firstView.unmount();
    if (activeCleanup === firstView.unmount) {
      activeCleanup = null;
    }

    const secondView = render(() => <PixelWorldHost locale="en" />);
    activeCleanup = secondView.unmount;

    const secondHost = document.querySelector(".pixel-world-host");
    expect(secondHost).toHaveAttribute("data-world-focus", "true");
    expect(document.body).toHaveClass("pixel-world-focus-active");
    expect(document.querySelector(".pixel-world-focus-drawer--diagnostics")?.open).toBe(true);
    expect(document.querySelector(".pixel-world-focus-drawer--command")).toHaveProperty("open", false);
  }, HEAVY_UI_TEST_TIMEOUT_MS);

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
    expect(numericInlineStyle(route, "width")).toBeCloseTo(4, 1);
    expect(route.style.transform).toBe("rotate(-111.1deg)");
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
