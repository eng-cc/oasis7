import { createEffect, createMemo, createSignal, For, Show, onCleanup } from "solid-js";

import * as core from "./legacy_core.js";
import { createPixelWorldRuntimeBridge } from "./pixel_world_runtime_loader.js";

function tr(locale, zh, en) {
  return core.isLocaleZh(locale) ? zh : en;
}

const PIXEL_WORLD_RUNTIME_CANVAS_ID = "pixel-world-embedded-runtime-canvas";

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
  return {
    async mount(canvas, renderState) {
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
      runtimeSource = runtime.source;
      runtimeModuleUrl = runtime.moduleUrl || null;
      const result = bridge.mount(canvas, renderState);
      return {
        status: result?.status || "ready",
        selection: renderState.selection,
        fatal: result?.fatal || null,
        runtimeSource,
        runtimeModuleUrl,
      };
    },
    update(renderState) {
      const result = bridge?.update(renderState) || { status: "detached" };
      return {
        status: result?.status || "ready",
        selection: renderState.selection,
        fatal: result?.fatal || null,
        runtimeSource,
        runtimeModuleUrl,
      };
    },
    unmount() {
      const result = bridge?.unmount() || { status: "detached" };
      bridge = null;
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
  };
}

export function buildPixelWorldRenderState(locale = core.state.uiLocale) {
  const lists = core.modelLists();
  const gameplay = core.buildGameplaySummary(locale);
  const worldScaleSurface = core.buildWorldScaleSurface(locale);
  const snapshot = core.state.snapshot;
  const selected = core.clone(core.state.selectedObject);
  const space = snapshot?.config?.space || null;

  const worldBounds = space
    ? {
        width_cm: Number(space.width_cm) || 0,
        depth_cm: Number(space.depth_cm) || 0,
        height_cm: Number(space.height_cm) || 0,
      }
    : null;
  const worldScaleBase = Math.max(1, Math.min(worldBounds?.width_cm || 1, worldBounds?.depth_cm || 1));

  const locations = lists.locations
    .map((location) => ({
      id: location.id,
      label: location.name || location.id,
      pos: normalizePosition(location.pos),
      radius_cm: Number(location?.profile?.radius_cm) || 0,
      resource_summary: core.resourceSummary(location.resources),
      resource_score: countResourceEntries(core.resourceSummary(location.resources)),
      size_hint_px: 16 + Math.min(
        18,
        (((Number(location?.profile?.radius_cm) || 0) / worldScaleBase) * 420)
          + (countResourceEntries(core.resourceSummary(location.resources)) * 2),
      ),
    }))
    .filter((location) => location.pos);

  const agents = lists.agents.map((agent) => ({
    id: agent.id,
    label: agent.name || agent.id,
    location_id: agent.location_id || null,
    pos: normalizePosition(agent.pos || (selected?.id === agent.id ? selected?.pos : null)),
    resource_summary: core.resourceSummary(agent.resources),
    resource_score: countResourceEntries(core.resourceSummary(agent.resources)),
    status_badges: [
      agent.location_id ? `location=${agent.location_id}` : null,
      agent.kind ? `kind=${agent.kind}` : null,
    ].filter(Boolean),
    size_hint_px: 12 + Math.min(
      10,
      (countResourceEntries(core.resourceSummary(agent.resources)) * 2)
        + (agent.location_id ? 2 : 0)
        + (agent.kind ? 1 : 0),
    ),
  }));

  const selection = core.state.selectedKind && core.state.selectedId
    ? {
        kind: core.state.selectedKind,
        id: core.state.selectedId,
      }
    : null;
  const locationById = new Map(locations.map((location) => [location.id, location]));
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
        kind: gameplay.blockerKind || "blocked",
        detail: gameplay.blockerDetail || null,
      }
    : null;
  const recentEventHotspots = buildRecentEventHotspots(core.state.recentEvents);
  const visualHotspots = buildVisualHotspots({
    worldBounds,
    anchor,
    goalHighlight,
    blockerHighlight,
    recentEventHotspots,
  });

  return {
    locale,
    world_bounds: worldBounds,
    locations,
    agents,
    links,
    selection,
    goal_highlight: goalHighlight,
    blocker_highlight: blockerHighlight,
    recent_event_hotspots: recentEventHotspots,
    visual_hotspots: visualHotspots,
    presentation: {
      world_bounds_label: worldScaleSurface.physicalTruth.worldBoundsLabel,
      marker_truth_note: worldScaleSurface.presentationScale.markerTruthNote,
    },
  };
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
    props.renderState();
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

function PixelWorldCanvasPlaceholder(props) {
  return (
    <div class="pixel-world-canvas" data-renderer-ready={props.ready() ? "true" : "false"}>
      <div class="pixel-world-canvas__grid" />
      <For each={props.renderState().locations.slice(0, 8)}>
        {(location, index) => (
          <button
            class="pixel-world-entity pixel-world-entity--location"
            style={{
              left: `${12 + ((index() % 4) * 21)}%`,
              top: `${18 + (Math.floor(index() / 4) * 26)}%`,
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
            style={{
              left: `${18 + ((index() % 5) * 15)}%`,
              top: `${14 + (Math.floor(index() / 5) * 22)}%`,
            }}
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
  const renderState = createMemo(() => buildPixelWorldRenderState(locale()));
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
    const result = adapter().update(renderState());
    if (result?.fatal) {
      setRendererFatal(result.fatal);
    }
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
    const result = await adapter().mount(mountedCanvas, renderState());
    if (result?.fatal) {
      setRendererFatal(result.fatal);
    }
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
          {tr(locale(), "嵌入式像素世界层（Host Skeleton）", "Embedded Pixel World Layer (Host Skeleton)")}
        </div>
        <div class="feedback-detail">
          {tr(
            locale(),
            "当前世界舞台优先依赖 wasm bridge、嵌入式 canvas、轻量拖拽缩放和事件回传。若 wasm bridge 缺失或启动失败，页面会显式退回 host fallback，而不是继续保留一套 JS renderer。",
            "The world stage now depends on the wasm bridge, embedded canvas, light pan-zoom interaction, and event callbacks. If the wasm bridge is missing or fails to boot, the page falls back explicitly instead of keeping a second JS renderer.",
          )}
        </div>
      </div>
      <div class="pixel-world-host__toolbar badge-row">
        <span class="badge badge--accent">{`locations=${renderState().locations.length}`}</span>
        <span class="badge badge--accent">{`agents=${renderState().agents.length}`}</span>
        <span class="badge">{`links=${renderState().links.length}`}</span>
        <span class="badge">{`hotspots=${renderState().visual_hotspots.length}`}</span>
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
      </div>
      <Show when={rendererStatus() !== "fallback"}>
        <PixelWorldCanvasRenderer
          locale={locale}
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
      <details class="diagnostic">
        <summary>{tr(locale(), "展开 Render DTO", "Expand Render DTO")}</summary>
        <div class="stack" style="margin-top:10px;">
          <pre class="json">{JSON.stringify(renderState(), null, 2)}</pre>
        </div>
      </details>
    </div>
  );
}
