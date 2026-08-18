import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

function installMockWebSocket() {
  const sockets = [];
  const sentMessages = [];
  class MockWebSocket {
    static CONNECTING = 0;
    static OPEN = 1;
    static CLOSING = 2;
    static CLOSED = 3;

    constructor(url) {
      this.url = url;
      this.readyState = MockWebSocket.CONNECTING;
      this.listeners = new Map();
      sockets.push(this);
    }

    addEventListener(type, listener) {
      const listeners = this.listeners.get(type) || [];
      listeners.push(listener);
      this.listeners.set(type, listeners);
    }

    send(payload) {
      sentMessages.push(JSON.parse(payload));
    }

    open() {
      this.readyState = MockWebSocket.OPEN;
      this.emit("open", {});
    }

    receive(message) {
      this.emit("message", { data: JSON.stringify(message) });
    }

    close() {
      this.readyState = MockWebSocket.CLOSED;
      this.emit("close", {});
    }

    emit(type, event) {
      for (const listener of this.listeners.get(type) || []) {
        listener(event);
      }
    }
  }
  Object.defineProperty(window, "WebSocket", { configurable: true, value: MockWebSocket });
  return { sockets, sentMessages };
}

function readyFeed(overrides = {}) {
  return {
    type: "world_feed",
    feed: {
      schema_version: "world_feed/v1",
      world_id: "test-world",
      reorg_epoch: 0,
      cursor: "wf1.cursor",
      status: "ready",
      events: [
        {
          event_seq: 7,
          kind: "resource_change",
          summary: "Ore changed",
          detail: "ore +1",
          receipt_ref: null,
        },
      ],
      gap_reason: null,
      unavailable_reason: null,
      snapshot_reload_required: false,
      ...overrides,
    },
  };
}

beforeEach(() => {
  vi.resetModules();
  vi.useFakeTimers();
  window.history.replaceState({}, "", "/software_safe.html?test_api=1&connect=1&hosted_bootstrap=0&locale=en&ws=ws://127.0.0.1:5011");
  document.body.innerHTML = "";
});

afterEach(() => {
  vi.restoreAllMocks();
  vi.useRealTimers();
});

describe("World Feed transport", () => {
  it("requests and consumes only the world_feed/v1 response through the existing socket path", async () => {
    const { sockets, sentMessages } = installMockWebSocket();
    const core = await import("./legacy_core.js");
    core.initializeSoftwareSafeCore();
    sockets[0].open();
    sockets[0].receive({ type: "hello_ack", server: "test-live", world_id: "test-world" });
    expect(sentMessages).toContainEqual({ type: "request_world_feed", cursor: null, limit: 50 });

    sockets[0].receive(readyFeed());
    expect(core.state.worldFeed.status).toBe("ready");
    expect(core.state.worldFeed.events).toHaveLength(1);
    expect(core.state.worldFeed.events[0].summary).toBe("Ore changed");
    expect(core.state.recentEvents).toEqual([]);
  });

  it("stops append on a runtime gap and asks the same socket path for an authoritative snapshot", async () => {
    const { sockets, sentMessages } = installMockWebSocket();
    const core = await import("./legacy_core.js");
    core.initializeSoftwareSafeCore();
    sockets[0].open();
    sockets[0].receive({ type: "hello_ack", server: "test-live", world_id: "test-world" });
    sockets[0].receive(readyFeed());
    const snapshotRequestsBeforeGap = sentMessages.filter((message) => message.type === "request_snapshot").length;

    sockets[0].receive(readyFeed({
      status: "gap",
      gap_reason: "reorg_epoch_changed",
      snapshot_reload_required: true,
      events: [{ event_seq: 8, kind: "must_not_append", summary: "must not append", detail: "", receipt_ref: null }],
    }));
    expect(core.state.worldFeed.status).toBe("gap");
    expect(core.state.worldFeed.stale).toBe(true);
    expect(core.state.worldFeed.events.map((event) => event.event_seq)).toEqual([7]);
    expect(sentMessages.filter((message) => message.type === "request_snapshot").length)
      .toBeGreaterThan(snapshotRequestsBeforeGap);
  });

  it("does not request a snapshot loop for source unavailable", async () => {
    const { sockets, sentMessages } = installMockWebSocket();
    const core = await import("./legacy_core.js");
    core.initializeSoftwareSafeCore();
    sockets[0].open();
    sockets[0].receive({ type: "hello_ack", server: "test-live", world_id: "test-world" });
    sockets[0].receive(readyFeed());
    const snapshotsBeforeUnavailable = sentMessages.filter((message) => message.type === "request_snapshot").length;

    sockets[0].receive(readyFeed({
      status: "unavailable",
      unavailable_reason: "source_unavailable",
      snapshot_reload_required: false,
      events: [],
    }));

    expect(core.state.worldFeed).toMatchObject({
      status: "unavailable",
      stale: true,
      snapshotReloadRequired: false,
    });
    expect(sentMessages.filter((message) => message.type === "request_snapshot")).toHaveLength(snapshotsBeforeUnavailable);
  });
});
