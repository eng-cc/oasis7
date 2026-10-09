import { authConnectionGeneration, authCredentials, installSession, clearSession, hasSigningIdentity, updateRegistrationGrant, captureSessionContext, isSessionContextCurrent } from "./viewer_auth_session_module.js";
export function createViewerHostedSessionRefreshModule({
  clone,
  ensureHostedAuthSigningKey,
  fetchImpl,
  legacyViewerAuthBootstrapSource,
  persistHostedPlayerSession,
  refreshRoute,
  state,
  captureConnection = () => state.wsUrl,
}) {
  let active = null;

  function refreshHostedPlayerLease() {
    const auth = state.auth;
    const connection = captureConnection();
    if (active?.auth === auth && active.connection === connection) return active.promise;
    active?.controller.abort();
    const controller = new AbortController();
    const current = { auth, connection, endpoint: state.wsUrl, controller, promise: null, generation: authConnectionGeneration() };
    active = current;
    current.promise = performRefresh(current).finally(() => { if (active === current) active = null; });
    return current.promise;
  }

  async function performRefresh(current) {
    const isCurrent = () => active === current && state.auth === current.auth && captureConnection() === current.connection && state.wsUrl === current.endpoint && authConnectionGeneration() === current.generation
      && state.auth.playerId === playerId && state.auth.publicKey === publicKey && authCredentials(state.auth).releaseToken === releaseToken;
    const auth = await ensureHostedAuthSigningKey(current.auth);
    const installedNewIdentity = auth !== current.auth && hasSigningIdentity(auth);
    const expectedGeneration = current.generation + (installedNewIdentity ? 1 : 0);
    if (active !== current || state.auth !== auth || captureConnection() !== current.connection
      || state.wsUrl !== current.endpoint || authConnectionGeneration() !== expectedGeneration) return null;
    current.auth = auth;
    current.generation = authConnectionGeneration();
    const playerId = String(auth.playerId || "").trim();
    const releaseToken = String(authCredentials(auth).releaseToken || "").trim();
    const publicKey = String(auth.publicKey || "").trim();
    if (!playerId || !releaseToken || !publicKey || legacyViewerAuthBootstrapSource != null && auth.source === legacyViewerAuthBootstrapSource) {
      return null;
    }
    try {
      const response = await fetchImpl(refreshRoute, {
        signal: current.controller.signal,
        method: "POST",
        cache: "no-store",
        headers: { Accept: "application/json", "Content-Type": "application/json" },
        body: JSON.stringify({ player_id: playerId, release_token: releaseToken, public_key: publicKey }),
      });
      const payload = await response.json();
      if (!isCurrent()) return null;
      if (payload?.admission) {
        state.hostedAdmission = clone(payload.admission);
      }
      if (!response.ok || !payload?.ok) {
        throw new Error(payload?.error || payload?.error_code || `hosted player-session refresh failed with HTTP ${response.status}`);
      }
      if (payload.registration_grant) {
        updateRegistrationGrant(auth, String(payload.registration_grant).trim() || null);
        auth.deviceSessionId = String(payload.device_session_id || auth.deviceSessionId || "").trim() || null;
        persistHostedPlayerSession(auth);
      }
      return payload;
    } catch (error) {
      if (isCurrent() && error?.name !== "AbortError") state.auth.error = String(error);
      return null;
    }
  }

  return { refreshHostedPlayerLease, cancelRefresh: () => { active?.controller.abort(); active = null; } };
}
