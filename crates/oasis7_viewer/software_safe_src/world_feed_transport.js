import { consumeWorldFeed, requestWorldFeedState } from "./world_feed_state.js";

export function createWorldFeedTransport({ getSocket, getState, render, requestSnapshot, sendJson }) {
  function requestWorldFeed({ cursor = getState().worldFeed?.cursor || null, limit = 50 } = {}) {
    const state = getState();
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
    const consumed = consumeWorldFeed(state.worldFeed, feed);
    state.worldFeed = consumed.state;
    if (consumed.requiresSnapshotReload) {
      requestSnapshot();
    }
  }

  function refreshAfterSnapshot() {
    if (getState().worldFeed?.snapshotReloadRequired) {
      requestWorldFeed({ cursor: null });
    }
  }

  function reloadWorldFeedFromAuthoritativeSnapshot() {
    requestSnapshot();
    return true;
  }

  return {
    handleWorldFeed,
    refreshAfterSnapshot,
    reloadWorldFeedFromAuthoritativeSnapshot,
    requestWorldFeed,
  };
}
