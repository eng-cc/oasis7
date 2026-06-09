let runtimeModulePromise = null;
let runtimeModule = null;

function browserSupportsWebGpu(nav = globalThis.navigator) {
  return typeof nav !== "undefined" && !!nav?.gpu;
}

function resolveBackendModuleUrl(nav = globalThis.navigator) {
  const backendDir = browserSupportsWebGpu(nav) ? "webgpu" : "webgl2";
  return new URL(`${backendDir}/pixel_world_bridge.js`, import.meta.url).href;
}

async function loadRuntimeModule() {
  if (!runtimeModulePromise) {
    runtimeModulePromise = import(/* @vite-ignore */ resolveBackendModuleUrl()).then((module) => {
      runtimeModule = module;
      return module;
    });
  }
  return runtimeModulePromise;
}

export const PIXEL_WORLD_RUNTIME_SOURCE = "wasm_bindgen_runtime";

export async function createPixelWorldBridge(options = {}) {
  const module = await loadRuntimeModule();
  return module.createPixelWorldBridge(options);
}

export function derivePixelWorldRenderState(input) {
  if (!runtimeModule?.derivePixelWorldRenderState) {
    throw new Error("pixel world bridge backend module is not initialized");
  }
  return runtimeModule.derivePixelWorldRenderState(input);
}

export default async function initPixelWorldBridgeSelector() {
  await loadRuntimeModule();
}

export function resolveBackendModuleUrlForTest(nav) {
  return resolveBackendModuleUrl(nav);
}

export function __resetPixelWorldRuntimeModuleForTest() {
  runtimeModulePromise = null;
  runtimeModule = null;
}
