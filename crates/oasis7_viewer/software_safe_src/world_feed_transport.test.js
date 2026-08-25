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
  // Keep these transport tests on the guest/auth-independent lane.  The
  // loopback test API otherwise auto-issues a local player session, and a
  // reconnect handshake can race its async reconnect_sync attempt with the
  // fake socket lifecycle, producing an unrelated unhandled send rejection.
  window.history.replaceState({}, "", "/software_safe.html?test_api=1&connect=1&hosted_bootstrap=0&hosted_access=%7B%22deployment_mode%22%3A%22hosted_public_join%22%7D&locale=en&ws=ws://127.0.0.1:5011");
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

  it("continues an initial page asynchronously from the returned cursor", async () => {
    const { sockets, sentMessages } = installMockWebSocket();
    const core = await import("./legacy_core.js");
    core.initializeSoftwareSafeCore();
    sockets[0].open();
    sockets[0].receive({ type: "hello_ack", server: "test-live", world_id: "test-world" });

    expect(sentMessages.filter((message) => message.type === "request_world_feed")).toHaveLength(1);
    sockets[0].receive(readyFeed({ cursor: "wf1.cursor-2" }));

    // Continuation must be deferred so the response handler cannot recurse
    // synchronously, while still issuing exactly one request for the latest cursor.
    expect(sentMessages.filter((message) => message.type === "request_world_feed")).toHaveLength(1);
    await vi.advanceTimersByTimeAsync(0);
    const worldFeedRequests = sentMessages.filter((message) => message.type === "request_world_feed");
    expect(worldFeedRequests).toHaveLength(2);
    expect(worldFeedRequests.at(-1)).toEqual({
      type: "request_world_feed",
      cursor: "wf1.cursor-2",
      limit: 50,
    });
  });

  it("drops a queued continuation when a gap arrives and lets snapshot recovery restart at null", async () => {
    const { sockets, sentMessages } = installMockWebSocket();
    const core = await import("./legacy_core.js");
    core.initializeSoftwareSafeCore();
    sockets[0].open();
    sockets[0].receive({ type: "hello_ack", server: "test-live", world_id: "test-world" });

    sockets[0].receive(readyFeed({ cursor: "wf1.cursor-2" }));
    sockets[0].receive(readyFeed({
      reorg_epoch: 1,
      cursor: "wf1.cursor-gap",
      status: "gap",
      gap_reason: "reorg_epoch_changed",
      snapshot_reload_required: true,
      events: [],
    }));
    const requestsBeforeTimer = sentMessages.filter((message) => message.type === "request_world_feed");

    await vi.advanceTimersByTimeAsync(0);

    expect(sentMessages.filter((message) => message.type === "request_world_feed")).toEqual(requestsBeforeTimer);
    expect(core.state.worldFeed.snapshotReloadRequired).toBe(true);

    sockets[0].receive({ type: "snapshot", snapshot: { model: {} } });
    const worldFeedRequests = sentMessages.filter((message) => message.type === "request_world_feed");
    expect(worldFeedRequests.at(-1)).toEqual({
      type: "request_world_feed",
      cursor: null,
      limit: 50,
    });
  });

  it("does not refresh the feed from world activity while a gap awaits snapshot recovery", async () => {
    const { sockets, sentMessages } = installMockWebSocket();
    const core = await import("./legacy_core.js");
    core.initializeSoftwareSafeCore();
    sockets[0].open();
    sockets[0].receive({ type: "hello_ack", server: "test-live", world_id: "test-world" });
    sockets[0].receive(readyFeed({ cursor: "wf1.cursor-2" }));

    sockets[0].receive(readyFeed({
      reorg_epoch: 1,
      cursor: "wf1.cursor-gap",
      status: "gap",
      gap_reason: "reorg_epoch_changed",
      snapshot_reload_required: true,
      events: [],
    }));
    const requestsBeforeEvent = sentMessages.filter((message) => message.type === "request_world_feed");

    sockets[0].receive({
      type: "event",
      event: { id: 8, time: 1, kind: { type: "resource_change" } },
    });
    await vi.advanceTimersByTimeAsync(0);

    expect(sentMessages.filter((message) => message.type === "request_world_feed")).toEqual(requestsBeforeEvent);
    expect(core.state.worldFeed.snapshotReloadRequired).toBe(true);
  });

  it("cancels an old continuation across reconnect and sends one fresh null request", async () => {
    const { sockets, sentMessages } = installMockWebSocket();
    const core = await import("./legacy_core.js");
    core.initializeSoftwareSafeCore();
    core.state.auth.available = false;
    sockets[0].open();
    sockets[0].receive({ type: "hello_ack", server: "test-live", world_id: "test-world" });
    sockets[0].receive(readyFeed({ cursor: "wf1.cursor-2" }));

    sockets[0].close();
    await vi.advanceTimersByTimeAsync(0);
    expect(sentMessages.filter((message) => message.type === "request_world_feed")).toHaveLength(1);

    await vi.advanceTimersByTimeAsync(1200);
    expect(sockets).toHaveLength(2);
    sockets[1].open();
    sockets[1].receive({ type: "hello_ack", server: "test-live", world_id: "test-world" });
    await vi.advanceTimersByTimeAsync(0);

    const worldFeedRequests = sentMessages.filter((message) => message.type === "request_world_feed");
    expect(worldFeedRequests).toHaveLength(2);
    expect(worldFeedRequests[0]).toEqual({ type: "request_world_feed", cursor: null, limit: 50 });
    expect(worldFeedRequests[1]).toEqual({ type: "request_world_feed", cursor: null, limit: 50 });
    vi.clearAllTimers();
  });

  it("continues after reconnect when the first page returns the same cursor as the stale local state", async () => {
    const { sockets, sentMessages } = installMockWebSocket();
    const core = await import("./legacy_core.js");
    core.initializeSoftwareSafeCore();
    core.state.auth.available = false;

    sockets[0].open();
    sockets[0].receive({ type: "hello_ack", server: "test-live", world_id: "test-world" });
    sockets[0].receive(readyFeed({ cursor: "wf1.cursor" }));
    await vi.advanceTimersByTimeAsync(0);
    sockets[0].receive(readyFeed({ cursor: "wf1.cursor" }));

    sockets[0].close();
    await vi.advanceTimersByTimeAsync(1200);
    expect(sockets).toHaveLength(2);
    sockets[1].open();
    sockets[1].receive({ type: "hello_ack", server: "test-live", world_id: "test-world" });
    const beforeResponse = sentMessages.filter((message) => message.type === "request_world_feed").length;
    sockets[1].receive(readyFeed({ cursor: "wf1.cursor" }));
    await vi.advanceTimersByTimeAsync(0);

    const requests = sentMessages.filter((message) => message.type === "request_world_feed");
    expect(requests.length).toBe(beforeResponse + 1);
    expect(requests.at(-1)).toEqual({
      type: "request_world_feed",
      cursor: "wf1.cursor",
      limit: 50,
    });
    vi.clearAllTimers();
  });

  it("refreshes the latest cursor once after world activity and does not overlap in-flight requests", async () => {
    const { sockets, sentMessages } = installMockWebSocket();
    const core = await import("./legacy_core.js");
    core.initializeSoftwareSafeCore();
    sockets[0].open();
    sockets[0].receive({ type: "hello_ack", server: "test-live", world_id: "test-world" });
    sockets[0].receive(readyFeed({ cursor: "wf1.cursor-2" }));
    await vi.advanceTimersByTimeAsync(0);

    // A response for the continuation makes the transport idle at the latest cursor.
    sockets[0].receive(readyFeed({ cursor: "wf1.cursor-2" }));
    const idleRequestCount = sentMessages.filter((message) => message.type === "request_world_feed").length;

    sockets[0].receive({
      type: "event",
      event: { id: 8, time: 1, kind: { type: "resource_change" } },
    });
    sockets[0].receive({
      type: "event",
      event: { id: 9, time: 2, kind: { type: "resource_change" } },
    });
    await vi.advanceTimersByTimeAsync(0);

    const worldFeedRequests = sentMessages.filter((message) => message.type === "request_world_feed");
    expect(worldFeedRequests).toHaveLength(idleRequestCount + 1);
    expect(worldFeedRequests.at(-1)).toMatchObject({
      type: "request_world_feed",
      cursor: "wf1.cursor-2",
    });
    expect(core.state.worldFeed.requestInFlight).toBe(true);
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
    expect(core.state.worldFeed.events).toEqual([]);
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

    const worldFeedRequestsBeforeRetry = sentMessages.filter((message) => message.type === "request_world_feed").length;
    expect(core.requestWorldFeed({ cursor: null, limit: 50 })).toBe(true);
    const worldFeedRequests = sentMessages.filter((message) => message.type === "request_world_feed");
    expect(worldFeedRequests).toHaveLength(worldFeedRequestsBeforeRetry + 1);
    expect(worldFeedRequests.at(-1)).toEqual({ type: "request_world_feed", cursor: null, limit: 50 });
    expect(sentMessages.filter((message) => message.type === "request_snapshot")).toHaveLength(snapshotsBeforeUnavailable);
  });
});
