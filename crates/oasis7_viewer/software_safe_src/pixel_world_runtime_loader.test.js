import { describe, expect, it, vi } from "vitest";

import {
  PIXEL_WORLD_RUNTIME_UNAVAILABLE_CODE,
  createPixelWorldRuntimeBridge,
} from "./pixel_world_runtime_loader.js";
import { derivePixelWorldRenderState as deriveStubRenderState } from "./pixel_world_runtime_module_stub.js";

describe("pixel world runtime loader", () => {
  it("uses the wasm runtime module when it loads successfully", async () => {
    const createPixelWorldBridge = vi.fn(async () => ({
      mount: vi.fn(() => ({ status: "ready" })),
      update: vi.fn(() => ({ status: "ready" })),
      unmount: vi.fn(() => ({ status: "detached" })),
    }));
    const derivePixelWorldRenderState = vi.fn(() => ({ locale: "en" }));

    const runtime = await createPixelWorldRuntimeBridge({
      loadRuntimeModule: async () => ({
        PIXEL_WORLD_RUNTIME_SOURCE: "test_runtime",
        createPixelWorldBridge,
        derivePixelWorldRenderState,
      }),
    });

    expect(runtime.source).toBe("test_runtime");
    expect(runtime.moduleUrl).toContain("pixel-world-bridge/pixel_world_bridge.js");
    expect(createPixelWorldBridge).toHaveBeenCalledTimes(1);
    expect(runtime.deriveRenderState({ locale: "en" })).toEqual({ locale: "en" });
  });

  it("surfaces a structured fatal path when the wasm runtime import fails", async () => {
    const onFatal = vi.fn();
    const runtime = await createPixelWorldRuntimeBridge({
      onFatal,
      loadRuntimeModule: async () => {
        throw new Error("missing wasm bridge");
      },
    });

    expect(runtime.source).toBe("wasm_import_failed");
    expect(runtime.fatal).toMatchObject({
      code: PIXEL_WORLD_RUNTIME_UNAVAILABLE_CODE,
    });

    const mountResult = runtime.bridge.mount(document.createElement("canvas"), {});
    expect(mountResult).toMatchObject({
      status: "fallback",
      fatal: runtime.fatal,
    });
    expect(onFatal).toHaveBeenCalledWith(expect.objectContaining({
      code: PIXEL_WORLD_RUNTIME_UNAVAILABLE_CODE,
      message: expect.stringContaining("missing wasm bridge"),
    }));
  });

  it("keeps the static runtime stub derivation explicitly unavailable", () => {
    expect(deriveStubRenderState({
      fallback_render_state: { locale: "en" },
    })).toBeNull();
  });
});
