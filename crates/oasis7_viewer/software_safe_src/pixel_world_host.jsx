import { createEffect, createMemo, createSignal, For, Show, onCleanup } from "solid-js";

import * as core from "./legacy_core.js";
import { createPixelWorldRuntimeBridge } from "./pixel_world_runtime_loader.js";

function tr(locale, zh, en) {
  return core.isLocaleZh(locale) ? zh : en;
}

const PIXEL_WORLD_RUNTIME_CANVAS_ID = "pixel-world-embedded-runtime-canvas";
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

function normalizePosition(pos) {
  if (!pos || typeof pos !== "object") {
    return null;
  }
  const x = Number(pos.x_cm);
  const y = Number(pos.y_cm);
  const z = Number(pos.z_cm);
  if (!Number.isFinite(x) || !Number.isFinite(y) || !Number.isFinite(z)) {
    return null;
  }
  return { x_cm: x, y_cm: y, z_cm: z };
}

function buildRecentEventHotspots(events) {
  if (!Array.isArray(events)) {
    return [];
  }
  return events
    .slice(0, 4)
    .map((event, index) => ({
      id: event?.eventId || event?.event_id || `recent-${index}`,
      title: event?.title || event?.summary || event?.kind || `event-${index}`,
      kind: event?.kind || "recent_event",
    }));
}

function countResourceEntries(summary) {
  if (!summary || summary === "-") {
    return 0;
  }
  return String(summary)
    .split(" · ")
    .map((entry) => entry.trim())
    .filter(Boolean)
    .length;
}

function safeNumber(value, fallback = 0) {
  const number = Number(value);
  return Number.isFinite(number) ? number : fallback;
}

function worldCenterPosition(worldBounds) {
  if (!worldBounds) {
    return null;
  }
  return {
    x_cm: worldBounds.width_cm / 2,
    y_cm: worldBounds.depth_cm / 2,
    z_cm: worldBounds.height_cm / 2,
  };
}

function clampWorldPosition(pos, worldBounds) {
  if (!pos || !worldBounds) {
    return null;
  }
  return {
    x_cm: Math.min(worldBounds.width_cm, Math.max(0, Number(pos.x_cm) || 0)),
    y_cm: Math.min(worldBounds.depth_cm, Math.max(0, Number(pos.y_cm) || 0)),
    z_cm: Math.min(worldBounds.height_cm, Math.max(0, Number(pos.z_cm) || 0)),
  };
}

function dominantCompound(block) {
  const ppm = block?.compounds?.ppm;
  if (!ppm || typeof ppm !== "object") {
    return "unknown";
  }
  const ranked = Object.entries(ppm)
    .map(([kind, value]) => [kind, safeNumber(value, 0)])
    .filter(([, value]) => value > 0)
    .sort((left, right) => right[1] - left[1] || left[0].localeCompare(right[0]));
  return ranked[0]?.[0] || "unknown";
}

function fragmentTerrainColor(compound) {
  return FRAGMENT_TERRAIN_PALETTE[compound] || FRAGMENT_TERRAIN_PALETTE.unknown;
}

function colorToCss(color, alpha = 0.36) {
  const [red, green, blue] = Array.isArray(color) ? color : FRAGMENT_TERRAIN_PALETTE.unknown;
  return `rgba(${red}, ${green}, ${blue}, ${alpha})`;
}

function fragmentBlocks(location) {
  const blocks = location?.fragment_profile?.blocks?.blocks;
  return Array.isArray(blocks) ? blocks : [];
}

function estimateFragmentHalfExtentCm(location, blocks) {
  const explicitRadius = safeNumber(location?.profile?.radius_cm, 0);
  if (explicitRadius > 0) {
    return explicitRadius;
  }
  const maxExtent = blocks.reduce((value, block) => {
    const originX = safeNumber(block?.origin_cm?.x_cm, 0);
    const originZ = safeNumber(block?.origin_cm?.z_cm ?? block?.origin_cm?.y_cm, 0);
    const sizeX = safeNumber(block?.size_cm?.x_cm, 0);
    const sizeZ = safeNumber(block?.size_cm?.z_cm ?? block?.size_cm?.y_cm, 0);
    return Math.max(value, originX + sizeX, originZ + sizeZ);
  }, 0);
  return Math.max(1, maxExtent / 2);
}

function buildFragmentTerrainForLocation(location, worldBounds) {
  const pos = normalizePosition(location?.pos);
  const blocks = fragmentBlocks(location);
  if (!pos || !worldBounds || !blocks.length) {
    return [];
  }

  const halfExtentCm = estimateFragmentHalfExtentCm(location, blocks);
  return blocks
    .map((block, index) => {
      const sizeX = safeNumber(block?.size_cm?.x_cm, 0);
      const sizeZ = safeNumber(block?.size_cm?.z_cm ?? block?.size_cm?.y_cm, 0);
      const originX = safeNumber(block?.origin_cm?.x_cm, 0);
      const originZ = safeNumber(block?.origin_cm?.z_cm ?? block?.origin_cm?.y_cm, 0);
      const footprintCm = Math.max(1, sizeX, sizeZ);
      if (sizeX <= 0 || sizeZ <= 0) {
        return null;
      }
      const dominant = dominantCompound(block);
      const localX = originX + (sizeX / 2) - halfExtentCm;
      const localY = originZ + (sizeZ / 2) - halfExtentCm;
      return {
        id: `fragment:${location.id}:${index}`,
        location_id: location.id,
        pos: clampWorldPosition({
          x_cm: pos.x_cm + localX,
          y_cm: pos.y_cm + localY,
          z_cm: pos.z_cm,
        }, worldBounds),
        footprint_cm: footprintCm,
        dominant_compound: dominant,
        color: fragmentTerrainColor(dominant),
        emphasis: 0.58,
      };
    })
    .filter((entry) => entry?.pos);
}

function deterministicHash(input) {
  return String(input || "").split("").reduce((hash, char) => (
    ((hash * 31) + char.charCodeAt(0)) >>> 0
  ), 2166136261);
}

function clampRatio(value) {
  return Math.min(1, Math.max(0, Number(value) || 0));
}

function offsetWorldPosition(anchor, worldBounds, xRatio, yRatio) {
  if (!worldBounds) {
    return null;
  }
  const base = anchor || worldCenterPosition(worldBounds);
  if (!base) {
    return null;
  }
  return clampWorldPosition({
    x_cm: base.x_cm + (worldBounds.width_cm * xRatio),
    y_cm: base.y_cm + (worldBounds.depth_cm * yRatio),
    z_cm: base.z_cm || 0,
  }, worldBounds);
}

function deriveAgentPosition(agent, locationById, worldBounds) {
  if (!agent?.location_id || !worldBounds || !locationById.has(agent.location_id)) {
    return null;
  }
  const location = locationById.get(agent.location_id);
  if (!location?.pos) {
    return null;
  }
  const hash = deterministicHash(`${agent.id}:${agent.location_id}`);
  const angle = ((hash % 360) * Math.PI) / 180;
  const radiusCm = Math.max(
    10_000,
    Math.min(
      Math.max(worldBounds.width_cm, worldBounds.depth_cm) * 0.015,
      Number(location.radius_cm) || 35_000,
    ),
  );
  return clampWorldPosition({
    x_cm: location.pos.x_cm + (Math.cos(angle) * radiusCm),
    y_cm: location.pos.y_cm + (Math.sin(angle) * radiusCm),
    z_cm: location.pos.z_cm || 0,
  }, worldBounds);
}

function resolveAgentPosition(agent, selected, locationById, worldBounds) {
  const snapshotPosition = normalizePosition(agent.pos || (selected?.id === agent.id ? selected?.pos : null));
  if (snapshotPosition) {
    return {
      pos: snapshotPosition,
      position_source: "snapshot",
    };
  }
  const derivedPosition = deriveAgentPosition(agent, locationById, worldBounds);
  if (derivedPosition) {
    return {
      pos: derivedPosition,
      position_source: "location_derived",
    };
  }
  return {
    pos: null,
    position_source: "missing",
  };
}

function resolveSelectionPosition(selection, agents, locations) {
  if (!selection) {
    return null;
  }
  if (selection.kind === "agent") {
    return agents.find((agent) => agent.id === selection.id)?.pos || null;
  }
  if (selection.kind === "location") {
    return locations.find((location) => location.id === selection.id)?.pos || null;
  }
  return null;
}

function buildPixelWorldLinks(agents, locationById) {
  return agents
    .filter((agent) => agent.location_id && agent.pos && locationById.has(agent.location_id))
    .map((agent) => ({
      id: `link:${agent.id}:${agent.location_id}`,
      kind: "agent_assignment",
      from: agent.pos,
      to: locationById.get(agent.location_id).pos,
      emphasis: 0.72,
    }));
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
  const sizePx = Math.max(7, Math.min(26, safeNumber(patch.footprint_cm, 1) / 1200));
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

function pickKnownAgentId(candidateIds, agents) {
  const knownAgentIds = new Set(agents.map((agent) => agent.id));
  return candidateIds.find((id) => id && knownAgentIds.has(id)) || null;
}

function actionReceiptTitle(locale, state, present) {
  if (!present) {
    return tr(locale, "暂无行动回执", "No action receipt yet");
  }
  switch (state) {
    case "accepted":
      return tr(locale, "行动已接受", "Action accepted");
    case "blocked":
      return tr(locale, "行动被阻塞", "Action blocked");
    case "completed":
      return tr(locale, "世界已改变", "World changed");
    case "rejected":
      return tr(locale, "行动被拒绝", "Action rejected");
    default:
      return tr(locale, "行动进行中", "Action in progress");
  }
}

function buildActionReceipt({ locale, gameplay, activeAgentId }) {
  const recentFeedback = gameplay?.recentFeedback;
  const hasWorldDelta = Boolean(gameplay?.lastWorldChange || recentFeedback?.effect);
  const hasPlayerIntent = Boolean(
    gameplay?.acceptedIntentId
    || gameplay?.acceptedIntentScope
    || gameplay?.acceptedIntentTarget
    || recentFeedback?.action,
  );
  const present = hasWorldDelta || hasPlayerIntent || Boolean(recentFeedback?.reason);
  const rawState = gameplay?.executionState || recentFeedback?.stage || "waiting_for_intent";
  const state = present ? rawState : "waiting_for_intent";
  const confidence = hasWorldDelta
    ? "world_delta"
    : hasPlayerIntent
      ? "accepted_intent"
      : "none";
  const summary = present
    ? gameplay?.lastWorldChange
      || recentFeedback?.effect
      || gameplay?.acceptedIntentSummary
      || recentFeedback?.action
      || gameplay?.executionSummary
    : tr(
      locale,
      "还没有一条玩家行动产生可确认的世界变化。",
      "No player-caused world change has been confirmed yet.",
    );
  const detail = present
    ? gameplay?.executionCauseDetail
      || recentFeedback?.reason
      || recentFeedback?.hint
      || gameplay?.acceptedIntentDetail
      || gameplay?.progressDetail
      || null
    : tr(
      locale,
      "先提交玩法动作或推进世界，再查看系统确认、阻塞或完成的回执。",
      "Submit a gameplay action or advance the world, then read whether the system accepted, blocked, or completed it.",
    );

  return {
    present,
    state,
    confidence,
    title: actionReceiptTitle(locale, state, present),
    summary,
    detail,
    target_agent_id: present
      ? gameplay?.acceptedIntentTarget
        || gameplay?.recommendedAction?.targetAgentId
        || activeAgentId
        || null
      : null,
    effect_kind: present ? gameplay?.executionCauseKind || recentFeedback?.stage || null : null,
    delta_logical_time: present ? recentFeedback?.deltaLogicalTime ?? null : null,
    delta_event_seq: present ? recentFeedback?.deltaEventSeq ?? null : null,
  };
}

function buildCommercialSurface({
  locale,
  gameplay,
  agents,
  links,
  fragmentTerrain,
  visualHotspots,
  selection,
}) {
  const activeAgentId = pickKnownAgentId([
    gameplay?.recommendedAction?.targetAgentId,
    gameplay?.acceptedIntentTarget,
    selection?.kind === "agent" ? selection.id : null,
    agents[0]?.id,
  ], agents);
  const objectiveTitle = gameplay?.goalTitle
    || tr(locale, "进入世界，建立第一条能力链", "Enter the world and build the first capability chain");
  const objectiveDetail = gameplay?.objective
    || gameplay?.progressDetail
    || tr(locale, "先让 Agent、路线和资源关系变得可读，再推进下一步。", "Read the agent, route, and resource relationship before pushing the next move.");
  const nextActionLabel = gameplay?.recommendedAction?.label
    || gameplay?.nextStepHint
    || gameplay?.narrativeNextStep
    || tr(locale, "选择一个 Agent 或推进世界一步", "Select an agent or advance the world one step");
  const nextActionDetail = gameplay?.recommendedAction?.disabledReason
    || gameplay?.nextStepHint
    || gameplay?.executionSummary
    || null;
  const leverageSummary = gameplay?.acceptedIntentSummary
    || gameplay?.lastWorldChange
    || tr(locale, "还没有一条被正式接受的玩家意图", "No player-facing accepted intent yet");
  const leverageDetail = gameplay?.lastWorldChange
    || gameplay?.executionCauseDetail
    || gameplay?.acceptedIntentDetail
    || gameplay?.progressDetail
    || null;
  const actionReceipt = buildActionReceipt({
    locale,
    gameplay,
    activeAgentId,
  });

  return {
    objective: {
      title: objectiveTitle,
      detail: objectiveDetail,
      progress_percent: gameplay?.progressPercent ?? null,
    },
    next_action: {
      label: nextActionLabel,
      detail: nextActionDetail,
      target_agent_id: gameplay?.recommendedAction?.targetAgentId || activeAgentId,
      execute_kind: gameplay?.recommendedAction?.executeKind || null,
    },
    active_agent_id: activeAgentId,
    player_leverage: {
      state: gameplay?.executionState || "waiting_for_intent",
      label: gameplay?.executionStateLabel || tr(locale, "等待玩家意图", "Waiting for Intent"),
      summary: leverageSummary,
      detail: leverageDetail,
    },
    action_receipt: actionReceipt,
    blocker: {
      label: gameplay?.blockerLabel || gameplay?.blockerKind || null,
      detail: gameplay?.narrativeBlockerDetail || gameplay?.blockerDetail || null,
    },
    world_read: {
      agents: agents.length,
      routes: links.length,
      fragments: fragmentTerrain.length,
      hotspots: visualHotspots.length,
    },
  };
}

function buildVisualHotspots({
  worldBounds,
  anchor,
  goalHighlight,
  blockerHighlight,
  recentEventHotspots,
}) {
  if (!worldBounds) {
    return [];
  }
  const offsets = [
    [-0.18, -0.14],
    [0.18, -0.12],
    [0.22, 0.14],
    [-0.2, 0.16],
    [0.0, -0.22],
    [0.0, 0.22],
  ];
  const staged = [];
  if (goalHighlight?.title) {
    staged.push({
      id: "goal-highlight",
      label: goalHighlight.title,
      kind: "goal",
      emphasis: 1,
      size_hint_px: 14,
    });
  }
  if (blockerHighlight?.kind) {
    staged.push({
      id: "blocker-highlight",
      label: blockerHighlight.kind,
      kind: "blocker",
      emphasis: 1,
      size_hint_px: 16,
    });
  }
  for (const hotspot of recentEventHotspots.slice(0, 4)) {
    staged.push({
      id: `recent:${hotspot.id}`,
      label: hotspot.title,
      kind: hotspot.kind || "recent_event",
      emphasis: 0.72,
      size_hint_px: 10,
    });
  }
  return staged.map((entry, index) => ({
    ...entry,
    pos: offsetWorldPosition(anchor, worldBounds, ...(offsets[index % offsets.length] || [0, 0])),
  })).filter((entry) => entry.pos);
}

function createPixelWorldHostAdapter({ onSelectEntity, onHoverEntity, onFatal }) {
  let bridge = null;
  let runtimeSource = "detached";
  let runtimeModuleUrl = null;
  let deriveRenderState = null;

  function deriveRenderStateOrFallback(renderInput, fallbackRenderState) {
    if (!deriveRenderState || !renderInput) {
      return fallbackRenderState;
    }
    try {
      const nextRenderState = deriveRenderState(renderInput);
      if (nextRenderState?.fatal) {
        onFatal?.(nextRenderState.fatal);
        return fallbackRenderState;
      }
      return nextRenderState || fallbackRenderState;
    } catch (error) {
      onFatal?.({
        code: "pixel_world_rust_render_state_failed",
        message: error instanceof Error ? error.message : String(error || "Rust render state derivation failed"),
      });
      return fallbackRenderState;
    }
  }

  return {
    async mount(canvas, renderState, renderInput) {
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
      const mountedRenderState = deriveRenderStateOrFallback(renderInput, renderState);
      const result = bridge.mount(canvas, mountedRenderState);
      return {
        status: result?.status || "ready",
        selection: mountedRenderState.selection,
        fatal: result?.fatal || null,
        renderState: mountedRenderState,
        runtimeSource,
        runtimeModuleUrl,
      };
    },
    update(renderState, renderInput) {
      const nextRenderState = deriveRenderStateOrFallback(renderInput, renderState);
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
    simulateFatal(message) {
      onFatal?.({
        code: "pixel_world_renderer_fatal",
        message: String(message || "renderer fatal"),
      });
    },
    runtimeSource() {
      return runtimeSource;
    },
    runtimeModuleUrl() {
      return runtimeModuleUrl;
    },
    deriveRenderState(renderInput) {
      return deriveRenderStateOrFallback(renderInput, null);
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

export function buildPixelWorldRenderStateFromInput(input) {
  const locale = input.locale || core.state.uiLocale;
  const lists = input.lists || { agents: [], locations: [] };
  const gameplay = input.gameplay;
  const worldScaleSurface = core.buildWorldScaleSurface(locale);
  const snapshot = input.snapshot;
  const selected = input.selected;
  const space = snapshot?.config?.space || null;

  const worldBounds = space
    ? {
        width_cm: Number(space.width_cm) || 0,
        depth_cm: Number(space.depth_cm) || 0,
        height_cm: Number(space.height_cm) || 0,
      }
    : null;
  const worldScaleBase = Math.max(1, Math.min(worldBounds?.width_cm || 1, worldBounds?.depth_cm || 1));

  const fragmentTerrain = [];
  const locations = lists.locations
    .map((location) => ({
      raw: location,
      terrain: buildFragmentTerrainForLocation(location, worldBounds),
    }))
    .map(({ raw: location, terrain }) => {
      fragmentTerrain.push(...terrain);
      const resourceSummary = core.resourceSummary(location.resources);
      const hasTerrain = terrain.length > 0;
      return {
        id: location.id,
        label: location.name || location.id,
        pos: normalizePosition(location.pos),
        radius_cm: Number(location?.profile?.radius_cm) || 0,
        resource_summary: resourceSummary,
        resource_score: countResourceEntries(resourceSummary),
        fragment_terrain_count: terrain.length,
        marker_role: hasTerrain ? "logic_anchor" : "primary_marker",
        marker_alpha: hasTerrain ? 0.32 : 0.72,
        size_hint_px: hasTerrain
          ? 10
          : 16 + Math.min(
            18,
            (((Number(location?.profile?.radius_cm) || 0) / worldScaleBase) * 420)
              + (countResourceEntries(resourceSummary) * 2),
          ),
      };
    })
    .filter((location) => location.pos);
  const locationById = new Map(locations.map((location) => [location.id, location]));

  const agents = lists.agents.map((agent) => {
    const resolvedPosition = resolveAgentPosition(agent, selected, locationById, worldBounds);
    const resourceSummary = core.resourceSummary(agent.resources);
    return {
      id: agent.id,
      label: agent.name || agent.id,
      location_id: agent.location_id || null,
      pos: resolvedPosition.pos,
      position_source: resolvedPosition.position_source,
      resource_summary: resourceSummary,
      resource_score: countResourceEntries(resourceSummary),
      status_badges: [
        agent.location_id ? `location=${agent.location_id}` : null,
        agent.kind ? `kind=${agent.kind}` : null,
        resolvedPosition.position_source === "location_derived" ? "position=location_derived" : null,
      ].filter(Boolean),
      size_hint_px: 12 + Math.min(
        10,
        (countResourceEntries(resourceSummary) * 2)
          + (agent.location_id ? 2 : 0)
          + (agent.kind ? 1 : 0),
      ),
    };
  });

  const selection = input.selectedKind && input.selectedId
    ? {
        kind: input.selectedKind,
        id: input.selectedId,
      }
    : null;
  const links = buildPixelWorldLinks(agents, locationById);
  const anchor = resolveSelectionPosition(selection, agents, locations)
    || agents.find((agent) => agent.pos)?.pos
    || locations[0]?.pos
    || worldCenterPosition(worldBounds);
  const goalHighlight = gameplay?.goalTitle
    ? {
        title: gameplay.goalTitle,
        objective: gameplay.objective || null,
      }
    : null;
  const blockerHighlight = gameplay?.blockerKind || gameplay?.blockerDetail
    ? {
        kind: gameplay?.blockerKind || "blocked",
        detail: gameplay?.blockerDetail || null,
      }
    : null;
  const recentEventHotspots = buildRecentEventHotspots(input.recentEvents);
  const visualHotspots = buildVisualHotspots({
    worldBounds,
    anchor,
    goalHighlight,
    blockerHighlight,
    recentEventHotspots,
  });
  const commercialSurface = buildCommercialSurface({
    locale,
    gameplay,
    agents,
    links,
    fragmentTerrain,
    visualHotspots,
    selection,
  });

  return {
    locale,
    world_bounds: worldBounds,
    locations,
    fragment_terrain: fragmentTerrain,
    agents,
    links,
    selection,
    goal_highlight: goalHighlight,
    blocker_highlight: blockerHighlight,
    recent_event_hotspots: recentEventHotspots,
    visual_hotspots: visualHotspots,
    commercial_surface: commercialSurface,
    presentation: {
      world_bounds_label: worldScaleSurface.physicalTruth.worldBoundsLabel,
      marker_truth_note: worldScaleSurface.presentationScale.markerTruthNote,
    },
  };
}

export function buildPixelWorldRenderState(locale = core.state.uiLocale) {
  return buildPixelWorldRenderStateFromInput(buildPixelWorldRenderInput(locale));
}

function PixelWorldCanvasRenderer(props) {
  let canvasRef;

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
    <div class="pixel-world-canvas pixel-world-canvas--rendered" data-renderer-ready="true">
      <canvas
        ref={canvasRef}
        id={PIXEL_WORLD_RUNTIME_CANVAS_ID}
        class="pixel-world-canvas__surface"
        width="960"
        height="540"
      />
      <div class="pixel-world-canvas__overlay">
        <Show when={props.renderState().goal_highlight}>
          <div class="pixel-world-canvas__callout pixel-world-canvas__callout--goal">
            {`${tr(props.locale(), "目标", "Goal")}: ${props.renderState().goal_highlight.title}`}
          </div>
        </Show>
        <Show when={props.renderState().blocker_highlight}>
          <div class="pixel-world-canvas__callout pixel-world-canvas__callout--blocker">
            {`${tr(props.locale(), "阻塞", "Blocker")}: ${props.renderState().blocker_highlight.kind}`}
          </div>
        </Show>
      </div>
      <Show when={props.renderState().selection}>
        <div class="pixel-world-canvas__selection">
          {`${tr(props.locale(), "已选中", "Selected")}: ${props.renderState().selection.kind}/${props.renderState().selection.id}`}
        </div>
      </Show>
    </div>
  );
}

function PixelWorldCommercialHud(props) {
  const surface = () => props.renderState().commercial_surface;
  return (
    <Show when={surface()}>
      <div
        class="pixel-world-command-strip"
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
        <div class="pixel-world-command-cell pixel-world-command-cell--next">
          <div class="pixel-world-command-cell__label">
            {tr(props.locale(), "下一步", "Next Move")}
          </div>
          <div class="pixel-world-command-cell__value">{surface().next_action.label}</div>
          <Show when={surface().next_action.detail}>
            <div class="pixel-world-command-cell__detail">{surface().next_action.detail}</div>
          </Show>
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
      <div
        class="pixel-world-action-receipt"
        data-receipt-present={surface().action_receipt.present ? "true" : "false"}
        data-receipt-state={surface().action_receipt.state}
        data-receipt-confidence={surface().action_receipt.confidence}
      >
        <div class="pixel-world-action-receipt__label">
          {tr(props.locale(), "行动回执", "Action Receipt")}
        </div>
        <div class="pixel-world-action-receipt__body">
          <div class="pixel-world-action-receipt__title">
            {surface().action_receipt.title}
          </div>
          <div class="pixel-world-action-receipt__summary">
            {surface().action_receipt.summary}
          </div>
          <Show when={surface().action_receipt.detail}>
            <div class="pixel-world-action-receipt__detail">
              {surface().action_receipt.detail}
            </div>
          </Show>
        </div>
        <div class="pixel-world-action-receipt__meta">
          <span>{surface().action_receipt.confidence}</span>
          <Show when={surface().action_receipt.target_agent_id}>
            <span>{`agent=${surface().action_receipt.target_agent_id}`}</span>
          </Show>
        </div>
      </div>
      <div class="pixel-world-readout badge-row">
        <span class="badge badge--accent">{`agents=${surface().world_read.agents}`}</span>
        <span class="badge">{`routes=${surface().world_read.routes}`}</span>
        <span class="badge">{`fragments=${surface().world_read.fragments}`}</span>
        <span class="badge">{`hotspots=${surface().world_read.hotspots}`}</span>
        <Show when={surface().blocker.label}>
          <span class="badge badge--warn">{`blocker=${surface().blocker.label}`}</span>
        </Show>
      </div>
    </Show>
  );
}

function PixelWorldCanvasPlaceholder(props) {
  return (
    <div class="pixel-world-canvas" data-renderer-ready={props.ready() ? "true" : "false"}>
      <div class="pixel-world-canvas__grid" />
      <For each={props.renderState().fragment_terrain.slice(0, 96)}>
        {(patch, index) => (
          <div
            class="pixel-world-fragment-terrain"
            data-compound={patch.dominant_compound}
            style={fragmentTerrainStyle(patch, props.renderState().world_bounds, index())}
            title={`${patch.location_id}:${patch.dominant_compound}`}
          />
        )}
      </For>
      <For each={props.renderState().links.slice(0, 10)}>
        {(link, index) => (
          <div
            class="pixel-world-route"
            data-route-kind={link.kind}
            style={routeStyle(link, props.renderState().world_bounds, index())}
            title={`${link.kind}:${link.id}`}
          />
        )}
      </For>
      <For each={props.renderState().locations.slice(0, 8)}>
        {(location, index) => (
          <button
            class="pixel-world-entity pixel-world-entity--location"
            data-marker-role={location.marker_role}
            style={{
              ...toWorldPercentStyle(location.pos, props.renderState().world_bounds, {
                left: `${12 + ((index() % 4) * 21)}%`,
                top: `${18 + (Math.floor(index() / 4) * 26)}%`,
              }),
              opacity: location.marker_alpha,
            }}
            title={location.label}
            onMouseEnter={() => props.onHover({ kind: "location", id: location.id })}
            onMouseLeave={() => props.onHover(null)}
            onClick={() => props.onSelect({ kind: "location", id: location.id })}
          >
            <span>{location.label.slice(0, 2).toUpperCase()}</span>
          </button>
        )}
      </For>
      <For each={props.renderState().agents.slice(0, 10)}>
        {(agent, index) => (
          <button
            class="pixel-world-entity pixel-world-entity--agent"
            data-position-source={agent.position_source}
            style={toWorldPercentStyle(agent.pos, props.renderState().world_bounds, {
              left: `${18 + ((index() % 5) * 15)}%`,
              top: `${14 + (Math.floor(index() / 5) * 22)}%`,
            })}
            title={agent.label}
            onMouseEnter={() => props.onHover({ kind: "agent", id: agent.id })}
            onMouseLeave={() => props.onHover(null)}
            onClick={() => props.onSelect({ kind: "agent", id: agent.id })}
          >
            <span>{agent.label.slice(0, 1).toUpperCase()}</span>
          </button>
        )}
      </For>
      <Show when={props.renderState().selection}>
        <div class="pixel-world-canvas__selection">
          {`${tr(props.locale(), "已选中", "Selected")}: ${props.renderState().selection.kind}/${props.renderState().selection.id}`}
        </div>
      </Show>
      <div class="pixel-world-canvas__overlay">
        <Show when={props.renderState().goal_highlight}>
          <div class="pixel-world-canvas__callout pixel-world-canvas__callout--goal">
            {`${tr(props.locale(), "目标", "Goal")}: ${props.renderState().goal_highlight.title}`}
          </div>
        </Show>
        <Show when={props.renderState().blocker_highlight}>
          <div class="pixel-world-canvas__callout pixel-world-canvas__callout--blocker">
            {`${tr(props.locale(), "阻塞", "Blocker")}: ${props.renderState().blocker_highlight.kind}`}
          </div>
        </Show>
      </div>
    </div>
  );
}

export function PixelWorldHost(props) {
  const locale = () => props.locale ?? core.state.uiLocale;
  const renderInput = createMemo(() => buildPixelWorldRenderInput(locale()));
  const fallbackRenderState = createMemo(() => buildPixelWorldRenderStateFromInput(renderInput()));
  const [rustRenderState, setRustRenderState] = createSignal(null);
  const renderState = () => rustRenderState() || fallbackRenderState();
  const [rendererStatus, setRendererStatus] = createSignal("booting");
  const [rendererFatal, setRendererFatal] = createSignal(null);
  const [hoverSelection, setHoverSelection] = createSignal(null);
  const [runtimeSource, setRuntimeSource] = createSignal("loading");
  const [cameraState, setCameraState] = createSignal(null);

  const adapter = createMemo(() => createPixelWorldHostAdapter({
    onSelectEntity(selection) {
      core.applySelection(selection);
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
      setRendererStatus("fallback");
      core.updatePixelWorldRuntimeMeta({
        runtimeStatus: "fallback",
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
    const result = adapter().update(fallbackRenderState(), renderInput());
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
      setRendererStatus("fallback");
      setRuntimeSource("detached");
      core.updatePixelWorldRuntimeMeta({
        runtimeStatus: "fallback",
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
      setRendererStatus("fallback");
      setRuntimeSource("detached");
      core.updatePixelWorldRuntimeMeta({
        runtimeStatus: "fallback",
        runtimeSource: "detached",
        runtimeModuleUrl: null,
        camera: cameraState(),
        fatal,
      });
      return;
    }
    const result = await adapter().mount(mountedCanvas, fallbackRenderState(), renderInput());
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

  function setFallbackMode() {
    adapter().unmount();
    setRustRenderState(null);
    setRendererStatus("fallback");
    setRuntimeSource("detached");
    setCameraState(null);
    core.updatePixelWorldRuntimeMeta({
      runtimeStatus: "fallback",
      runtimeSource: "detached",
      runtimeModuleUrl: null,
      camera: null,
      fatal: rendererFatal(),
    });
  }

  function simulateFatal() {
    adapter().simulateFatal("simulated embedded renderer fatal fallback");
  }

  onCleanup(() => {
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
    <div class="pixel-world-host stack">
      <div class="pixel-world-host__summary">
        <div class="pixel-world-host__headline">
          {tr(locale(), "世界指挥棋盘", "World Command Board")}
        </div>
        <div class="feedback-detail">
          {renderState().commercial_surface?.objective?.detail}
        </div>
      </div>
      <PixelWorldCommercialHud locale={locale} renderState={renderState} />
      <Show when={rendererStatus() !== "fallback"}>
        <PixelWorldCanvasRenderer
          locale={locale}
          renderInput={renderInput}
          renderState={renderState}
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
      <Show when={rendererStatus() === "fallback"}>
        <div class="callout callout--warn">
          <div class="callout__header">
            <div class="callout__title">{tr(locale(), "Renderer 未接管", "Renderer Not Attached")}</div>
          </div>
          <div class="callout__body">
            <div class="feedback-summary">
              {tr(
                locale(),
                "嵌入式 renderer 启动失败，页面已退回 host fallback 模式。正式玩法摘要、目标和明细主链继续可用。",
                "The embedded renderer failed to attach, so the page returned to host fallback mode. Formal gameplay summary, targets, and details remain available.",
              )}
            </div>
            <Show when={rendererFatal()}>
              <div class="feedback-detail">{`${rendererFatal().code}: ${rendererFatal().message}`}</div>
            </Show>
          </div>
        </div>
      </Show>
      <Show when={rendererStatus() !== "ready"}>
        <PixelWorldCanvasPlaceholder
          locale={locale}
          renderState={renderState}
          ready={() => false}
          onSelect={(selection) => adapter().simulateSelect(selection)}
          onHover={(selection) => adapter().simulateHover(selection)}
        />
      </Show>
      <details class="diagnostic pixel-world-render-diagnostics">
        <summary>{tr(locale(), "Renderer 诊断", "Renderer Diagnostics")}</summary>
        <div class="pixel-world-host__toolbar badge-row">
          <span class="badge badge--accent">{`locations=${renderState().locations.length}`}</span>
          <span class="badge badge--accent">{`fragments=${renderState().fragment_terrain.length}`}</span>
          <span class="badge badge--accent">{`agents=${renderState().agents.length}`}</span>
          <span class="badge">{`links=${renderState().links.length}`}</span>
          <span class="badge">{`hotspots=${renderState().visual_hotspots.length}`}</span>
          <span class="badge">{`derived_positions=${renderState().agents.filter((agent) => agent.position_source === "location_derived").length}`}</span>
          <span class="badge">{renderState().world_bounds ? "world_bounds=ready" : "world_bounds=missing"}</span>
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
          <button type="button" onClick={() => { void setReadyMode(); }}>
            {tr(locale(), "重新挂载嵌入式 Renderer", "Reattach Embedded Renderer")}
          </button>
          <button type="button" onClick={simulateFatal}>
            {tr(locale(), "模拟 Renderer Fatal", "Simulate Renderer Fatal")}
          </button>
          <button type="button" onClick={setFallbackMode}>
            {tr(locale(), "切回 Host Fallback", "Back To Host Fallback")}
          </button>
          <div class="feedback-detail">
            {tr(
              locale(),
              "当前世界舞台优先依赖 wasm bridge、嵌入式 canvas、轻量拖拽缩放和事件回传。若 wasm bridge 缺失或启动失败，页面会显式退回 host fallback，而不是继续保留一套 JS renderer。",
              "The world stage now depends on the wasm bridge, embedded canvas, light pan-zoom interaction, and event callbacks. If the wasm bridge is missing or fails to boot, the page falls back explicitly instead of keeping a second JS renderer.",
            )}
          </div>
        </div>
      </details>
      <details class="diagnostic">
        <summary>{tr(locale(), "展开 Render DTO", "Expand Render DTO")}</summary>
        <div class="stack" style="margin-top:10px;">
          <pre class="json">{JSON.stringify(renderState(), null, 2)}</pre>
        </div>
      </details>
    </div>
  );
}
