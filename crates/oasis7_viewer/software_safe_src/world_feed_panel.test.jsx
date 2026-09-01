import { fireEvent, render, screen } from "@solidjs/testing-library";
import { describe, expect, it, vi } from "vitest";
import { WorldFeedPanel } from "./world_feed_panel.jsx";

const tr = (_locale, zh, en) => en;

const majorEvent = (overrides = {}) => ({
  schema_version: "major_world_event/v1",
  identity: { world_id: "world-a", reorg_epoch: 2, event_seq: 7 },
  category: "crisis",
  severity: 4,
  lifecycle: "active",
  source: { authority: "runtime_journal", event_kind: "crisis_spawned" },
  freshness: "current",
  visibility: "public",
  logical_time: 42,
  causal_reference: { type: "action", data: 44 },
  world_anchor: null,
  ...overrides,
});

describe("WorldFeedPanel", () => {
  it("keeps an explicit contextual surface for every protocol state", () => {
    const onReloadSnapshot = vi.fn();
    const onRetryFeed = vi.fn();
    const { container } = render(() => (
      <WorldFeedPanel
        feed={() => ({
          status: "gap",
          events: [],
          worldId: "world-a",
          reorgEpoch: 4,
          gapReason: "reorg_epoch_changed",
          snapshotReloadRequired: true,
          stale: true,
        })}
        locale={() => "en"}
        tr={tr}
        onReloadSnapshot={onReloadSnapshot}
        onRetryFeed={onRetryFeed}
      />
    ));
    expect(container.querySelector("#viewer-world-feed")).toBeTruthy();
    expect(screen.getByText("World activity is stale")).toBeInTheDocument();
    const reloadButton = screen.getByRole("button", { name: /reload authoritative snapshot/i });
    expect(reloadButton).toBeInTheDocument();
    fireEvent.click(reloadButton);
    expect(onReloadSnapshot).toHaveBeenCalledTimes(1);
    expect(onRetryFeed).not.toHaveBeenCalled();
  });

  it("offers a distinct World Feed retry for an unavailable source without snapshot recovery", () => {
    const onReloadSnapshot = vi.fn();
    const onRetryFeed = vi.fn();
    render(() => (
      <WorldFeedPanel
        feed={() => ({
          status: "unavailable",
          events: [],
          worldId: "world-a",
          reorgEpoch: "0",
          unavailableReason: "source_unavailable",
          snapshotReloadRequired: false,
          stale: true,
        })}
        locale={() => "en"}
        tr={tr}
        onReloadSnapshot={onReloadSnapshot}
        onRetryFeed={onRetryFeed}
      />
    ));
    expect(screen.getByText("World activity unavailable")).toBeInTheDocument();
    const retryButton = screen.getByRole("button", { name: /retry world feed/i });
    expect(retryButton).toBeInTheDocument();
    fireEvent.click(retryButton);
    expect(onRetryFeed).toHaveBeenCalledTimes(1);
    expect(onReloadSnapshot).not.toHaveBeenCalled();
    expect(screen.queryByRole("button", { name: /reload authoritative snapshot/i })).not.toBeInTheDocument();
  });

  it("explains why an empty feed is expected and names the next authoritative signal", () => {
    render(() => (
      <WorldFeedPanel
        feed={() => ({ status: "empty", events: [] })}
        locale={() => "en"}
        tr={tr}
      />
    ));

    expect(screen.getByText("No world activity yet")).toBeInTheDocument();
    expect(screen.getByText(
      "No authoritative world update has published events yet. This feed is context only—continue your Player goal; the feed will update after the next authoritative world update.",
    )).toBeInTheDocument();
  });

  it("renders events as ambient context and links only explicit receipt refs", () => {
    render(() => (
      <WorldFeedPanel
        feed={() => ({
          status: "ready",
          events: [
            { event_seq: 7, kind: "resource_change", summary: "Ore changed", detail: "ore +1", receipt_ref: null },
            { event_seq: 8, kind: "agent_spoke", summary: "Agent spoke", detail: "hello", receipt_ref: "receipt-8" },
          ],
          worldId: "world-a",
          reorgEpoch: 1,
          stale: false,
        })}
        locale={() => "en"}
        tr={tr}
        onReloadSnapshot={() => {}}
      />
    ));
    expect(screen.getByText("World Feed")).toBeInTheDocument();
    expect(screen.getByText("Ore changed")).toBeInTheDocument();
    expect(screen.getByText("Agent spoke")).toBeInTheDocument();
    expect(document.querySelectorAll("a[data-world-feed-receipt-ref]")).toHaveLength(1);
  });

  it("formats runtime kinds and keeps diagnostic JSON out of the player feed", () => {
    render(() => (
      <WorldFeedPanel
        feed={() => ({
          status: "ready",
          events: [{
            event_seq: 9,
            kind: "snapshot_created",
            summary: "World snapshot updated",
            detail: '{"kind":"SnapshotCreated","internal_state":"raw"}',
            receipt_ref: null,
          }],
        })}
        locale={() => "en"}
        tr={tr}
      />
    ));

    expect(screen.getByText("World snapshot")).toBeInTheDocument();
    expect(screen.queryByText("snapshot_created")).not.toBeInTheDocument();
    expect(document.body).not.toHaveTextContent("internal_state");
  });

  it("presents World Feed events in ascending event sequence order", () => {
    const { container } = render(() => (
      <WorldFeedPanel
        feed={() => ({
          status: "ready",
          events: [
            { event_seq: 7, kind: "resource_change", summary: "Older event", detail: "ore +1", receipt_ref: null },
            { event_seq: 8, kind: "agent_spoke", summary: "Newer event", detail: "hello", receipt_ref: null },
          ],
        })}
        locale={() => "en"}
        tr={tr}
      />
    ));
    expect(Array.from(container.querySelectorAll("[data-world-feed-event]"), (event) => event.getAttribute("data-world-feed-event"))).toEqual([
      "7",
      "8",
    ]);
  });

  it("sorts exact decimal u64 sequences without Number precision loss", () => {
    const { container } = render(() => (
      <WorldFeedPanel
        feed={() => ({
          status: "ready",
          events: [
            { event_seq: "9007199254740993", kind: "newer", summary: "Newer exact event", detail: "", receipt_ref: null },
            { event_seq: "9007199254740992", kind: "older", summary: "Older exact event", detail: "", receipt_ref: null },
          ],
        })}
        locale={() => "en"}
        tr={tr}
      />
    ));

    expect(Array.from(container.querySelectorAll("[data-world-feed-event]"), (event) => event.getAttribute("data-world-feed-event"))).toEqual([
      "9007199254740992",
      "9007199254740993",
    ]);
  });

  it("renders an anchored-less current crisis as ambient context, never as a stage marker or formal receipt", () => {
    render(() => (
      <WorldFeedPanel
        feed={() => ({
          status: "ready",
          events: [{
            event_seq: 7,
            kind: "major_world_event",
            summary: "A crisis is active",
            detail: "Ambient crisis context",
            receipt_ref: null,
            major_event: majorEvent(),
          }],
          worldId: "world-a",
          reorgEpoch: 2,
          stale: false,
        })}
        locale={() => "en"}
        tr={tr}
      />
    ));

    const event = document.querySelector('[data-world-feed-major-event="7"]');
    expect(event).toBeInTheDocument();
    expect(event).toHaveAttribute("data-major-event-category", "crisis");
    expect(event).toHaveAttribute("data-major-event-lifecycle", "active");
    expect(event).toHaveAttribute("data-major-event-severity", "4");
    expect(event.querySelector("[data-major-event-stage-marker]")).toBeNull();
    expect(event.querySelector("[data-major-event-highlight]")).toBeNull();
    expect(event.querySelector("[data-world-feed-receipt-ref]")).toBeNull();
    expect(document.querySelectorAll("#viewer-action-receipt")).toHaveLength(0);
  });

  it("keeps replayed historical crisis context in the feed without a toast or attention announcement", () => {
    render(() => (
      <WorldFeedPanel
        feed={() => ({
          status: "replay",
          events: [{
            event_seq: 7,
            kind: "major_world_event",
            summary: "Historical crisis",
            detail: "Replay context",
            receipt_ref: null,
            major_event: majorEvent({ freshness: "replay" }),
          }],
          worldId: "world-a",
          reorgEpoch: 2,
          stale: false,
        })}
        locale={() => "en"}
        tr={tr}
      />
    ));

    expect(document.querySelector('[data-world-feed-major-event="7"]')).toBeInTheDocument();
    expect(document.querySelector('[data-world-feed-major-event-toast="7"]')).toBeNull();
    expect(document.querySelector('[data-world-feed-major-event="7"] [role="status"]')).toBeNull();
  });

  it("provides a CJK-readable polite status for current crisis context without leaking raw protocol enums", () => {
    const chineseTr = (_locale, zh) => zh;
    render(() => (
      <WorldFeedPanel
        feed={() => ({
          status: "ready",
          events: [{
            event_seq: 7,
            kind: "major_world_event",
            summary: "发生危机",
            detail: "环境上下文",
            receipt_ref: null,
            major_event: majorEvent(),
          }],
        })}
        locale={() => "zh-CN"}
        tr={chineseTr}
      />
    ));

    const status = screen.getByRole("status");
    expect(status).toHaveAttribute("aria-live", "polite");
    expect(status).toHaveTextContent(/危机/);
    expect(document.body).toHaveTextContent(/重大世界事件/);
    expect(document.body).not.toHaveTextContent(/crisis_spawned|runtime_journal|public|current/);
  });

  it("does not surface a major event after permission is lost", () => {
    render(() => (
      <WorldFeedPanel
        feed={() => ({
          status: "unavailable",
          unavailableReason: "permission_denied",
          stale: true,
          events: [],
        })}
        locale={() => "en"}
        tr={tr}
      />
    ));

    expect(screen.getByText("World activity unavailable")).toBeInTheDocument();
    expect(document.querySelector("[data-world-feed-major-event]")).toBeNull();
    expect(document.querySelector("[data-world-feed-major-event-toast]")).toBeNull();
  });
});
