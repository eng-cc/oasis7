export function createViewerBrowserPersistenceModule({
  chatHistoryLimit,
  chatHistoryStoragePrefix,
  clone,
  initialWsUrl,
  localTestPlayerIdPrefix,
  localTestPlayerSessionStoragePrefix,
  state,
  windowRef,
}) {
  function storageSafe() {
    try {
      return windowRef?.localStorage || null;
    } catch (_) {
      return null;
    }
  }

  function chatHistoryStorageKey() {
    const worldId = state.worldId || state.snapshot?.world_id || state.snapshot?.worldId || null;
    if (!worldId) {
      return null;
    }
    const wsUrl = state.wsUrl || initialWsUrl();
    return `${chatHistoryStoragePrefix}:${encodeURIComponent(String(worldId))}:${encodeURIComponent(String(wsUrl || "viewer"))}`;
  }

  function localTestPlayerSessionStorageKey() {
    const wsUrl = state.wsUrl || initialWsUrl();
    return `${localTestPlayerSessionStoragePrefix}:${encodeURIComponent(String(wsUrl || "viewer"))}`;
  }

  function persistLocalTestPlayerSession(auth) {
    if (!auth?.available || auth.source !== "local_test_api_ephemeral" || !auth.playerId) {
      return;
    }
    const storage = storageSafe();
    if (!storage) {
      return;
    }
    try {
      storage.setItem(
        localTestPlayerSessionStorageKey(),
        JSON.stringify({
          playerId: auth.playerId,
          deviceSessionId: auth.deviceSessionId || auth.playerId,
          publicKey: auth.publicKey || null,
          privateKey: auth.privateKey || null,
          issuedAtUnixMs: auth.issuedAtUnixMs || Date.now(),
        }),
      );
    } catch (_) {
    }
  }

  function resolveStoredLocalTestPlayerSession() {
    const storage = storageSafe();
    if (!storage) {
      return null;
    }
    try {
      const raw = storage.getItem(localTestPlayerSessionStorageKey());
      if (!raw) {
        return null;
      }
      const parsed = JSON.parse(raw);
      const playerId = String(parsed?.playerId || "").trim();
      const publicKey = String(parsed?.publicKey || "").trim().toLowerCase();
      const privateKey = String(parsed?.privateKey || "").trim().toLowerCase();
      if (!playerId.startsWith(localTestPlayerIdPrefix) || !publicKey || !privateKey) {
        storage.removeItem(localTestPlayerSessionStorageKey());
        return null;
      }
      return {
        available: true,
        hostedAccountId: null,
        playerId,
        loginChannel: null,
        maskedLoginHint: null,
        deviceSessionId: String(parsed?.deviceSessionId || parsed?.device_session_id || playerId).trim() || playerId,
        publicKey,
        privateKey,
        releaseToken: null,
        error: null,
        revokeReason: null,
        revokedBy: null,
        source: "local_test_api_ephemeral",
        registrationStatus: "issued",
        sessionEpoch: null,
        issuedAtUnixMs: parsed?.issuedAtUnixMs == null ? Date.now() : Number(parsed.issuedAtUnixMs),
        recoveryErrorCode: null,
        recoveryErrorMessage: null,
        issueInFlight: false,
        syncInFlight: false,
        runtimeStatus: "issued",
        boundAgentId: null,
        pendingRequestedAgentId: null,
        pendingForceRebind: false,
        rebindNotice: null,
      };
    } catch (_) {
      try {
        storage.removeItem(localTestPlayerSessionStorageKey());
      } catch (_) {
      }
      return null;
    }
  }

  function normalizeChatHistoryEntry(entry) {
    if (!entry || typeof entry !== "object") {
      return null;
    }
    const message = String(entry.message || "").trim();
    if (!message) {
      return null;
    }
    return {
      id: entry.id || `${entry.source || "chat"}-${Date.now()}-${Math.random().toString(16).slice(2)}`,
      source: entry.source || "event",
      agentId: entry.agentId || null,
      locationId: entry.locationId || null,
      message,
      tick: Number(entry.tick || 0),
      speaker: entry.speaker || null,
      playerId: entry.playerId || null,
      targetAgentId: entry.targetAgentId || null,
      intentSeq: entry.intentSeq || null,
      code: entry.code || null,
      response: entry.response ? clone(entry.response) : null,
    };
  }

  function setChatHistory(entries) {
    const seen = new Set();
    const next = [];
    for (const raw of entries || []) {
      const entry = normalizeChatHistoryEntry(raw);
      if (!entry || seen.has(entry.id)) {
        continue;
      }
      seen.add(entry.id);
      next.push(entry);
      if (next.length >= chatHistoryLimit) {
        break;
      }
    }
    state.chatHistory = next;
  }

  function persistChatHistory() {
    const storage = storageSafe();
    const key = chatHistoryStorageKey();
    if (!storage || !key) {
      return;
    }
    try {
      storage.setItem(key, JSON.stringify(state.chatHistory.slice(0, chatHistoryLimit)));
    } catch (_) {
    }
  }

  function hydrateChatHistoryFromStorage() {
    const storage = storageSafe();
    const key = chatHistoryStorageKey();
    if (!storage || !key) {
      return;
    }
    try {
      const raw = storage.getItem(key);
      if (!raw) {
        return;
      }
      const stored = JSON.parse(raw);
      if (!Array.isArray(stored)) {
        return;
      }
      setChatHistory([...(state.chatHistory || []), ...stored]);
    } catch (_) {
    }
  }

  return {
    chatHistoryStorageKey,
    hydrateChatHistoryFromStorage,
    normalizeChatHistoryEntry,
    persistChatHistory,
    persistLocalTestPlayerSession,
    resolveStoredLocalTestPlayerSession,
    setChatHistory,
  };
}
