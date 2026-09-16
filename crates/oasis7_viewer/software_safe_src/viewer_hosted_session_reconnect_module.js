export function createViewerHostedSessionReconnectModule({
  authHasSigningKeyMaterial,
  legacyViewerAuthBootstrapSource,
  onRefreshFailure,
  refreshHostedPlayerLease,
  registerHostedPlayerSession,
  sendReconnectSync,
  state,
}) {
  function needsHostedKeyRecovery() {
    const auth = state.auth;
    return auth?.available
      && auth.source !== legacyViewerAuthBootstrapSource
      && !!String(auth.releaseToken || "").trim()
      && !authHasSigningKeyMaterial(auth);
  }

  async function syncHostedPlayerSessionOnConnect() {
    if (
      !state.auth.available
      || state.auth.source === legacyViewerAuthBootstrapSource
      || state.auth.syncInFlight
    ) {
      return { ok: false, skipped: true };
    }

    if (needsHostedKeyRecovery()) {
      const payload = await refreshHostedPlayerLease();
      if (!payload?.ok) {
        onRefreshFailure?.();
        return { ok: false, reason: "session_refresh_failed" };
      }
      await registerHostedPlayerSession();
      return { ok: true, mode: "registration" };
    }

    await sendReconnectSync();
    return { ok: true, mode: "reconnect" };
  }

  return {
    needsHostedKeyRecovery,
    syncHostedPlayerSessionOnConnect,
  };
}
