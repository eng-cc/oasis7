function resolvePixelWorldRuntimeModuleUrl() {
  if (typeof window !== "undefined" && window.location) {
    return new URL("./pixel-world-bridge/pixel_world_bridge.js", window.location.href).href;
  }
  return "./pixel-world-bridge/pixel_world_bridge.js";
}

const PIXEL_WORLD_WASM_MODULE_URL = resolvePixelWorldRuntimeModuleUrl();
const PIXEL_WORLD_RUNTIME_UNAVAILABLE_CODE = "pixel_world_renderer_runtime_unavailable";

function defaultLoadRuntimeModule() {
  return import(/* @vite-ignore */ PIXEL_WORLD_WASM_MODULE_URL);
}

function normalizeRuntimeModuleError(error) {
  if (error instanceof Error) {
    return error;
  }
  return new Error(String(error || "unknown pixel world runtime import failure"));
}

export function probePixelWorldWebgl2Surface(canvas) {
  try {
    if (canvas?.getContext?.("webgl2", { antialias: false })) return null;
    return { code: "pixel_world_webgl2_unavailable", message: "canvas.getContext() returned null; webgl2 not available or canvas already in use" };
  } catch (error) {
    return { code: "pixel_world_webgl2_unavailable", message: `canvas.getContext() threw exception: ${error instanceof Error ? error.message : String(error || "unknown WebGL2 error")}` };
  }
}

async function tryLoadWasmBridgeModule(loadRuntimeModule = defaultLoadRuntimeModule) {
  try {
    const module = await loadRuntimeModule();
    if (!module?.createPixelWorldBridge) {
      throw new Error("pixel world runtime module is missing createPixelWorldBridge export");
    }
    return {
      module,
      moduleUrl: PIXEL_WORLD_WASM_MODULE_URL,
      error: null,
    };
  } catch (error) {
    return {
      module: null,
      moduleUrl: PIXEL_WORLD_WASM_MODULE_URL,
      error: normalizeRuntimeModuleError(error),
    };
  }
}

async function tryCreateWasmBridge(module, options) {
  try {
    return {
      bridge: await module.createPixelWorldBridge(options),
      error: null,
    };
  } catch (error) {
    return {
      bridge: null,
      error: normalizeRuntimeModuleError(error),
    };
  }
}

function buildRuntimeUnavailableFatal(moduleUrl, error) {
  const message = [
    "pixel world wasm runtime is unavailable",
    moduleUrl ? `module=${moduleUrl}` : null,
    error?.message || null,
  ].filter(Boolean).join(": ");

  return {
    code: error?.code || PIXEL_WORLD_RUNTIME_UNAVAILABLE_CODE,
    message,
  };
}

function createUnavailableBridge({ fatal, onFatal }) {
  let emitted = false;

  function emitFatal() {
    if (!emitted) {
      emitted = true;
      onFatal?.(fatal);
    }
  }

  return {
    mount() {
      emitFatal();
      return {
        status: "unavailable",
        fatal,
      };
    },
    update() {
      return {
        status: "unavailable",
        fatal,
      };
    },
    unmount() {
      return {
        status: "detached",
      };
    },
  };
}

export async function createPixelWorldRuntimeBridge({
  onEvent,
  onFatal,
  loadRuntimeModule = defaultLoadRuntimeModule,
} = {}) {
  const runtimeModule = await tryLoadWasmBridgeModule(loadRuntimeModule);
  if (runtimeModule.module?.createPixelWorldBridge) {
    const initializedBridge = await tryCreateWasmBridge(runtimeModule.module, { onEvent, onFatal });
    if (!initializedBridge.error) {
      return {
        bridge: initializedBridge.bridge,
        deriveRenderState: runtimeModule.module.derivePixelWorldRenderState || null,
        source: runtimeModule.module.PIXEL_WORLD_RUNTIME_SOURCE || "runtime_module",
        moduleUrl: runtimeModule.moduleUrl,
      };
    }
    const fatal = buildRuntimeUnavailableFatal(runtimeModule.moduleUrl, initializedBridge.error);
    return {
      bridge: createUnavailableBridge({ fatal, onFatal }),
      deriveRenderState: null,
      source: "wasm_bridge_init_failed",
      moduleUrl: runtimeModule.moduleUrl,
      fatal,
    };
  }

  const fatal = buildRuntimeUnavailableFatal(runtimeModule.moduleUrl, runtimeModule.error);
  return {
    bridge: createUnavailableBridge({ fatal, onFatal }),
    deriveRenderState: null,
    source: "wasm_import_failed",
    moduleUrl: runtimeModule.moduleUrl,
    fatal,
  };
}

export {
  PIXEL_WORLD_RUNTIME_UNAVAILABLE_CODE,
  PIXEL_WORLD_WASM_MODULE_URL,
};
