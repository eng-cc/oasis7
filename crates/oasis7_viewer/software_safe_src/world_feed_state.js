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

function normalizeEvent(event) {
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
  return {
    event_seq: typeof event.event_seq === "number" ? event.event_seq : eventSeq,
    kind,
    summary,
    detail,
    receipt_ref: event.receipt_ref == null ? null : event.receipt_ref,
  };
}

function invalidState(previous, reason, error, { requiresSnapshotReload = true } = {}) {
  return {
    ...previous,
    status: "unavailable",
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
  const events = feed.events.map(normalizeEvent);
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
  const requiresSnapshotReload = snapshotReloadRequired || status === "gap" || status === "unavailable";
  const stale = requiresSnapshotReload || status === "gap";
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
        snapshotReloadRequired: snapshotReloadRequired,
        requestInFlight: false,
        lastError: null,
      },
      requiresSnapshotReload,
    };
  }

  const sameScope = state.worldId === worldId && String(state.reorgEpoch) === String(reorgEpoch);
  const existing = sameScope ? state.events : [];
  const seen = new Set(existing.map((event) => worldFeedEventIdentity(worldId, reorgEpoch, event.event_seq)));
  const appended = [];
  let dedupedCount = 0;
  for (const event of events) {
    const identity = worldFeedEventIdentity(worldId, reorgEpoch, event.event_seq);
    if (seen.has(identity)) {
      dedupedCount += 1;
      continue;
    }
    seen.add(identity);
    appended.push(event);
  }
  const combinedEvents = [...existing, ...appended];
  // Store the canonical timeline order for every payload shape. The comparator
  // operates on decimal text so full u64 values never pass through JS Number.
  const storedEvents = combinedEvents.slice().sort(
    (left, right) => compareUnsignedDecimal(left.event_seq, right.event_seq),
  );
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
