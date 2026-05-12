import { createPixelWorldBevyBridge } from "./pixel_world_bevy_bridge.js";

function resolvePixelWorldRuntimeModuleUrl() {
  if (typeof window !== "undefined" && window.location) {
    return new URL("./pixel-world-bridge/pixel_world_bridge.js", window.location.href).href;
  }
  return "./pixel-world-bridge/pixel_world_bridge.js";
}

const PIXEL_WORLD_WASM_MODULE_URL = resolvePixelWorldRuntimeModuleUrl();

function shouldBypassWasmRuntimeForCurrentBrowser() {
  if (typeof navigator === "undefined") {
    return false;
  }
  const userAgent = String(navigator.userAgent || "");
  return navigator.webdriver === true || /HeadlessChrome/i.test(userAgent);
}

async function tryLoadWasmBridgeModule() {
  try {
    return {
      module: await import(/* @vite-ignore */ PIXEL_WORLD_WASM_MODULE_URL),
      moduleUrl: PIXEL_WORLD_WASM_MODULE_URL,
    };
  } catch (_) {
    return null;
  }
}

export async function createPixelWorldRuntimeBridge({ onEvent, onFatal } = {}) {
  if (shouldBypassWasmRuntimeForCurrentBrowser()) {
    return {
      bridge: createPixelWorldBevyBridge({ onEvent, onFatal }),
      source: "js_fallback",
      moduleUrl: null,
    };
  }

  const runtimeModule = await tryLoadWasmBridgeModule();
  if (runtimeModule?.module?.createPixelWorldBridge) {
    return {
      bridge: await runtimeModule.module.createPixelWorldBridge({ onEvent, onFatal }),
      source: runtimeModule.module.PIXEL_WORLD_RUNTIME_SOURCE || "runtime_module",
      moduleUrl: runtimeModule.moduleUrl,
    };
  }

  return {
    bridge: createPixelWorldBevyBridge({ onEvent, onFatal }),
    source: "js_fallback",
    moduleUrl: null,
  };
}

export { PIXEL_WORLD_WASM_MODULE_URL };
