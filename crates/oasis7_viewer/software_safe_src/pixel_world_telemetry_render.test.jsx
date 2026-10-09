import { waitFor } from "@solidjs/testing-library";
import { afterEach, expect, it, vi } from "vitest";
import { renderPixelWorldHost, runtimeMock, sampleSnapshot, useTestRustRenderState } from "./pixel_world_host_test_support.jsx";

afterEach(() => vi.unstubAllGlobals());

it("refreshes telemetry without deriving or updating the map, while world and default renders remain dirty", async () => {
  const sockets = [];
  class MockWebSocket {
    static OPEN = 1;
    constructor() { this.readyState = 0; this.listeners = new Map(); sockets.push(this); }
    addEventListener(type, listener) { this.listeners.set(type, [...(this.listeners.get(type) || []), listener]); }
    send() {}
    close() { this.readyState = 3; }
    receive(message) { for (const listener of this.listeners.get("message") || []) listener({ data: JSON.stringify(message) }); }
  }
  vi.stubGlobal("WebSocket", MockWebSocket);
  useTestRustRenderState();
  const derive = runtimeMock.deriveRenderState = vi.fn(runtimeMock.deriveRenderState);
  const { core } = await renderPixelWorldHost(sampleSnapshot());
  window.history.replaceState({}, "", "/viewer.html?test_api=1&connect=1&locale=en");
  await core.initializeSoftwareSafeCore();
  await waitFor(() => expect(runtimeMock.mountCalls).toBe(1));
  await waitFor(() => expect(document.querySelector('[data-world-tick]')).toBeInTheDocument());
  const notify = vi.fn();
  const unsubscribe = core.subscribeRenderHook(notify);
  const beforeDerive = derive.mock.calls.length;
  const beforeUpdate = runtimeMock.updateCalls;
  const socket = sockets.at(-1);
  for (let time = 50; time < 60; time += 1) {
    socket.receive({ type: "metrics", time, metrics: { decision_trace_count: time, total_ticks: time } });
    socket.receive({ type: "decision_trace", trace: { agent_id: "agent-0", time, decision: { type: "wait" } } });
  }
  expect(core.state.metrics.total_ticks).toBe(59);
  expect(core.state.recentDecisionTraces[0].time).toBe(59);
  expect(core.state.tick).toBe(59);
  expect(notify).toHaveBeenCalledTimes(20);
  expect(derive.mock.calls.length).toBe(beforeDerive);
  expect(runtimeMock.updateCalls).toBe(beforeUpdate);

  // Default invalidation must handle mutable state rather than relying on identity.
  const previousSnapshot = core.state.snapshot;
  core.state.snapshot.time = 70;
  core.requestRender();
  expect(core.state.snapshot).toBe(previousSnapshot);
  expect(derive.mock.calls.length).toBeGreaterThan(beforeDerive);
  expect(runtimeMock.updateCalls).toBeGreaterThan(beforeUpdate);
  expect(document.querySelector('[data-world-tick="70"]')).toBeInTheDocument();
  let lastUpdate = runtimeMock.updateCalls;
  socket.receive({ type: "event", event: { time: 71, kind: "agent_waited", agent_id: "agent-0" } });
  expect(runtimeMock.updateCalls).toBeGreaterThan(lastUpdate);
  lastUpdate = runtimeMock.updateCalls;
  expect(core.applySelection({ kind: "agent", id: "agent-0" })).toEqual({ kind: "agent", id: "agent-0" });
  expect(runtimeMock.updateCalls).toBeGreaterThan(lastUpdate);
  lastUpdate = runtimeMock.updateCalls;
  core.state.auth.boundAgentId = null;
  core.requestRender();
  expect(runtimeMock.updateCalls).toBeGreaterThan(lastUpdate);
  lastUpdate = runtimeMock.updateCalls;
  socket.receive({ type: "snapshot", snapshot: { ...sampleSnapshot(), time: 72 } });
  expect(runtimeMock.updateCalls).toBeGreaterThan(lastUpdate);
  unsubscribe();
});
