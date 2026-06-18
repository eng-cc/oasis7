import { createEffect, createMemo, createSignal, For, Show, onCleanup, onMount } from "solid-js";

import * as core from "./legacy_core.js";
import { createPixelWorldRuntimeBridge } from "./pixel_world_runtime_loader.js";

function tr(locale, zh, en) {
  return core.isLocaleZh(locale) ? zh : en;
}

const PIXEL_WORLD_RUNTIME_CANVAS_ID = "pixel-world-embedded-runtime-canvas";
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

function shouldAutoAttachRenderer() {
  if (typeof window === "undefined" || !window.location) {
    return true;
  }
  const params = new URLSearchParams(window.location.search || "");
  const value = String(params.get("pixel_world_renderer") || "").trim().toLowerCase();
  if (value) {
    return !DEFER_RENDERER_VALUES.has(value);
  }
  return true;
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
  };
}

function pickKnownAgentId(candidateIds, agents) {
  const knownAgentIds = new Set(agents.map((agent) => agent.id));
  return candidateIds.find((id) => id && knownAgentIds.has(id)) || null;
}

function normalizeGameplayToken(value) {
  return String(value || "")
    .trim()
    .toLowerCase()
    .replaceAll(/\s+/g, "")
    .replaceAll("_", "")
    .replaceAll("-", "");
}

function containsCjk(value) {
  return /[\u3400-\u9fff]/u.test(String(value || ""));
}

function zhOrPublished(locale, published, zhFallback, enFallback) {
  if (!core.isLocaleZh(locale)) {
    return published || enFallback;
  }
  if (published && containsCjk(published)) {
    return published;
  }
  return zhFallback;
}

function localizedGoalTitle(locale, gameplay) {
  const published = gameplay?.goalTitle || null;
  const goalKind = normalizeGameplayToken(gameplay?.goalKind);
  switch (goalKind) {
    case "recovercapability":
      return zhOrPublished(locale, published, "恢复可持续能力", "Recover sustainable capability");
    case "stabilizefirstline":
    case "establishfirstcapability":
      return zhOrPublished(locale, published, "稳定第一条生产线", "Stabilize the first production line");
    case "choosefirstexpansiontradeoff":
    case "choosemidlooppath":
      return zhOrPublished(locale, published, "选择下一条扩张路径", "Choose the next expansion path");
    case "createfirstworldfeedback":
      return zhOrPublished(locale, published, "确认第一条世界反馈", "Confirm the first world feedback");
    default:
      return zhOrPublished(
        locale,
        published,
        "进入世界，建立第一条能力链",
        "Enter the world and build the first capability chain",
      );
  }
}

function localizedObjectiveDetail(locale, gameplay) {
  const published = gameplay?.objective || gameplay?.progressDetail || null;
  const goalKind = normalizeGameplayToken(gameplay?.goalKind);
  switch (goalKind) {
    case "recovercapability":
      return zhOrPublished(locale, published, "先恢复阻塞点，再确认生产线重新具备可经营能力。", "Recover the blocker first, then confirm the line is operable again.");
    case "stabilizefirstline":
    case "establishfirstcapability":
      return zhOrPublished(locale, published, "先稳定第一条产线，再决定扩张、恢复或分工。", "Stabilize the first line before choosing expansion, recovery, or specialization.");
    case "choosefirstexpansiontradeoff":
    case "choosemidlooppath":
      return zhOrPublished(locale, published, "比较下一步带来的用途、弹性和分支价值，再推进。", "Compare the next move's use, resilience, and branch value before advancing.");
    case "createfirstworldfeedback":
      return zhOrPublished(locale, published, "先拿到一条明确世界反馈，再继续后续工业选择。", "Get one clear world feedback signal before continuing industrial choices.");
    default:
      return zhOrPublished(
        locale,
        published,
        "先让 Agent、路线和资源关系变得可读，再推进下一步。",
        "Read the agent, route, and resource relationship before pushing the next move.",
      );
  }
}

function localizedNextActionLabel(locale, gameplay) {
  const published = gameplay?.recommendedAction?.label
    || gameplay?.nextStepHint
    || gameplay?.narrativeNextStep
    || null;
  const executeKind = gameplay?.recommendedAction?.executeKind;
  const actionId = normalizeGameplayToken(gameplay?.recommendedAction?.actionId);
  const labelToken = normalizeGameplayToken(gameplay?.recommendedAction?.label);
  if (core.isLocaleZh(locale) && published && containsCjk(published)) {
    return published;
  }
  if (!core.isLocaleZh(locale) && published) {
    return published;
  }
  if (actionId === "buildfactorysmeltermk1" || labelToken.includes("smeltermk1")) {
    return tr(locale, "排队建造一型冶炼炉", "Queue Smelter MK1 construction");
  }
  switch (executeKind) {
    case "gameplay_action":
      return tr(locale, "提交推荐玩法动作", "Submit recommended gameplay action");
    case "step":
      return tr(locale, "推进世界一步", "Advance the world one step");
    case "play":
      return tr(locale, "继续运行世界", "Keep the world running");
    case "request_snapshot":
      return tr(locale, "刷新世界快照", "Refresh world snapshot");
    case "agent_chat":
      return tr(locale, "向选中 Agent 发送消息", "Message the selected agent");
    default:
      return tr(locale, "选择一个 Agent 或推进世界一步", "Select an agent or advance the world one step");
  }
}

function localizedOptionalDetail(locale, published) {
  if (!published) {
    return null;
  }
  if (!core.isLocaleZh(locale) || containsCjk(published)) {
    return published;
  }
  const token = normalizeGameplayToken(published);
  if (
    token.includes("requestasnapshot")
    || token.includes("advance1step")
    || token.includes("inspectthenewdelta")
  ) {
    return "先请求一次快照，推进 1 步，再检查新的世界变化和事件。";
  }
  return tr(locale, "查看当前回执和阻塞原因，再决定下一步。", "Read the current receipt and blocker before choosing the next move.");
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
  worldTick,
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
  const objectiveTitle = localizedGoalTitle(locale, gameplay);
  const objectiveDetail = localizedObjectiveDetail(locale, gameplay);
  const nextActionLabel = localizedNextActionLabel(locale, gameplay);
  const nextActionDetail = localizedOptionalDetail(
    locale,
    gameplay?.recommendedAction?.disabledReason
      || gameplay?.nextStepHint
      || gameplay?.executionSummary
      || null,
  );
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
      tick: worldTick,
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

function PixelWorldHostVisualLayer(props) {
  const visualState = () => pixelWorldVisualState(props.renderState());
  if (!props.enabled) {
    return <></>;
  }
  return (
    <>
      <div class="pixel-world-canvas__grid" />
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
          <div
            class="pixel-world-route"
            data-route-kind={link.kind}
            style={routeStyle(link, visualState().worldBounds, index())}
            title={`${link.kind}:${link.id}`}
          />
        )}
      </For>
      <For each={visualState().locations.slice(0, 8)}>
        {(location, index) => (
          <button
            class="pixel-world-entity pixel-world-entity--location"
            data-marker-role={location.marker_role}
            style={{
              ...toWorldPercentStyle(location.pos, visualState().worldBounds, {
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
      <For each={visualState().agents.slice(0, 10)}>
        {(agent, index) => (
          <button
            class="pixel-world-entity pixel-world-entity--agent"
            data-pixel-world-agent-marker="true"
            data-agent-id={agent.id}
            data-position-source={agent.position_source}
            aria-label={`${tr(props.locale(), "选择 Agent", "Select Agent")} ${agent.id}`}
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
          aria-label={`${tr(props.locale(), "选择 Agent", "Select Agent")} ${agent.id}`}
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
      return withWorldTickReadout(nextRenderState, renderInput) || fallbackRenderState;
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
  const worldTick = snapshotTick(snapshot);
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
  const localizedGoal = localizedGoalTitle(locale, gameplay);
  const localizedGoalDetail = localizedObjectiveDetail(locale, gameplay);
  const goalHighlight = localizedGoal
    ? {
        title: localizedGoal,
        objective: localizedGoalDetail || null,
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
    worldTick,
    agents,
    links,
    fragmentTerrain,
    visualHotspots,
    selection,
  });

  return {
    locale,
    world_tick: worldTick,
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
    <div class="pixel-world-canvas pixel-world-canvas--rendered" data-renderer-ready="true">
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
          locale={props.locale}
          renderState={props.renderState}
          onSelect={props.onSelect}
          onHover={props.onHover}
        />
        <PixelWorldHostVisualLayer
          enabled={false}
          locale={props.locale}
          renderState={props.renderState}
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
            {`${tr(props.locale(), "阻塞", "Blocker")}: ${visualState().blockerHighlight.kind}`}
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
      class={`pixel-world-action-receipt ${props.class ?? ""}`}
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
      <div class="pixel-world-action-receipt__meta">
        <span>{receipt().confidence}</span>
        <Show when={receipt().target_agent_id}>
          <span>{`agent=${receipt().target_agent_id}`}</span>
        </Show>
      </div>
    </div>
  );
}

function PixelWorldCommercialHud(props) {
  const surface = () => props.renderState().commercial_surface;
  const executableNextMoveKinds = new Set(["gameplay_action", "step", "play", "request_snapshot"]);
  const nextMoveRoutesToGameplayDetails = () => executableNextMoveKinds.has(surface().next_action.execute_kind);
  const nextMoveRoute = () => nextMoveRoutesToGameplayDetails() ? "gameplay_details" : "command";
  const nextMoveHref = () => nextMoveRoutesToGameplayDetails() ? "#viewer-gameplay-details" : "#viewer-details-panel";
  const openGameplayDetails = () => {
    if (!nextMoveRoutesToGameplayDetails()) {
      return;
    }
    const details = document.getElementById("viewer-gameplay-details");
    if (details) {
      details.open = true;
    }
  };
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
        <div
          class="pixel-world-command-cell pixel-world-command-cell--next"
          data-next-move-route={nextMoveRoute()}
          data-execute-kind={surface().next_action.execute_kind || "none"}
        >
          <div class="pixel-world-command-cell__label">
            {tr(props.locale(), "下一步", "Next Move")}
          </div>
          <div class="pixel-world-command-cell__value">{surface().next_action.label}</div>
          <Show when={surface().next_action.detail}>
            <div class="pixel-world-command-cell__detail">{surface().next_action.detail}</div>
          </Show>
          <a class="pixel-world-command-cell__action" href={nextMoveHref()} onClick={openGameplayDetails}>
            {nextMoveRoutesToGameplayDetails()
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
      <PixelWorldActionReceipt
        locale={props.locale}
        surface={surface}
      />
      <div class="pixel-world-readout badge-row">
        <Show when={surface().world_read.tick !== null && surface().world_read.tick !== undefined}>
          <span class="badge badge--accent" data-world-tick={String(surface().world_read.tick)}>{`tick=${surface().world_read.tick}`}</span>
        </Show>
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

function PixelWorldFocusHud(props) {
  const surface = () => props.renderState().commercial_surface;
  return (
    <Show when={surface()}>
      <div class="pixel-world-focus-hud" data-focus-hud="true">
        <div class="pixel-world-focus-hud__identity">
          <div class="pixel-world-focus-hud__eyebrow">
            {tr(props.locale(), "沉浸模式", "World Focus")}
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
          <div class="pixel-world-focus-hud__cell pixel-world-focus-hud__cell--tick" data-world-tick={String(surface().world_read.tick)}>
            <span>{tr(props.locale(), "世界 Tick", "World Tick")}</span>
            <strong>{surface().world_read.tick}</strong>
            <em>{`tick=${surface().world_read.tick}`}</em>
          </div>
        </Show>
        <div
          class="pixel-world-focus-hud__cell pixel-world-focus-hud__cell--blocker"
          data-blocker-present={surface().blocker.label ? "true" : "false"}
        >
          <span>{tr(props.locale(), "阻塞", "Blocker")}</span>
          <strong>{surface().blocker.label || tr(props.locale(), "暂无阻塞", "No blocker")}</strong>
        </div>
        <div
          class="pixel-world-focus-hud__cell pixel-world-focus-hud__cell--receipt"
          data-receipt-confidence={surface().action_receipt.confidence}
        >
          <span>{tr(props.locale(), "回执", "Receipt")}</span>
          <strong>{surface().action_receipt.title}</strong>
          <em>{surface().action_receipt.confidence}</em>
        </div>
        <div class="pixel-world-focus-controls" aria-label={tr(props.locale(), "沉浸模式控制", "World focus controls")}>
          <button type="button" class="pixel-world-focus-control pixel-world-focus-control--primary" onClick={props.onOpenCommand}>
            {tr(props.locale(), "命令", "Command")}
          </button>
          <button type="button" class="pixel-world-focus-control pixel-world-focus-control--secondary" onClick={props.onOpenDiagnostics}>
            {tr(props.locale(), "诊断", "Diagnostics")}
          </button>
          <button type="button" class="pixel-world-focus-control pixel-world-focus-control--secondary" onClick={props.onToggleMaximized}>
            {props.maximized()
              ? tr(props.locale(), "退出最大化", "Minimize")
              : tr(props.locale(), "最大化", "Maximize")}
          </button>
          <button type="button" class="pixel-world-focus-control pixel-world-focus-control--quiet" onClick={props.onExit}>
            {tr(props.locale(), "退出", "Exit")}
          </button>
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
        <Show when={surface()?.blocker.label}>
          <div class="pixel-world-focus-rail__item">
            <span>{tr(props.locale(), "阻塞", "Blocker")}</span>
            <strong>{surface().blocker.label}</strong>
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
        class="pixel-world-focus-fallback-map"
        data-focus-fallback-map={props.variant === "fallback" ? "true" : null}
        data-focus-minimap="true"
      >
        <div class="pixel-world-focus-fallback-map__label">
          {tr(props.locale(), "任务地图", "Mission Map")}
        </div>
        <Show when={primaryLocation()}>
          <span class="sr-only">
            {`${tr(props.locale(), "参照", "Reference")}: ${primaryLocation().label || primaryLocation().id}`}
          </span>
        </Show>
        <div class="pixel-world-focus-fallback-map__grid" />
        <div class="pixel-world-focus-fallback-map__route" data-routes={props.renderState().links.length} />
        <div class="pixel-world-focus-fallback-map__node pixel-world-focus-fallback-map__node--target">
          <span>{tr(props.locale(), "目标", "Target")}</span>
          <strong>{surface().next_action.label}</strong>
        </div>
        <div class="pixel-world-focus-fallback-map__node pixel-world-focus-fallback-map__node--agent">
          <span>{tr(props.locale(), "Agent", "Agent")}</span>
          <strong>{activeAgent() || tr(props.locale(), "待分配", "Unassigned")}</strong>
        </div>
        <Show when={selected()}>
          <div
            class="pixel-world-focus-fallback-map__node pixel-world-focus-fallback-map__node--selected"
            data-selected="true"
          >
            <span>{tr(props.locale(), "选中", "Selected")}</span>
            <strong>{`${selected().kind}/${selected().id}`}</strong>
          </div>
        </Show>
        <div class="pixel-world-focus-fallback-map__meta" aria-label={tr(props.locale(), "Fallback 世界摘要", "Fallback world summary")}>
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
  if (entry.source === "player") {
    return `${tr(locale, "玩家", "Player")} -> ${entry.targetAgentId || entry.agentId || "agent"}`;
  }
  return `${entry.agentId || "agent"} ${tr(locale, "已发言", "spoke")}`;
}

function chatEntryMeta(entry, locale) {
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
  const agentId = () => core.selectedAgentId();
  const authSurface = () => core.buildAuthSurfaceModel();
  const chatCapability = () => authSurface().capabilities.agent_chat;
  const binding = () => core.selectedAgentBindingInfo();
  const chatFeedback = () => core.snapshotSemanticFeedback(core.state.lastChatFeedback);
  const chatFeedbackDisplay = () => core.describeSemanticFeedback(chatFeedback(), locale());
  const chatHistory = () =>
    core.state.chatHistory
      .filter((entry) => entry.agentId === agentId() || entry.targetAgentId === agentId())
      .slice(0, 12);

  return (
    <div class="pixel-world-focus-command-surface stack">
      <Show
        when={agentId()}
        fallback={<div class="empty">{tr(locale(), "先选中一个行动体，才能在沉浸模式里直接下指令。", "Select an agent to issue direct commands in World Focus.")}</div>}
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
                  "沉浸态保持世界视图在前，但这里可以直接给当前目标发消息并读取反馈。",
                  "World Focus keeps the world view in front, while this surface sends messages to the current target and reads feedback directly.",
                )}
              </div>
            </div>
          </div>
          <div class="panel__body stack">
            <div class="field">
              <label for="agent-chat-message">{tr(locale(), "消息", "Message")}</label>
              <textarea
                id="agent-chat-message"
                rows="4"
                placeholder={tr(locale(), "给当前选中的行动体发一条消息", "Send a message to the selected agent")}
                disabled={!chatCapability().enabled}
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
                disabled={!chatCapability().enabled}
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
                      <div class="event-card">
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

function PixelWorldCanvasPlaceholder(props) {
  const visualState = () => pixelWorldVisualState(props.renderState());
  return (
    <div class="pixel-world-canvas" data-renderer-ready={props.ready() ? "true" : "false"}>
      <PixelWorldHostVisualLayer
        enabled={true}
        locale={props.locale}
        renderState={props.renderState}
        onSelect={props.onSelect}
        onHover={props.onHover}
      />
      <Show when={visualState().selection}>
        <div class="pixel-world-canvas__selection">
          {`${tr(props.locale(), "已选中", "Selected")}: ${visualState().selection.kind}/${visualState().selection.id}`}
        </div>
      </Show>
      <div class="pixel-world-canvas__overlay">
        <Show when={visualState().goalHighlight}>
          <div class="pixel-world-canvas__callout pixel-world-canvas__callout--goal">
            {`${tr(props.locale(), "目标", "Goal")}: ${visualState().goalHighlight.title}`}
          </div>
        </Show>
        <Show when={visualState().blockerHighlight}>
          <div class="pixel-world-canvas__callout pixel-world-canvas__callout--blocker">
            {`${tr(props.locale(), "阻塞", "Blocker")}: ${visualState().blockerHighlight.kind}`}
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
  const visualState = () => pixelWorldVisualState(renderState());
  const autoAttachRenderer = shouldAutoAttachRenderer();
  const [rendererStatus, setRendererStatus] = createSignal(autoAttachRenderer ? "booting" : "fallback");
  const [rendererFatal, setRendererFatal] = createSignal(null);
  const [hoverSelection, setHoverSelection] = createSignal(null);
  const [runtimeSource, setRuntimeSource] = createSignal(autoAttachRenderer ? "loading" : "deferred");
  const [cameraState, setCameraState] = createSignal(null);
  const [renderDtoOpen, setRenderDtoOpen] = createSignal(false);
  const [focusMode, setFocusMode] = createSignal(pixelWorldFocusUiSessionState.focusMode);
  const [commandDrawerOpen, setCommandDrawerOpen] = createSignal(pixelWorldFocusUiSessionState.commandDrawerOpen);
  const [diagnosticsDrawerOpen, setDiagnosticsDrawerOpen] = createSignal(pixelWorldFocusUiSessionState.diagnosticsDrawerOpen);
  const [maximized, setMaximized] = createSignal(pixelWorldFocusUiSessionState.maximized);

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

  function enterFocusMode() {
    setPersistentFocusMode(true);
    setPersistentCommandDrawerOpen(true);
    setPersistentDiagnosticsDrawerOpen(false);
    setPersistentMaximized(false);
  }

  function exitFocusMode() {
    setPersistentFocusMode(false);
    setPersistentCommandDrawerOpen(false);
    setPersistentDiagnosticsDrawerOpen(false);
    setPersistentMaximized(false);
  }

  function openCommandDrawer() {
    setPersistentCommandDrawerOpen(true);
    setPersistentDiagnosticsDrawerOpen(false);
  }

  function openDiagnosticsDrawer() {
    setPersistentDiagnosticsDrawerOpen(true);
    setPersistentCommandDrawerOpen(false);
  }

  function toggleMaximized() {
    setPersistentMaximized(!maximized());
  }

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

  function requestReadyMode() {
    setRendererFatal(null);
    setRendererStatus("booting");
    setRuntimeSource("loading");
    if (mountedCanvas) {
      void setReadyMode();
    }
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

  onMount(() => {
    function handleKeyDown(event) {
      if (event.key === "Escape" && focusMode()) {
        event.preventDefault();
        exitFocusMode();
      }
    }

    window.addEventListener("keydown", handleKeyDown);
    onCleanup(() => window.removeEventListener("keydown", handleKeyDown));
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
      data-world-focus={focusMode() ? "true" : "false"}
      data-world-focus-maximized={focusMode() && maximized() ? "true" : "false"}
    >
      <Show when={!focusMode() || !maximized()}>
        <div class="pixel-world-host__summary">
          <div class="pixel-world-host__headline">
            {tr(locale(), "世界指挥棋盘", "World Command Board")}
          </div>
          <div class="feedback-detail">
            {renderState().commercial_surface?.objective?.detail}
          </div>
          <div class="pixel-world-focus-entry">
            <button type="button" onClick={enterFocusMode} aria-pressed={focusMode() ? "true" : "false"}>
              {tr(locale(), "进入沉浸模式", "Enter World Focus")}
            </button>
          </div>
        </div>
      </Show>
      <Show when={focusMode()}>
        <Show when={!maximized()}>
          <PixelWorldFocusCinematicBanner
            locale={locale}
            renderState={renderState}
          />
        </Show>
        <PixelWorldFocusHud
          locale={locale}
          renderState={renderState}
          onExit={exitFocusMode}
          onOpenCommand={openCommandDrawer}
          onOpenDiagnostics={openDiagnosticsDrawer}
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
      <PixelWorldCommercialHud locale={locale} renderState={renderState} />
      <Show when={rendererStatus() !== "fallback"}>
        <PixelWorldCanvasRenderer
          locale={locale}
          renderInput={renderInput}
          renderState={renderState}
          onSelect={(selection) => adapter().simulateSelect(selection)}
          onHover={(selection) => adapter().simulateHover(selection)}
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
      <Show when={focusMode() && rendererStatus() === "fallback" && !maximized()}>
        <PixelWorldFocusMinimapCard locale={locale} renderState={renderState} variant="fallback" />
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
      <Show when={rendererStatus() === "fallback"}>
        <details class="diagnostic pixel-world-render-fallback" data-renderer-state="fallback">
          <summary>{tr(locale(), "Renderer 未接管", "Renderer Not Attached")}</summary>
          <div class="stack flow-top">
            <div class="feedback-summary">
              {tr(
                locale(),
                "页面先使用 host fallback；正式玩法摘要、目标和指挥主链继续可用。",
                "The page is using host fallback first; formal gameplay summary, target, and command flows remain available.",
              )}
            </div>
            <Show when={rendererFatal()}>
              <div class="feedback-detail">{`${rendererFatal().code}: ${rendererFatal().message}`}</div>
            </Show>
          </div>
        </details>
      </Show>
      <Show when={focusMode() && renderState().commercial_surface && !maximized()}>
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
          class="pixel-world-focus-drawer pixel-world-focus-drawer--command"
          open={commandDrawerOpen()}
          onToggle={(event) => setPersistentCommandDrawerOpen(event.currentTarget.open)}
        >
          <summary>{tr(locale(), "命令与目标", "Command and Target")}</summary>
          <div class="pixel-world-focus-drawer__body">
            <PixelWorldFocusCommandSurface locale={locale} />
          </div>
        </details>
      </Show>
      <Show when={!focusMode() || !maximized()}>
        <details class="diagnostic pixel-world-render-diagnostics">
          <summary>{tr(locale(), "Renderer 诊断", "Renderer Diagnostics")}</summary>
          <div class="pixel-world-host__toolbar badge-row">
            <span class="badge badge--accent">{`locations=${visualState().locations.length}`}</span>
            <span class="badge badge--accent">{`fragments=${visualState().fragmentTerrain.length}`}</span>
            <span class="badge badge--accent">{`agents=${visualState().agents.length}`}</span>
            <Show when={renderState().world_tick !== null && renderState().world_tick !== undefined}>
              <span class="badge badge--accent" data-world-tick={String(renderState().world_tick)}>{`tick=${renderState().world_tick}`}</span>
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
      </Show>
      <Show when={focusMode()}>
        <details
          class="pixel-world-focus-drawer pixel-world-focus-drawer--diagnostics"
          open={diagnosticsDrawerOpen()}
          onToggle={(event) => setPersistentDiagnosticsDrawerOpen(event.currentTarget.open)}
        >
          <summary>{tr(locale(), "沉浸诊断", "Focus Diagnostics")}</summary>
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
              <button type="button" onClick={setFallbackMode}>
                {tr(locale(), "切回 Host Fallback", "Back To Host Fallback")}
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
