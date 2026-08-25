import { createEffect, createMemo, createSignal, For, Index, Show, onCleanup, onMount } from "solid-js";

import * as core from "./legacy_core.js";
import { createPixelWorldRuntimeBridge, probePixelWorldWebgl2Surface } from "./pixel_world_runtime_loader.js";
import { installPixelWorldHotspotPointerProbe } from "./pixel_world_hotspot_probe.js"; import { createPixelWorldFocusController } from "./pixel_world_focus_controller.js";
import { installPixelWorldRenderDtoProbe, installPixelWorldVisualFixtureHook, pixelWorldTestApiEnabled } from "./pixel_world_visual_fixture.js";
import { pixelWorldSelectedBlockerVisualFixture } from "./pixel_world_visual_fixture_data.js";
export { pixelWorldSelectedBlockerVisualFixture };

function tr(locale, zh, en) {
  return core.isLocaleZh(locale) ? zh : en;
}

const PIXEL_WORLD_RUNTIME_CANVAS_ID = "pixel-world-embedded-runtime-canvas";
const PIXEL_WORLD_RENDERER_UNAVAILABLE_MESSAGE_ID = "pixel-world-renderer-unavailable-message";
const DEFER_RENDERER_VALUES = new Set(["0", "false", "no", "off", "defer", "fallback"]);
const pixelWorldFocusUiSessionState = {
  focusMode: false,
  commandDrawerOpen: false,
  diagnosticsDrawerOpen: false,
  maximized: false,
};
const FRAGMENT_TERRAIN_PALETTE = {
  silicate_matrix: [126, 144, 99],
  iron_nickel_alloy: [176, 184, 196],
  water_ice: [125, 211, 252],
  hydrated_mineral: [96, 165, 250],
  carbonaceous_organic: [120, 113, 108],
  sulfide_ore: [202, 138, 4],
  rare_earth_oxide: [167, 139, 250],
  uranium_bearing_ore: [132, 204, 22],
  thorium_bearing_ore: [244, 114, 182],
  unknown: [148, 163, 184],
};

async function waitForRuntimeCanvasAttachment(canvas) {
  for (let attempt = 0; attempt < 12; attempt += 1) {
    if (
      canvas?.isConnected
      && document.getElementById(PIXEL_WORLD_RUNTIME_CANVAS_ID) === canvas
    ) {
      return true;
    }
    await new Promise((resolve) => {
      requestAnimationFrame(() => resolve());
    });
  }
  return false;
}

function safeNumber(value, fallback = 0) {
  const number = Number(value);
  return Number.isFinite(number) ? number : fallback;
}

function snapshotTick(snapshot) {
  if (!snapshot || typeof snapshot !== "object") {
    return null;
  }
  const tick = Number(fieldValue(snapshot, "time", "time", null));
  if (!Number.isFinite(tick)) {
    return null;
  }
  return Math.max(0, Math.floor(tick));
}

function colorToCss(color, alpha = 0.36) {
  const [red, green, blue] = Array.isArray(color) ? color : FRAGMENT_TERRAIN_PALETTE.unknown;
  return `rgba(${red}, ${green}, ${blue}, ${alpha})`;
}

function clampRatio(value) {
  return Math.min(1, Math.max(0, Number(value) || 0));
}

function toWorldPercentStyle(pos, worldBounds, fallbackStyle) {
  if (!pos || !worldBounds) {
    return fallbackStyle;
  }
  const point = worldPercentPoint(pos, worldBounds, 8, 10);
  return {
    left: `${point.x.toFixed(1)}%`,
    top: `${point.y.toFixed(1)}%`,
  };
}

function agentMarkerStyle(agent, index, worldBounds) {
  const base = toWorldPercentStyle(agent.pos, worldBounds, {
    left: `${18 + ((index % 5) * 15)}%`,
    top: `${14 + (Math.floor(index / 5) * 22)}%`,
  });
  const offsets = [
    [-18, -18],
    [18, -18],
    [-18, 18],
    [18, 18],
    [0, -30],
    [0, 30],
    [-30, 0],
    [30, 0],
    [-28, -28],
    [28, 28],
  ];
  const [x, y] = offsets[index % offsets.length] || [0, 0];
  return {
    ...base,
    transform: `translate(${x}px, ${y}px)`,
  };
}

function worldPercentPoint(pos, worldBounds, fallbackX = 50, fallbackY = 50) {
  if (!pos || !worldBounds) {
    return { x: fallbackX, y: fallbackY };
  }
  return {
    x: 8 + (clampRatio(pos.x_cm / Math.max(1, worldBounds.width_cm)) * 84),
    y: 10 + (clampRatio(pos.y_cm / Math.max(1, worldBounds.depth_cm)) * 78),
  };
}

const FALLBACK_ROUTE_HEIGHT_TO_WIDTH_RATIO = 9 / 16;

function routeStyle(link, worldBounds, index) {
  const fallbackFrom = {
    x: 14 + ((index % 5) * 15),
    y: 18 + (Math.floor(index / 5) * 14),
  };
  const fallbackTo = {
    x: fallbackFrom.x + 14,
    y: fallbackFrom.y + 8,
  };
  const from = worldPercentPoint(link.from, worldBounds, fallbackFrom.x, fallbackFrom.y);
  const to = worldPercentPoint(link.to, worldBounds, fallbackTo.x, fallbackTo.y);
  const deltaX = to.x - from.x;
  const deltaY = to.y - from.y;
  const scaledDeltaY = deltaY * FALLBACK_ROUTE_HEIGHT_TO_WIDTH_RATIO;
  const length = Math.max(4, Math.hypot(deltaX, scaledDeltaY));
  const angle = Math.atan2(scaledDeltaY, deltaX) * (180 / Math.PI);
  return {
    left: `${from.x.toFixed(1)}%`,
    top: `${from.y.toFixed(1)}%`,
    width: `${length.toFixed(1)}%`,
    opacity: `${0.32 + (clampRatio(link.emphasis ?? 0.72) * 0.38)}`,
    transform: `rotate(${angle.toFixed(1)}deg)`,
    "transform-origin": "0 50%",
  };
}

function fragmentTerrainStyle(patch, worldBounds, index) {
  const sizePx = Math.max(12, Math.min(48, safeNumber(patch.footprint_cm, 1) / 840));
  return {
    ...toWorldPercentStyle(patch.pos, worldBounds, {
      left: `${12 + ((index % 6) * 13)}%`,
      top: `${16 + (Math.floor(index / 6) * 13)}%`,
    }),
    width: `${sizePx.toFixed(1)}px`,
    height: `${sizePx.toFixed(1)}px`,
    "background-color": colorToCss(patch.color),
    transform: "translate(-50%, -50%)",
  };
}

function routeWaypointStyle(link, worldBounds, index, stop) {
  const fallbackFrom = {
    x: 14 + ((index % 5) * 15),
    y: 18 + (Math.floor(index / 5) * 14),
  };
  const fallbackTo = {
    x: fallbackFrom.x + 14,
    y: fallbackFrom.y + 8,
  };
  const from = worldPercentPoint(link.from, worldBounds, fallbackFrom.x, fallbackFrom.y);
  const to = worldPercentPoint(link.to, worldBounds, fallbackTo.x, fallbackTo.y);
  const ratio = stop === "to" ? 1 : 0.52;
  return {
    left: `${(from.x + ((to.x - from.x) * ratio)).toFixed(1)}%`,
    top: `${(from.y + ((to.y - from.y) * ratio)).toFixed(1)}%`,
  };
}

function hotspotStyle(hotspot, worldBounds, index) {
  const sizePx = Math.max(14, Math.min(32, safeNumber(hotspot.size_hint_px, 16)));
  return {
    ...toWorldPercentStyle(hotspot.pos, worldBounds, {
      left: `${20 + ((index % 4) * 16)}%`,
      top: `${22 + (Math.floor(index / 4) * 16)}%`,
    }),
    width: `${sizePx}px`,
    height: `${sizePx}px`,
    transform: "translate(-50%, -50%)",
  };
}

function fieldValue(value, snakeName, camelName, fallback = undefined) {
  if (!value || typeof value !== "object") {
    return fallback;
  }
  if (value[snakeName] !== undefined) {
    return value[snakeName];
  }
  if (camelName && value[camelName] !== undefined) {
    return value[camelName];
  }
  return fallback;
}

function arrayField(value, snakeName, camelName) {
  const candidate = fieldValue(value, snakeName, camelName, []);
  return Array.isArray(candidate) ? candidate : [];
}

function normalizeVisualEntity(entry) {
  if (!entry || typeof entry !== "object") {
    return entry;
  }
  return {
    ...entry,
    location_id: fieldValue(entry, "location_id", "locationId", null),
    marker_role: fieldValue(entry, "marker_role", "markerRole", null),
    marker_alpha: fieldValue(entry, "marker_alpha", "markerAlpha", undefined),
    position_source: fieldValue(entry, "position_source", "positionSource", null),
    dominant_compound: fieldValue(entry, "dominant_compound", "dominantCompound", undefined),
    footprint_cm: fieldValue(entry, "footprint_cm", "footprintCm", undefined),
  };
}

function pixelWorldVisualState(renderState) {
  const state = renderState || {};
  return {
    worldBounds: fieldValue(state, "world_bounds", "worldBounds", null),
    fragmentTerrain: arrayField(state, "fragment_terrain", "fragmentTerrain").map(normalizeVisualEntity),
    links: arrayField(state, "links", "links"),
    locations: arrayField(state, "locations", "locations").map(normalizeVisualEntity),
    agents: arrayField(state, "agents", "agents").map(normalizeVisualEntity),
    selection: fieldValue(state, "selection", "selection", null),
    goalHighlight: fieldValue(state, "goal_highlight", "goalHighlight", null),
    blockerHighlight: fieldValue(state, "blocker_highlight", "blockerHighlight", null),
    visualHotspots: arrayField(state, "visual_hotspots", "visualHotspots").map(normalizeVisualEntity),
  };
}

function PixelWorldHostVisualLayer(props) {
  const visualState = () => pixelWorldVisualState(props.renderState());
  const selection = () => props.selection?.() || visualState().selection;
  if (!props.enabled) {
    return <></>;
  }
  return (
    <>
      <div class="pixel-world-canvas__grid" />
      <div class="pixel-world-canvas__terrain-band pixel-world-canvas__terrain-band--one" />
      <div class="pixel-world-canvas__terrain-band pixel-world-canvas__terrain-band--two" />
      <For each={visualState().fragmentTerrain.slice(0, 96)}>
        {(patch, index) => (
          <div
            class="pixel-world-fragment-terrain"
            data-compound={patch.dominant_compound}
            style={fragmentTerrainStyle(patch, visualState().worldBounds, index())}
            title={`${patch.location_id}:${patch.dominant_compound}`}
          />
        )}
      </For>
      <For each={visualState().links.slice(0, 10)}>
        {(link, index) => (
          <>
            <div
              class="pixel-world-route"
              data-route-kind={link.kind}
              style={routeStyle(link, visualState().worldBounds, index())}
              title={`${link.kind}:${link.id}`}
            />
            <div
              class="pixel-world-route-waypoint pixel-world-route-waypoint--mid"
              data-route-kind={link.kind}
              style={routeWaypointStyle(link, visualState().worldBounds, index(), "mid")}
              title={`${link.kind}:waypoint`}
            />
            <div
              class="pixel-world-route-waypoint pixel-world-route-waypoint--target"
              data-route-kind={link.kind}
              style={routeWaypointStyle(link, visualState().worldBounds, index(), "to")}
              title={`${link.kind}:target`}
            />
          </>
        )}
      </For>
      <For each={visualState().visualHotspots.slice(0, 8)}>
        {(hotspot, index) => (
          <div
            class="pixel-world-hotspot"
            data-hotspot-kind={hotspot.kind}
            style={hotspotStyle(hotspot, visualState().worldBounds, index())}
            title={`${hotspot.kind}:${hotspot.label}`}
          >
            <span>{hotspot.kind === "blocker" ? "!" : hotspot.kind === "goal" ? "G" : "i"}</span>
          </div>
        )}
      </For>
      <Index each={visualState().locations.slice(0, 8)}>
        {(location, index) => (
          <button
            class="pixel-world-entity pixel-world-entity--location"
            data-pixel-world-location-marker="true"
            data-location-id={location().id}
            data-selected={selection()?.kind === "location" && selection()?.id === location().id ? "true" : "false"}
            aria-pressed={selection()?.kind === "location" && selection()?.id === location().id ? "true" : "false"} aria-label={`${tr(props.locale(), "选择地点", "Select Location")} ${location().label || location().id}`}
            data-marker-role={location().marker_role}
            style={{
              ...toWorldPercentStyle(location().pos, visualState().worldBounds, {
                left: `${12 + ((index % 4) * 21)}%`,
                top: `${18 + (Math.floor(index / 4) * 26)}%`,
              }),
              opacity: location().marker_alpha,
            }}
            title={location().label}
            onMouseEnter={() => props.onHover({ kind: "location", id: location().id })}
            onMouseLeave={() => props.onHover(null)}
            onClick={() => props.onSelect({ kind: "location", id: location().id })}
          >
            <span>{location().label.slice(0, 2).toUpperCase()}</span>
          </button>
        )}
      </Index>
      <Index each={visualState().agents.slice(0, 10)}>
        {(agent, index) => (
          <button
            class="pixel-world-entity pixel-world-entity--agent"
            data-pixel-world-agent-marker="true"
            data-agent-id={agent().id}
            data-selected={selection()?.kind === "agent" && selection()?.id === agent().id ? "true" : "false"}
            data-position-source={agent().position_source}
            aria-pressed={selection()?.kind === "agent" && selection()?.id === agent().id ? "true" : "false"} aria-label={`${tr(props.locale(), "选择 Agent", "Select Agent")} ${agent().label || agent().id}`}
            style={agentMarkerStyle(agent(), index, visualState().worldBounds)}
            title={agent().label}
            onMouseEnter={() => props.onHover({ kind: "agent", id: agent().id })}
            onMouseLeave={() => props.onHover(null)}
            onClick={() => props.onSelect({ kind: "agent", id: agent().id })}
          >
            <span>{agent().label.slice(0, 1).toUpperCase()}</span>
          </button>
        )}
      </Index>
    </>
  );
}

function PixelWorldCanvasAgentHitTargets(props) {
  const visualState = () => pixelWorldVisualState(props.renderState());
  return (
    <For each={visualState().agents.slice(0, 10)}>
      {(agent, index) => (
        <button
          type="button"
          class="pixel-world-entity pixel-world-entity--agent pixel-world-entity--canvas-hit-target"
          data-pixel-world-agent-marker="true"
          data-agent-id={agent.id}
          data-position-source={agent.position_source}
          data-selected={props.selection()?.kind === "agent" && props.selection()?.id === agent.id ? "true" : "false"} aria-pressed={props.selection()?.kind === "agent" && props.selection()?.id === agent.id ? "true" : "false"} aria-label={`${tr(props.locale(), "选择 Agent", "Select Agent")} ${agent.label || agent.id}`}
          style={agentMarkerStyle(agent, index(), visualState().worldBounds)}
          title={agent.label}
          onMouseEnter={() => props.onHover({ kind: "agent", id: agent.id })}
          onMouseLeave={() => props.onHover(null)}
          onClick={() => props.onSelect({ kind: "agent", id: agent.id })}
        >
          <span>{agent.label.slice(0, 1).toUpperCase()}</span>
        </button>
      )}
    </For>
  );
}

function createPixelWorldHostAdapter({ onSelectEntity, onHoverEntity, onFatal }) {
  let bridge = null;
  let runtimeSource = "detached";
  let runtimeModuleUrl = null;
  let deriveRenderState = null;

  function withWorldTickReadout(renderState, renderInput) {
    if (!renderState || !renderInput) {
      return renderState;
    }
    const worldTick = snapshotTick(renderInput.snapshot);
    if (worldTick === null) {
      return renderState;
    }
    return {
      ...renderState,
      world_tick: renderState.world_tick ?? worldTick,
      commercial_surface: renderState.commercial_surface
        ? {
          ...renderState.commercial_surface,
          world_read: {
            ...(renderState.commercial_surface.world_read || {}),
            tick: renderState.commercial_surface.world_read?.tick ?? worldTick,
          },
        }
        : renderState.commercial_surface,
    };
  }

  function deriveRenderStateOrUnavailable(renderInput) {
    if (!deriveRenderState || !renderInput) {
      return {
        renderState: null,
        fatal: {
          code: "pixel_world_render_state_unavailable",
          message: "pixel world Rust render-state derivation is unavailable",
        },
      };
    }
    try {
      const nextRenderState = deriveRenderState(renderInput);
      if (nextRenderState?.fatal) {
        onFatal?.(nextRenderState.fatal);
        return {
          renderState: null,
          fatal: nextRenderState.fatal,
        };
      }
      return {
        renderState: withWorldTickReadout(nextRenderState, renderInput) || null,
        fatal: null,
      };
    } catch (error) {
      const fatal = {
        code: "pixel_world_rust_render_state_failed",
        message: error instanceof Error ? error.message : String(error || "Rust render state derivation failed"),
      };
      onFatal?.(fatal);
      return {
        renderState: null,
        fatal,
      };
    }
  }

  return {
    async mount(canvas, renderInput) {
      const runtime = await createPixelWorldRuntimeBridge({
        onEvent(event) {
          if (event?.type === "canvas_ready") {
            return;
          }
          if (event?.type === "select_entity") {
            onSelectEntity?.(event.selection);
            return;
          }
          if (event?.type === "hover_entity") {
            onHoverEntity?.(event.selection || null);
            return;
          }
          if (event?.type === "camera_state_changed") {
            onFatal?.(null, event.camera || null);
          }
        },
        onFatal,
      });
      bridge = runtime.bridge;
      deriveRenderState = runtime.deriveRenderState || null;
      runtimeSource = runtime.source;
      runtimeModuleUrl = runtime.moduleUrl || null;
      const derived = deriveRenderStateOrUnavailable(renderInput);
      if (!derived.renderState) {
        const fatal = derived.fatal || runtime.fatal || {
          code: "pixel_world_render_state_unavailable",
          message: "pixel world Rust render-state derivation is unavailable",
        };
        onFatal?.(fatal);
        return {
          status: "unavailable",
          selection: null,
          fatal,
          renderState: null,
          runtimeSource,
          runtimeModuleUrl,
        };
      }
      const mountedRenderState = derived.renderState;
      try {
        const result = await bridge.mount(canvas, mountedRenderState);
        return { status: result?.status || "ready", selection: mountedRenderState.selection, fatal: result?.fatal || null, renderState: mountedRenderState, runtimeSource, runtimeModuleUrl };
      } catch (error) {
        const fatal = { code: "pixel_world_webgl2_unavailable", message: error instanceof Error ? error.message : String(error || "embedded WebGL2 renderer mount failed") };
        onFatal?.(fatal);
        return { status: "unavailable", selection: null, fatal, renderState: null, runtimeSource, runtimeModuleUrl };
      }
    },
    update(renderInput) {
      const derived = deriveRenderStateOrUnavailable(renderInput);
      if (!derived.renderState) {
        const result = bridge?.update(null) || { status: "unavailable", fatal: derived.fatal };
        return {
          status: result?.status || "unavailable",
          selection: null,
          fatal: result?.fatal || derived.fatal,
          renderState: null,
          runtimeSource,
          runtimeModuleUrl,
        };
      }
      const nextRenderState = derived.renderState;
      const result = bridge?.update(nextRenderState) || { status: "detached" };
      return {
        status: result?.status || "ready",
        selection: nextRenderState.selection,
        fatal: result?.fatal || null,
        renderState: nextRenderState,
        runtimeSource,
        runtimeModuleUrl,
      };
    },
    unmount() {
      const result = bridge?.unmount() || { status: "detached" };
      bridge = null;
      deriveRenderState = null;
      runtimeSource = "detached";
      runtimeModuleUrl = null;
      return result;
    },
    simulateSelect(selection) {
      if (!selection?.kind || !selection?.id) {
        return;
      }
      onSelectEntity?.(selection);
    },
    simulateHover(selection) {
      onHoverEntity?.(selection || null);
    },
    simulateFatal(message) { onFatal?.(typeof message === "object" ? message : { code: "pixel_world_renderer_fatal", message: String(message || "renderer fatal") }); },
    runtimeSource() {
      return runtimeSource;
    },
    runtimeModuleUrl() {
      return runtimeModuleUrl;
    },
    hotspotTestHitTargets() {
      return bridge?.hotspotTestHitTargets?.() || [];
    },
    locationTestHitTargets() { return bridge?.locationTestHitTargets?.() || []; },
    deriveRenderState(renderInput) {
      return deriveRenderStateOrUnavailable(renderInput).renderState;
    },
  };
}

export function buildPixelWorldRenderInput(locale = core.state.uiLocale) {
  const worldScaleSurface = core.buildWorldScaleSurface(locale);
  return {
    locale,
    snapshot: core.state.snapshot,
    lists: core.modelLists(),
    gameplay: core.buildGameplaySummary(locale),
    selected: core.clone(core.state.selectedObject),
    selectedKind: core.state.selectedKind,
    selectedId: core.state.selectedId,
    recentEvents: core.clone(core.state.recentEvents),
    presentation: {
      world_bounds_label: worldScaleSurface.physicalTruth.worldBoundsLabel,
      marker_truth_note: worldScaleSurface.presentationScale.markerTruthNote,
    },
  };
}

function PixelWorldCanvasRenderer(props) {
  let canvasRef;
  const visualState = () => pixelWorldVisualState(props.renderState());

  createEffect(() => {
    if (!canvasRef) {
      return;
    }
    props.onCanvasMount?.(canvasRef);
  });

  createEffect(() => {
    props.renderInput?.();
    if (!canvasRef) {
      return;
    }
    props.onCanvasUpdate?.();
  });

  return (
    <div class="pixel-world-canvas pixel-world-canvas--rendered" data-renderer-ready={props.rendererStatus?.() === "ready" ? "true" : undefined}>
      <canvas
        ref={canvasRef}
        id={PIXEL_WORLD_RUNTIME_CANVAS_ID}
        class="pixel-world-canvas__surface"
        tabIndex="0"
        role="img"
        aria-label={tr(props.locale(), "世界 Canvas 概览", "World canvas overview")}
        aria-describedby="pixel-world-canvas-accessible-summary"
        width="960"
        height="540"
      />
      <div id="pixel-world-canvas-accessible-summary" class="sr-only">
        {tr(
          props.locale(),
          "Canvas 提供当前世界的只读概览；相邻 HUD、焦点栏和命令抽屉提供当前 Agent、阻塞、回执与命令路径。",
          "The canvas provides a read-only overview of the current world; adjacent HUD, focus rail, and command drawer expose the current agent, blocker, receipt, and command path.",
        )}
      </div>
      <div class="pixel-world-canvas__overlay">
        <PixelWorldCanvasAgentHitTargets
          locale={props.locale} renderState={props.renderState} selection={props.selection}
          onSelect={props.onSelect}
          onHover={props.onHover}
        />
        <PixelWorldHostVisualLayer
          enabled={props.visualOverlayEnabled?.() ?? false}
          locale={props.locale}
          renderState={props.renderState}
          selection={props.selection}
          onSelect={props.onSelect}
          onHover={props.onHover}
        />
        <Show when={visualState().goalHighlight}>
          <div class="pixel-world-canvas__callout pixel-world-canvas__callout--goal">
            {`${tr(props.locale(), "目标", "Goal")}: ${visualState().goalHighlight.title}`}
          </div>
        </Show>
        <Show when={visualState().blockerHighlight}>
          <div class="pixel-world-canvas__callout pixel-world-canvas__callout--blocker">
            {`${tr(props.locale(), "阻塞", "Blocker")}: ${visualState().blockerHighlight.label || visualState().blockerHighlight.kind}`}
          </div>
        </Show>
        <Show when={props.hoveredHotspot?.()}>
          <div class="pixel-world-canvas__hotspot-tooltip" data-hotspot-tooltip role="status">
            {props.hoveredHotspot().label}
          </div>
        </Show>
      </div>
      <Show when={visualState().selection}>
        <div class="pixel-world-canvas__selection">
          {`${tr(props.locale(), "已选中", "Selected")}: ${visualState().selection.kind}/${visualState().selection.id}`}
        </div>
      </Show>
    </div>
  );
}

function PixelWorldActionReceipt(props) {
  const receipt = () => props.surface().action_receipt;
  return (
    <div
      id={props.id}
      class={`pixel-world-action-receipt ${props.class ?? ""}`} data-viewer-overlay="receipt"
      data-receipt-present={receipt().present ? "true" : "false"}
      data-receipt-state={receipt().state}
      data-receipt-confidence={receipt().confidence}
    >
      <div class="pixel-world-action-receipt__label">
        {tr(props.locale(), "行动回执", "Action Receipt")}
      </div>
      <div class="pixel-world-action-receipt__body">
        <div class="pixel-world-action-receipt__title">
          {receipt().title}
        </div>
        <div class="pixel-world-action-receipt__summary">
          {receipt().summary}
        </div>
        <Show when={receipt().detail}>
          <div class="pixel-world-action-receipt__detail">
            {receipt().detail}
          </div>
        </Show>
      </div>
      <Show when={receipt().present}>
        <div class="pixel-world-action-receipt__meta">
          <span>{receiptConfidenceLabel(receipt().confidence, props.locale())}</span>
          <Show when={receipt().target_agent_id}>
            <span>{`agent=${receipt().target_agent_id}`}</span>
          </Show>
        </div>
      </Show>
    </div>
  );
}
function receiptConfidenceLabel(confidence, locale) {
  const value = String(confidence || "").trim().toLowerCase();
  return value === "world_delta" ? tr(locale, "世界变化已确认", "World change confirmed") : value === "accepted_intent" ? tr(locale, "行动已接受", "Action accepted") : value === "none" ? tr(locale, "等待确认", "Waiting for confirmation") : tr(locale, "状态已记录", "Status recorded");
}
function worldReadoutStatus(locale, renderState) {
  renderState?.(); const status = String(core.state.connectionStatus || "").toLowerCase(); const feed = core.state.worldFeed || {};
  const warn = (label) => ({ label, className: "badge badge--warn" });
  if (feed.stale) return warn(tr(locale, "陈旧", "STALE"));
  if (status === "connecting" || status === "reconnecting") return warn(tr(locale, "正在重连", "RECONNECTING"));
  if (status !== "connected") return warn(tr(locale, "离线", "OFFLINE"));
  return ["ready", "replay", "empty"].includes(String(feed.status || "").toLowerCase()) ? { label: "LIVE", className: "badge badge--good" } : warn(tr(locale, "同步中", "SYNCING"));
}

const DIRECT_PIXEL_WORLD_NEXT_MOVE_KINDS = new Set(["claim_first_agent", "claim_starter_oc"]);

export function resolvePixelWorldDirectNextMoveAction(gameplay, executeKind) {
  if (!DIRECT_PIXEL_WORLD_NEXT_MOVE_KINDS.has(executeKind)) {
    return null;
  }
  const actions = Array.isArray(gameplay?.availableActions) ? gameplay.availableActions : [];
  return actions.find((action) => (
    action?.executeKind === executeKind
    && !action?.disabledReason
  )) || null;
}

function PixelWorldCommercialHud(props) {
  const surface = () => props.renderState().commercial_surface; const readoutStatus = () => worldReadoutStatus(props.locale(), props.renderState);
  const executableNextMoveKinds = new Set([
    "gameplay_action",
    "claim_first_agent",
    "claim_starter_oc",
    "step",
    "play",
    "request_snapshot",
  ]);
  const nextMoveRoutesToGameplayDetails = () => executableNextMoveKinds.has(surface().next_action.execute_kind);
  const nextMoveRoute = () => nextMoveRoutesToGameplayDetails() ? "gameplay_details" : "command";
  const nextMoveHref = () => nextMoveRoutesToGameplayDetails() ? "#viewer-gameplay-details" : "#viewer-details-panel";
  const directNextMoveAction = () => resolvePixelWorldDirectNextMoveAction(
    core.buildGameplaySummary(props.locale()),
    surface().next_action.execute_kind,
  );
  const openGameplayDetails = () => {
    if (!nextMoveRoutesToGameplayDetails()) {
      return;
    }
    const details = document.getElementById("viewer-gameplay-details");
    if (details) {
      details.open = true;
    }
  };
  const activateNextMove = (event) => {
    event?.preventDefault?.();
    event?.stopPropagation?.();
    openGameplayDetails();
    const action = directNextMoveAction();
    if (action) {
      core.sendGameplayAction(action);
    } else if (nextMoveHref().startsWith("#")) {
      window.location.hash = nextMoveHref();
    }
  };
  return (
    <Show when={surface()}>
      <div
        class="pixel-world-command-strip" data-viewer-overlay="next-move"
        data-active-agent={surface().active_agent_id || ""}
        data-leverage-state={surface().player_leverage.state}
      >
        <div class="pixel-world-command-cell pixel-world-command-cell--objective">
          <div class="pixel-world-command-cell__label">
            {tr(props.locale(), "目标", "Objective")}
          </div>
          <div class="pixel-world-command-cell__value">{surface().objective.title}</div>
          <div class="pixel-world-command-cell__detail">{surface().objective.detail}</div>
        </div>
        <div
          class="pixel-world-command-cell pixel-world-command-cell--next"
          data-next-move-route={nextMoveRoute()}
          data-execute-kind={surface().next_action.execute_kind || "none"}
          data-blocker-present={surface().blocker.label ? "true" : "false"}
        >
          <div class="pixel-world-command-cell__header">
            <div class="pixel-world-command-cell__label">
              {tr(props.locale(), "下一步", "Next Move")}
            </div>
            <Show when={surface().blocker.label}>
              <span class="pixel-world-command-cell__blocker-chip">
                {`${tr(props.locale(), "阻塞", "Blocker")}: ${surface().blocker.label}`}
              </span>
            </Show>
          </div>
          <div class="pixel-world-command-cell__value">{surface().next_action.label}</div>
          <Show when={surface().next_action.detail}>
            <div class="pixel-world-command-cell__detail">{surface().next_action.detail}</div>
          </Show>
          <a
            class="pixel-world-command-cell__action"
            href={nextMoveHref()}
            aria-label={`${tr(props.locale(), "下一步", "Next Move")}: ${surface().next_action.label}`}
            onClick={activateNextMove}
          >
            {directNextMoveAction()
              ? surface().next_action.label
              : nextMoveRoutesToGameplayDetails()
                ? tr(props.locale(), "打开玩法明细", "Open Gameplay Details")
                : tr(props.locale(), "去指挥面板", "Go to Command")}
          </a>
        </div>
        <div class="pixel-world-command-cell pixel-world-command-cell--leverage">
          <div class="pixel-world-command-cell__label">
            {tr(props.locale(), "玩家杠杆", "Player Leverage")}
          </div>
          <div class="pixel-world-command-cell__value">{surface().player_leverage.summary}</div>
          <div class="pixel-world-command-cell__detail">
            {surface().active_agent_id
              ? `${surface().player_leverage.label} · agent=${surface().active_agent_id}`
              : surface().player_leverage.label}
          </div>
        </div>
      </div>
      <Show when={!props.focusMode?.()}><PixelWorldActionReceipt id="viewer-action-receipt" locale={props.locale} surface={surface} /></Show>
      <div class="pixel-world-readout badge-row">
        <span class={readoutStatus().className}>{readoutStatus().label}</span>
        <Show when={surface().world_read.tick !== null && surface().world_read.tick !== undefined}>
          <span class="badge badge--accent" data-world-tick={String(surface().world_read.tick)}>{`tick=${surface().world_read.tick}`}</span>
        </Show>
        <span class="badge badge--accent">{`agents=${surface().world_read.agents}`}</span>
      </div>
    </Show>
  );
}

function PixelWorldFocusHud(props) {
  const surface = () => props.renderState().commercial_surface;
  return (
    <Show when={surface()}>
      <div class="pixel-world-focus-hud" data-focus-hud="true">
        <div class="pixel-world-focus-hud__identity">
          <div class="pixel-world-focus-hud__eyebrow">
            {tr(props.locale(), "电影视图", "Cinematic View")}
          </div>
          <div class="pixel-world-focus-hud__title">
            {tr(props.locale(), "世界指挥棋盘", "World Command Board")}
          </div>
        </div>
        <div class="pixel-world-focus-hud__cell pixel-world-focus-hud__cell--prompt">
          <span>{tr(props.locale(), "当前目标", "Current Objective")}</span>
          <strong>{surface().objective.title}</strong>
          <em>{surface().next_action.label}</em>
        </div>
        <div class="pixel-world-focus-hud__cell pixel-world-focus-hud__cell--mission">
          <span>{tr(props.locale(), "任务进度", "Mission Progress")}</span>
          <strong>
            {surface().objective.progress_percent == null
              ? tr(props.locale(), "进行中", "In Progress")
              : `${surface().objective.progress_percent}%`}
          </strong>
          <em>{surface().next_action.detail || surface().objective.detail}</em>
        </div>
        <Show when={surface().world_read.tick !== null && surface().world_read.tick !== undefined}>
          <div
            class="pixel-world-focus-hud__cell pixel-world-focus-hud__cell--tick"
            data-world-tick={String(surface().world_read.tick)}
            data-hud-priority="telemetry"
          >
            <span>{tr(props.locale(), "世界 Tick", "World Tick")}</span>
            <strong>{surface().world_read.tick}</strong>
            <em>{`tick=${surface().world_read.tick}`}</em>
          </div>
        </Show>
        <div
          class="pixel-world-focus-hud__cell pixel-world-focus-hud__cell--blocker"
          data-blocker-present={surface().blocker.label ? "true" : "false"}
          data-hud-priority={surface().blocker.label ? "critical" : "clear"}
        >
          <span>{tr(props.locale(), "阻塞", "Blocker")}</span>
          <strong>{surface().blocker.label || tr(props.locale(), "暂无阻塞", "No blocker")}</strong>
        </div>
        <div
          class="pixel-world-focus-hud__cell pixel-world-focus-hud__cell--receipt"
          data-receipt-confidence={surface().action_receipt.confidence}
          data-hud-priority={surface().action_receipt.present ? "receipt" : "waiting"}
        >
          <span>{tr(props.locale(), "回执", "Receipt")}</span>
          <strong>{surface().action_receipt.title}</strong>
          <em>{receiptConfidenceLabel(surface().action_receipt.confidence, props.locale())}</em>
        </div>
        <div class="pixel-world-focus-controls" aria-label={tr(props.locale(), "电影视图控制", "Cinematic controls")}>
          <button type="button" class="pixel-world-focus-control pixel-world-focus-control--primary" onClick={props.onOpenCommand}>
            {tr(props.locale(), "命令与目标", "Command & Target")}
          </button>
          <button id="viewer-focus-exit" type="button" class="pixel-world-focus-control pixel-world-focus-control--quiet" onClick={props.onExit}>{tr(props.locale(), "退出电影视图", "Exit Cinematic")}</button>
          <details class="pixel-world-focus-more-controls">
            <summary>{tr(props.locale(), "更多控制", "More controls")}</summary>
            <button type="button" class="pixel-world-focus-control pixel-world-focus-control--secondary" onClick={props.onOpenDiagnostics}>{tr(props.locale(), "世界状态", "World Status")}</button>
            <button type="button" class="pixel-world-focus-control pixel-world-focus-control--secondary" onClick={props.onToggleMaximized}>
              {props.maximized()
                ? tr(props.locale(), "还原布局", "Restore Layout")
                : tr(props.locale(), "最大化", "Maximize")}
            </button>
          </details>
        </div>
      </div>
    </Show>
  );
}

function PixelWorldFocusCinematicBanner(props) {
  const surface = () => props.renderState().commercial_surface;
  return (
    <Show when={surface()}>
      <div class="pixel-world-focus-cinematic" data-focus-cinematic="true">
        <div class="pixel-world-focus-cinematic__eyebrow">
          {tr(props.locale(), "电影化首屏", "Cinematic Opening")}
        </div>
        <div class="pixel-world-focus-cinematic__title">
          {tr(props.locale(), "工业世界指挥台", "Industrial World Command Board")}
        </div>
        <div class="pixel-world-focus-cinematic__body">
          {surface().objective.detail}
        </div>
        <div class="badge-row">
          <span class="badge badge--accent">{surface().objective.title}</span>
          <Show when={surface().blocker.label}>
            <span class="badge badge--warn">{surface().blocker.label}</span>
          </Show>
        </div>
      </div>
    </Show>
  );
}

function shouldShowFocusCinematic(renderState) {
  const surface = renderState?.commercial_surface;
  if (!surface) {
    return false;
  }
  const hasComparableFocusState = Boolean(
    renderState.selection
      || surface.active_agent_id
      || renderState.links?.length
      || renderState.fragment_terrain?.length
      || surface.blocker?.label
      || surface.action_receipt?.present,
  );
  return !hasComparableFocusState;
}

function PixelWorldFocusRail(props) {
  const surface = () => props.renderState().commercial_surface;
  const selected = () => props.renderState().selection;
  const activeAgent = () => surface()?.active_agent_id || props.renderState().agents[0]?.id || null;
  const routeCount = () => props.renderState().links.length;
  const hasFocusItems = () => Boolean(activeAgent() || selected() || routeCount() > 0);
  return (
    <Show when={surface() && hasFocusItems()}>
      <div class="pixel-world-focus-rail" data-focus-rail="true">
        <div class="pixel-world-focus-rail__label">
          {tr(props.locale(), "焦点", "Focus")}
        </div>
        <Show when={surface()?.blocker.label}>
          <div class="pixel-world-focus-rail__item pixel-world-focus-rail__item--blocker" data-focus-priority="blocker">
            <span>{tr(props.locale(), "阻塞", "Blocker")}</span>
            <strong>{surface().blocker.label}</strong>
          </div>
        </Show>
        <Show when={activeAgent()}>
          <div class="pixel-world-focus-rail__item">
            <span>{tr(props.locale(), "Agent", "Agent")}</span>
            <strong>{activeAgent()}</strong>
          </div>
        </Show>
        <Show when={selected()}>
          <div class="pixel-world-focus-rail__item">
            <span>{tr(props.locale(), "选中", "Selected")}</span>
            <strong>{`${selected().kind}/${selected().id}`}</strong>
          </div>
        </Show>
        <Show when={routeCount() > 0}>
          <div class="pixel-world-focus-rail__item">
            <span>{tr(props.locale(), "路线", "Routes")}</span>
            <strong>{routeCount()}</strong>
          </div>
        </Show>
      </div>
    </Show>
  );
}

function PixelWorldFocusMinimapCard(props) {
  const surface = () => props.renderState().commercial_surface;
  const selected = () => props.renderState().selection;
  const activeAgent = () => surface()?.active_agent_id || props.renderState().agents[0]?.id || null;
  const primaryLocation = () => props.renderState().locations[0] || null;
  return (
    <Show when={surface()}>
      <div
        class="pixel-world-focus-minimap"
        data-focus-minimap="true"
      >
        <div class="pixel-world-focus-minimap__label">
          {tr(props.locale(), "任务地图", "Mission Map")}
        </div>
        <Show when={primaryLocation()}>
          <span class="sr-only">
            {`${tr(props.locale(), "参照", "Reference")}: ${primaryLocation().label || primaryLocation().id}`}
          </span>
        </Show>
        <div class="pixel-world-focus-minimap__grid" />
        <div class="pixel-world-focus-minimap__route" data-routes={props.renderState().links.length} />
        <div class="pixel-world-focus-minimap__node pixel-world-focus-minimap__node--target">
          <span>{tr(props.locale(), "目标", "Target")}</span>
          <strong>{surface().next_action.label}</strong>
        </div>
        <div class="pixel-world-focus-minimap__node pixel-world-focus-minimap__node--agent">
          <span>{tr(props.locale(), "Agent", "Agent")}</span>
          <strong>{activeAgent() || tr(props.locale(), "待分配", "Unassigned")}</strong>
        </div>
        <Show when={selected()}>
          <div
            class="pixel-world-focus-minimap__node pixel-world-focus-minimap__node--selected"
            data-selected="true"
          >
            <span>{tr(props.locale(), "选中", "Selected")}</span>
            <strong>{`${selected().kind}/${selected().id}`}</strong>
          </div>
        </Show>
        <div class="pixel-world-focus-minimap__meta" aria-label={tr(props.locale(), "世界摘要", "World summary")}>
          <span>{`agents=${props.renderState().agents.length}`}</span>
          <span>{`targets=${props.renderState().locations.length}`}</span>
          <span>{`routes=${props.renderState().links.length}`}</span>
          <span>{`fragments=${props.renderState().fragment_terrain.length}`}</span>
        </div>
      </div>
    </Show>
  );
}

function chatEntryTitle(entry, locale) {
  if (entry.source === "error") {
    return `${entry.targetAgentId || entry.agentId || "agent"} ${tr(locale, "回复失败", "reply failed")}`;
  }
  if (entry.source === "player") {
    return `${tr(locale, "玩家", "Player")} -> ${entry.targetAgentId || entry.agentId || "agent"}`;
  }
  return `${entry.agentId || "agent"} ${tr(locale, "已发言", "spoke")}`;
}

function chatEntryCardClass(entry) {
  if (entry.source === "error") return "event-card event-card--chat-error";
  if (entry.source === "player") return "event-card event-card--chat-player";
  return "event-card event-card--chat-agent";
}

function chatEntryMeta(entry, locale) {
  if (entry.source === "error") {
    const code = entry.code ? ` · code=${entry.code}` : "";
    return `${entry.speaker || "runtime"}${code} · tick=${Number(entry.tick || 0)}`;
  }
  const speaker = entry.speaker || entry.playerId || tr(locale, "未知发言者", "unknown speaker");
  const location = entry.locationId || tr(locale, "未知地点", "unknown location");
  return `${speaker} · ${location} · tick=${Number(entry.tick || 0)}`;
}

function PixelRawDiagnostics(props) {
  const locale = () => props.locale();
  const [open, setOpen] = createSignal(false);
  const value = () => (typeof props.value === "function" ? props.value() : props.value);
  return (
    <details class="diagnostic" onToggle={(event) => setOpen(event.currentTarget.open)}>
      <summary>{tr(locale(), "原始诊断", "Raw diagnostics")}</summary>
      <Show when={open()}>
        <pre class="json">{JSON.stringify(value(), null, 2)}</pre>
      </Show>
    </details>
  );
}

function PixelWorldFocusCommandSurface(props) {
  const locale = () => props.locale();
  const agentId = () => {
    const id = String(core.selectedAgentId() || "").trim();
    return id && core.isAgentVisibleToCurrentSession(id) ? id : null;
  };
  const authSurface = () => core.buildAuthSurfaceModel();
  const chatCapability = () => authSurface().capabilities.agent_chat;
  const binding = () => core.selectedAgentBindingInfo();
  const chatFeedback = () => core.snapshotSemanticFeedback(core.state.lastChatFeedback);
  const chatFeedbackDisplay = () => core.describeSemanticFeedback(chatFeedback(), locale());
  const chatControlsEnabled = () => chatCapability().enabled && !core.isAgentChatInFlight();
  const gameplaySummary = () => core.buildGameplaySummary(locale());
  const blockerLabel = () =>
    gameplaySummary()?.blockerLabel || gameplaySummary()?.blockerKind || tr(locale(), "无阻塞", "No blocker");
  const receiptLabel = () =>
    gameplaySummary()?.executionStateLabel
      || gameplaySummary()?.recentFeedback?.stage
      || tr(locale(), "等待回执", "Waiting");
  const chatHistory = () =>
    core.state.chatHistory
      .filter((entry) => entry.agentId === agentId() || entry.targetAgentId === agentId())
      .slice(0, 12);

  return (
    <div id="viewer-command-console" class="pixel-world-focus-command-surface stack">
      <Show
        when={agentId()}
        fallback={<div class="empty">{tr(locale(), "先选中一个行动体，才能在电影视图里直接下指令。", "Select an agent to issue direct commands in Cinematic View.")}</div>}
      >
        <div class="badge-row">
          <span class="badge badge--accent">{tr(locale(), "当前交互目标", "Current Target")}</span>
          <span class="badge">{`agent=${agentId()}`}</span>
          <Show when={binding()?.playerId}>
            <span class="badge">{`boundPlayer=${binding().playerId}`}</span>
          </Show>
          <span class={chatCapability().enabled ? "badge badge--good" : "badge badge--warn"}>
            {chatCapability().enabled ? tr(locale(), "聊天可用", "Chat Ready") : tr(locale(), "聊天受限", "Chat Limited")}
          </span>
        </div>
        <div class="pixel-world-focus-command-tray" data-chat-ready={chatControlsEnabled() ? "true" : "false"}>
          <div class="pixel-world-focus-command-chip pixel-world-focus-command-chip--target">
            <span>{tr(locale(), "目标", "Target")}</span>
            <strong>{`agent=${agentId()}`}</strong>
          </div>
          <div class="pixel-world-focus-command-chip pixel-world-focus-command-chip--blocker" data-blocker-present={blockerLabel() !== tr(locale(), "无阻塞", "No blocker") ? "true" : "false"}>
            <span>{tr(locale(), "阻塞", "Blocker")}</span>
            <strong>{blockerLabel()}</strong>
          </div>
          <div class="pixel-world-focus-command-chip pixel-world-focus-command-chip--receipt">
            <span>{tr(locale(), "回执", "Receipt")}</span>
            <strong>{receiptLabel()}</strong>
          </div>
          <button
            type="button"
            class="pixel-world-focus-command-chip pixel-world-focus-command-chip--primary"
            data-chat-send="1"
            disabled={!chatControlsEnabled()}
            onClick={() => core.sendAgentChat(agentId(), core.state.chatDraft.message)}
          >
            {tr(locale(), "发送聊天", "Send Chat")}
          </button>
        </div>
        <Show when={!chatCapability().enabled}>
          <div class="empty">{chatCapability().reason}</div>
        </Show>
        <div class="panel panel--nested">
          <div class="panel__header">
            <div class="stack stack--compact">
              <div class="panel__eyebrow">{tr(locale(), "指挥面板", "Command Surface")}</div>
              <div class="panel__title">{tr(locale(), "行动体聊天", "Agent Chat")}</div>
              <div class="panel__meta-copy">
                {tr(
                  locale(),
                  "给当前目标发消息并读取反馈。",
                  "Message the current target and read feedback.",
                )}
              </div>
            </div>
          </div>
          <div class="panel__body stack">
            <div class="field">
              <label for="agent-chat-message">{tr(locale(), "消息", "Message")}</label>
              <textarea
                id="agent-chat-message"
                rows="2"
                placeholder={tr(locale(), "给当前选中的行动体发一条消息", "Send a message to the selected agent")}
                disabled={!chatControlsEnabled()}
                value={core.state.chatDraft.message}
                onInput={(event) => {
                  core.state.chatDraft.message = String(event.currentTarget.value || "");
                  core.state.chatDraft.dirty = true;
                }}
              />
            </div>
            <div class="toolbar">
              <button
                type="button"
                data-chat-send="1"
                disabled={!chatControlsEnabled()}
                onClick={() => core.sendAgentChat(agentId(), core.state.chatDraft.message)}
              >
                {tr(locale(), "发送聊天", "Send Chat")}
              </button>
            </div>
            <Show when={chatFeedback()} fallback={<div class="empty">{tr(locale(), "还没有聊天反馈。", "No chat feedback yet.")}</div>}>
              {(feedback) => (
                <div class="feedback-card">
                  <div class="badge-row">
                    <span class={chatFeedbackDisplay().badgeClass}>{chatFeedbackDisplay().label}</span>
                    <Show when={chatFeedbackDisplay().code}>
                      <span class="badge">{`code=${chatFeedbackDisplay().code}`}</span>
                    </Show>
                  </div>
                  <div class="feedback-summary">{chatFeedbackDisplay().summary}</div>
                  <Show when={chatFeedbackDisplay().detail}>
                    <div class="feedback-detail">{chatFeedbackDisplay().detail}</div>
                  </Show>
                  <PixelRawDiagnostics locale={locale} value={feedback} />
                </div>
              )}
            </Show>
            <div>
              <div class="panel__title panel__title--spaced">{tr(locale(), "消息流", "Message Flow")}</div>
              <div class="event-list">
                <Show when={chatHistory().length > 0} fallback={<div class="empty">{tr(locale(), "这个行动体还没有聊天历史。", "No chat history for this agent yet.")}</div>}>
                  <For each={chatHistory()}>
                    {(entry) => (
                      <div class={chatEntryCardClass(entry)}>
                        <div class="event-card__title">
                          <span>{chatEntryTitle(entry, locale())}</span>
                        </div>
                        <div class="event-card__meta">{chatEntryMeta(entry, locale())}</div>
                        <div class="feedback-summary">{entry.message || tr(locale(), "没有消息正文。", "No message body.")}</div>
                        <PixelRawDiagnostics locale={locale} value={entry} />
                      </div>
                    )}
                  </For>
                </Show>
              </div>
            </div>
          </div>
        </div>
      </Show>
    </div>
  );
}

export function PixelWorldHost(props) {
  const locale = () => props.locale ?? core.state.uiLocale;
  const visualFixtureName = installPixelWorldVisualFixtureHook();
  const [coreRevision, setCoreRevision] = createSignal(0);
  const selectedEntity = () => {
    coreRevision();
    return core.state.selectedKind && core.state.selectedId
      ? { kind: core.state.selectedKind, id: core.state.selectedId }
      : null;
  };
  const renderInput = createMemo(() => {
    coreRevision();
    return buildPixelWorldRenderInput(locale());
  });
  const [rustRenderState, setRustRenderState] = createSignal(null);
  const renderState = () => rustRenderState();
  const visualState = () => pixelWorldVisualState(renderState());
  const [rendererStatus, setRendererStatus] = createSignal("booting");
  const [rendererFatal, setRendererFatal] = createSignal(null);
  const [hoverSelection, setHoverSelection] = createSignal(null);
  const [runtimeSource, setRuntimeSource] = createSignal("loading");
  const [cameraState, setCameraState] = createSignal(null);
  const [renderDtoOpen, setRenderDtoOpen] = createSignal(false);
  const [focusMode, setFocusMode] = createSignal(pixelWorldFocusUiSessionState.focusMode);
  const [commandDrawerOpen, setCommandDrawerOpen] = createSignal(pixelWorldFocusUiSessionState.commandDrawerOpen);
  const [diagnosticsDrawerOpen, setDiagnosticsDrawerOpen] = createSignal(pixelWorldFocusUiSessionState.diagnosticsDrawerOpen);
  const [maximized, setMaximized] = createSignal(pixelWorldFocusUiSessionState.maximized);
  installPixelWorldRenderDtoProbe(visualFixtureName, renderState, onCleanup);
  const visualOverlayEnabled = () => Boolean(
    visualFixtureName
      || document.body?.getAttribute("data-viewer-visual-fixture"),
  );
  const hoveredHotspot = () => {
    const hover = hoverSelection();
    if (hover?.kind !== "hotspot") {
      return null;
    }
    return visualState().visualHotspots.find((hotspot) => hotspot.id === hover.id) || null;
  };

  function setPersistentFocusMode(next) {
    pixelWorldFocusUiSessionState.focusMode = next;
    setFocusMode(next);
  }

  function setPersistentCommandDrawerOpen(next) {
    pixelWorldFocusUiSessionState.commandDrawerOpen = next;
    setCommandDrawerOpen(next);
  }

  function setPersistentDiagnosticsDrawerOpen(next) {
    pixelWorldFocusUiSessionState.diagnosticsDrawerOpen = next;
    setDiagnosticsDrawerOpen(next);
  }

  function setPersistentMaximized(next) {
    pixelWorldFocusUiSessionState.maximized = next;
    setMaximized(next);
  }

  function toggleMaximized() {
    setPersistentMaximized(!maximized());
  }

  const focusController = createPixelWorldFocusController({
    focusMode,
    commandDrawerOpen,
    diagnosticsDrawerOpen,
    setFocusMode: setPersistentFocusMode,
    setCommandDrawerOpen: setPersistentCommandDrawerOpen,
    setDiagnosticsDrawerOpen: setPersistentDiagnosticsDrawerOpen,
    setMaximized: setPersistentMaximized,
  });

  const adapter = createMemo(() => createPixelWorldHostAdapter({
    onSelectEntity(selection) {
      core.applySelection(selection);
      setCoreRevision((revision) => revision + 1);
      applyRendererUpdate();
    },
    onHoverEntity(selection) {
      setHoverSelection(selection);
    },
    onFatal(fatal, nextCameraState) {
      if (nextCameraState) {
        setCameraState(nextCameraState);
        core.updatePixelWorldRuntimeMeta({
          runtimeStatus: rendererStatus(),
          runtimeSource: runtimeSource(),
          runtimeModuleUrl: adapter().runtimeModuleUrl(),
          camera: nextCameraState,
          fatal: rendererFatal(),
        });
        return;
      }
      setRendererFatal(fatal);
      setRendererStatus("unavailable");
      setRuntimeSource(fatal?.code === "pixel_world_webgl2_unavailable" ? "surface_unavailable" : runtimeSource());
      setRustRenderState(null);
      core.updatePixelWorldRuntimeMeta({
        runtimeStatus: "unavailable",
        runtimeSource: runtimeSource(),
        runtimeModuleUrl: adapter().runtimeModuleUrl(),
        camera: cameraState(),
        fatal,
      });
      core.reportFatalError(fatal.message, "pixel_world_host");
    },
  }));

  let mountedCanvas = null;

  function applyRendererUpdate() {
    if (rendererStatus() === "unavailable") {
      return;
    }
    const result = adapter().update(renderInput());
    if (result?.fatal) {
      setRendererFatal(result.fatal);
    }
    setRustRenderState(result?.renderState || null);
    setRendererStatus(result?.status || "ready");
    setRuntimeSource(result?.runtimeSource || adapter().runtimeSource());
    core.updatePixelWorldRuntimeMeta({
      runtimeStatus: result?.status || "ready",
      runtimeSource: result?.runtimeSource || adapter().runtimeSource(),
      runtimeModuleUrl: result?.runtimeModuleUrl || adapter().runtimeModuleUrl(),
      camera: cameraState(),
      fatal: result?.fatal || rendererFatal(),
    });
  }

  async function setReadyMode() {
    if (!mountedCanvas) {
      const fatal = {
        code: "pixel_world_renderer_mount_missing_canvas",
        message: "pixel world canvas is not mounted yet",
      };
      setRendererFatal(fatal);
      setRendererStatus("unavailable");
      setRuntimeSource("detached");
      setRustRenderState(null);
      core.updatePixelWorldRuntimeMeta({
        runtimeStatus: "unavailable",
        runtimeSource: "detached",
        runtimeModuleUrl: null,
        camera: null,
        fatal,
      });
      return;
    }
    setRendererFatal(null);
    setRendererStatus("booting");
    setRuntimeSource("loading");
    const attached = await waitForRuntimeCanvasAttachment(mountedCanvas);
    if (!attached) {
      const fatal = {
        code: "pixel_world_renderer_canvas_detached",
        message: "pixel world runtime canvas never became queryable in document",
      };
      setRendererFatal(fatal);
      setRendererStatus("unavailable");
      setRuntimeSource("detached");
      setRustRenderState(null);
      core.updatePixelWorldRuntimeMeta({
        runtimeStatus: "unavailable",
        runtimeSource: "detached",
        runtimeModuleUrl: null,
        camera: cameraState(),
        fatal,
      });
      return;
    }
    const surfaceFatal = probePixelWorldWebgl2Surface(mountedCanvas);
    if (surfaceFatal) {
      adapter().simulateFatal(surfaceFatal);
      return;
    }
    const result = await adapter().mount(mountedCanvas, renderInput());
    if (result?.fatal) {
      setRendererFatal(result.fatal);
    }
    setRustRenderState(result?.renderState || null);
    setRendererStatus(result?.status || "ready");
    setRuntimeSource(result?.runtimeSource || adapter().runtimeSource());
    core.updatePixelWorldRuntimeMeta({
      runtimeStatus: result?.status || "ready",
      runtimeSource: result?.runtimeSource || adapter().runtimeSource(),
      runtimeModuleUrl: result?.runtimeModuleUrl || adapter().runtimeModuleUrl(),
      camera: cameraState(),
      fatal: result?.fatal || null,
    });
  }

  function requestReadyMode() {
    setRendererFatal(null);
    setRendererStatus("booting");
    setRuntimeSource("loading");
    if (mountedCanvas) {
      void setReadyMode();
    }
  }

  function simulateFatal() {
    adapter().simulateFatal("simulated embedded renderer fatal");
  }

  onMount(() => {
    function handleKeyDown(event) {
      focusController.handleKeyDown(event);
    }

    window.addEventListener("keydown", handleKeyDown);
    onCleanup(() => window.removeEventListener("keydown", handleKeyDown));

    if (pixelWorldTestApiEnabled()) {
      core.setRenderHook(() => {
        setCoreRevision((revision) => revision + 1);
        applyRendererUpdate();
      });
      onCleanup(() => core.setRenderHook(null));
      onCleanup(installPixelWorldHotspotPointerProbe({ fixtureName: visualFixtureName, getCanvas: () => mountedCanvas, getRendererStatus: rendererStatus, getHotspotHitTargets: () => adapter().hotspotTestHitTargets(), getLocationHitTargets: () => adapter().locationTestHitTargets(), getHoverSelection: hoverSelection, getHoveredHotspot: hoveredHotspot }));
    }
  });

  createEffect(() => {
    document.body.classList.toggle("pixel-world-focus-active", focusMode());
    document.body.classList.toggle("pixel-world-focus-maximized", focusMode() && maximized());
  });

  onCleanup(() => {
    document.body.classList.remove("pixel-world-focus-active");
    document.body.classList.remove("pixel-world-focus-maximized");
    adapter().unmount();
    core.updatePixelWorldRuntimeMeta({
      runtimeStatus: "detached",
      runtimeSource: "detached",
      runtimeModuleUrl: null,
      camera: null,
      fatal: null,
    });
  });

  return (
    <div
      class={`pixel-world-host stack ${focusMode() ? "pixel-world-host--focus" : ""} ${focusMode() && maximized() ? "pixel-world-host--focus-maximized" : ""}`}
      data-viewer-overlay="world-hud"
      data-world-focus={focusMode() ? "true" : "false"}
      data-world-focus-maximized={focusMode() && maximized() ? "true" : "false"}
      data-visual-fixture={visualFixtureName || ""}
      data-focus-comparable={shouldShowFocusCinematic(renderState()) ? "false" : "true"}
    >
      <Show when={!focusMode() || !maximized()}>
        <div class="pixel-world-host__summary">
          <div class="pixel-world-host__summary-copy">
            <div class="pixel-world-host__headline">
              {tr(locale(), "世界指挥棋盘", "World Command Board")}
            </div>
            <div class="feedback-detail">
              {renderState()?.commercial_surface?.objective?.detail
                || tr(locale(), "等待 Rust bridge 生成世界显示状态。", "Waiting for the Rust bridge to derive the world display state.")}
            </div>
          </div>
        </div>
      </Show>
      <div
        class="pixel-world-focus-entry"
        data-viewer-overlay="cinematic-entry"
        hidden={focusMode()}
      >
          <div id="pixel-world-focus-entry-hint" class="pixel-world-focus-entry__hint">
            {tr(locale(), "拖动、缩放并检查世界", "Pan, zoom, and inspect the world")}
          </div>
          <button
            type="button"
            class="pixel-world-focus-entry__button"
            disabled={!renderState()}
            onClick={focusController.enterFocusMode}
            aria-pressed="false"
            aria-describedby={rendererStatus() === "unavailable" ? PIXEL_WORLD_RENDERER_UNAVAILABLE_MESSAGE_ID : "pixel-world-focus-entry-hint"}
          >
            {rendererStatus() === "unavailable"
              ? tr(locale(), "电影视图（当前不可用）", "Cinematic View (unavailable)")
              : tr(locale(), "电影视图", "Cinematic View")}
          </button>
      </div>
      <Show when={focusMode() && renderState()}>
        <Show when={!maximized() && shouldShowFocusCinematic(renderState())}>
          <PixelWorldFocusCinematicBanner
            locale={locale}
            renderState={renderState}
          />
        </Show>
        <PixelWorldFocusHud
          locale={locale}
          renderState={renderState}
          onExit={focusController.exitFocusMode}
          onOpenCommand={focusController.openCommandDrawer}
          onOpenDiagnostics={focusController.openDiagnosticsDrawer}
          onToggleMaximized={toggleMaximized}
          maximized={maximized}
        />
        <Show when={!maximized()}>
          <PixelWorldFocusRail
            locale={locale}
            renderState={renderState}
          />
          <PixelWorldFocusMinimapCard
            locale={locale}
            renderState={renderState}
            variant="immersive"
          />
        </Show>
      </Show>
      <Show when={renderState()}>
        <PixelWorldCommercialHud locale={locale} renderState={renderState} focusMode={focusMode}/>
      </Show>
      <Show when={rendererStatus() !== "fallback" && rendererStatus() !== "unavailable"}>
        <PixelWorldCanvasRenderer
          locale={locale}
          rendererStatus={rendererStatus}
          renderInput={renderInput}
          renderState={renderState}
          selection={selectedEntity}
          visualOverlayEnabled={visualOverlayEnabled}
          onSelect={(selection) => adapter().simulateSelect(selection)}
          onHover={(selection) => adapter().simulateHover(selection)}
          hoveredHotspot={hoveredHotspot}
          onFatal={(message) => adapter().simulateFatal(message)}
          onCanvasMount={(canvas) => {
            mountedCanvas = canvas;
            if (rendererStatus() !== "ready") {
              void setReadyMode();
            }
          }}
          onCanvasUpdate={() => {
            if (rendererStatus() === "ready") {
              applyRendererUpdate();
            }
          }}
        />
      </Show>
      <Show when={!renderState()}>
        <div
          id={rendererStatus() === "unavailable" ? PIXEL_WORLD_RENDERER_UNAVAILABLE_MESSAGE_ID : undefined}
          class="empty pixel-world-render-unavailable"
          data-viewer-overlay="renderer-unavailable"
          data-renderer-state={rendererStatus()}
        >
          {rendererStatus() === "unavailable" ? <><div>{tr(locale(), "此浏览器中的图形不可用", "Graphics unavailable in this browser")}</div><button type="button" class="pixel-world-render-unavailable__retry" onClick={requestReadyMode}>{tr(locale(), "重试 Renderer", "Retry Renderer")}</button></> : tr(locale(), "Rust bridge 正在生成世界显示状态。", "Rust bridge is deriving the world display state.")}
        </div>
      </Show>
      <Show when={focusMode() && renderState()?.commercial_surface && !maximized()}>
        <div class="pixel-world-focus-receipt">
          <PixelWorldActionReceipt
            class="pixel-world-action-receipt--focus-compact"
            locale={locale}
            surface={() => renderState().commercial_surface}
          />
        </div>
      </Show>
      <Show when={focusMode()}>
        <details
          id="viewer-focus-command-drawer"
          class="pixel-world-focus-drawer pixel-world-focus-drawer--command"
          open={commandDrawerOpen()}
          onToggle={(event) => {
            const open = event.currentTarget.open;
            if (open) {
              setPersistentCommandDrawerOpen(true);
            } else {
              focusController.closeCommandDrawer({ returnFocus: true });
            }
          }}
        >
          <summary>{tr(locale(), "命令与目标", "Command and Target")}</summary>
          <div class="pixel-world-focus-drawer__body">
            <PixelWorldFocusCommandSurface locale={locale} />
          </div>
        </details>
      </Show>
      <Show when={!focusMode() || !maximized()}>
        <details class="diagnostic pixel-world-render-diagnostics" data-renderer-state={rendererStatus()}>
          <summary>{tr(locale(), "Renderer 诊断", "Renderer Diagnostics")}</summary>
          <div class="pixel-world-host__toolbar badge-row">
            <span class="badge badge--accent">{`locations=${visualState().locations.length}`}</span>
            <span class="badge badge--accent">{`fragments=${visualState().fragmentTerrain.length}`}</span>
            <span class="badge badge--accent">{`agents=${visualState().agents.length}`}</span>
            <Show when={renderState()?.world_tick !== null && renderState()?.world_tick !== undefined}>
              <span class="badge badge--accent" data-world-tick={String(renderState()?.world_tick)}>{`tick=${renderState()?.world_tick}`}</span>
            </Show>
            <span class="badge">{`links=${visualState().links.length}`}</span>
            <span class="badge">{`hotspots=${arrayField(renderState(), "visual_hotspots", "visualHotspots").length}`}</span>
            <span class="badge">{`derived_positions=${visualState().agents.filter((agent) => agent.position_source === "location_derived").length}`}</span>
            <span class="badge">{visualState().worldBounds ? "world_bounds=ready" : "world_bounds=missing"}</span>
            <span class="badge">{`renderer=${rendererStatus()}`}</span>
            <span class="badge">{`runtime=${runtimeSource()}`}</span>
            <Show when={cameraState()}>
              <span class="badge">{`zoom=${cameraState().zoom.toFixed(2)}`}</span>
            </Show>
            <Show when={cameraState()}>
              <span class="badge">{`pan=${cameraState().pan_x_px},${cameraState().pan_y_px}`}</span>
            </Show>
            <Show when={hoverSelection()}>
              <span class="badge">{`hover=${hoverSelection().kind}/${hoverSelection().id}`}</span>
            </Show>
            <button type="button" onClick={requestReadyMode}>
              {tr(locale(), "重新挂载嵌入式 Renderer", "Reattach Embedded Renderer")}
            </button>
            <button type="button" onClick={simulateFatal}>
              {tr(locale(), "模拟 Renderer Fatal", "Simulate Renderer Fatal")}
            </button>
            <div class="feedback-detail">
              {tr(
                locale(),
                "当前世界舞台只依赖 wasm/Rust bridge、嵌入式 canvas、轻量拖拽缩放和事件回传。",
                "The world stage depends only on the wasm/Rust bridge, embedded canvas, light pan-zoom interaction, and event callbacks.",
              )}
            </div>
            <Show when={rendererStatus() === "unavailable" && rendererFatal()}>
              <div class="feedback-detail" data-renderer-fatal>{`${rendererFatal().code}: ${rendererFatal().message}`}</div>
            </Show>
          </div>
        </details>
      </Show>
      <Show when={focusMode() && renderState()}>
        <details
          id="viewer-focus-diagnostics-drawer"
          class="pixel-world-focus-drawer pixel-world-focus-drawer--diagnostics"
          open={diagnosticsDrawerOpen()}
          onToggle={(event) => {
            const open = event.currentTarget.open;
            if (open) {
              setPersistentDiagnosticsDrawerOpen(true);
            } else {
              focusController.closeDiagnosticsDrawer({ returnFocus: true });
            }
          }}
        >
          <summary>{tr(locale(), "电影视图诊断", "Cinematic Diagnostics")}</summary>
          <div class="pixel-world-focus-drawer__body">
            <div class="badge-row">
              <Show when={renderState().world_tick !== null && renderState().world_tick !== undefined}>
                <span class="badge badge--accent" data-world-tick={String(renderState().world_tick)}>{`tick=${renderState().world_tick}`}</span>
              </Show>
              <span class="badge">{`renderer=${rendererStatus()}`}</span>
              <span class="badge">{`runtime=${runtimeSource()}`}</span>
              <span class="badge">{`derived_positions=${renderState().agents.filter((agent) => agent.position_source === "location_derived").length}`}</span>
              <Show when={rendererFatal()}>
                <span class="badge badge--warn">{rendererFatal().code}</span>
              </Show>
            </div>
            <div class="toolbar toolbar--spaced">
              <button type="button" onClick={requestReadyMode}>
                {tr(locale(), "重新挂载嵌入式 Renderer", "Reattach Embedded Renderer")}
              </button>
            </div>
          </div>
        </details>
      </Show>
      <details
        class="diagnostic"
        onToggle={(event) => setRenderDtoOpen(event.currentTarget.open)}
      >
        <summary>{tr(locale(), "展开 Render DTO", "Expand Render DTO")}</summary>
        <Show when={renderDtoOpen()}>
          <div class="stack flow-top">
            <pre class="json">{JSON.stringify(renderState(), null, 2)}</pre>
          </div>
        </Show>
      </details>
    </div>
  );
}
