import { createSignal } from "solid-js";
import { fireEvent, render, screen, waitFor } from "@solidjs/testing-library";
import { describe, expect, it, vi } from "vitest";
import { WorldFeedSurface } from "./world_feed_integration.jsx";

const tr = (_locale, _zh, en) => en;

function installMockWebSocket() {
  const sockets = [];
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

    send() {}

    open() {
      this.readyState = MockWebSocket.OPEN;
      this.emit("open", {});
    }

    receive(message) {
      this.emit("message", { data: JSON.stringify(message) });
    }

    emit(type, event) {
      for (const listener of this.listeners.get(type) || []) {
        listener(event);
      }
    }
  }
  Object.defineProperty(window, "WebSocket", { configurable: true, value: MockWebSocket });
  return sockets;
}

describe("WorldFeedSurface", () => {
  it("passes unavailable-feed retry through its distinct callback", () => {
    const onRetryFeed = vi.fn();
    const onReloadSnapshot = vi.fn();
    const core = {
      state: {
        worldFeed: {
          status: "unavailable",
          events: [],
          worldId: "world-a",
          reorgEpoch: "0",
          unavailableReason: "permission_denied",
          snapshotReloadRequired: false,
          stale: true,
        },
      },
    };

    render(() => (
      <WorldFeedSurface
        core={core}
        locale={() => "en"}
        tr={tr}
        onRetryFeed={onRetryFeed}
        onReloadSnapshot={onReloadSnapshot}
      />
    ));

    fireEvent.click(screen.getByRole("button", { name: /retry world feed/i }));
    expect(onRetryFeed).toHaveBeenCalledTimes(1);
    expect(onReloadSnapshot).not.toHaveBeenCalled();
  });

  it("opens the visible command details route after locating a live module", async () => {
    const detailsPanel = document.createElement("section");
    detailsPanel.id = "viewer-details-panel";
    detailsPanel.tabIndex = -1;
    document.body.appendChild(detailsPanel);
    const core = {
      state: {
        worldFeed: {
          status: "ready",
          events: [{ event_seq: 101, kind: "ModuleVisualEntityUpserted", module_visual_entity_id: "module-relay" }],
        },
      },
      entityCollections: () => ({ moduleVisualEntities: [{ id: "module-relay", label: "Relay Seven" }] }),
      focusModuleFromEvent: vi.fn(() => ({ kind: "module_visual", id: "module-relay" })),
    };

    render(() => <WorldFeedSurface core={core} locale={() => "en"} tr={tr} />);
    fireEvent.click(screen.getByRole("button", { name: /locate module relay seven/i }));

    expect(core.focusModuleFromEvent).toHaveBeenCalledTimes(1);
    expect(window.location.hash).toBe("#viewer-details-panel");
    expect(document.activeElement).toBe(detailsPanel);
  });

  it("observes the viewer revision when plain feed and snapshot accessors are replaced", async () => {
    const [revision, setRevision] = createSignal(0);
    const core = {
      state: { worldFeed: { status: "empty", events: [] }, snapshot: null },
      entityCollections: () => ({
        moduleVisualEntities: Object.values(core.state.snapshot?.model?.module_visual_entities || {}).map((entry) => ({
          ...entry,
          id: entry.entity_id || entry.id,
        })),
      }),
    };
    const feedEvent = {
      event_seq: 101,
      kind: "module_visual_entity_upserted",
      summary: "Relay marker published",
      module_visual_entity_id: "module-relay",
    };

    render(() => (
      <WorldFeedSurface
        core={core}
        locale={() => "en"}
        tr={tr}
        observeState={() => revision()}
      />
    ));
    expect(screen.queryByRole("button", { name: /locate module/i })).not.toBeInTheDocument();

    core.state.worldFeed = { status: "ready", events: [feedEvent] };
    core.state.snapshot = {
      model: {
        module_visual_entities: {
          "module-relay": { entity_id: "module-relay", label: "Relay Seven" },
        },
      },
    };
    setRevision(1);
    await waitFor(() => {
      expect(screen.getByRole("button", { name: /locate module relay seven/i })).toBeInTheDocument();
    });

    core.state.snapshot = { model: { module_visual_entities: {} } };
    setRevision(2);
    await waitFor(() => {
      expect(screen.queryByRole("button", { name: /locate module relay seven/i })).not.toBeInTheDocument();
    });
  });

  it("refreshes locate availability after post-mount feed and snapshot replacement", async () => {
    const originalWebSocket = window.WebSocket;
    const sockets = installMockWebSocket();
    window.history.replaceState({}, "", "/software_safe.html?test_api=1&connect=1&hosted_bootstrap=0&locale=en&ws=ws://127.0.0.1:5011");

    try {
      vi.resetModules();
      const core = await import("./legacy_core.js");
      core.initializeSoftwareSafeCore();
      render(() => <WorldFeedSurface core={core} locale={() => "en"} tr={tr} />);
      expect(sockets).toHaveLength(1);
      expect(screen.queryByRole("button", { name: /locate module/i })).not.toBeInTheDocument();

      sockets[0].open();
      sockets[0].receive({ type: "hello_ack", server: "test-live", world_id: "test-world" });
      sockets[0].receive({
        type: "world_feed",
        feed: {
          schema_version: "world_feed/v1",
          world_id: "test-world",
          reorg_epoch: 0,
          cursor: "wf1.cursor-101",
          status: "ready",
          events: [{
            event_seq: 101,
            kind: "module_visual_entity_upserted",
            summary: "Relay marker published",
            detail: "module",
            receipt_ref: null,
            module_visual_entity_id: "module-relay",
          }],
          gap_reason: null,
          unavailable_reason: null,
          snapshot_reload_required: false,
        },
      });
      await waitFor(() => {
        expect(screen.getByText("Relay marker published")).toBeInTheDocument();
        expect(screen.queryByRole("button", { name: /locate module/i })).not.toBeInTheDocument();
      });

      sockets[0].receive({
        type: "snapshot",
        snapshot: {
          time: 1,
          model: {
            module_visual_entities: {
              "module-relay": {
                entity_id: "module-relay",
                module_id: "module-7",
                kind: "relay",
                label: "Relay Seven",
              },
            },
          },
        },
      });
      await waitFor(() => {
        expect(screen.getByRole("button", { name: /locate module relay seven/i })).toBeInTheDocument();
      });

      sockets[0].receive({
        type: "snapshot",
        snapshot: { time: 2, model: { module_visual_entities: {} } },
      });
      await waitFor(() => {
        expect(screen.queryByRole("button", { name: /locate module relay seven/i })).not.toBeInTheDocument();
      });
    } finally {
      Object.defineProperty(window, "WebSocket", { configurable: true, value: originalWebSocket });
    }
  });
});
