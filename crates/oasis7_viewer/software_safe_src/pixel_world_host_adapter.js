import { createPixelWorldRuntimeBridge } from "./pixel_world_runtime_loader.js";
function snapshotTick(snapshot) {
  if (!snapshot || typeof snapshot !== "object") {
    return null;
  }
  const tick = Number(snapshot.time);
  return Number.isFinite(tick) ? Math.max(0, Math.floor(tick)) : null;
}
export function createPixelWorldHostAdapter({ onSelectEntity, onHoverEntity, onFatal }) {
  let bridge = null;
  let generation = 0;
  const detached = () => ({ status: "detached", renderState: null, selection: null, fatal: null });
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
      const attempt = ++generation;
      bridge?.unmount();
      bridge = null;
      deriveRenderState = null;
      const isCurrent = () => attempt === generation;
      const runtime = await createPixelWorldRuntimeBridge({
        onEvent(event) {
          if (!isCurrent()) return;
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
        onFatal: (...args) => { if (isCurrent()) onFatal?.(...args); },
      });
      if (!isCurrent()) { runtime.bridge.unmount(); return detached(); }
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
        const result = await runtime.bridge.mount(canvas, mountedRenderState);
        if (!isCurrent()) { runtime.bridge.unmount(); return detached(); }
        return { status: result?.status || "ready", selection: mountedRenderState.selection, fatal: result?.fatal || null, renderState: mountedRenderState, runtimeSource, runtimeModuleUrl };
      } catch (error) {
        if (!isCurrent()) { runtime.bridge.unmount(); return detached(); }
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
      generation += 1;
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
