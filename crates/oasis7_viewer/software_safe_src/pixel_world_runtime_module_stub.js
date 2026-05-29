import { createPixelWorldBevyBridge } from "./pixel_world_bevy_bridge.js";

export const PIXEL_WORLD_RUNTIME_SOURCE = "static_runtime_module_stub";

export async function createPixelWorldBridge({ onEvent, onFatal } = {}) {
  return createPixelWorldBevyBridge({ onEvent, onFatal });
}

export function derivePixelWorldRenderState(input) {
  return input?.fallback_render_state || null;
}
