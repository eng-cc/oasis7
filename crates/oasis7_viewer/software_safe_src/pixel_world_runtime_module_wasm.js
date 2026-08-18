import initPixelWorldBridgeModule, {
  PixelWorldBridge,
  build_pixel_world_render_state,
} from "./pixel_world_bridge_bindgen.js";

export const PIXEL_WORLD_RUNTIME_SOURCE = "wasm_bindgen_runtime";
const DRAG_CLICK_SUPPRESSION_THRESHOLD_PX = 4;
const HOTSPOT_TEST_READBACK_CONTRACT = "oasis7_hotspot_pointer_evidence_v1";
const AMBIENT_FRAME_INTERVAL_MS = 1000 / 12;
const LOCATION_TEST_READBACK_CONTRACT = "oasis7_location_frame_evidence_v1";

let runtimeInitPromise = null;

const PIXEL_WORLD_WASM_PAYLOAD_NAME = "pixel_world_bridge_bindgen_bg.wasm";
const PIXEL_WORLD_OPTIONAL_PAYLOAD_MANIFEST_NAME = "optional-payloads.json";
const PIXEL_WORLD_OPTIONAL_PAYLOAD_MISSING_CODE = "pixel_world_optional_payload_missing";

function legacyPixelWorldWasmUrl(moduleUrl = import.meta.url) {
  return new URL(PIXEL_WORLD_WASM_PAYLOAD_NAME, moduleUrl);
}

function optionalPayloadManifestUrl(moduleUrl = import.meta.url) {
  // The generated bridge lives at pixel-world-bridge/webgl2/. The manifest is
  // written at the primary viewer-dist root by copy-viewer-web-dist.sh.
  return new URL(`../../${PIXEL_WORLD_OPTIONAL_PAYLOAD_MANIFEST_NAME}`, moduleUrl);
}

function configuredOptionalPayloadBaseUrl(manifestUrl) {
  const configured = globalThis.__OASIS7_VIEWER_OPTIONAL_PAYLOAD_BASE_URL__;
  if (typeof configured === "string" && configured.trim()) {
    return new URL(configured, manifestUrl);
  }
  if (typeof window !== "undefined" && window.location) {
    const queryBase = new URL(window.location.href).searchParams.get("optional_payload_base");
    if (queryBase) {
      return new URL(queryBase, manifestUrl);
    }
  }
  return manifestUrl;
}

function optionalPayloadMissingError(reason = "source_missing") {
  const error = new Error(`optional pixel world WASM payload is unavailable: ${reason}`);
  error.code = PIXEL_WORLD_OPTIONAL_PAYLOAD_MISSING_CODE;
  return error;
}

async function resolvePixelWorldWasmUrl({ moduleUrl = import.meta.url } = {}) {
  const fallbackUrl = legacyPixelWorldWasmUrl(moduleUrl);
  // Node/jsdom unit tests and source-tree development do not serve a JSON
  // manifest. Keep the legacy adjacent-WASM path in those environments.
  if (typeof window === "undefined" || typeof fetch !== "function") {
    return fallbackUrl;
  }

  const manifestUrl = optionalPayloadManifestUrl(moduleUrl);
  if (manifestUrl.protocol !== "http:" && manifestUrl.protocol !== "https:") {
    return fallbackUrl;
  }
  let response;
  try {
    response = await fetch(manifestUrl, { cache: "no-store" });
  } catch (error) {
    throw optionalPayloadMissingError(error instanceof Error ? error.message : "manifest_fetch_failed");
  }
  if (response.status === 404) {
    return fallbackUrl;
  }
  if (!response.ok) {
    throw optionalPayloadMissingError(`manifest_http_${response.status}`);
  }

  let manifest;
  try {
    manifest = await response.json();
  } catch (error) {
    throw optionalPayloadMissingError(error instanceof Error ? error.message : "manifest_invalid_json");
  }
  const payload = manifest?.[PIXEL_WORLD_WASM_PAYLOAD_NAME];
  if (payload?.available !== true || typeof payload.path !== "string" || !payload.path.trim()) {
    throw optionalPayloadMissingError(payload?.reason || "source_missing");
  }

  return new URL(payload.path, configuredOptionalPayloadBaseUrl(manifestUrl));
}

function pixelWorldTestApiEnabled() {
  return new URLSearchParams(window.location.search || "").get("test_api") === "1";
}

function ensurePixelWorldBridgeModule() {
  if (!runtimeInitPromise) {
    runtimeInitPromise = resolvePixelWorldWasmUrl().then((wasmUrl) => (
      initPixelWorldBridgeModule(wasmUrl)
    ));
  }
  return runtimeInitPromise;
}

function toCanvasPoint(canvas, event) {
  const rect = canvas.getBoundingClientRect();
  if (!rect.width || !rect.height) {
    return null;
  }
  return {
    x: event.clientX - rect.left,
    y: event.clientY - rect.top,
  };
}

export async function createPixelWorldBridge({ onEvent, onFatal } = {}) {
  await ensurePixelWorldBridgeModule();

  let mountedCanvas = null;
  let animationFrameId = null;
  let removeCanvasListeners = () => {};
  let dragState = null;
  let suppressNextClick = false;
  let lastAmbientTickMs = Number.NEGATIVE_INFINITY;
  let reducedMotion = false;
  let documentVisible = true;
  let canvasVisible = true;
  let visibilityObserver = null;
  let removeLifecycleListeners = () => {};
  let animationGeneration = 0;

  const runtime = new PixelWorldBridge(
    (event) => {
      if (mountedCanvas && event?.type === "hover_entity") {
        mountedCanvas.style.cursor = event.selection ? "pointer" : "grab";
      }
      onEvent?.(event);
    },
    (fatal) => {
      onFatal?.(fatal);
    },
  );

  function stopAnimationLoop() {
    animationGeneration += 1;
    if (animationFrameId !== null) {
      cancelAnimationFrame(animationFrameId);
      animationFrameId = null;
    }
  }

  function canTick() {
    return documentVisible && canvasVisible;
  }

  function syncRuntime() {
    if (!canTick()) return;
    runtime.tick(performance.now());
  }

  function startAnimationLoop() {
    stopAnimationLoop();
    reducedMotion = globalThis.matchMedia?.("(prefers-reduced-motion: reduce)")?.matches === true;
    if (reducedMotion || !canTick()) return;
    const generation = animationGeneration;
    const tick = (animationMs) => {
      animationFrameId = null;
      if (generation !== animationGeneration || !canTick()) return;
      try {
        if (animationMs - lastAmbientTickMs >= AMBIENT_FRAME_INTERVAL_MS) {
          runtime.tick(animationMs);
          lastAmbientTickMs = animationMs;
        }
        if (generation === animationGeneration && canTick()) animationFrameId = requestAnimationFrame(tick);
      } catch (error) {
        stopAnimationLoop();
        onFatal?.({
          code: "pixel_world_renderer_fatal",
          message: error instanceof Error ? error.message : String(error || "renderer fatal"),
        });
      }
    };
    animationFrameId = requestAnimationFrame(tick);
  }

  function attachLifecycleHooks(canvas) {
    removeLifecycleListeners();
    documentVisible = document.hidden !== true;
    canvasVisible = true;
    const mediaQuery = globalThis.matchMedia?.("(prefers-reduced-motion: reduce)");
    const onMotionChange = (event) => {
      reducedMotion = event.matches === true;
      if (reducedMotion) stopAnimationLoop(); else resume();
    };
    mediaQuery?.addEventListener?.("change", onMotionChange);
    const resume = () => {
      if (!canTick()) return;
      try {
        syncRuntime();
        startAnimationLoop();
      } catch (error) {
        onFatal?.({ code: "pixel_world_renderer_fatal", message: error instanceof Error ? error.message : String(error) });
      }
    };
    const onVisibilityChange = () => {
      documentVisible = document.hidden !== true;
      if (documentVisible) resume(); else stopAnimationLoop();
    };
    document.addEventListener("visibilitychange", onVisibilityChange);
    const disposers = [
      () => document.removeEventListener("visibilitychange", onVisibilityChange),
      () => mediaQuery?.removeEventListener?.("change", onMotionChange),
    ];
    if (globalThis.IntersectionObserver) {
      visibilityObserver = new IntersectionObserver((entries) => {
        canvasVisible = entries.some((entry) => entry.isIntersecting);
        if (canvasVisible) resume(); else stopAnimationLoop();
      });
      visibilityObserver.observe(canvas);
      disposers.push(() => visibilityObserver?.disconnect());
    }
    removeLifecycleListeners = () => {
      for (const dispose of disposers.splice(0)) dispose();
      visibilityObserver = null;
    };
  }

  function cleanupCanvasListeners() {
    removeCanvasListeners();
    removeCanvasListeners = () => {};
    dragState = null;
    if (mountedCanvas) {
      mountedCanvas.style.cursor = "default";
    }
  }

  function attachCanvasListeners(canvas) {
    cleanupCanvasListeners();
    const disposers = [];

    const onPointerDown = (event) => {
      const point = toCanvasPoint(canvas, event);
      if (!point) {
        return;
      }
      dragState = {
        pointerId: event.pointerId,
        startX: point.x,
        startY: point.y,
        moved: false,
      };
      suppressNextClick = false;
      canvas.style.cursor = "grabbing";
      canvas.setPointerCapture?.(event.pointerId);
      runtime.pointer_down(point.x, point.y, event.pointerId);
      syncRuntime();
    };

    const onPointerMove = (event) => {
      const point = toCanvasPoint(canvas, event);
      if (!point) {
        return;
      }
      if (dragState && dragState.pointerId === event.pointerId) {
        const deltaX = point.x - dragState.startX;
        const deltaY = point.y - dragState.startY;
        dragState.moved = Math.hypot(deltaX, deltaY) > DRAG_CLICK_SUPPRESSION_THRESHOLD_PX;
      }
      runtime.pointer_move(point.x, point.y, false, event.pointerId);
      syncRuntime();
    };

    const onPointerLeave = (event) => {
      canvas.style.cursor = "default";
      runtime.pointer_move(0, 0, true, event.pointerId ?? -1);
      syncRuntime();
    };

    const onPointerUp = (event) => {
      runtime.pointer_up(event.pointerId);
      syncRuntime();
      canvas.releasePointerCapture?.(event.pointerId);
      canvas.style.cursor = "grab";
      // A cancelled pointer sequence cannot produce the compatibility click
      // emitted after a completed drag, so it must not suppress a later user click.
      suppressNextClick = event.type === "pointerup" && dragState?.moved === true;
      dragState = null;
    };

    const onWheel = (event) => {
      event.preventDefault();
      runtime.wheel(event.deltaY);
      syncRuntime();
    };

    const onClick = (event) => {
      const point = toCanvasPoint(canvas, event);
      if (!point) {
        return;
      }
      if (suppressNextClick) {
        suppressNextClick = false;
        return;
      }
      runtime.click(point.x, point.y);
      syncRuntime();
    };

    const bind = (name, handler, options) => {
      canvas.addEventListener(name, handler, options);
      disposers.push(() => canvas.removeEventListener(name, handler, options));
    };

    bind("pointerdown", onPointerDown);
    bind("pointermove", onPointerMove);
    bind("pointerleave", onPointerLeave);
    bind("pointerup", onPointerUp);
    bind("pointercancel", onPointerUp);
    bind("wheel", onWheel, { passive: false });
    bind("click", onClick);

    canvas.style.cursor = "grab";
    removeCanvasListeners = () => {
      for (const dispose of disposers.splice(0)) {
        dispose();
      }
      suppressNextClick = false;
    };
  }

  return {
    mount(canvas, renderState) {
      mountedCanvas = canvas;
      const result = runtime.mount(canvas, renderState);
      if (result?.status === "ready") {
        attachCanvasListeners(canvas);
        attachLifecycleHooks(canvas);
        syncRuntime();
        lastAmbientTickMs = performance.now();
        startAnimationLoop();
      } else {
        stopAnimationLoop();
        cleanupCanvasListeners();
        mountedCanvas = null;
      }
      return result;
    },
    update(renderState) {
      const result = runtime.update(renderState);
      syncRuntime();
      return result;
    },
    unmount() {
      stopAnimationLoop();
      removeLifecycleListeners();
      cleanupCanvasListeners();
      const result = runtime.unmount();
      mountedCanvas = null;
      return result;
    },
    hotspotTestHitTargets() {
      if (!pixelWorldTestApiEnabled()) {
        return [];
      }
      return runtime.hotspot_test_hit_targets(HOTSPOT_TEST_READBACK_CONTRACT) || [];
    },
    locationTestHitTargets() {
      if (!pixelWorldTestApiEnabled()) {
        return [];
      }
      return runtime.location_test_hit_targets(LOCATION_TEST_READBACK_CONTRACT) || [];
    },
  };
}

export function derivePixelWorldRenderState(input) {
  if (!runtimeInitPromise) {
    throw new Error("pixel world bridge module is not initialized");
  }
  return build_pixel_world_render_state(input);
}

export {
  PIXEL_WORLD_OPTIONAL_PAYLOAD_MANIFEST_NAME,
  PIXEL_WORLD_OPTIONAL_PAYLOAD_MISSING_CODE,
  PIXEL_WORLD_WASM_PAYLOAD_NAME,
  resolvePixelWorldWasmUrl as resolvePixelWorldWasmUrlForTest,
};
