import { afterEach, describe, expect, it, vi } from "vitest";

import { createPixelWorldBevyBridge } from "./pixel_world_bevy_bridge.js";

const originalRequestAnimationFrame = globalThis.requestAnimationFrame;
const originalCancelAnimationFrame = globalThis.cancelAnimationFrame;

function renderState(agentId, xCm, yCm) {
  return {
    world_bounds: { width_cm: 1_000, depth_cm: 1_000 },
    locations: [],
    agents: [{ id: agentId, pos: { x_cm: xCm, y_cm: yCm, z_cm: 0 } }],
    selection: null,
  };
}

afterEach(() => {
  globalThis.requestAnimationFrame = originalRequestAnimationFrame;
  globalThis.cancelAnimationFrame = originalCancelAnimationFrame;
  vi.restoreAllMocks();
});

describe("pixel world static canvas bridge", () => {
  it("updates an already-mounted canvas when the live render state changes", () => {
    globalThis.requestAnimationFrame = vi.fn(() => 1);
    globalThis.cancelAnimationFrame = vi.fn();
    const canvas = document.createElement("canvas");
    canvas.width = 320;
    canvas.height = 180;
    canvas.getContext = vi.fn(() => ({
      clearRect: vi.fn(),
      fillRect: vi.fn(),
      strokeRect: vi.fn(),
      beginPath: vi.fn(),
      moveTo: vi.fn(),
      lineTo: vi.fn(),
      stroke: vi.fn(),
    }));
    canvas.getBoundingClientRect = () => ({
      left: 0,
      top: 0,
      width: 320,
      height: 180,
    });
    const onEvent = vi.fn();
    const bridge = createPixelWorldBevyBridge({ onEvent });

    expect(bridge.mount(canvas, renderState("agent-1", 100, 100))).toEqual({ status: "ready" });
    expect(bridge.update(renderState("agent-2", 800, 800))).toEqual({ status: "ready" });
    expect(bridge.getLastRenderState().agents[0].id).toBe("agent-2");
    onEvent.mockClear();

    canvas.dispatchEvent(new MouseEvent("click", { bubbles: true, clientX: 48, clientY: 34 }));
    expect(onEvent).not.toHaveBeenCalledWith({
      type: "select_entity",
      selection: { kind: "agent", id: "agent-1" },
    });

    canvas.dispatchEvent(new MouseEvent("click", { bubbles: true, clientX: 244, clientY: 132 }));
    expect(onEvent).toHaveBeenCalledWith({
      type: "select_entity",
      selection: { kind: "agent", id: "agent-2" },
    });

    bridge.unmount();
  });
});
