import { consumeWorldFeed, requestWorldFeedState } from "./world_feed_state.js";

export function createWorldFeedTransport({ getSocket, getState, render, requestSnapshot, sendJson }) {
  let generation = 0;
  let continuationTimer = null;

  function clearContinuation() {
    if (continuationTimer == null) {
      return;
    }
    window.clearTimeout(continuationTimer);
    continuationTimer = null;
  }

  function resetGeneration(targetSocket = null) {
    if (targetSocket && getSocket() !== targetSocket) {
      return;
    }
    generation += 1;
    clearContinuation();
    const state = getState();
    if (state.worldFeed?.requestInFlight) {
      state.worldFeed = {
        ...state.worldFeed,
        requestInFlight: false,
      };
    }
  }

  function requestWorldFeed({ cursor = getState().worldFeed?.cursor || null, limit = 50 } = {}) {
    const state = getState();
    if (state.worldFeed?.requestInFlight) {
      render();
      return false;
    }
    clearContinuation();
    state.worldFeed = requestWorldFeedState(state.worldFeed, { cursor, limit });
    const socket = getSocket();
    if (!socket || socket.readyState !== WebSocket.OPEN) {
      render();
      return false;
    }
    sendJson({
      type: "request_world_feed",
      cursor: cursor == null ? null : String(cursor),
      limit,
    });
    render();
    return true;
  }

  function handleWorldFeed(feed) {
    const state = getState();
    const previous = state.worldFeed;
    const responseGeneration = generation;
    const consumed = consumeWorldFeed(state.worldFeed, feed);
    state.worldFeed = consumed.state;
    const status = consumed.state.status;
    if (consumed.requiresSnapshotReload || !["ready", "replay", "empty"].includes(status)) {
      clearContinuation();
    }
    if (consumed.requiresSnapshotReload) {
      requestSnapshot();
    }
    const returnedCursor = String(consumed.state.cursor || "");
    const previousCursor = String(previous?.cursor || "");
    if (
      responseGeneration === generation
      && !consumed.requiresSnapshotReload
      && ["ready", "replay", "empty"].includes(status)
      && returnedCursor
      && returnedCursor !== previousCursor
      && continuationTimer == null
    ) {
      continuationTimer = window.setTimeout(() => {
        continuationTimer = null;
        if (responseGeneration !== generation) {
          return;
        }
        const latest = getState().worldFeed;
        if (
          latest?.requestInFlight
          || latest?.snapshotReloadRequired
          || !["ready", "replay", "empty"].includes(latest?.status)
          || !latest?.cursor
        ) {
          return;
        }
        requestWorldFeed({ cursor: latest.cursor, limit: latest.requestLimit || 50 });
      }, 0);
    }
  }

  function refreshAfterSnapshot() {
    if (getState().worldFeed?.snapshotReloadRequired) {
      requestWorldFeed({ cursor: null });
    }
  }

  function refreshAfterWorldActivity() {
    const state = getState();
    const cursor = state.worldFeed?.cursor;
    const socket = getSocket();
    if (
      !cursor
      || !socket
      || socket.readyState !== WebSocket.OPEN
      || state.worldFeed?.requestInFlight
      || state.worldFeed?.snapshotReloadRequired
      || !["ready", "replay", "empty"].includes(state.worldFeed?.status)
    ) {
      return false;
    }
    return requestWorldFeed({ cursor });
  }

  function reloadWorldFeedFromAuthoritativeSnapshot() {
    requestSnapshot();
    return true;
  }

  return {
    handleWorldFeed,
    refreshAfterSnapshot,
    refreshAfterWorldActivity,
    reloadWorldFeedFromAuthoritativeSnapshot,
    requestWorldFeed,
    resetGeneration,
  };
}
