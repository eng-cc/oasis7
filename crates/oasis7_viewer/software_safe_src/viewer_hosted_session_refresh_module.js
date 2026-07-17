export function createViewerHostedSessionRefreshModule({
  clone,
  ensureHostedAuthSigningKey,
  fetchImpl,
  legacyViewerAuthBootstrapSource,
  persistHostedPlayerSession,
  refreshRoute,
  state,
}) {
  async function refreshHostedPlayerLease() {
    const auth = await ensureHostedAuthSigningKey(state.auth);
    const playerId = String(auth.playerId || "").trim();
    const releaseToken = String(auth.releaseToken || "").trim();
    const publicKey = String(auth.publicKey || "").trim();
    if (!playerId || !releaseToken || !publicKey || auth.source === legacyViewerAuthBootstrapSource) {
      return null;
    }
    try {
      const response = await fetchImpl(refreshRoute, {
        method: "POST",
        cache: "no-store",
        headers: { Accept: "application/json", "Content-Type": "application/json" },
        body: JSON.stringify({ player_id: playerId, release_token: releaseToken, public_key: publicKey }),
      });
      const payload = await response.json();
      if (payload?.admission) {
        state.hostedAdmission = clone(payload.admission);
      }
      if (!response.ok || !payload?.ok) {
        throw new Error(payload?.error || payload?.error_code || `hosted player-session refresh failed with HTTP ${response.status}`);
      }
      if (payload.registration_grant) {
        auth.registrationGrant = String(payload.registration_grant).trim() || null;
        auth.deviceSessionId = String(payload.device_session_id || auth.deviceSessionId || "").trim() || null;
        persistHostedPlayerSession(auth);
      }
      return payload;
    } catch (error) {
      state.auth.error = String(error);
      return null;
    }
  }

  return { refreshHostedPlayerLease };
}
