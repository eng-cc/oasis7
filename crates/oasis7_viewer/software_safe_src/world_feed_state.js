export const WORLD_FEED_SCHEMA_VERSION = "world_feed/v1";
const MAX_U64_DECIMAL = "18446744073709551615";

const FEED_STATUSES = new Set([
  "loading",
  "ready",
  "empty",
  "replay",
  "gap",
  "unavailable",
]);

const GAP_REASONS = new Set(["cursor_gap", "reorg_epoch_changed", "cursor_invalid"]);
const UNAVAILABLE_REASONS = new Set(["source_unavailable", "schema_unsupported", "permission_denied"]);
const EVENT_IDENTITY_CONFLICT_GAP_REASON = "event_identity_conflict";

export function createInitialWorldFeedState() {
  return {
    status: "loading",
    schemaVersion: WORLD_FEED_SCHEMA_VERSION,
    worldId: null,
    reorgEpoch: null,
    cursor: null,
    events: [],
    stale: false,
    gapReason: null,
    unavailableReason: null,
    snapshotReloadRequired: false,
    requestInFlight: false,
    requestCursor: null,
    requestLimit: 50,
    dedupedCount: 0,
    lastError: null,
  };
}

export function requestWorldFeedState(previous, { cursor = null, limit = 50 } = {}) {
  const state = previous || createInitialWorldFeedState();
  return {
    ...state,
    status: "loading",
    requestInFlight: true,
    requestCursor: cursor == null ? null : String(cursor),
    requestLimit: normalizeLimit(limit),
    lastError: null,
  };
}

export function worldFeedEventIdentity(worldId, reorgEpoch, eventSeq) {
  return `${String(worldId)}|${String(reorgEpoch)}|${String(eventSeq)}`;
}

function normalizeLimit(limit) {
  const value = Number(limit);
  if (!Number.isInteger(value) || value <= 0) {
    return 50;
  }
  return Math.min(value, 200);
}

function normalizeUnsignedInteger(value) {
  let text = null;
  if (typeof value === "number") {
    text = Number.isSafeInteger(value) && value >= 0 ? String(value) : null;
  } else if (typeof value === "bigint") {
    text = value >= 0n ? value.toString() : null;
  } else {
    text = String(value ?? "").trim();
  }
  if (!text || !/^\d+$/.test(text)) {
    return null;
  }
  const canonical = text.replace(/^0+(?=\d)/, "");
  if (canonical.length > MAX_U64_DECIMAL.length
    || (canonical.length === MAX_U64_DECIMAL.length && canonical > MAX_U64_DECIMAL)) {
    return null;
  }
  return canonical;
}

export function compareUnsignedDecimal(left, right) {
  const leftText = normalizeUnsignedInteger(left);
  const rightText = normalizeUnsignedInteger(right);
  if (leftText == null || rightText == null || leftText === rightText) {
    return 0;
  }
  if (leftText.length !== rightText.length) {
    return leftText.length - rightText.length;
  }
  return leftText < rightText ? -1 : 1;
}

function normalizeCausalReference(value) {
  if (value == null) return null;
  if (!value || typeof value !== "object") return undefined;
  if (value.type === "action" && normalizeUnsignedInteger(value.data) != null) {
    return { type: "action", data: value.data };
  }
  if (value.type === "effect"
    && value.data && typeof value.data === "object"
    && String(value.data.intent_id ?? "").trim()) {
    return { type: "effect", data: { intent_id: String(value.data.intent_id).trim() } };
  }
  return undefined;
}

function normalizeMajorEvent(value, { worldId, reorgEpoch, eventSeq }) {
  if (!value || typeof value !== "object"
    || value.schema_version !== "major_world_event/v1"
    || value.category !== "crisis"
    || !String(value.subtype ?? "").trim()
    || !Number.isInteger(value.severity)
    || value.severity < 1
    || value.severity > 5
    || !value.identity
    || !value.source) {
    return null;
  }
  const identityWorldId = String(value.identity.world_id ?? "").trim();
  const identityEpoch = normalizeUnsignedInteger(value.identity.reorg_epoch);
  const identitySeq = normalizeUnsignedInteger(value.identity.event_seq);
  const logicalTime = normalizeUnsignedInteger(value.logical_time);
  if (identityWorldId !== worldId
    || identityEpoch !== reorgEpoch
    || identitySeq !== eventSeq
    || logicalTime == null
    || value.source.authority !== "runtime_journal") {
    return null;
  }
  const lifecycleByKind = {
    crisis_spawned: "active",
    crisis_resolved: "resolved",
    crisis_timed_out: "timed_out",
  };
  if (lifecycleByKind[value.source.event_kind] !== value.lifecycle
    || !new Set(["current", "replay"]).has(value.freshness)
    || !new Set(["public", "restricted"]).has(value.visibility)) {
    return null;
  }
  const causalReference = normalizeCausalReference(value.causal_reference);
  if (causalReference === undefined) return null;
  let worldAnchor = null;
  if (value.world_anchor != null) {
    if (!value.world_anchor || typeof value.world_anchor !== "object"
      || value.world_anchor.scope !== "world"
      || value.world_anchor.position != null) {
      return null;
    }
    const entityId = value.world_anchor.entity_id == null
      ? null
      : String(value.world_anchor.entity_id).trim();
    if (value.world_anchor.entity_id != null && !entityId) return null;
    worldAnchor = { scope: "world", ...(entityId ? { entity_id: entityId } : {}) };
  }
  return {
    schema_version: "major_world_event/v1",
    identity: {
      world_id: identityWorldId,
      reorg_epoch: value.identity.reorg_epoch,
      event_seq: value.identity.event_seq,
    },
    category: "crisis",
    subtype: String(value.subtype).trim(),
    severity: value.severity,
    lifecycle: value.lifecycle,
    source: { authority: "runtime_journal", event_kind: value.source.event_kind },
    freshness: value.freshness,
    visibility: value.visibility,
    logical_time: value.logical_time,
    causal_reference: causalReference,
    world_anchor: worldAnchor,
  };
}

function normalizeEvent(event, context) {
  if (!event || typeof event !== "object") {
    return null;
  }
  const eventSeq = normalizeUnsignedInteger(event.event_seq);
  const kind = String(event.kind ?? "").trim();
  const summary = String(event.summary ?? "").trim();
  const detail = String(event.detail ?? "");
  if (!eventSeq || !kind || !summary) {
    return null;
  }
  if (event.receipt_ref != null && typeof event.receipt_ref !== "string") {
    return null;
  }
  const normalizedSeq = normalizeUnsignedInteger(event.event_seq);
  const majorEvent = normalizeMajorEvent(event.major_event, {
    ...context,
    eventSeq: normalizedSeq,
  });
  return {
    event_seq: typeof event.event_seq === "number" ? event.event_seq : eventSeq,
    kind,
    summary,
    detail,
    receipt_ref: event.receipt_ref == null ? null : event.receipt_ref,
    ...(majorEvent ? { major_event: majorEvent } : {}),
  };
}

function invalidState(previous, reason, error, { requiresSnapshotReload = true } = {}) {
  return {
    ...previous,
    status: "unavailable",
    events: [],
    stale: true,
    unavailableReason: reason,
    gapReason: null,
    snapshotReloadRequired: requiresSnapshotReload,
    requestInFlight: false,
    lastError: error || null,
  };
}

function normalizeEnvelope(feed) {
  if (!feed || typeof feed !== "object") {
    return { error: "world feed response is missing" };
  }
  if (feed.schema_version !== WORLD_FEED_SCHEMA_VERSION) {
    return { unsupportedSchema: true };
  }
  const worldId = String(feed.world_id ?? "").trim();
  const reorgEpoch = normalizeUnsignedInteger(feed.reorg_epoch);
  const cursor = feed.cursor == null ? "" : String(feed.cursor);
  const status = String(feed.status ?? "").trim().toLowerCase();
  if (!worldId || !reorgEpoch || !FEED_STATUSES.has(status) || !Array.isArray(feed.events)) {
    return { error: "world feed response is invalid" };
  }
  const gapReason = feed.gap_reason == null ? null : String(feed.gap_reason).trim().toLowerCase();
  if (gapReason != null && !GAP_REASONS.has(gapReason)) {
    return { error: "world feed gap reason is invalid" };
  }
  const unavailableReason = feed.unavailable_reason == null
    ? null
    : String(feed.unavailable_reason).trim().toLowerCase();
  if (unavailableReason != null && !UNAVAILABLE_REASONS.has(unavailableReason)) {
    return { error: "world feed unavailable reason is invalid" };
  }
  const events = feed.events.map((event) => normalizeEvent(event, { worldId, reorgEpoch }));
  if (events.some((event) => event == null)) {
    return { error: "world feed contains an invalid event" };
  }
  return {
    worldId,
    reorgEpoch,
    cursor,
    status,
    events,
    gapReason,
    unavailableReason,
    snapshotReloadRequired: feed.snapshot_reload_required === true,
  };
}

export function consumeWorldFeed(previous, feed) {
  const state = previous || createInitialWorldFeedState();
  const envelope = normalizeEnvelope(feed);
  if (envelope.unsupportedSchema) {
    return {
      state: invalidState(state, "schema_unsupported", "unsupported world feed schema", {
        requiresSnapshotReload: false,
      }),
      requiresSnapshotReload: false,
    };
  }
  if (envelope.error) {
    return {
      state: invalidState(state, "source_unavailable", envelope.error),
      requiresSnapshotReload: true,
    };
  }

  const {
    worldId,
    reorgEpoch,
    cursor,
    status,
    events,
    gapReason,
    unavailableReason,
    snapshotReloadRequired,
  } = envelope;
  // The runtime contract reserves snapshot reload for continuity gaps. A
  // source/schema/permission outage is unavailable, but reloading a snapshot
  // cannot repair it and must not start a recovery loop.
  const requiresSnapshotReload = snapshotReloadRequired || status === "gap";
  const stale = requiresSnapshotReload || status === "gap" || status === "unavailable";
  if (status === "gap") {
    return {
      state: {
        ...state,
        status,
        schemaVersion: WORLD_FEED_SCHEMA_VERSION,
        worldId,
        reorgEpoch,
        cursor,
        stale: true,
        gapReason: gapReason || "cursor_invalid",
        unavailableReason: null,
        // A gap invalidates the prior timeline. Keep the new epoch/status
        // visible, but never carry events from the branch that must be
        // replaced by the authoritative snapshot.
        events: [],
        snapshotReloadRequired: true,
        requestInFlight: false,
        lastError: null,
      },
      requiresSnapshotReload: true,
    };
  }
  if (status === "unavailable") {
    return {
      state: {
        ...state,
        status,
        schemaVersion: WORLD_FEED_SCHEMA_VERSION,
        worldId,
        reorgEpoch,
        cursor,
        stale,
        gapReason: null,
        unavailableReason: unavailableReason || "source_unavailable",
        events: [],
        snapshotReloadRequired: snapshotReloadRequired,
        requestInFlight: false,
        lastError: null,
      },
      requiresSnapshotReload,
    };
  }

  const sameScope = state.worldId === worldId && String(state.reorgEpoch) === String(reorgEpoch);
  if (state.status !== "gap" && state.worldId != null && !sameScope) {
    return {
      state: {
        ...state,
        status: "gap",
        schemaVersion: WORLD_FEED_SCHEMA_VERSION,
        worldId,
        reorgEpoch,
        cursor,
        events: [],
        stale: true,
        gapReason: state.worldId === worldId ? "reorg_epoch_changed" : "cursor_invalid",
        unavailableReason: null,
        snapshotReloadRequired: true,
        requestInFlight: false,
        lastError: null,
      },
      requiresSnapshotReload: true,
    };
  }
  const existing = sameScope ? state.events : [];
  const byIdentity = new Map(existing.map((event) => [
    worldFeedEventIdentity(worldId, reorgEpoch, event.event_seq),
    event,
  ]));
  const conflicted = new Set();
  let dedupedCount = 0;
  for (const event of events) {
    const identity = worldFeedEventIdentity(worldId, reorgEpoch, event.event_seq);
    if (conflicted.has(identity)) continue;
    const previous = byIdentity.get(identity);
    if (previous) {
      if (JSON.stringify(previous) !== JSON.stringify(event)) {
        byIdentity.delete(identity);
        conflicted.add(identity);
        continue;
      }
      dedupedCount += 1;
      continue;
    }
    byIdentity.set(identity, event);
  }
  // Store the canonical timeline order for every payload shape. The comparator
  // operates on decimal text so full u64 values never pass through JS Number.
  const storedEvents = [...byIdentity.values()].sort(
    (left, right) => compareUnsignedDecimal(left.event_seq, right.event_seq),
  );
  if (conflicted.size > 0) {
    const [identity] = conflicted;
    return {
      state: {
        ...state,
        status: "gap",
        schemaVersion: WORLD_FEED_SCHEMA_VERSION,
        worldId,
        reorgEpoch,
        cursor,
        events: [],
        stale: true,
        gapReason: EVENT_IDENTITY_CONFLICT_GAP_REASON,
        unavailableReason: null,
        snapshotReloadRequired: true,
        requestInFlight: false,
        dedupedCount: 0,
        lastError: `conflicting payloads for world feed identity ${identity}`,
      },
      requiresSnapshotReload: true,
    };
  }
  return {
    state: {
      ...state,
      status,
      schemaVersion: WORLD_FEED_SCHEMA_VERSION,
      worldId,
      reorgEpoch,
      cursor,
      events: storedEvents,
      stale,
      gapReason: null,
      unavailableReason: null,
      snapshotReloadRequired: snapshotReloadRequired,
      requestInFlight: false,
      dedupedCount,
      lastError: null,
    },
    requiresSnapshotReload,
  };
}
