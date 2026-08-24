import { describe, expect, it } from "vitest";
import {
  createInitialWorldFeedState,
  consumeWorldFeed,
  requestWorldFeedState,
} from "./world_feed_state.js";

function feed(overrides = {}) {
  return {
    schema_version: "world_feed/v1",
    world_id: "world-a",
    reorg_epoch: 2,
    cursor: "wf1.cursor-2",
    status: "ready",
    events: [
      {
        event_seq: 2,
        kind: "resource_change",
        summary: "Resource changed",
        detail: "ore +1",
        receipt_ref: null,
      },
      {
        event_seq: 1,
        kind: "agent_spoke",
        summary: "Agent spoke",
        detail: "hello",
        receipt_ref: "receipt-1",
      },
    ],
    gap_reason: null,
    unavailable_reason: null,
    snapshot_reload_required: false,
    ...overrides,
  };
}

describe("World Feed v1 state", () => {
  it("starts in an explicit loading state and records a request", () => {
    const initial = createInitialWorldFeedState();
    expect(initial.status).toBe("loading");
    expect(initial.events).toEqual([]);
    expect(requestWorldFeedState(initial, { cursor: null, limit: 50 })).toMatchObject({
      status: "loading",
      requestInFlight: true,
      cursor: null,
    });
  });

  it("stores ascending source order, appends by world/epoch/seq identity, and preserves null receipts", () => {
    const initial = createInitialWorldFeedState();
    const first = consumeWorldFeed(initial, feed());
    expect(first.state.status).toBe("ready");
    expect(first.state.events.map((event) => event.event_seq)).toEqual([1, 2]);
    expect(first.state.events.find((event) => event.event_seq === 2)?.receipt_ref).toBeNull();
    const replay = consumeWorldFeed(first.state, feed({ status: "replay", cursor: "wf1.cursor-3" }));
    expect(replay.state.events).toHaveLength(2);
    expect(replay.state.events.map((event) => event.event_seq)).toEqual([1, 2]);
    expect(replay.state.dedupedCount).toBe(2);
  });

  it("accepts the initial runtime epoch zero", () => {
    const consumed = consumeWorldFeed(createInitialWorldFeedState(), feed({ reorg_epoch: 0 }));
    expect(consumed.state).toMatchObject({ status: "ready", reorgEpoch: "0" });
    expect(consumed.requiresSnapshotReload).toBe(false);
  });

  it("stores an ascending exact decimal u64 sequence and deduplicates beyond MAX_SAFE_INTEGER", () => {
    const initial = createInitialWorldFeedState();
    const first = consumeWorldFeed(initial, feed({
      events: [
        { event_seq: "9007199254740993", kind: "newer", summary: "Newer event", detail: "", receipt_ref: null },
        { event_seq: "9007199254740992", kind: "older", summary: "Older event", detail: "", receipt_ref: null },
      ],
    }));

    expect(first.state.events.map((event) => event.event_seq)).toEqual([
      "9007199254740992",
      "9007199254740993",
    ]);

    const replay = consumeWorldFeed(first.state, feed({
      status: "replay",
      events: [
        { event_seq: "9007199254740993", kind: "newer", summary: "Newer event", detail: "", receipt_ref: null },
        { event_seq: "9007199254740992", kind: "older", summary: "Older event", detail: "", receipt_ref: null },
      ],
    }));
    expect(replay.state.events.map((event) => event.event_seq)).toEqual([
      "9007199254740992",
      "9007199254740993",
    ]);
    expect(replay.state.dedupedCount).toBe(2);
  });

  it("stops appending on gap/reorg and requires authoritative snapshot reload", () => {
    const initial = createInitialWorldFeedState();
    const first = consumeWorldFeed(initial, feed()).state;
    const gap = consumeWorldFeed(first, feed({
      status: "gap",
      events: [{ event_seq: 3, kind: "unknown", summary: "must not append", detail: "", receipt_ref: null }],
      gap_reason: "reorg_epoch_changed",
      snapshot_reload_required: true,
    }));
    expect(gap.state.status).toBe("gap");
    expect(gap.state.stale).toBe(true);
    expect(gap.state.events).toEqual([]);
    expect(gap.requiresSnapshotReload).toBe(true);

    const recovered = consumeWorldFeed(gap.state, feed({
      reorg_epoch: 3,
      cursor: "wf1.cursor-3",
      events: [{ event_seq: 1, kind: "reorg_ready", summary: "new epoch", detail: "", receipt_ref: null }],
    }));
    expect(recovered.state.status).toBe("ready");
    expect(recovered.state.events).toEqual([
      { event_seq: 1, kind: "reorg_ready", summary: "new epoch", detail: "", receipt_ref: null },
    ]);
  });

  it("fails closed for unsupported schema and unknown statuses without splicing events", () => {
    const initial = createInitialWorldFeedState();
    const unsupported = consumeWorldFeed(initial, feed({ schema_version: "world_feed/v9" }));
    expect(unsupported.state.status).toBe("unavailable");
    expect(unsupported.state.unavailableReason).toBe("schema_unsupported");
    expect(unsupported.state.events).toEqual([]);
    const invalid = consumeWorldFeed(initial, feed({ status: "mystery" }));
    expect(invalid.state.status).toBe("unavailable");
    expect(invalid.state.unavailableReason).toBe("source_unavailable");
    expect(invalid.state.events).toEqual([]);
  });

  it("keeps unavailable distinct from a continuity gap", () => {
    const unavailable = consumeWorldFeed(createInitialWorldFeedState(), feed({
      status: "unavailable",
      unavailable_reason: "source_unavailable",
      snapshot_reload_required: false,
      events: [],
    }));
    expect(unavailable.state).toMatchObject({
      status: "unavailable",
      stale: true,
      snapshotReloadRequired: false,
    });
    expect(unavailable.requiresSnapshotReload).toBe(false);
  });
});
