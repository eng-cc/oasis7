export function createViewerHostedTestLoginModule({
  clone,
  fetchImpl,
  generateEphemeralEd25519Keypair,
  getSearchParams,
  isHostedPublicJoinDeploymentMode,
  persistHostedPlayerSession,
  render,
  resetHostedLoginChallenge,
  route,
  state,
}) {
  function isOptedIn() {
    const value = String(getSearchParams().get("hosted_test_login") || "").trim().toLowerCase();
    return value === "1" || value === "true" || value === "yes" || value === "on";
  }

  async function start() {
    if (
      !isOptedIn()
      || !isHostedPublicJoinDeploymentMode(state.hostedAccess?.deployment_mode)
      || state.auth.available
    ) {
      return { ok: false, reason: "hosted test login is unavailable on this lane" };
    }
    state.hostedLogin.channel = "test";
    state.hostedLogin.startInFlight = true;
    state.hostedLogin.error = null;
    state.auth.issueInFlight = true;
    state.auth.error = null;
    render();
    try {
      const keypair = await generateEphemeralEd25519Keypair();
      const response = await fetchImpl(route, {
        method: "POST",
        cache: "no-store",
        headers: {
          Accept: "application/json",
          "Content-Type": "application/json",
        },
        body: JSON.stringify({ public_key: keypair.publicKey }),
      });
      const payload = await response.json();
      const grant = payload?.grant;
      if (
        !response.ok
        || !payload?.ok
        || !grant?.player_id
        || !grant?.device_session_id
        || !grant?.release_token
        || !grant?.registration_grant
      ) {
        throw new Error(payload?.error || payload?.error_code || `hosted test login failed with HTTP ${response.status}`);
      }
      state.hostedAdmission = payload?.admission ? clone(payload.admission) : state.hostedAdmission;
      state.auth = {
        available: true,
        hostedAccountId: null,
        playerId: String(grant.player_id).trim(),
        loginChannel: "test",
        maskedLoginHint: "local test login",
        deviceSessionId: String(grant.device_session_id).trim(),
        publicKey: keypair.publicKey,
        privateKey: keypair.privateKey,
        releaseToken: String(grant.release_token).trim(),
        registrationGrant: String(grant.registration_grant).trim(),
        error: null,
        revokeReason: null,
        revokedBy: null,
        source: "hosted_test_login",
        registrationStatus: "issued",
        sessionEpoch: null,
        bindingEpoch: null,
        authorityEpoch: null,
        issuedAtUnixMs: grant.issued_at_unix_ms == null ? Date.now() : Number(grant.issued_at_unix_ms),
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
      persistHostedPlayerSession(state.auth);
      resetHostedLoginChallenge();
      state.hostedLogin.channel = "test";
      render();
      return state.auth;
    } catch (error) {
      state.auth.issueInFlight = false;
      state.hostedLogin.startInFlight = false;
      state.hostedLogin.error = String(error);
      state.auth.error = String(error);
      render();
      return { ok: false, reason: state.hostedLogin.error };
    }
  }

  return { isOptedIn, start };
}
