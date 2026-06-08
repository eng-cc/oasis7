import { afterEach, describe, expect, it, vi } from "vitest";

const runtimeState = vi.hoisted(() => ({
  mountImpl: null,
  instances: [],
}));

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
  });

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
});
