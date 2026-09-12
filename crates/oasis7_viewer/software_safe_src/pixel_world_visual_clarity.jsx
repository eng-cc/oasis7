import { For, Index, Show } from "solid-js";
import * as core from "./legacy_core.js";
import { pixelWorldEntityMarkerCode, pixelWorldReadableAgentLabel, pixelWorldReadableModuleLabel } from "./pixel_world_identity.js";
import { pixelWorldSparseScenePresentation } from "./pixel_world_presentation.js";

const FRAGMENT_TERRAIN_PALETTE = {
  silicate_matrix: [126, 144, 99], iron_nickel_alloy: [176, 184, 196], water_ice: [125, 211, 252],
  hydrated_mineral: [96, 165, 250], carbonaceous_organic: [120, 113, 108], sulfide_ore: [202, 138, 4],
  rare_earth_oxide: [167, 139, 250], uranium_bearing_ore: [132, 204, 22], thorium_bearing_ore: [244, 114, 182],
  unknown: [148, 163, 184],
};

function tr(locale, zh, en) { return core.isLocaleZh(locale) ? zh : en; }
function safeNumber(value, fallback = 0) { const number = Number(value); return Number.isFinite(number) ? number : fallback; }
function colorToCss(color, alpha = 0.36) {
  const [red, green, blue] = Array.isArray(color) ? color : FRAGMENT_TERRAIN_PALETTE.unknown;
  return `rgba(${red}, ${green}, ${blue}, ${alpha})`;
}
function clampRatio(value) { return Math.min(1, Math.max(0, Number(value) || 0)); }
export function toWorldPercentStyle(pos, worldBounds, fallbackStyle) {
  if (!pos || !worldBounds) return fallbackStyle;
  const point = worldPercentPoint(pos, worldBounds, 8, 10);
  return { left: `${point.x.toFixed(1)}%`, top: `${point.y.toFixed(1)}%` };
}
export function agentMarkerStyle(agent, index, worldBounds) {
  const base = toWorldPercentStyle(agent.pos, worldBounds, { left: `${18 + ((index % 5) * 15)}%`, top: `${14 + (Math.floor(index / 5) * 22)}%` });
  const offsets = [[-18, -18], [18, -18], [-18, 18], [18, 18], [0, -30], [0, 30], [-30, 0], [30, 0], [-28, -28], [28, 28]];
  const [x, y] = offsets[index % offsets.length] || [0, 0];
  return { ...base, transform: `translate(${x}px, ${y}px)` };
}
export function moduleMarkerStyle(module, index, worldBounds) {
  const base = toWorldPercentStyle(module.pos, worldBounds, { left: `${18 + ((index % 5) * 15)}%`, top: `${14 + (Math.floor(index / 5) * 22)}%` });
  const offsets = [[0, 0], [-12, -12], [12, -12], [-12, 12], [12, 12], [-24, 0], [24, 0], [0, -24], [0, 24]];
  const [x, y] = offsets[index % offsets.length] || [0, 0];
  return { ...base, transform: `translate(${x}px, ${y}px)` };
}
function worldPercentPoint(pos, worldBounds, fallbackX = 50, fallbackY = 50) {
  if (!pos || !worldBounds) return { x: fallbackX, y: fallbackY };
  return { x: 8 + (clampRatio(pos.x_cm / Math.max(1, worldBounds.width_cm)) * 84), y: 10 + (clampRatio(pos.y_cm / Math.max(1, worldBounds.depth_cm)) * 78) };
}
const FALLBACK_ROUTE_HEIGHT_TO_WIDTH_RATIO = 9 / 16;
export function routeStyle(link, worldBounds, index) {
  const fallbackFrom = { x: 14 + ((index % 5) * 15), y: 18 + (Math.floor(index / 5) * 14) };
  const fallbackTo = { x: fallbackFrom.x + 14, y: fallbackFrom.y + 8 };
  const from = worldPercentPoint(link.from, worldBounds, fallbackFrom.x, fallbackFrom.y);
  const to = worldPercentPoint(link.to, worldBounds, fallbackTo.x, fallbackTo.y);
  const deltaX = to.x - from.x; const deltaY = to.y - from.y;
  const scaledDeltaY = deltaY * FALLBACK_ROUTE_HEIGHT_TO_WIDTH_RATIO;
  const length = Math.max(4, Math.hypot(deltaX, scaledDeltaY));
  const angle = Math.atan2(scaledDeltaY, deltaX) * (180 / Math.PI);
  return { left: `${from.x.toFixed(1)}%`, top: `${from.y.toFixed(1)}%`, width: `${length.toFixed(1)}%`, opacity: `${0.32 + (clampRatio(link.emphasis ?? 0.72) * 0.38)}`, transform: `rotate(${angle.toFixed(1)}deg)`, "transform-origin": "0 50%" };
}
export function fragmentTerrainStyle(patch, worldBounds, index) {
  const sizePx = Math.max(12, Math.min(48, safeNumber(patch.footprint_cm, 1) / 840));
  return { ...toWorldPercentStyle(patch.pos, worldBounds, { left: `${12 + ((index % 6) * 13)}%`, top: `${16 + (Math.floor(index / 6) * 13)}%` }), width: `${sizePx.toFixed(1)}px`, height: `${sizePx.toFixed(1)}px`, "background-color": colorToCss(patch.color), transform: "translate(-50%, -50%)" };
}
export function routeWaypointStyle(link, worldBounds, index, stop) {
  const fallbackFrom = { x: 14 + ((index % 5) * 15), y: 18 + (Math.floor(index / 5) * 14) };
  const fallbackTo = { x: fallbackFrom.x + 14, y: fallbackFrom.y + 8 };
  const from = worldPercentPoint(link.from, worldBounds, fallbackFrom.x, fallbackFrom.y);
  const to = worldPercentPoint(link.to, worldBounds, fallbackTo.x, fallbackTo.y);
  const ratio = stop === "to" ? 1 : 0.52;
  return { left: `${(from.x + ((to.x - from.x) * ratio)).toFixed(1)}%`, top: `${(from.y + ((to.y - from.y) * ratio)).toFixed(1)}%` };
}
export function fieldValue(value, snakeName, camelName, fallback = undefined) {
  if (!value || typeof value !== "object") return fallback;
  if (value[snakeName] !== undefined) return value[snakeName];
  if (camelName && value[camelName] !== undefined) return value[camelName];
  return fallback;
}
const AUTHORITATIVE_LINK_KINDS = new Set([
  "agent_assignment",
  "route",
  "logistics",
  "logistics_route",
  "supply_route",
  "delivery_route",
  "resource_flow",
  "resource_transfer",
  "material_transfer",
  "material_transit",
]);
function hasCurrentRuntimeLinkAuthority(link) {
  const kind = String(fieldValue(link, "kind", "kind", "")).trim();
  return AUTHORITATIVE_LINK_KINDS.has(kind)
    && fieldValue(link, "status", "status", null) === "active"
    && fieldValue(link, "source_class", "sourceClass", null) === "runtime_projection"
    && fieldValue(link, "freshness", "freshness", null) === "current";
}
function explicitLinkEndpointIds(link) {
  if (!link || typeof link !== "object") return { agent: [], location: [] };
  const endpoint = (names) => names.map((name) => fieldValue(link, name, name.replace(/_([a-z])/g, (_, character) => character.toUpperCase()), null)).filter((value) => value != null && String(value).trim()).map((value) => String(value).trim());
  const explicit = {
    agent: endpoint(["agent_id", "source_agent_id", "target_agent_id", "from_agent_id", "to_agent_id"]),
    location: endpoint(["location_id", "source_location_id", "target_location_id", "from_location_id", "to_location_id"]),
  };
  return explicit;
}
function hasCurrentRuntimeRelation(agent, expectedKind) {
  const envelope = [agent?.relation, agent?.assignment].find((candidate) => candidate && typeof candidate === "object");
  return fieldValue(envelope, "kind", "kind", null) === expectedKind
    && envelope?.status === "active"
    && envelope?.source_class === "runtime_projection"
    && envelope?.freshness === "current";
}
function linkReferencesSelection(link, selection, agents) {
  if (!selection?.id || !selection?.kind) return false;
  if (!hasCurrentRuntimeLinkAuthority(link)) return false;
  const endpointIds = explicitLinkEndpointIds(link);
  if ((selection.kind === "agent" ? endpointIds.agent : endpointIds.location).includes(String(selection.id))) {
    return true;
  }
  const linkKind = fieldValue(link, "kind", "kind", null);
  return agents.some((agent) => {
    const agentId = String(agent?.id || "").trim();
    const locationId = String(fieldValue(agent, "location_id", "locationId", "")).trim();
    if (!agentId || !locationId || !hasCurrentRuntimeRelation(agent, linkKind)) {
      return false;
    }
    if (link.id !== `link:${agentId}:${locationId}`) {
      return false;
    }
    return selection.kind === "agent" ? selection.id === agentId : selection.id === locationId;
  });
}
function terrainReferencesSelection(patch, selection) {
  return selection?.kind === "location" && String(fieldValue(patch, "location_id", "locationId", "")).trim() === String(selection.id);
}
export function arrayField(value, snakeName, camelName) {
  const candidate = fieldValue(value, snakeName, camelName, []);
  return Array.isArray(candidate) ? candidate : [];
}
function normalizeVisualEntity(entry) {
  if (!entry || typeof entry !== "object") return entry;
  return { ...entry, location_id: fieldValue(entry, "location_id", "locationId", null), marker_role: fieldValue(entry, "marker_role", "markerRole", null), marker_alpha: fieldValue(entry, "marker_alpha", "markerAlpha", undefined), position_source: fieldValue(entry, "position_source", "positionSource", null), dominant_compound: fieldValue(entry, "dominant_compound", "dominantCompound", undefined), footprint_cm: fieldValue(entry, "footprint_cm", "footprintCm", undefined), module_id: fieldValue(entry, "module_id", "moduleId", null), anchor: fieldValue(entry, "anchor", "anchor", null) };
}
export function pixelWorldVisualState(renderState) {
  const state = renderState || {};
  return { worldBounds: fieldValue(state, "world_bounds", "worldBounds", null), fragmentTerrain: arrayField(state, "fragment_terrain", "fragmentTerrain").map(normalizeVisualEntity), links: arrayField(state, "links", "links"), locations: arrayField(state, "locations", "locations").map(normalizeVisualEntity), agents: arrayField(state, "agents", "agents").map(normalizeVisualEntity), moduleVisualEntities: arrayField(state, "module_visual_entities", "moduleVisualEntities").map(normalizeVisualEntity), selection: fieldValue(state, "selection", "selection", null), goalHighlight: fieldValue(state, "goal_highlight", "goalHighlight", null), blockerHighlight: fieldValue(state, "blocker_highlight", "blockerHighlight", null), visualHotspots: arrayField(state, "visual_hotspots", "visualHotspots").map(normalizeVisualEntity) };
}

export function PixelWorldCanvasAgentHitTargets(props) {
  const visualState = () => pixelWorldVisualState(props.renderState());
  return <>
    <For each={visualState().agents.slice(0, 10)}>{(agent, index) => {
      const label = pixelWorldReadableAgentLabel(agent, agent.id, core.isLocaleZh(props.locale()));
      return <button type="button" class="pixel-world-entity pixel-world-entity--agent pixel-world-entity--canvas-hit-target" data-pixel-world-agent-marker="true" data-agent-id={agent.id} data-marker-code={pixelWorldEntityMarkerCode(agent, agent.id, "agent")} data-position-source={agent.position_source} data-selected={props.selection()?.kind === "agent" && props.selection()?.id === agent.id ? "true" : "false"} aria-pressed={props.selection()?.kind === "agent" && props.selection()?.id === agent.id ? "true" : "false"} aria-label={`${tr(props.locale(), "选择 Agent", "Select Agent")} ${label}`} style={agentMarkerStyle(agent, index(), visualState().worldBounds)} title={label} onMouseEnter={() => props.onHover({ kind: "agent", id: agent.id })} onMouseLeave={() => props.onHover(null)} onClick={() => props.onSelect({ kind: "agent", id: agent.id })}>
      <span class="pixel-world-entity__code">{pixelWorldEntityMarkerCode(agent, agent.id, "agent")}</span>
      </button>;
    }}</For>
    <For each={visualState().moduleVisualEntities.slice(0, 24)}>{(module, index) => {
      const label = pixelWorldReadableModuleLabel(module, module.id, core.isLocaleZh(props.locale()));
      return <button type="button" class="pixel-world-entity pixel-world-entity--module pixel-world-entity--canvas-hit-target" data-pixel-world-module-marker="true" data-module-id={module.id} data-module-kind={module.kind} data-module-label={module.label || undefined} data-marker-code={pixelWorldEntityMarkerCode(module, module.id, "module_visual")} data-selected={props.selection()?.kind === "module_visual" && props.selection()?.id === module.id ? "true" : "false"} aria-pressed={props.selection()?.kind === "module_visual" && props.selection()?.id === module.id ? "true" : "false"} aria-label={`${tr(props.locale(), "选择模块", "Select Module")} ${label}`} style={moduleMarkerStyle(module, index(), visualState().worldBounds)} title={label} onMouseEnter={() => props.onHover({ kind: "module_visual", id: module.id })} onMouseLeave={() => props.onHover(null)} onClick={() => props.onSelect({ kind: "module_visual", id: module.id })}>
        <span class="pixel-world-entity__code">{pixelWorldEntityMarkerCode(module, module.id, "module_visual")}</span>
      </button>;
    }}</For>
  </>;
}

export function PixelWorldHostVisualLayer(props) {
  const enabled = () => typeof props.enabled === "function" ? props.enabled() : props.enabled;
  const visualState = () => pixelWorldVisualState(props.renderState());
  const selection = () => props.selection?.() || visualState().selection;
  const projectedAgents = () => visualState().agents;
  return <Show when={enabled()}>
    <div class="pixel-world-canvas__grid" /><div class="pixel-world-canvas__terrain-band pixel-world-canvas__terrain-band--one" /><div class="pixel-world-canvas__terrain-band pixel-world-canvas__terrain-band--two" />
    <For each={visualState().fragmentTerrain.slice(0, 96)}>{(patch, index) => <div class={`pixel-world-fragment-terrain${terrainReferencesSelection(patch, selection()) ? " pixel-world-fragment-terrain--associated" : selection() ? " pixel-world-fragment-terrain--muted" : ""}`} data-compound={patch.dominant_compound} data-associated={terrainReferencesSelection(patch, selection()) ? "true" : "false"} style={fragmentTerrainStyle(patch, visualState().worldBounds, index())} title={`${patch.location_id}:${patch.dominant_compound}`} />}</For>
    <For each={visualState().links.slice(0, 10)}>{(link, index) => <>
      <div class={`pixel-world-route${linkReferencesSelection(link, selection(), projectedAgents()) ? " pixel-world-route--associated" : selection() ? " pixel-world-route--muted" : ""}`} data-route-id={link.id} data-route-kind={link.kind} data-associated={linkReferencesSelection(link, selection(), projectedAgents()) ? "true" : "false"} style={routeStyle(link, visualState().worldBounds, index())} title={`${link.kind}:${link.id}`} />
      <div class={`pixel-world-route-waypoint pixel-world-route-waypoint--mid${linkReferencesSelection(link, selection(), projectedAgents()) ? " pixel-world-route-waypoint--associated" : selection() ? " pixel-world-route-waypoint--muted" : ""}`} data-route-id={link.id} data-route-kind={link.kind} data-associated={linkReferencesSelection(link, selection(), projectedAgents()) ? "true" : "false"} style={routeWaypointStyle(link, visualState().worldBounds, index(), "mid")} title={`${link.kind}:waypoint`} />
      <div class={`pixel-world-route-waypoint pixel-world-route-waypoint--target${linkReferencesSelection(link, selection(), projectedAgents()) ? " pixel-world-route-waypoint--associated" : selection() ? " pixel-world-route-waypoint--muted" : ""}`} data-route-id={link.id} data-route-kind={link.kind} data-associated={linkReferencesSelection(link, selection(), projectedAgents()) ? "true" : "false"} style={routeWaypointStyle(link, visualState().worldBounds, index(), "to")} title={`${link.kind}:target`} />
    </>}</For>
    <Index each={visualState().locations.slice(0, 8)}>{(location, index) => <button class="pixel-world-entity pixel-world-entity--location" data-pixel-world-location-marker="true" data-location-id={location().id} data-selected={selection()?.kind === "location" && selection()?.id === location().id ? "true" : "false"} data-marker-code={pixelWorldEntityMarkerCode(location(), location().id, "location")} aria-pressed={selection()?.kind === "location" && selection()?.id === location().id ? "true" : "false"} aria-label={`${tr(props.locale(), "选择地点", "Select Location")} ${location().label || location().id}`} data-marker-role={location().marker_role} style={{ ...toWorldPercentStyle(location().pos, visualState().worldBounds, { left: `${12 + ((index % 4) * 21)}%`, top: `${18 + (Math.floor(index / 4) * 26)}%` }), opacity: location().marker_alpha }} title={location().label} onMouseEnter={() => props.onHover({ kind: "location", id: location().id })} onMouseLeave={() => props.onHover(null)} onClick={() => props.onSelect({ kind: "location", id: location().id })}>
      <span class="pixel-world-entity__code">{pixelWorldEntityMarkerCode(location(), location().id, "location")}</span>
    </button>}</Index>
    <Index each={visualState().agents.slice(0, 10)}>{(agent, index) => { const label = () => pixelWorldReadableAgentLabel(agent(), agent().id, core.isLocaleZh(props.locale())); return <button class="pixel-world-entity pixel-world-entity--agent" data-pixel-world-agent-marker="true" data-agent-id={agent().id} data-selected={selection()?.kind === "agent" && selection()?.id === agent().id ? "true" : "false"} data-marker-code={pixelWorldEntityMarkerCode(agent(), agent().id, "agent")} data-position-source={agent().position_source} aria-pressed={selection()?.kind === "agent" && selection()?.id === agent().id ? "true" : "false"} aria-label={`${tr(props.locale(), "选择 Agent", "Select Agent")} ${label()}`} style={agentMarkerStyle(agent(), index, visualState().worldBounds)} title={label()} onMouseEnter={() => props.onHover({ kind: "agent", id: agent().id })} onMouseLeave={() => props.onHover(null)} onClick={() => props.onSelect({ kind: "agent", id: agent().id })}>
      <span class="pixel-world-entity__code">{pixelWorldEntityMarkerCode(agent(), agent().id, "agent")}</span>
    </button>; }}</Index>
    <Index each={visualState().moduleVisualEntities.slice(0, 24)}>{(module, index) => { const label = () => pixelWorldReadableModuleLabel(module(), module().id, core.isLocaleZh(props.locale())); return <button type="button" class="pixel-world-entity pixel-world-entity--module" data-pixel-world-module-marker="true" data-module-id={module().id} data-module-kind={module().kind} data-module-label={module().label || undefined} data-selected={selection()?.kind === "module_visual" && selection()?.id === module().id ? "true" : "false"} data-marker-code={pixelWorldEntityMarkerCode(module(), module().id, "module_visual")} aria-pressed={selection()?.kind === "module_visual" && selection()?.id === module().id ? "true" : "false"} aria-label={`${tr(props.locale(), "选择模块", "Select Module")} ${label()}`} style={moduleMarkerStyle(module(), index, visualState().worldBounds)} title={label()} onMouseEnter={() => props.onHover({ kind: "module_visual", id: module().id })} onMouseLeave={() => props.onHover(null)} onClick={() => props.onSelect({ kind: "module_visual", id: module().id })}>
      <span class="pixel-world-entity__code">{pixelWorldEntityMarkerCode(module(), module().id, "module_visual")}</span>
    </button>; }}</Index>
  </Show>;
}

export function PixelWorldCanvasLegend(props) {
  return <div class="pixel-world-canvas__legend" data-pixel-world-legend="true" aria-label={tr(props.locale(), "世界图例", "World legend")}>
    <div class="pixel-world-canvas__legend-title">{tr(props.locale(), "图例", "Legend")}</div>
    <div class="pixel-world-canvas__legend-item pixel-world-canvas__legend-item--route"><span class="pixel-world-canvas__legend-swatch" aria-hidden="true" /><span>{tr(props.locale(), "路线", "Route")}</span></div>
    <div class="pixel-world-canvas__legend-item pixel-world-canvas__legend-item--goal"><span class="pixel-world-canvas__legend-swatch" aria-hidden="true">◆</span><span>{tr(props.locale(), "目标", "Goal")}</span></div>
    <div class="pixel-world-canvas__legend-item pixel-world-canvas__legend-item--blocker"><span class="pixel-world-canvas__legend-swatch" aria-hidden="true">!</span><span>{tr(props.locale(), "阻塞", "Blocker")}</span></div>
    <div class="pixel-world-canvas__legend-item pixel-world-canvas__legend-item--resource"><span class="pixel-world-canvas__legend-swatch" aria-hidden="true">▪</span><span>{tr(props.locale(), "资源地形", "Resource terrain")}</span></div>
  </div>;
}

export function PixelWorldSparseSceneGuidance(props) {
  const visualState = () => pixelWorldVisualState(typeof props.renderState === "function" ? props.renderState() : props.renderState);
  const presentation = () => pixelWorldSparseScenePresentation({
    routeCount: visualState().links.length,
    terrainCount: visualState().fragmentTerrain.length,
    locationCount: visualState().locations.length,
    agentCount: visualState().agents.length,
    worldBounds: visualState().worldBounds,
  }, typeof props.locale === "function" ? props.locale() : props.locale);
  return (
    <Show when={presentation().hasSparseTopology}>
      <div
        class="pixel-world-canvas__sparse-guidance"
        data-pixel-world-sparse-guidance="true"
        aria-label={tr(typeof props.locale === "function" ? props.locale() : props.locale, "当前世界的已发布数据范围", "Published data coverage for this world")}
      >
        <div class="pixel-world-canvas__sparse-title">{tr(typeof props.locale === "function" ? props.locale() : props.locale, "已发布数据范围", "Published data coverage")}</div>
        <div data-sparse-field="entities">{presentation().entities}</div>
        <div data-sparse-field="routes">{presentation().routes}</div>
        <div data-sparse-field="terrain">{presentation().terrain}</div>
        <div data-sparse-field="bounds">{presentation().bounds}</div>
      </div>
    </Show>
  );
}
