import { afterEach, describe, expect, it, vi } from "vitest";

import { createPixelWorldBevyBridge } from "./pixel_world_bevy_bridge.js";

const originalRequestAnimationFrame = globalThis.requestAnimationFrame;
const originalCancelAnimationFrame = globalThis.cancelAnimationFrame;
const originalMatchMedia = globalThis.matchMedia;
const originalIntersectionObserver = globalThis.IntersectionObserver;
const originalDocumentHidden = Object.getOwnPropertyDescriptor(document, "hidden");

function renderState(agentId, xCm, yCm) {
  return {
    world_bounds: { width_cm: 1_000, depth_cm: 1_000 },
    locations: [],
    agents: [{ id: agentId, pos: { x_cm: xCm, y_cm: yCm, z_cm: 0 } }],
    selection: null,
  };
}

function pointerEvent(type, { clientX, clientY, pointerId = 1 }) {
  const event = new MouseEvent(type, { bubbles: true, clientX, clientY });
  Object.defineProperty(event, "pointerId", { configurable: true, value: pointerId });
  return event;
}

afterEach(() => {
  globalThis.requestAnimationFrame = originalRequestAnimationFrame;
  globalThis.cancelAnimationFrame = originalCancelAnimationFrame;
  globalThis.matchMedia = originalMatchMedia;
  globalThis.IntersectionObserver = originalIntersectionObserver;
  if (originalDocumentHidden) {
    Object.defineProperty(document, "hidden", originalDocumentHidden);
  }
  vi.restoreAllMocks();
});

function schedulerHarness({ reducedMotion = false } = {}) {
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
    matches: reducedMotion,
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
    pendingCount() {
      return callbacks.size;
    },
  };
}

function canvasWithSpy() {
  const canvas = document.createElement("canvas");
  canvas.width = 320;
  canvas.height = 180;
  const context = {
    clearRect: vi.fn(), fillRect: vi.fn(), strokeRect: vi.fn(), beginPath: vi.fn(), moveTo: vi.fn(), lineTo: vi.fn(), stroke: vi.fn(),
  };
  canvas.getContext = vi.fn(() => context);
  canvas.getBoundingClientRect = () => ({ left: 0, top: 0, width: 320, height: 180 });
  return { canvas, context };
}

describe("pixel world static canvas bridge", () => {
  it("does not redraw an unchanged static scene from ambient animation callbacks", () => {
    const scheduler = schedulerHarness({ reducedMotion: true });
    const { canvas, context } = canvasWithSpy();
    const bridge = createPixelWorldBevyBridge();

    bridge.mount(canvas, renderState("agent-1", 100, 100));
    expect(context.clearRect).toHaveBeenCalledTimes(1);
    for (let timestamp = 0; timestamp <= 1_000; timestamp += 16) {
      scheduler.runNext(timestamp);
    }

    expect(context.clearRect).toHaveBeenCalledTimes(1);
    expect(scheduler.pendingCount()).toBe(0);
    bridge.unmount();
  });

  it("enters full idle when the mounted scene has no animatable entities", () => {
    const scheduler = schedulerHarness();
    const { canvas, context } = canvasWithSpy();
    const bridge = createPixelWorldBevyBridge();

    bridge.mount(canvas, { world_bounds: { width_cm: 1_000, depth_cm: 1_000 }, locations: [], agents: [], selection: null });
    expect(context.clearRect).toHaveBeenCalledTimes(1);
    expect(scheduler.pendingCount()).toBe(0);
    bridge.unmount();
  });

  it("caps ambient animated redraws at 12Hz while allowing an invalidated state to draw immediately", () => {
    const scheduler = schedulerHarness();
    const { canvas, context } = canvasWithSpy();
    const bridge = createPixelWorldBevyBridge();

    bridge.mount(canvas, renderState("agent-1", 100, 100));
    for (let timestamp = 0; timestamp <= 1_000; timestamp += 16) {
      scheduler.runNext(timestamp);
    }
    expect(context.clearRect.mock.calls.length).toBeLessThanOrEqual(13);

    const beforeUpdate = context.clearRect.mock.calls.length;
    bridge.update(renderState("agent-2", 800, 800));
    expect(context.clearRect).toHaveBeenCalledTimes(beforeUpdate + 1);
    bridge.unmount();
  });

  it("pauses while hidden or offscreen, then resumes with one full sync and disposes lifecycle hooks on unmount", () => {
    const scheduler = schedulerHarness();
    const observers = [];
    globalThis.IntersectionObserver = vi.fn(function IntersectionObserver(callback) {
      this.callback = callback;
      this.observe = vi.fn();
      this.disconnect = vi.fn();
      observers.push(this);
    });
    const { canvas, context } = canvasWithSpy();
    const bridge = createPixelWorldBevyBridge();

    bridge.mount(canvas, renderState("agent-1", 100, 100));
    expect(observers).toHaveLength(1);
    const initialDraws = context.clearRect.mock.calls.length;

    Object.defineProperty(document, "hidden", { configurable: true, value: true });
    document.dispatchEvent(new Event("visibilitychange"));
    for (let timestamp = 16; timestamp <= 300; timestamp += 16) scheduler.runNext(timestamp);
    expect(context.clearRect).toHaveBeenCalledTimes(initialDraws);

    Object.defineProperty(document, "hidden", { configurable: true, value: false });
    document.dispatchEvent(new Event("visibilitychange"));
    expect(context.clearRect).toHaveBeenCalledTimes(initialDraws + 1);

    observers[0].callback([{ isIntersecting: false }]);
    for (let timestamp = 320; timestamp <= 600; timestamp += 16) scheduler.runNext(timestamp);
    expect(context.clearRect).toHaveBeenCalledTimes(initialDraws + 1);

    observers[0].callback([{ isIntersecting: true }]);
    expect(context.clearRect).toHaveBeenCalledTimes(initialDraws + 2);
    bridge.unmount();
    expect(observers[0].disconnect).toHaveBeenCalledTimes(1);
    expect(scheduler.pendingCount()).toBe(0);
  });

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

    canvas.dispatchEvent(new MouseEvent("click", { bubbles: true, clientX: 80, clientY: 34 }));
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

  it("does not select an entity from the synthetic click after panning", () => {
    globalThis.requestAnimationFrame = vi.fn(() => 1);
    globalThis.cancelAnimationFrame = vi.fn();
    const canvas = document.createElement("canvas");
    canvas.width = 320;
    canvas.height = 180;
    canvas.getContext = vi.fn(() => ({
      clearRect: vi.fn(), fillRect: vi.fn(), strokeRect: vi.fn(), beginPath: vi.fn(), moveTo: vi.fn(), lineTo: vi.fn(), stroke: vi.fn(),
    }));
    canvas.getBoundingClientRect = () => ({ left: 0, top: 0, width: 320, height: 180 });
    const onEvent = vi.fn();
    const bridge = createPixelWorldBevyBridge({ onEvent });

    bridge.mount(canvas, renderState("agent-1", 100, 100));
    onEvent.mockClear();
    canvas.dispatchEvent(pointerEvent("pointerdown", { clientX: 48, clientY: 34 }));
    canvas.dispatchEvent(pointerEvent("pointermove", { clientX: 80, clientY: 34 }));
    canvas.dispatchEvent(pointerEvent("pointerup", { clientX: 80, clientY: 34 }));
    canvas.dispatchEvent(new MouseEvent("click", { bubbles: true, clientX: 80, clientY: 34 }));

    expect(onEvent).not.toHaveBeenCalledWith({
      type: "select_entity",
      selection: { kind: "agent", id: "agent-1" },
    });

    canvas.dispatchEvent(new MouseEvent("click", { bubbles: true, clientX: 80, clientY: 34 }));
    const selections = onEvent.mock.calls
      .map(([event]) => event)
      .filter((event) => event.type === "select_entity");
    expect(selections).toEqual([{
      type: "select_entity",
      selection: { kind: "agent", id: "agent-1" },
    }]);
    bridge.unmount();
  });

  it("does not suppress a later click after a cancelled pan", () => {
    globalThis.requestAnimationFrame = vi.fn(() => 1);
    globalThis.cancelAnimationFrame = vi.fn();
    const canvas = document.createElement("canvas");
    canvas.width = 320;
    canvas.height = 180;
    canvas.getContext = vi.fn(() => ({
      clearRect: vi.fn(), fillRect: vi.fn(), strokeRect: vi.fn(), beginPath: vi.fn(), moveTo: vi.fn(), lineTo: vi.fn(), stroke: vi.fn(),
    }));
    canvas.getBoundingClientRect = () => ({ left: 0, top: 0, width: 320, height: 180 });
    const onEvent = vi.fn();
    const bridge = createPixelWorldBevyBridge({ onEvent });

    bridge.mount(canvas, renderState("agent-1", 100, 100));
    onEvent.mockClear();
    canvas.dispatchEvent(pointerEvent("pointerdown", { clientX: 48, clientY: 34 }));
    canvas.dispatchEvent(pointerEvent("pointermove", { clientX: 80, clientY: 34 }));
    canvas.dispatchEvent(pointerEvent("pointercancel", { clientX: 80, clientY: 34 }));
    canvas.dispatchEvent(new MouseEvent("click", { bubbles: true, clientX: 80, clientY: 34 }));

    expect(onEvent).toHaveBeenCalledWith({
      type: "select_entity",
      selection: { kind: "agent", id: "agent-1" },
    });
    bridge.unmount();
  });

  it("keeps selection active when pointer jitter stays within one pixel", () => {
    globalThis.requestAnimationFrame = vi.fn(() => 1);
    globalThis.cancelAnimationFrame = vi.fn();
    const canvas = document.createElement("canvas");
    canvas.width = 320;
    canvas.height = 180;
    canvas.getContext = vi.fn(() => ({
      clearRect: vi.fn(), fillRect: vi.fn(), strokeRect: vi.fn(), beginPath: vi.fn(), moveTo: vi.fn(), lineTo: vi.fn(), stroke: vi.fn(),
    }));
    canvas.getBoundingClientRect = () => ({ left: 0, top: 0, width: 320, height: 180 });
    const onEvent = vi.fn();
    const bridge = createPixelWorldBevyBridge({ onEvent });

    bridge.mount(canvas, renderState("agent-1", 100, 100));
    onEvent.mockClear();
    canvas.dispatchEvent(pointerEvent("pointerdown", { clientX: 48, clientY: 34 }));
    canvas.dispatchEvent(pointerEvent("pointermove", { clientX: 49, clientY: 35 }));
    canvas.dispatchEvent(pointerEvent("pointerup", { clientX: 49, clientY: 35 }));
    canvas.dispatchEvent(new MouseEvent("click", { bubbles: true, clientX: 49, clientY: 35 }));

    const selections = onEvent.mock.calls
      .map(([event]) => event)
      .filter((event) => event.type === "select_entity");
    expect(selections).toEqual([{
      type: "select_entity",
      selection: { kind: "agent", id: "agent-1" },
    }]);
    bridge.unmount();
  });
});
