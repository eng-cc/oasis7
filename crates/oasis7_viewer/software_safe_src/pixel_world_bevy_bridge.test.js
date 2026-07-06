import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { createPixelWorldBevyBridge } from "./pixel_world_bevy_bridge.js";

const context2d = {
  beginPath: vi.fn(),
  clearRect: vi.fn(),
  fillRect: vi.fn(),
  lineTo: vi.fn(),
  moveTo: vi.fn(),
  stroke: vi.fn(),
  strokeRect: vi.fn(),
};

let animationCallbacks = [];
let animationHandle = 0;

function flushAnimationFrame(animationMs = 16) {
  const callback = animationCallbacks.shift();
  expect(callback).toBeTypeOf("function");
  callback(animationMs);
}

function makeCanvas() {
  const canvas = document.createElement("canvas");
  canvas.width = 960;
  canvas.height = 540;
  canvas.getContext = vi.fn(() => context2d);
  canvas.getBoundingClientRect = () => ({
    left: 0,
    top: 0,
    width: 960,
    height: 540,
    right: 960,
    bottom: 540,
    x: 0,
    y: 0,
    toJSON() {
      return this;
    },
  });
  canvas.setPointerCapture = vi.fn();
  canvas.releasePointerCapture = vi.fn();
  return canvas;
}

function makeRenderState() {
  return makeRenderStateWithEntities({
    locations: [{
      id: "loc-0",
      pos: { x_cm: 5_000_000, y_cm: 2_500_000, z_cm: 0 },
    }],
    agents: [{
      id: "agent-0",
      pos: { x_cm: 5_020_000, y_cm: 2_510_000, z_cm: 0 },
    }],
  });
}

function makeRenderStateWithEntities({ locations = [], agents = [] } = {}) {
  const state = {
    world_bounds: {
      width_cm: 10_000_000,
      depth_cm: 5_000_000,
      height_cm: 1_000_000,
    },
    selection: null,
  };
  let locationReads = 0;
  let agentReads = 0;
  Object.defineProperty(state, "locations", {
    get() {
      locationReads += 1;
      return locations;
    },
  });
  Object.defineProperty(state, "agents", {
    get() {
      agentReads += 1;
      return agents;
    },
  });
  return {
    state,
    getReads() {
      return {
        locations: locationReads,
        agents: agentReads,
      };
    },
  };
}

function pointerEvent(type, clientX, clientY) {
  return new PointerEvent(type, {
    bubbles: true,
    clientX,
    clientY,
    pointerId: 1,
  });
}

beforeEach(() => {
  animationCallbacks = [];
  animationHandle = 0;
  vi.stubGlobal("requestAnimationFrame", vi.fn((callback) => {
    animationCallbacks.push(callback);
    animationHandle += 1;
    return animationHandle;
  }));
  vi.stubGlobal("cancelAnimationFrame", vi.fn());
});

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("pixel world bevy bridge hit regions", () => {
  it("does not rebuild hit regions on unchanged animation frames", () => {
    const bridge = createPixelWorldBevyBridge();
    const canvas = makeCanvas();
    const renderState = makeRenderState();

    expect(bridge.mount(canvas, renderState.state)).toEqual({ status: "ready" });
    expect(renderState.getReads()).toEqual({
      locations: 3,
      agents: 3,
    });

    flushAnimationFrame(32);

    expect(renderState.getReads()).toEqual({
      locations: 4,
      agents: 4,
    });

    bridge.unmount();
  });

  it("rebuilds hit regions when camera geometry changes", () => {
    const bridge = createPixelWorldBevyBridge();
    const canvas = makeCanvas();
    const renderState = makeRenderState();

    expect(bridge.mount(canvas, renderState.state)).toEqual({ status: "ready" });
    canvas.dispatchEvent(new WheelEvent("wheel", {
      deltaY: -1,
      cancelable: true,
    }));

    expect(renderState.getReads()).toEqual({
      locations: 6,
      agents: 6,
    });

    bridge.unmount();
  });

  it("refreshes hit regions after render state updates", () => {
    const onEvent = vi.fn();
    const bridge = createPixelWorldBevyBridge({ onEvent });
    const canvas = makeCanvas();
    const initialRenderState = makeRenderStateWithEntities({
      agents: [{
        id: "agent-old",
        pos: { x_cm: 1_000_000, y_cm: 1_000_000, z_cm: 0 },
      }],
    });
    const updatedRenderState = makeRenderStateWithEntities({
      agents: [{
        id: "agent-new",
        pos: { x_cm: 5_000_000, y_cm: 2_500_000, z_cm: 0 },
      }],
    });

    expect(bridge.mount(canvas, initialRenderState.state)).toEqual({ status: "ready" });
    expect(bridge.update(updatedRenderState.state)).toEqual({ status: "ready" });

    canvas.dispatchEvent(pointerEvent("pointermove", 480, 270));

    expect(onEvent).toHaveBeenCalledWith({
      type: "hover_entity",
      selection: { kind: "agent", id: "agent-new" },
    });

    bridge.unmount();
  });
});
