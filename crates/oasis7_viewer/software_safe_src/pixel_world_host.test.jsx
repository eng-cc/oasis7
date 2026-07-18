import { render, screen, waitFor } from "@solidjs/testing-library";
import { readFileSync } from "node:fs";
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

function fieldValue(source, snake, camel, fallback = null) {
  if (!source || typeof source !== "object") {
    return fallback;
  }
  if (source[snake] != null) {
    return source[snake];
  }
  if (source[camel] != null) {
    return source[camel];
  }
  return fallback;
}

function dominantCompound(block) {
  const ppm = block?.compounds?.ppm || {};
  const entries = Object.entries(ppm);
  if (!entries.length) {
    return "unknown";
  }
  return entries.sort((left, right) => Number(right[1] || 0) - Number(left[1] || 0))[0][0];
}

function locationPos(location) {
  return location?.pos || { x_cm: 0, y_cm: 0, z_cm: 0 };
}

function buildTestRustRenderState(input) {
  const snapshot = input.snapshot || {};
  const model = snapshot.model || {};
  const gameplay = snapshot.player_gameplay || {};
  const locations = Object.values(model.locations || {});
  const agents = Object.values(model.agents || {});
  const fragments = locations.flatMap((location) => {
    const base = locationPos(location);
    return (location.fragment_profile?.blocks?.blocks || []).map((block, index) => ({
      id: `fragment:${location.id}:${index}`,
      locationId: location.id,
      pos: {
        x_cm: base.x_cm + Number(block.origin_cm?.x_cm || 0),
        y_cm: base.y_cm + Number(block.origin_cm?.z_cm || block.origin_cm?.y_cm || 0),
        z_cm: base.z_cm + Number(block.origin_cm?.y_cm || 0),
      },
      dominantCompound: dominantCompound(block),
      footprintCm: Math.max(Number(block.size_cm?.x_cm || 12_000), Number(block.size_cm?.z_cm || block.size_cm?.y_cm || 12_000)),
    }));
  });
  const firstAction = (gameplay.available_actions || [])[0] || {};
  const activeAgentId = gameplay.intent_target || agents[0]?.id || null;
  const receiptPresent = Boolean(gameplay.recent_feedback || gameplay.last_world_change);
  const blockerLabel = gameplay.blocker_kind === "material_shortage" ? "Missing Material" : gameplay.blocker_kind || null;

  const renderState = {
    locale: input.locale || "en",
    worldBounds: snapshot.config?.space || { width_cm: 10_000_000, depth_cm: 5_000_000, height_cm: 1_000_000 },
    world_bounds: snapshot.config?.space || { width_cm: 10_000_000, depth_cm: 5_000_000, height_cm: 1_000_000 },
    locations: locations.map((location) => ({
      id: location.id,
      label: location.name || location.id,
      pos: locationPos(location),
      markerRole: "logic_anchor",
      markerAlpha: 0.32,
    })),
    fragmentTerrain: fragments,
    fragment_terrain: fragments,
    agents: agents.map((agent, index) => {
      const base = locationPos(model.locations?.[agent.location_id]);
      return {
        id: agent.id,
        label: agent.name || agent.id,
        pos: agent.pos || {
          x_cm: base.x_cm + 20_000 + index * 15_000,
          y_cm: base.y_cm + 10_000 + index * 12_000,
          z_cm: base.z_cm,
        },
        positionSource: agent.pos ? "runtime_agent" : "location_derived",
      };
    }),
    links: agents
      .filter((agent) => agent.location_id && model.locations?.[agent.location_id])
      .map((agent) => {
        const from = agent.pos || {
          x_cm: locationPos(model.locations[agent.location_id]).x_cm + 20_000,
          y_cm: locationPos(model.locations[agent.location_id]).y_cm + 10_000,
          z_cm: 0,
        };
        return {
          id: `link:${agent.id}:${agent.location_id}`,
          kind: "agent_assignment",
          from,
          to: locationPos(model.locations[agent.location_id]),
          emphasis: 0.72,
        };
      }),
    selection: activeAgentId ? { kind: "agent", id: activeAgentId } : null,
    goalHighlight: {
      title: gameplay.goal_title || "Current Objective",
      objective: gameplay.objective || gameplay.progress_detail || "",
    },
    blockerHighlight: blockerLabel
      ? { kind: gameplay.blocker_kind, label: blockerLabel, detail: gameplay.blocker_detail || null }
      : null,
    recentEventHotspots: [],
    visualHotspots: [],
    commercial_surface: {
      objective: {
        title: gameplay.goal_title || "Current Objective",
        detail: gameplay.objective || gameplay.progress_detail || "No current objective.",
        progress_percent: gameplay.progress_percent ?? null,
      },
      next_action: {
        label: fieldValue(firstAction, "label", "label", "Unassigned"),
        detail: gameplay.intent_summary || null,
        target_agent_id: fieldValue(firstAction, "target_agent_id", "targetAgentId", activeAgentId),
        execute_kind: fieldValue(firstAction, "execute_kind", "executeKind", "gameplay_action"),
      },
      active_agent_id: activeAgentId,
      player_leverage: {
        state: gameplay.stage_status || "waiting_for_intent",
        label: receiptPresent ? "Blocked" : "Waiting for Intent",
        summary: gameplay.progress_detail || "Waiting",
        detail: gameplay.next_step_hint || null,
      },
      action_receipt: {
        present: receiptPresent,
        state: receiptPresent ? "blocked" : "waiting_for_intent",
        confidence: receiptPresent ? "world_delta" : "none",
        title: receiptPresent ? "Action blocked" : "No action receipt yet",
        summary: receiptPresent ? "Action blocked" : "No receipt",
        detail: gameplay.last_world_change || gameplay.recent_feedback?.effect || "No player-caused world change has been confirmed yet.",
        target_agent_id: receiptPresent ? activeAgentId : null,
        effect_kind: gameplay.causality_kind || null,
        delta_logical_time: gameplay.recent_feedback?.delta_logical_time ?? null,
        delta_event_seq: gameplay.recent_feedback?.delta_event_seq ?? null,
      },
      blocker: {
        label: blockerLabel,
        detail: gameplay.next_step_hint || gameplay.blocker_detail || null,
      },
      world_read: {
        agents: agents.length,
        routes: agents.filter((agent) => agent.location_id).length,
        fragments: fragments.length,
        hotspots: 0,
      },
    },
    presentation: input.presentation || { world_bounds_label: "bounds", marker_truth_note: "truth" },
  };
  return renderState;
}

function useTestRustRenderState() {
  runtimeMock.deriveRenderState = vi.fn((input) => buildTestRustRenderState(input));
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
      agent_player_bindings: {
        "agent-0": "player-one",
      },
      agent_player_public_key_bindings: {
        "agent-0": "abcdef0123456789abcdef0123456789",
      },
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
  snapshot.model.agent_player_bindings = {};
  snapshot.model.agent_player_public_key_bindings = {};
  return snapshot;
}

function bindFirstSnapshotAgentForTest(core, snapshot) {
  const agentId = Object.keys(snapshot?.model?.agents || {})[0];
  const playerId = snapshot?.model?.agent_player_bindings?.[agentId];
  if (!agentId || !playerId) {
    return;
  }
  core.state.auth = {
    ...core.state.auth,
    available: true,
    playerId,
    publicKey: snapshot?.model?.agent_player_public_key_bindings?.[agentId] || "abcdef0123456789abcdef0123456789",
    privateKey: "private-key-must-stay-hidden",
    source: "local_test_api_ephemeral",
    registrationStatus: "registered",
    runtimeStatus: "registered",
    boundAgentId: agentId,
  };
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
  bindFirstSnapshotAgentForTest(core, snapshot);

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
  it("keeps world focus stage resets scoped away from nested command panels", () => {
    const html = readFileSync("software_safe.html", "utf8");

    expect(html).toContain("body.pixel-world-focus-active .panel--stage > .panel__body");
    expect(html).toContain(".panel--stage > .panel__body");
    expect(html).not.toContain("body.pixel-world-focus-active .panel--stage .panel__body");
    expect(html).not.toContain(".panel--stage .panel__body {");
    expect(html).toMatch(/\.pixel-world-focus-hud__cell--tick\s*\{[^}]*grid-column:\s*3;/s);
    expect(html).toMatch(/\.pixel-world-focus-hud__cell--tick strong,[\s\S]*?\.pixel-world-focus-hud__cell--tick em\s*\{[^}]*white-space:\s*nowrap;/s);
    expect(html).toMatch(/\.pixel-world-focus-hud__cell--blocker::after,[\s\S]*?\.pixel-world-focus-hud__cell--receipt::after\s*\{[^}]*width:\s*3px;/s);
    expect(html).toMatch(/\.pixel-world-focus-hud__cell--blocker\[data-blocker-present="true"\]::after\s*\{[^}]*background:\s*var\(--bad\);/s);
    expect(html).toMatch(/\.pixel-world-focus-hud__cell--receipt\[data-hud-priority="receipt"\]::after\s*\{[^}]*background:\s*var\(--good\);/s);
    expect(html).toMatch(/\.pixel-world-focus-rail\s*\{[^}]*top:\s*112px;/s);
    expect(html).toContain("max-height: min(42vh, 340px);");
    expect(html).toContain(".pixel-world-focus-command-tray");
    expect(html).toMatch(/\.pixel-world-focus-minimap__node::before\s*\{[^}]*width:\s*7px;[^}]*height:\s*7px;/s);
    expect(html).toMatch(/\.pixel-world-focus-minimap__node--target::before\s*\{[^}]*background:\s*var\(--good\);/s);
    expect(html).toMatch(/\.pixel-world-focus-minimap__node--agent::before\s*\{[^}]*background:\s*var\(--accent\);/s);
    expect(html).toMatch(/\.pixel-world-focus-minimap__node--selected\s*\{[^}]*border-color:\s*rgba\(208,\s*168,\s*91,\s*0\.58\);/s);
    expect(html).toMatch(/\.pixel-world-focus-minimap__node--selected::before\s*\{[^}]*width:\s*18px;[^}]*height:\s*18px;[^}]*border:\s*1px solid rgba\(208,\s*168,\s*91,\s*0\.78\);/s);
  });

  it("resolves claim onboarding next moves to executable gameplay actions", async () => {
    const { resolvePixelWorldDirectNextMoveAction } = await import("./pixel_world_host.jsx");
    const gameplay = {
      availableActions: [
        {
          actionId: "claim_first_agent",
          executeKind: "claim_first_agent",
          label: "Claim First Agent",
        },
      ],
    };

    expect(resolvePixelWorldDirectNextMoveAction(gameplay, "claim_first_agent")).toEqual(
      gameplay.availableActions[0],
    );
  }, HEAVY_UI_TEST_TIMEOUT_MS);

  it("does not directly execute disabled or generic pixel world next moves", async () => {
    const { resolvePixelWorldDirectNextMoveAction } = await import("./pixel_world_host.jsx");
    const gameplay = {
      availableActions: [
        {
          actionId: "claim_first_agent",
          executeKind: "claim_first_agent",
          disabledReason: "already claimed",
        },
        {
          actionId: "build_factory_smelter_mk1",
          executeKind: "gameplay_action",
        },
      ],
    };

    expect(resolvePixelWorldDirectNextMoveAction(gameplay, "claim_first_agent")).toBeNull();
    expect(resolvePixelWorldDirectNextMoveAction(gameplay, "gameplay_action")).toBeNull();
  }, HEAVY_UI_TEST_TIMEOUT_MS);

  it("shows the explicit unavailable surface when renderer deferral is requested", async () => {
    const { core } = await renderPixelWorldHost(
      sampleSnapshot(),
      "?test_api=1&connect=0&locale=en&pixel_world_renderer=defer",
    );

    await waitFor(() => {
      expect(screen.getByText("Renderer Unavailable")).toBeInTheDocument();
    });

    expect(runtimeMock.mountCalls).toBe(0);
    expect(screen.getByText("World Command Board")).toBeInTheDocument();
    expect(screen.getAllByText(/pixel_world_render_state_unavailable/i).length).toBeGreaterThan(0);
    expect(document.querySelector('[data-renderer-state="unavailable"]')).toHaveTextContent("pixel_world_render_state_unavailable");
    expect(screen.queryByText("Recover sustainable capability")).not.toBeInTheDocument();
    expect(screen.queryByText("Build smelter mk1")).not.toBeInTheDocument();
    expect(screen.queryByText("Action Receipt")).not.toBeInTheDocument();
    const diagnostics = screen.getByText("Renderer Diagnostics").closest("details");
    expect(diagnostics.open).toBe(false);
    expect(screen.getByText(/the page no longer keeps a second JS world renderer/i)).toBeInTheDocument();

    screen.getByRole("button", { name: "Reattach Embedded Renderer" }).click();

    await waitFor(() => {
      expect(screen.getAllByText(/pixel_world_render_state_unavailable/i).length).toBeGreaterThan(0);
    });
    expect(runtimeMock.mountCalls).toBe(0);
    expect(document.querySelectorAll(".pixel-world-fragment-terrain")).toHaveLength(0);
    expect(core.state.lastError).toContain("pixel world Rust render-state derivation is unavailable");
  }, HEAVY_UI_TEST_TIMEOUT_MS);

  it("auto-attaches the embedded renderer for test_api pages unless deferral is explicit", async () => {
    const { core } = await renderPixelWorldHost();

    await waitFor(() => {
      expect(screen.getAllByText(/pixel_world_render_state_unavailable/i).length).toBeGreaterThan(0);
    });

    expect(runtimeMock.mountCalls).toBe(0);
    expect(core.state.lastError).toContain("pixel world Rust render-state derivation is unavailable");
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
      }, {
        id: "agent-1",
        label: "Agent 1",
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
    expect(document.querySelector(".pixel-world-readout")).toHaveTextContent("tick=12");
    expect(document.querySelector(".pixel-world-readout [data-world-tick='12']")).toHaveTextContent("tick=12");
    await waitFor(() => {
      expect(document.querySelector(".pixel-world-canvas--rendered")).toBeInTheDocument();
    });
    const canvas = document.querySelector(".pixel-world-canvas--rendered");
    expect(canvas.querySelectorAll(".pixel-world-fragment-terrain")).toHaveLength(0);
    expect(canvas.querySelector(".pixel-world-entity--location")).toBeNull();
    const agentMarker = canvas.querySelector("[data-pixel-world-agent-marker='true'][data-agent-id='agent-0']");
    const secondAgentMarker = canvas.querySelector("[data-pixel-world-agent-marker='true'][data-agent-id='agent-1']");
    expect(agentMarker).not.toBeNull();
    expect(secondAgentMarker).not.toBeNull();
    expect(agentMarker).toHaveAttribute("aria-label", "Select Agent agent-0");
    expect(secondAgentMarker).toHaveAttribute("aria-label", "Select Agent agent-1");
    expect(agentMarker.style.transform).not.toEqual(secondAgentMarker.style.transform);
    agentMarker.click();
    expect(canvas.querySelector(".pixel-world-canvas__selection")).toHaveTextContent("Selected: agent/agent-0");
    expect(canvas.querySelector(".pixel-world-route")).toBeNull();
    expect(canvas.querySelector(".pixel-world-canvas__selection")).toHaveTextContent("Selected: agent/agent-0");
    expect(runtimeMock.deriveRenderState).toHaveBeenCalled();
  }, HEAVY_UI_TEST_TIMEOUT_MS);

  it("makes the rendered canvas focusable with a read-only accessible world description", async () => {
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

    const canvas = await screen.findByRole("img", { name: "World canvas overview" });
    expect(canvas).toHaveAttribute("tabindex", "0");
    expect(canvas).toHaveAttribute("aria-describedby", "pixel-world-canvas-accessible-summary");
    expect(document.getElementById("pixel-world-canvas-accessible-summary")).toHaveTextContent(/read-only overview/i);
    expect(document.getElementById("pixel-world-canvas-accessible-summary")).toHaveTextContent(/adjacent HUD/i);
    canvas.focus();
    expect(document.activeElement).toBe(canvas);
  }, HEAVY_UI_TEST_TIMEOUT_MS);

  it("renders the no-receipt fallback without implying an active agent caused progress", async () => {
    useTestRustRenderState();
    await renderPixelWorldHost(noReceiptSnapshot());

    await waitFor(() => {
      expect(screen.getByText("No action receipt yet")).toBeInTheDocument();
    });

    const receipt = document.querySelector(".pixel-world-action-receipt");
    expect(screen.getByText("Action Receipt")).toBeInTheDocument();
    expect(screen.getByText("No action receipt yet")).toBeInTheDocument();
    expect(screen.getByText("No player-caused world change has been confirmed yet.")).toBeInTheDocument();
    expect(receipt).toHaveAttribute("data-receipt-present", "false");
    expect(receipt).toHaveAttribute("data-receipt-state", "waiting_for_intent");
    expect(receipt).toHaveAttribute("data-receipt-confidence", "none");
    expect(receipt.querySelector(".pixel-world-action-receipt__meta")).toBeNull();
    expect(receipt.textContent).not.toContain("agent=agent-0");
  });

  it("keeps secondary focus controls in a collapsed native mobile disclosure without changing their actions", async () => {
    useTestRustRenderState();
    await renderPixelWorldHost(
      sampleSnapshot(),
      "?test_api=1&connect=0&locale=en&pixel_world_renderer=defer",
    );

    await waitFor(() => {
      expect(screen.getByText("Recover sustainable capability")).toBeInTheDocument();
    });

    const host = document.querySelector(".pixel-world-host");
    screen.getByRole("button", { name: "Enter World Focus" }).click();
    await waitFor(() => {
      expect(host).toHaveAttribute("data-world-focus", "true");
    });

    const controls = document.querySelector(".pixel-world-focus-controls");
    const commandButton = screen.getByRole("button", { name: "Command & Target" });
    const moreControls = screen.getByText("More controls").closest("details");
    expect(moreControls).toBeTruthy();
    expect(moreControls).not.toHaveAttribute("open");
    expect(controls).toContainElement(commandButton);
    expect(commandButton).toHaveClass("pixel-world-focus-control--primary");
    expect(moreControls).toContainElement(screen.getByRole("button", { name: "World Status" }));
    expect(moreControls).toContainElement(screen.getByRole("button", { name: "Maximize" }));
    expect(moreControls).toContainElement(screen.getByRole("button", { name: "Leave Focus · Esc" }));

    screen.getByRole("button", { name: "World Status" }).click();
    expect(document.querySelector(".pixel-world-focus-drawer--diagnostics")?.open).toBe(true);

    window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    expect(host).toHaveAttribute("data-world-focus", "false");
  }, HEAVY_UI_TEST_TIMEOUT_MS);

  it("offers an app-level world focus mode with command and diagnostics drawers", async () => {
    useTestRustRenderState();
    await renderPixelWorldHost(
      sampleSnapshot(),
      "?test_api=1&connect=0&locale=en&pixel_world_renderer=defer",
    );

    await waitFor(() => {
      expect(screen.getByText("Recover sustainable capability")).toBeInTheDocument();
    });

    const host = document.querySelector(".pixel-world-host");
    expect(host).toHaveAttribute("data-world-focus", "false");
    expect(screen.queryByText("World Focus")).not.toBeInTheDocument();

    const worldFocusButton = screen.getByRole("button", { name: "Enter World Focus" });
    expect(screen.getByText("Pan, zoom, and inspect the world")).toBeInTheDocument();
    expect(worldFocusButton).toHaveAccessibleDescription("Pan, zoom, and inspect the world");

    worldFocusButton.click();

    await waitFor(() => {
      expect(host).toHaveAttribute("data-world-focus", "true");
    });
    expect(document.body).toHaveClass("pixel-world-focus-active");
    expect(screen.getByText("World Focus")).toBeInTheDocument();
    expect(screen.queryByText("No blocker")).not.toBeInTheDocument();
    expect(document.querySelector(".pixel-world-focus-hud")).toHaveTextContent("Current Objective");
    expect(document.querySelector(".pixel-world-focus-hud")).toHaveTextContent("Recover sustainable capability");
    expect(document.querySelector(".pixel-world-focus-hud")).toHaveTextContent("Build smelter mk1");
    expect(document.querySelector(".pixel-world-focus-hud")).toHaveTextContent("Missing Material");
    expect(document.querySelector(".pixel-world-focus-hud")).toHaveTextContent("Mission Progress");
    expect(document.querySelector(".pixel-world-focus-hud")).toHaveTextContent("World Tick");
    expect(document.querySelector(".pixel-world-focus-hud")).toHaveTextContent("tick=12");
    expect(document.querySelector(".pixel-world-focus-hud")).toHaveTextContent("Receipt");
    expect(document.querySelector(".pixel-world-focus-hud")).toHaveTextContent("Action blocked");
    expect(document.querySelector(".pixel-world-focus-hud")).toHaveTextContent("68%");
    expect(document.querySelector(".pixel-world-focus-hud")).not.toHaveTextContent("Next Move");
    expect(document.querySelector(".pixel-world-focus-rail")).toHaveTextContent("agent-0");
    expect(document.querySelector(".pixel-world-focus-rail")).toHaveTextContent("Routes");
    expect(document.querySelector(".pixel-world-focus-rail")).toHaveTextContent("Missing Material");
    expect(document.querySelectorAll(".pixel-world-focus-rail__item")[0]).toHaveClass("pixel-world-focus-rail__item--blocker");
    expect(document.querySelector(".pixel-world-focus-rail__item--blocker")).toHaveAttribute("data-focus-priority", "blocker");
    expect(document.querySelector(".pixel-world-focus-rail__item--blocker")).toHaveTextContent("Missing Material");
    expect(
      document.querySelector(".pixel-world-focus-rail__item--blocker").compareDocumentPosition(
        Array.from(document.querySelectorAll(".pixel-world-focus-rail__item")).find((item) => item.textContent.includes("Routes")),
      ) & Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
    expect(document.querySelector(".pixel-world-focus-receipt")).toHaveTextContent("Action blocked");
    expect(document.querySelector(".pixel-world-focus-receipt .pixel-world-action-receipt")).toHaveClass("pixel-world-action-receipt--focus-compact");
    expect(document.querySelector(".pixel-world-focus-receipt .pixel-world-action-receipt")).toHaveAttribute("data-receipt-confidence", "world_delta");
    expect(host).toHaveAttribute("data-focus-comparable", "true");
    expect(document.querySelector('[data-focus-cinematic="true"]')).toBeNull();
    expect(document.querySelector('[data-renderer-state="unavailable"]')).toBeNull();
    expect(document.querySelector(".pixel-world-canvas--rendered")).not.toBeNull();
    expect(document.querySelector('[data-focus-minimap="true"]')).toHaveTextContent("Mission Map");
    expect(document.querySelector('[data-focus-minimap="true"]')).not.toHaveTextContent("ref: Factory Anchor");
    expect(document.querySelector(".pixel-world-focus-fallback-map__reference-marker")).toBeNull();
    expect(document.querySelector('[data-focus-minimap="true"] .sr-only')).toHaveTextContent("Reference: Factory Anchor");
    expect(document.querySelector(".pixel-world-focus-minimap__node--target")).not.toHaveTextContent("Anchor");
    expect(document.querySelector('[data-focus-minimap="true"]')).toHaveTextContent("Build smelter mk1");
    expect(document.querySelector('[data-focus-minimap="true"]')).toHaveTextContent("agent-0");
    expect(document.querySelector('[data-focus-minimap="true"]')).toHaveTextContent("agents=1");
    expect(document.querySelector('[data-focus-minimap="true"]')).toHaveTextContent("targets=1");
    expect(document.querySelector('[data-focus-minimap="true"]')).toHaveTextContent("routes=1");
    expect(document.querySelector('[data-focus-minimap="true"]')).toHaveTextContent("fragments=2");
    expect(document.querySelector(".pixel-world-focus-controls")).toHaveAttribute("aria-label", "World focus controls");
    expect(document.querySelector(".pixel-world-focus-controls")).toContainElement(screen.getByRole("button", { name: "Command & Target" }));
    expect(document.querySelector(".pixel-world-focus-hud__cell--prompt")).toHaveTextContent("Current Objective");
    expect(document.querySelector(".pixel-world-focus-hud__cell--tick")).toHaveAttribute("data-world-tick", "12");
    expect(document.querySelector(".pixel-world-focus-hud__cell--tick")).toHaveAttribute("data-hud-priority", "telemetry");
    expect(document.querySelector(".pixel-world-focus-hud__cell--blocker")).toHaveAttribute("data-blocker-present", "true");
    expect(document.querySelector(".pixel-world-focus-hud__cell--blocker")).toHaveAttribute("data-hud-priority", "critical");
    expect(document.querySelector(".pixel-world-focus-hud__cell--receipt")).toHaveAttribute("data-receipt-confidence", "world_delta");
    expect(document.querySelector(".pixel-world-focus-hud__cell--receipt")).toHaveAttribute("data-hud-priority", "receipt");
    expect(document.querySelector('[data-focus-minimap="true"]')).toHaveClass("pixel-world-focus-minimap");

    const commandDrawer = document.querySelector(".pixel-world-focus-drawer--command");
    expect(commandDrawer.open).toBe(true);
    expect(commandDrawer.querySelector(".pixel-world-focus-command-tray")).toHaveAttribute("data-chat-ready", "true");
    expect(commandDrawer.querySelector(".pixel-world-focus-command-chip--target")).toHaveTextContent("agent=agent-0");
    expect(commandDrawer.querySelector(".pixel-world-focus-command-chip--blocker")).toHaveAttribute("data-blocker-present", "true");
    expect(commandDrawer.querySelector(".pixel-world-focus-command-chip--receipt")).toHaveTextContent("Blocked");
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

    expect(screen.getByRole("button", { name: "Command & Target" })).toHaveClass("pixel-world-focus-control--primary");
    expect(screen.getByRole("button", { name: "World Status" })).toHaveClass("pixel-world-focus-control--secondary");
    expect(screen.getByRole("button", { name: "Maximize" })).toHaveClass("pixel-world-focus-control--secondary");
    expect(screen.getByRole("button", { name: "Leave Focus · Esc" })).toHaveClass("pixel-world-focus-control--quiet");

    screen.getByRole("button", { name: "Command & Target" }).click();
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
    expect(document.querySelector(".pixel-world-focus-hud")).toHaveTextContent("Restore Layout");
    expect(document.querySelector(".pixel-world-focus-hud")).toHaveTextContent("Action blocked");
    expect(document.querySelector(".pixel-world-focus-drawer--command")?.open).toBe(true);

    screen.getByRole("button", { name: "Restore Layout" }).click();
    expect(host).toHaveAttribute("data-world-focus-maximized", "false");
    expect(document.body).not.toHaveClass("pixel-world-focus-maximized");
    expect(document.querySelector(".pixel-world-host__summary")).not.toBeNull();
    expect(document.querySelector('[data-focus-cinematic="true"]')).toBeNull();

    screen.getByRole("button", { name: "World Status" }).click();
    const diagnosticsDrawer = document.querySelector(".pixel-world-focus-drawer--diagnostics");
    expect(commandDrawer.open).toBe(false);
    expect(diagnosticsDrawer.open).toBe(true);
    expect(diagnosticsDrawer).toHaveTextContent("renderer=ready");
    expect(diagnosticsDrawer).toHaveTextContent("runtime=test_rust_runtime");

    window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    expect(host).toHaveAttribute("data-world-focus", "false");
    expect(document.body).not.toHaveClass("pixel-world-focus-active");
    expect(document.querySelector(".pixel-world-focus-drawer--diagnostics")).toBeNull();
  }, HEAVY_UI_TEST_TIMEOUT_MS);

  it("provides a test-only selected blocker visual fixture for comparable focus screenshots", async () => {
    useTestRustRenderState();
    await renderPixelWorldHost(
      emptyWorldSnapshot(),
      "?test_api=1&connect=0&locale=en&pixel_world_renderer=defer&pixel_world_visual_fixture=selected_blocker",
    );

    await waitFor(() => {
      expect(screen.getByText("Recover sustainable capability")).toBeInTheDocument();
    });

    const host = document.querySelector(".pixel-world-host");
    expect(host).toHaveAttribute("data-visual-fixture", "selected_blocker");
    expect(window.__OASIS7_PIXEL_WORLD_VISUAL_FIXTURES__.selected_blocker()).toMatchObject({
      player_gameplay: {
        blocker_kind: "material_shortage",
        intent_target: "agent-0",
      },
    });

    screen.getByRole("button", { name: "Enter World Focus" }).click();

    await waitFor(() => {
      expect(host).toHaveAttribute("data-world-focus", "true");
    });
    expect(host).toHaveAttribute("data-focus-comparable", "true");
    expect(document.querySelector('[data-focus-cinematic="true"]')).toBeNull();
    expect(document.querySelector(".pixel-world-focus-rail")).toHaveTextContent("agent-0");
    expect(document.querySelector(".pixel-world-focus-rail")).toHaveTextContent("agent/agent-0");
    expect(document.querySelector(".pixel-world-focus-hud__cell--blocker")).toHaveAttribute("data-hud-priority", "critical");
    expect(document.querySelector(".pixel-world-focus-hud__cell--receipt")).toHaveAttribute("data-hud-priority", "receipt");
    expect(document.querySelector(".pixel-world-focus-receipt")).toHaveTextContent("Action blocked");
    expect(document.querySelector('[data-focus-minimap="true"]')).toHaveTextContent("agents=2");
    expect(document.querySelector('[data-focus-minimap="true"]')).toHaveTextContent("routes=2");
    expect(document.querySelector('[data-focus-minimap="true"]')).toHaveTextContent("fragments=4");
    expect(document.querySelector(".pixel-world-focus-drawer--command")).toHaveTextContent("agent=agent-0");
  }, HEAVY_UI_TEST_TIMEOUT_MS);

  it("demotes raw focus command feedback and chat history behind diagnostics", async () => {
    useTestRustRenderState();
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
      expect(screen.getByText("Recover sustainable capability")).toBeInTheDocument();
    });
    screen.getByRole("button", { name: "Enter World Focus" }).click();
    await waitFor(() => {
      expect(document.querySelector(".pixel-world-focus-drawer--command")).not.toBeNull();
    });

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
    useTestRustRenderState();
    await renderPixelWorldHost(
      emptyWorldSnapshot(),
      "?test_api=1&connect=0&locale=en&pixel_world_renderer=defer",
    );

    await waitFor(() => {
      expect(screen.getByText("No action receipt yet")).toBeInTheDocument();
    });

    expect(document.querySelector('[data-focus-fallback-map="true"]')).toBeNull();

    screen.getByRole("button", { name: "Enter World Focus" }).click();
    await waitFor(() => {
      expect(document.querySelector('[data-focus-minimap="true"]')).not.toBeNull();
    });

    expect(document.querySelector(".pixel-world-focus-rail")).toBeNull();
    const minimap = document.querySelector('[data-focus-minimap="true"]');
    expect(minimap).not.toBeNull();
    expect(minimap).toHaveTextContent("agents=0");
    expect(minimap).toHaveTextContent("targets=0");
    expect(minimap).toHaveTextContent("routes=0");
    expect(minimap).toHaveTextContent("fragments=0");
    expect(minimap).toHaveTextContent("Unassigned");
    expect(minimap).not.toHaveTextContent("Selected");
  });

  it("preserves world focus UI state across host remounts", async () => {
    useTestRustRenderState();
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
      expect(screen.getByText("Recover sustainable capability")).toBeInTheDocument();
    });

    const firstHost = document.querySelector(".pixel-world-host");
    screen.getByRole("button", { name: "Enter World Focus" }).click();
    await waitFor(() => {
      expect(firstHost).toHaveAttribute("data-world-focus", "true");
    });
    screen.getByRole("button", { name: "World Status" }).click();

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
    await waitFor(() => {
      expect(document.querySelector(".pixel-world-focus-drawer--diagnostics")).not.toBeNull();
    });
    expect(document.querySelector(".pixel-world-focus-drawer--diagnostics").open).toBe(true);
    expect(document.querySelector(".pixel-world-focus-drawer--command")).toHaveProperty("open", false);
  }, HEAVY_UI_TEST_TIMEOUT_MS);

  it("keeps the Rust canvas primary while preserving readable agent hit targets", async () => {
    useTestRustRenderState();
    await renderPixelWorldHost();

    await waitFor(() => {
      expect(screen.getByText("Recover sustainable capability")).toBeInTheDocument();
    });

    const canvas = document.querySelector(".pixel-world-canvas");
    const fragments = Array.from(canvas.querySelectorAll(".pixel-world-fragment-terrain"));
    const route = canvas.querySelector(".pixel-world-route");
    const location = canvas.querySelector(".pixel-world-entity--location");
    const agent = screen.getByRole("button", { name: "Select Agent agent-0" });

    expect(screen.getByRole("img", { name: "World canvas overview" })).toBeInTheDocument();
    expect(fragments).toHaveLength(0);
    expect(route).toBeNull();
    expect(location).toBeNull();
    expect(agent).toHaveAttribute("data-pixel-world-agent-marker", "true");
    expect(agent).toHaveAttribute("data-position-source", "location_derived");
    expect(agent).toHaveAttribute("aria-label", "Select Agent agent-0");
  });
});
