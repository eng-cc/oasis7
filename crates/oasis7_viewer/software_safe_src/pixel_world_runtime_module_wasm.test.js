import { afterEach, describe, expect, it, vi } from "vitest";

const runtimeState = vi.hoisted(() => ({
  mountImpl: null,
  instances: [],
}));

const originalRequestAnimationFrame = globalThis.requestAnimationFrame;
const originalCancelAnimationFrame = globalThis.cancelAnimationFrame;
const originalMatchMedia = globalThis.matchMedia;

function animationHarness() {
  const callbacks = new Map();
  let nextFrameId = 1;
  globalThis.requestAnimationFrame = vi.fn((callback) => {
    const frameId = nextFrameId;
    nextFrameId += 1;
    callbacks.set(frameId, callback);
    return frameId;
  });
  globalThis.cancelAnimationFrame = vi.fn((frameId) => callbacks.delete(frameId));
  globalThis.matchMedia = vi.fn(() => ({
    matches: false,
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
  }));
  return {
    runNext(timestamp) {
      const next = callbacks.entries().next().value;
      if (!next) return false;
      const [frameId, callback] = next;
      callbacks.delete(frameId);
      callback(timestamp);
      return true;
    },
  };
}

vi.mock("./pixel_world_bridge_bindgen.js", () => {
  class MockPixelWorldBridge {
    constructor(onEvent, onFatal) {
      this.onEvent = onEvent;
      this.onFatal = onFatal;
      this.pointer_down = vi.fn();
      this.pointer_move = vi.fn();
      this.pointer_up = vi.fn();
      this.wheel = vi.fn();
      this.click = vi.fn();
      this.tick = vi.fn();
      this.unmount = vi.fn(() => ({ status: "detached" }));
      this.update = vi.fn(() => ({ status: "ready" }));
      this.mount = vi.fn((canvas, renderState) => {
        if (runtimeState.mountImpl) {
          return runtimeState.mountImpl(this, canvas, renderState);
        }
        return { status: "ready" };
      });
      runtimeState.instances.push(this);
    }
  }

  return {
    __esModule: true,
    default: vi.fn(async () => undefined),
    PixelWorldBridge: MockPixelWorldBridge,
    build_pixel_world_render_state: vi.fn(),
  };
});

describe("pixel world wasm runtime bridge", () => {
  afterEach(() => {
    runtimeState.mountImpl = null;
    runtimeState.instances.length = 0;
    globalThis.requestAnimationFrame = originalRequestAnimationFrame;
    globalThis.cancelAnimationFrame = originalCancelAnimationFrame;
    globalThis.matchMedia = originalMatchMedia;
    vi.restoreAllMocks();
  });

  function pointerEvent(type, options = {}) {
    const event = new MouseEvent(type, {
      bubbles: true,
      cancelable: true,
      clientX: options.clientX ?? 0,
      clientY: options.clientY ?? 0,
    });
    Object.defineProperty(event, "pointerId", {
      configurable: true,
      value: options.pointerId ?? 1,
    });
    return event;
  }

  it("forwards click coordinates in CSS pixels so Bevy hit testing matches the embedded window", async () => {
    const { createPixelWorldBridge } = await import("./pixel_world_runtime_module_wasm.js");
    const onEvent = vi.fn();
    const bridge = await createPixelWorldBridge({ onEvent });
    const canvas = document.createElement("canvas");
    canvas.width = 960;
    canvas.height = 540;
    canvas.getBoundingClientRect = () => ({
      left: 10,
      top: 20,
      width: 480,
      height: 270,
      right: 490,
      bottom: 290,
      x: 10,
      y: 20,
      toJSON() {
        return this;
      },
    });
    canvas.setPointerCapture = vi.fn();
    canvas.releasePointerCapture = vi.fn();

    runtimeState.mountImpl = (instance) => {
      instance.click.mockImplementation((x, y) => {
        instance.onEvent?.({
          type: "select_entity",
          selection: {
            kind: "agent",
            id: `${x},${y}`,
          },
        });
        return { status: "ready" };
      });
      return { status: "ready" };
    };

    bridge.mount(canvas, { selection: null });
    canvas.dispatchEvent(new MouseEvent("click", {
      clientX: 130,
      clientY: 95,
      bubbles: true,
    }));

    expect(runtimeState.instances).toHaveLength(1);
    expect(runtimeState.instances[0].click).toHaveBeenCalledWith(120, 75);
    expect(onEvent).toHaveBeenCalledWith({
      type: "select_entity",
      selection: {
        kind: "agent",
        id: "120,75",
      },
    });
  });

  it("resolves an available optional WASM payload from the served manifest route", async () => {
    const originalFetch = globalThis.fetch;
    const originalBaseUrl = globalThis.__OASIS7_VIEWER_OPTIONAL_PAYLOAD_BASE_URL__;
    const fetchMock = vi.fn(async () => ({
      status: 200,
      ok: true,
      json: async () => ({
        "pixel_world_bridge_bindgen_bg.wasm": {
          available: true,
          path: "pixel_world_bridge_bindgen_bg.wasm",
        },
      }),
    }));
    globalThis.fetch = fetchMock;
    globalThis.__OASIS7_VIEWER_OPTIONAL_PAYLOAD_BASE_URL__ = "/optional-payload/";
    try {
      const { resolvePixelWorldWasmUrlForTest } = await import("./pixel_world_runtime_module_wasm.js");
      const resolved = await resolvePixelWorldWasmUrlForTest({
        moduleUrl: "https://example.test/viewer/pixel-world-bridge/webgl2/pixel_world_bridge.js",
      });
      expect(resolved.href).toBe("https://example.test/optional-payload/pixel_world_bridge_bindgen_bg.wasm");
      expect(fetchMock).toHaveBeenCalledWith(
        new URL("https://example.test/viewer/optional-payloads.json"),
        { cache: "no-store" },
      );
    } finally {
      globalThis.fetch = originalFetch;
      if (originalBaseUrl === undefined) {
        delete globalThis.__OASIS7_VIEWER_OPTIONAL_PAYLOAD_BASE_URL__;
      } else {
        globalThis.__OASIS7_VIEWER_OPTIONAL_PAYLOAD_BASE_URL__ = originalBaseUrl;
      }
    }
  });

  it("resolves the published split-delivery payload beside the archive web root", async () => {
    const originalFetch = globalThis.fetch;
    const originalBaseUrl = globalThis.__OASIS7_VIEWER_OPTIONAL_PAYLOAD_BASE_URL__;
    const fetchMock = vi.fn(async () => ({
      status: 200,
      ok: true,
      json: async () => ({
        "pixel_world_bridge_bindgen_bg.wasm": {
          available: true,
          path: "viewer-optional-payload/pixel_world_bridge_bindgen_bg.wasm",
        },
      }),
    }));
    globalThis.fetch = fetchMock;
    delete globalThis.__OASIS7_VIEWER_OPTIONAL_PAYLOAD_BASE_URL__;
    try {
      const { resolvePixelWorldWasmUrlForTest } = await import("./pixel_world_runtime_module_wasm.js");
      const resolved = await resolvePixelWorldWasmUrlForTest({
        moduleUrl: "https://example.test/viewer/pixel-world-bridge/webgl2/pixel_world_bridge.js",
      });
      expect(resolved.href).toBe("https://example.test/viewer/viewer-optional-payload/pixel_world_bridge_bindgen_bg.wasm");
      expect(fetchMock).toHaveBeenCalledWith(
        new URL("https://example.test/viewer/optional-payloads.json"),
        { cache: "no-store" },
      );
    } finally {
      globalThis.fetch = originalFetch;
      if (originalBaseUrl === undefined) {
        delete globalThis.__OASIS7_VIEWER_OPTIONAL_PAYLOAD_BASE_URL__;
      } else {
        globalThis.__OASIS7_VIEWER_OPTIONAL_PAYLOAD_BASE_URL__ = originalBaseUrl;
      }
    }
  });

  it("returns a deterministic missing-payload error instead of probing a stale adjacent WASM", async () => {
    const originalFetch = globalThis.fetch;
    const fetchMock = vi.fn(async () => ({
      status: 200,
      ok: true,
      json: async () => ({
        "pixel_world_bridge_bindgen_bg.wasm": {
          available: false,
          reason: "source_missing",
        },
      }),
    }));
    globalThis.fetch = fetchMock;
    try {
      const { resolvePixelWorldWasmUrlForTest, PIXEL_WORLD_OPTIONAL_PAYLOAD_MISSING_CODE } = await import("./pixel_world_runtime_module_wasm.js");
      await expect(resolvePixelWorldWasmUrlForTest({
        moduleUrl: "https://example.test/viewer/pixel-world-bridge/webgl2/pixel_world_bridge.js",
      })).rejects.toMatchObject({
        code: PIXEL_WORLD_OPTIONAL_PAYLOAD_MISSING_CODE,
        message: expect.stringContaining("source_missing"),
      });
    } finally {
      globalThis.fetch = originalFetch;
    }
  });

  it("caps WASM ambient ticks at 12Hz while leaving pointer input immediate", async () => {
    const animation = animationHarness();
    const { createPixelWorldBridge } = await import("./pixel_world_runtime_module_wasm.js");
    const bridge = await createPixelWorldBridge();
    const canvas = document.createElement("canvas");
    canvas.getBoundingClientRect = () => ({ left: 0, top: 0, width: 480, height: 270 });
    canvas.setPointerCapture = vi.fn();
    canvas.releasePointerCapture = vi.fn();

    bridge.mount(canvas, { selection: null });
    for (let timestamp = 0; timestamp <= 1_000; timestamp += 16) animation.runNext(timestamp);

    expect(runtimeState.instances).toHaveLength(1);
    expect(runtimeState.instances[0].tick.mock.calls.length).toBeGreaterThan(0);
    expect(runtimeState.instances[0].tick.mock.calls.length).toBeLessThanOrEqual(12);
    canvas.dispatchEvent(pointerEvent("pointerdown", { clientX: 10, clientY: 10 }));
    expect(runtimeState.instances[0].pointer_down).toHaveBeenCalledWith(10, 10, 1);
    bridge.unmount();
  });

  it("reacts to reduced-motion preference changes and removes its media listener on unmount", async () => {
    const animation = animationHarness();
    let mediaChange = null;
    const mediaQuery = {
      matches: false,
      addEventListener: vi.fn((event, callback) => { if (event === "change") mediaChange = callback; }),
      removeEventListener: vi.fn(),
    };
    globalThis.matchMedia = vi.fn(() => mediaQuery);
    const { createPixelWorldBridge } = await import("./pixel_world_runtime_module_wasm.js");
    const bridge = await createPixelWorldBridge();
    const canvas = document.createElement("canvas");
    canvas.getBoundingClientRect = () => ({ left: 0, top: 0, width: 480, height: 270 });

    bridge.mount(canvas, { selection: null });
    expect(mediaQuery.addEventListener).toHaveBeenCalledWith("change", expect.any(Function));
    mediaChange?.({ matches: true });
    expect(runtimeState.instances[0].tick).toHaveBeenCalledTimes(1);
    expect(animation.runNext(100)).toBe(false);
    bridge.unmount();
    expect(mediaQuery.removeEventListener).toHaveBeenCalledWith("change", expect.any(Function));
  });

  it("suppresses the synthetic click that follows a drag gesture", async () => {
    const { createPixelWorldBridge } = await import("./pixel_world_runtime_module_wasm.js");
    const bridge = await createPixelWorldBridge();
    const canvas = document.createElement("canvas");
    canvas.getBoundingClientRect = () => ({
      left: 0,
      top: 0,
      width: 480,
      height: 270,
      right: 480,
      bottom: 270,
      x: 0,
      y: 0,
      toJSON() {
        return this;
      },
    });
    canvas.setPointerCapture = vi.fn();
    canvas.releasePointerCapture = vi.fn();

    bridge.mount(canvas, { selection: null });
    canvas.dispatchEvent(pointerEvent("pointerdown", { clientX: 10, clientY: 10 }));
    canvas.dispatchEvent(pointerEvent("pointermove", { clientX: 48, clientY: 10 }));
    canvas.dispatchEvent(pointerEvent("pointerup", { clientX: 48, clientY: 10 }));
    canvas.dispatchEvent(new MouseEvent("click", {
      bubbles: true,
      clientX: 48,
      clientY: 10,
    }));

    expect(runtimeState.instances).toHaveLength(1);
    expect(runtimeState.instances[0].click).not.toHaveBeenCalled();
  });

  it("does not suppress a later click after a cancelled drag", async () => {
    const { createPixelWorldBridge } = await import("./pixel_world_runtime_module_wasm.js");
    const bridge = await createPixelWorldBridge();
    const canvas = document.createElement("canvas");
    canvas.getBoundingClientRect = () => ({
      left: 0,
      top: 0,
      width: 480,
      height: 270,
      right: 480,
      bottom: 270,
      x: 0,
      y: 0,
      toJSON() {
        return this;
      },
    });
    canvas.setPointerCapture = vi.fn();
    canvas.releasePointerCapture = vi.fn();

    bridge.mount(canvas, { selection: null });
    canvas.dispatchEvent(pointerEvent("pointerdown", { clientX: 10, clientY: 10 }));
    canvas.dispatchEvent(pointerEvent("pointermove", { clientX: 48, clientY: 10 }));
    canvas.dispatchEvent(pointerEvent("pointercancel", { clientX: 48, clientY: 10 }));
    canvas.dispatchEvent(new MouseEvent("click", {
      bubbles: true,
      clientX: 96,
      clientY: 30,
    }));

    expect(runtimeState.instances).toHaveLength(1);
    expect(runtimeState.instances[0].click).toHaveBeenCalledWith(96, 30);
  });

  it("keeps click selection active when pointer jitter stays below the drag threshold", async () => {
    const { createPixelWorldBridge } = await import("./pixel_world_runtime_module_wasm.js");
    const bridge = await createPixelWorldBridge();
    const canvas = document.createElement("canvas");
    canvas.getBoundingClientRect = () => ({
      left: 0,
      top: 0,
      width: 480,
      height: 270,
      right: 480,
      bottom: 270,
      x: 0,
      y: 0,
      toJSON() {
        return this;
      },
    });
    canvas.setPointerCapture = vi.fn();
    canvas.releasePointerCapture = vi.fn();

    bridge.mount(canvas, { selection: null });
    canvas.dispatchEvent(pointerEvent("pointerdown", { clientX: 10, clientY: 10 }));
    canvas.dispatchEvent(pointerEvent("pointermove", { clientX: 12, clientY: 11 }));
    canvas.dispatchEvent(pointerEvent("pointerup", { clientX: 12, clientY: 11 }));
    canvas.dispatchEvent(new MouseEvent("click", {
      bubbles: true,
      clientX: 12,
      clientY: 11,
    }));

    expect(runtimeState.instances).toHaveLength(1);
    expect(runtimeState.instances[0].click).toHaveBeenCalledWith(12, 11);
  });
});
