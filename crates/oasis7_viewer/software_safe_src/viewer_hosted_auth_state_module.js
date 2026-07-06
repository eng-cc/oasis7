function buildDefaultAuthState(overrides = {}) {
  return {
    available: false,
    hostedAccountId: null,
    playerId: null,
    loginChannel: null,
    maskedLoginHint: null,
    deviceSessionId: null,
    publicKey: null,
    privateKey: null,
    releaseToken: null,
    error: null,
    revokeReason: null,
    revokedBy: null,
    source: "guest_only",
    registrationStatus: "guest",
    sessionEpoch: null,
    issuedAtUnixMs: null,
    recoveryErrorCode: null,
    recoveryErrorMessage: null,
    issueInFlight: false,
    syncInFlight: false,
    runtimeStatus: "guest",
    boundAgentId: null,
    pendingRequestedAgentId: null,
    pendingForceRebind: false,
    rebindNotice: null,
    ...overrides,
  };
}

export function createViewerHostedAuthStateModule({
  hostedPlayerSessionStoragePrefix,
  initialWsUrl,
  viewerAuthBootstrapObject,
  viewerAuthPrivateKey,
  viewerAuthPublicKey,
  viewerPlayerIdKey,
  windowRef,
}) {
  function resolveAuthBootstrap() {
    const raw = windowRef[viewerAuthBootstrapObject];
    if (!raw || typeof raw !== "object") {
      return buildDefaultAuthState({
        error: "viewer auth bootstrap is unavailable",
      });
    }
    const playerId = String(raw[viewerPlayerIdKey] || "").trim();
    const publicKey = String(raw[viewerAuthPublicKey] || "")
      .trim()
      .toLowerCase();
    const privateKey = String(raw[viewerAuthPrivateKey] || "")
      .trim()
      .toLowerCase();
    if (!playerId || !publicKey || !privateKey) {
      return buildDefaultAuthState({
        playerId: playerId || null,
        publicKey: publicKey || null,
        privateKey: privateKey || null,
        error: "viewer auth bootstrap is incomplete",
      });
    }
    return buildDefaultAuthState({
      available: true,
      playerId,
      publicKey,
      privateKey,
      source: "legacy_viewer_auth_bootstrap",
      registrationStatus: "registered",
      sessionEpoch: 1,
      runtimeStatus: "legacy_preview",
      error: null,
    });
  }

  function hostedPlayerSessionStorageKey() {
    return `${hostedPlayerSessionStoragePrefix}:${initialWsUrl()}`;
  }

  function persistHostedPlayerSession(auth) {
    if (!auth?.available || !auth?.playerId || auth.source === "legacy_viewer_auth_bootstrap") {
      return;
    }
    try {
      windowRef.localStorage?.setItem(
        hostedPlayerSessionStorageKey(),
        JSON.stringify({
          hostedAccountId: auth.hostedAccountId || null,
          playerId: auth.playerId,
          loginChannel: auth.loginChannel || null,
          maskedLoginHint: auth.maskedLoginHint || null,
          deviceSessionId: auth.deviceSessionId || auth.releaseToken || null,
          releaseToken: auth.releaseToken || null,
          issuedAtUnixMs: auth.issuedAtUnixMs ?? null,
          sessionEpoch: auth.sessionEpoch ?? null,
        }),
      );
    } catch (_) {
    }
  }

  function clearHostedPlayerSession() {
    try {
      windowRef.localStorage?.removeItem(hostedPlayerSessionStorageKey());
    } catch (_) {
    }
  }

  function resolveStoredHostedPlayerSession() {
    try {
      const raw = windowRef.localStorage?.getItem(hostedPlayerSessionStorageKey());
      if (!raw) {
        return null;
      }
      const parsed = JSON.parse(raw);
      const hostedAccountId = String(parsed?.hostedAccountId || parsed?.hosted_account_id || "").trim();
      const playerId = String(parsed?.playerId || parsed?.player_id || "").trim();
      const loginChannel = String(parsed?.loginChannel || parsed?.login_channel || "").trim();
      const maskedLoginHint = String(parsed?.maskedLoginHint || parsed?.masked_login_hint || "").trim();
      const releaseToken = String(parsed?.releaseToken || parsed?.release_token || "").trim();
      const deviceSessionId = String(
        parsed?.deviceSessionId || parsed?.device_session_id || parsed?.releaseToken || parsed?.release_token || "",
      ).trim();
      const issuedAtUnixMs = parsed?.issuedAtUnixMs ?? parsed?.issued_at_unix_ms ?? null;
      const sessionEpoch = parsed?.sessionEpoch ?? parsed?.session_epoch ?? null;
      const normalizedIssuedAtUnixMs = normalizeOptionalFiniteNumber(issuedAtUnixMs);
      const normalizedSessionEpoch = normalizeOptionalFiniteNumber(sessionEpoch);
      if (!playerId || !releaseToken) {
        clearHostedPlayerSession();
        return null;
      }
      windowRef.localStorage?.setItem(
        hostedPlayerSessionStorageKey(),
        JSON.stringify({
          hostedAccountId: hostedAccountId || null,
          playerId,
          loginChannel: loginChannel || null,
          maskedLoginHint: maskedLoginHint || null,
          deviceSessionId: deviceSessionId || releaseToken,
          releaseToken,
          issuedAtUnixMs: normalizedIssuedAtUnixMs,
          sessionEpoch: normalizedSessionEpoch,
        }),
      );
      return buildDefaultAuthState({
        available: true,
        hostedAccountId: hostedAccountId || null,
        playerId,
        loginChannel: loginChannel || null,
        maskedLoginHint: maskedLoginHint || null,
        deviceSessionId: deviceSessionId || releaseToken,
        releaseToken,
        source: "hosted_browser_storage",
        registrationStatus: "issued",
        sessionEpoch: normalizedSessionEpoch,
        issuedAtUnixMs: normalizedIssuedAtUnixMs,
        runtimeStatus: "issued",
        error: null,
      });
    } catch (_) {
      clearHostedPlayerSession();
      return null;
    }
  }

  function normalizeOptionalFiniteNumber(value) {
    if (value === null || value === undefined || value === "") {
      return null;
    }
    const number = Number(value);
    return Number.isFinite(number) ? number : null;
  }

  function resolveViewerAuthState() {
    const bootstrap = resolveAuthBootstrap();
    if (bootstrap.available) {
      return bootstrap;
    }
    return resolveStoredHostedPlayerSession() || bootstrap;
  }

  function authHasSigningKeyMaterial(auth) {
    return !!String(auth?.publicKey || "").trim() && !!String(auth?.privateKey || "").trim();
  }

  return {
    authHasSigningKeyMaterial,
    clearHostedPlayerSession,
    persistHostedPlayerSession,
    resolveAuthBootstrap,
    resolveViewerAuthState,
  };
}
