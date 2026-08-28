const PIXEL_WORLD_RENDERER_DEFER_VALUES = new Set([
  "0",
  "false",
  "no",
  "off",
  "defer",
  "fallback",
]);

const DEFERRED_RENDERER_FATAL = {
  code: "pixel_world_renderer_deferred",
  message: "pixel world renderer was explicitly deferred by the viewer route",
};

/**
 * Resolve the explicit renderer route without deciding whether the default
 * renderer can boot. A deferred route is an honest unavailable state; retry
 * remains responsible for reattaching the renderer in the host.
 */
export function resolvePixelWorldRendererRoute(
  locationRef = typeof window === "undefined" ? null : window.location,
) {
  if (!locationRef) {
    return { deferred: false, source: "loading", fatal: null };
  }

  const value = String(
    new URLSearchParams(locationRef.search || "").get("pixel_world_renderer") || "",
  ).trim().toLowerCase();
  if (!PIXEL_WORLD_RENDERER_DEFER_VALUES.has(value)) {
    return { deferred: false, source: "loading", fatal: null };
  }

  return {
    deferred: true,
    source: "deferred",
    fatal: { ...DEFERRED_RENDERER_FATAL },
  };
}

export function applyPixelWorldRendererRoute(route, updateRuntimeMeta) {
  if (!route.deferred) {
    return;
  }
  updateRuntimeMeta({
    runtimeStatus: "unavailable",
    runtimeSource: "deferred",
    runtimeModuleUrl: null,
    camera: null,
    fatal: route.fatal,
  });
}

export function createPixelWorldRendererRouteSignals(route, signalFactory) {
  const [rendererStatus, setRendererStatus] = signalFactory(route.deferred ? "unavailable" : "booting");
  const [rendererFatal, setRendererFatal] = signalFactory(route.fatal);
  const [runtimeSource, setRuntimeSource] = signalFactory(route.source);
  return [rendererStatus, setRendererStatus, rendererFatal, setRendererFatal, runtimeSource, setRuntimeSource];
}
