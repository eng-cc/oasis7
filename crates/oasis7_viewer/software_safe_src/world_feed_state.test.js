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

function majorEvent(overrides = {}) {
  const base = {
    schema_version: "major_world_event/v1",
    identity: {
      world_id: "world-a",
      reorg_epoch: 2,
      event_seq: 7,
    },
    category: "crisis",
    subtype: "power_shortage",
    severity: 4,
    lifecycle: "active",
    source: {
      authority: "runtime_journal",
      event_kind: "crisis_spawned",
    },
    freshness: "current",
    visibility: "public",
    logical_time: 42,
    causal_reference: null,
    world_anchor: {
      scope: "world",
      entity_id: "crisis-1",
    },
  };
  return {
    ...base,
    ...overrides,
    identity: { ...base.identity, ...(overrides.identity || {}) },
    source: { ...base.source, ...(overrides.source || {}) },
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

  it("fails closed and requests snapshot recovery for a materially conflicting duplicate identity", () => {
    const first = consumeWorldFeed(createInitialWorldFeedState(), feed()).state;
    const conflict = consumeWorldFeed(first, feed({
      status: "replay",
      events: [{
        event_seq: 1,
        kind: "agent_spoke",
        summary: "Conflicting payload",
        detail: "different",
        receipt_ref: null,
      }],
    }));
    expect(conflict).toMatchObject({ requiresSnapshotReload: true });
    expect(conflict.state).toMatchObject({
      status: "gap",
      gapReason: "event_identity_conflict",
      stale: true,
      snapshotReloadRequired: true,
      events: [],
    });
  });

  it("deduplicates equivalent numeric and canonical decimal-string identities", () => {
    const event = {
      event_seq: 7,
      kind: "major_world_event",
      summary: "Same crisis",
      detail: "ambient only",
      receipt_ref: null,
      major_event: majorEvent(),
    };
    const first = consumeWorldFeed(createInitialWorldFeedState(), feed({ events: [event] })).state;
    const replay = consumeWorldFeed(first, feed({
      status: "replay",
      events: [{
        ...event,
        event_seq: "7",
        major_event: {
          ...event.major_event,
          identity: { world_id: "world-a", reorg_epoch: "2", event_seq: "7" },
        },
      }],
    }));

    expect(replay.requiresSnapshotReload).toBe(false);
    expect(replay.state.status).toBe("replay");
    expect(replay.state.events).toHaveLength(1);
    expect(replay.state.dedupedCount).toBe(1);
  });

  it("normalizes a blank receipt reference to no receipt", () => {
    const consumed = consumeWorldFeed(createInitialWorldFeedState(), feed({
      events: [{
        event_seq: 7,
        kind: "resource_change",
        summary: "No durable receipt",
        detail: "",
        receipt_ref: "   ",
      }],
    }));

    expect(consumed.state.events[0].receipt_ref).toBeNull();
  });

  it("fails closed when one page contains conflicting rows for the same identity", () => {
    const conflict = consumeWorldFeed(createInitialWorldFeedState(), feed({
      events: [
        {
          event_seq: 7,
          kind: "resource_change",
          summary: "First payload",
          detail: "ore +1",
          receipt_ref: null,
        },
        {
          event_seq: 7,
          kind: "resource_change",
          summary: "Conflicting payload",
          detail: "ore +2",
          receipt_ref: null,
        },
      ],
    }));
    expect(conflict).toMatchObject({ requiresSnapshotReload: true });
    expect(conflict.state).toMatchObject({
      status: "gap",
      gapReason: "event_identity_conflict",
      snapshotReloadRequired: true,
      events: [],
    });
  });

  it("preserves an explicit runtime identity-conflict gap", () => {
    const conflict = consumeWorldFeed(createInitialWorldFeedState(), feed({
      status: "gap",
      events: [],
      gap_reason: "event_identity_conflict",
      snapshot_reload_required: true,
    }));

    expect(conflict).toMatchObject({ requiresSnapshotReload: true });
    expect(conflict.state).toMatchObject({
      status: "gap",
      gapReason: "event_identity_conflict",
      snapshotReloadRequired: true,
      events: [],
    });
  });

  it("fails the envelope when a present major-event authority payload is invalid", () => {
    const consumed = consumeWorldFeed(createInitialWorldFeedState(), feed({
      events: [
        {
          event_seq: 7,
          kind: "major_world_event",
          summary: "invalid severity",
          detail: "",
          receipt_ref: null,
          major_event: majorEvent({ severity: 0 }),
        },
        {
          event_seq: 7,
          kind: "major_world_event",
          summary: "invalid severity",
          detail: "",
          receipt_ref: null,
          major_event: majorEvent({ severity: 6 }),
        },
      ],
    }));

    expect(consumed).toMatchObject({ requiresSnapshotReload: true });
    expect(consumed.state).toMatchObject({
      status: "unavailable",
      unavailableReason: "source_unavailable",
      events: [],
    });
  });

  it("treats every consumable envelope that changes epoch as reorg requiring snapshot reload", () => {
    for (const status of ["ready", "empty", "replay"]) {
      const first = consumeWorldFeed(createInitialWorldFeedState(), feed()).state;
      const changed = consumeWorldFeed(first, feed({
        status,
        reorg_epoch: 3,
        ...(status === "empty" ? { events: [] } : {}),
      }));
      expect(changed.requiresSnapshotReload, status).toBe(true);
      expect(changed.state, status).toMatchObject({
        status: "gap",
        snapshotReloadRequired: true,
        events: [],
      });
    }
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

  it("clears previously visible events when a later envelope is unsupported or malformed", () => {
    const populated = consumeWorldFeed(createInitialWorldFeedState(), feed()).state;
    expect(populated.events).toHaveLength(2);

    const unsupported = consumeWorldFeed(populated, feed({ schema_version: "world_feed/v9" }));
    expect(unsupported.state).toMatchObject({ status: "unavailable", events: [] });

    const malformed = consumeWorldFeed(populated, feed({ status: "mystery" }));
    expect(malformed.state).toMatchObject({ status: "unavailable", events: [] });
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

  it("normalizes an additive current crisis projection without promoting causal context to a receipt", () => {
    const consumed = consumeWorldFeed(createInitialWorldFeedState(), feed({
      events: [{
        event_seq: 7,
        kind: "major_world_event",
        summary: "A crisis is active",
        detail: "ambient context",
        receipt_ref: null,
        major_event: majorEvent({
          causal_reference: { type: "action", data: 44 },
        }),
      }],
    }));

    const event = consumed.state.events[0];
    expect(event.major_event).toMatchObject({
      schema_version: "major_world_event/v1",
      category: "crisis",
      subtype: "power_shortage",
      severity: 4,
      lifecycle: "active",
      freshness: "current",
      visibility: "public",
      source: {
        authority: "runtime_journal",
        event_kind: "crisis_spawned",
      },
      identity: {
        world_id: "world-a",
        reorg_epoch: 2,
        event_seq: 7,
      },
      causal_reference: { type: "action", data: 44 },
    });
    expect(event.receipt_ref).toBeNull();
  });

  it("accepts only exact crisis lifecycle/source pairs and the numeric severity range", () => {
    for (const [eventKind, lifecycle] of [
      ["crisis_spawned", "active"],
      ["crisis_resolved", "resolved"],
      ["crisis_timed_out", "timed_out"],
    ]) {
      const consumed = consumeWorldFeed(createInitialWorldFeedState(), feed({
        events: [{
          event_seq: 7,
          kind: "major_world_event",
          summary: `${eventKind} ambient event`,
          detail: "",
          receipt_ref: null,
          major_event: majorEvent({
            lifecycle,
            source: { event_kind: eventKind },
          }),
        }],
      }));
      expect(consumed.state.events[0].major_event, eventKind).toBeTruthy();
    }

    for (const candidate of [
      majorEvent({ severity: 0 }),
      majorEvent({ severity: 6 }),
      majorEvent({ severity: "4" }),
      majorEvent({ category: "war", source: { event_kind: "war_started" } }),
      majorEvent({ source: { event_kind: "CrisisSpawned" } }),
      majorEvent({ lifecycle: "completed" }),
    ]) {
      const consumed = consumeWorldFeed(createInitialWorldFeedState(), feed({
        events: [{
          event_seq: 7,
          kind: "major_world_event",
          summary: "candidate event",
          detail: "",
          receipt_ref: null,
          major_event: candidate,
        }],
      }));
      expect(consumed).toMatchObject({ requiresSnapshotReload: true });
      expect(consumed.state).toMatchObject({
        status: "unavailable",
        unavailableReason: "source_unavailable",
        events: [],
      });
    }
  });

  it("fails closed for missing authority fields, mismatched identity, permission, or freshness", () => {
    const missingFields = [
      "schema_version",
      "identity",
      "category",
      "subtype",
      "severity",
      "lifecycle",
      "source",
      "freshness",
      "visibility",
      "logical_time",
    ];
    for (const field of missingFields) {
      const candidate = majorEvent();
      delete candidate[field];
      const consumed = consumeWorldFeed(createInitialWorldFeedState(), feed({
        events: [{
          event_seq: 7,
          kind: "major_world_event",
          summary: `missing ${field}`,
          detail: "",
          receipt_ref: null,
          major_event: candidate,
        }],
      }));
      expect(consumed.requiresSnapshotReload, field).toBe(true);
      expect(consumed.state, field).toMatchObject({
        status: "unavailable",
        unavailableReason: "source_unavailable",
        events: [],
      });
    }

    for (const candidate of [
      majorEvent({ identity: { event_seq: 8 } }),
      majorEvent({ freshness: "unknown" }),
      majorEvent({ freshness: "stale" }),
      majorEvent({ visibility: "unknown" }),
      majorEvent({ visibility: "denied" }),
    ]) {
      const consumed = consumeWorldFeed(createInitialWorldFeedState(), feed({
        events: [{
          event_seq: 7,
          kind: "major_world_event",
          summary: "invalid authority candidate",
          detail: "",
          receipt_ref: null,
          major_event: candidate,
        }],
      }));
      expect(consumed).toMatchObject({ requiresSnapshotReload: true });
      expect(consumed.state).toMatchObject({
        status: "unavailable",
        unavailableReason: "source_unavailable",
        events: [],
      });
    }
  });

  it("keeps legacy events compatible and allows a world-scoped crisis with no spatial anchor", () => {
    const legacy = consumeWorldFeed(createInitialWorldFeedState(), feed()).state;
    expect(legacy.events[0]).toMatchObject({
      event_seq: 1,
      kind: "agent_spoke",
      summary: "Agent spoke",
      receipt_ref: "receipt-1",
    });
    expect(legacy.events[0].major_event == null).toBe(true);

    const noAnchor = consumeWorldFeed(createInitialWorldFeedState(), feed({
      events: [{
        event_seq: 7,
        kind: "major_world_event",
        summary: "Crisis without a stage anchor",
        detail: "",
        receipt_ref: null,
        major_event: majorEvent({ world_anchor: null }),
      }],
    })).state.events[0].major_event;
    expect(noAnchor).toBeTruthy();
    expect(noAnchor.world_anchor).toBeNull();
  });

  it("keeps replay history readable but does not manufacture a current event, and clears major events on gap/reorg", () => {
    const replay = consumeWorldFeed(createInitialWorldFeedState(), feed({
      status: "replay",
      events: [{
        event_seq: 7,
        kind: "major_world_event",
        summary: "Historical crisis",
        detail: "",
        receipt_ref: null,
        major_event: majorEvent({ freshness: "replay" }),
      }],
    })).state;
    expect(replay.events[0].major_event.freshness).toBe("replay");

    const gap = consumeWorldFeed(replay, feed({
      status: "gap",
      reorg_epoch: 3,
      events: [{
        event_seq: 8,
        kind: "major_world_event",
        summary: "must not survive gap",
        detail: "",
        receipt_ref: null,
        major_event: majorEvent({
          identity: { reorg_epoch: 3, event_seq: 8 },
          freshness: "current",
        }),
      }],
      gap_reason: "reorg_epoch_changed",
      snapshot_reload_required: true,
    })).state;
    expect(gap.events).toEqual([]);

    const afterReorg = consumeWorldFeed(gap, feed({
      reorg_epoch: 3,
      events: [{
        event_seq: 1,
        kind: "major_world_event",
        summary: "new epoch crisis",
        detail: "",
        receipt_ref: null,
        major_event: majorEvent({
          identity: { reorg_epoch: 3, event_seq: 1 },
        }),
      }],
    })).state;
    expect(afterReorg.events).toHaveLength(1);
    expect(afterReorg.events[0].major_event.identity.reorg_epoch).toBe(3);
  });

  it("clears previously visible events when the source loses permission", () => {
    const ready = consumeWorldFeed(createInitialWorldFeedState(), feed({
      events: [{
        event_seq: 7,
        kind: "major_world_event",
        summary: "Restricted crisis",
        detail: "",
        receipt_ref: null,
        major_event: majorEvent(),
      }],
    })).state;
    const denied = consumeWorldFeed(ready, feed({
      status: "unavailable",
      unavailable_reason: "permission_denied",
      events: [],
      snapshot_reload_required: false,
    })).state;
    expect(denied).toMatchObject({
      status: "unavailable",
      unavailableReason: "permission_denied",
      stale: true,
    });
    expect(denied.events).toEqual([]);
  });
});
