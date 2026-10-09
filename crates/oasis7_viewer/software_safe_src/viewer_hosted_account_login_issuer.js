export function createViewerHostedAccountLoginIssuer({ state, canAutoIssueHostedPlayerSession, generateEphemeralEd25519Keypair, installSession, persistHostedPlayerSession, resetHostedLoginChallenge, render, clone, fetch, completeRoute }) {
return async function issueHostedAccountLogin() {
  if (!canAutoIssueHostedPlayerSession()) {
    return state.auth;
  }
  if (state.auth.available) {
    return state.auth;
  }
  const challengeId = String(state.hostedLogin.challengeId || "").trim();
  const otpCode = String(state.hostedLogin.code || "").trim();
  if (!challengeId || !otpCode) {
    state.hostedLogin.error = "verification code is required before hosted login can complete";
    render();
    return state.auth;
  }
  state.auth.issueInFlight = true;
  state.hostedLogin.completeInFlight = true;
  state.hostedLogin.error = null;
  state.auth.error = null;
  render();
  try {
    const keypair = await generateEphemeralEd25519Keypair();
    const response = await fetch(completeRoute, {
      method: "POST",
      cache: "no-store",
      headers: {
        Accept: "application/json",
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        challenge_id: challengeId,
        otp_code: otpCode,
        public_key: keypair.publicKey,
      }),
    });
    const payload = await response.json();
    if (!response.ok || !payload?.ok || !payload?.grant?.player_id || !payload?.account?.hosted_account_id) {
      if (payload?.admission) {
        state.hostedAdmission = clone(payload.admission);
      }
      throw new Error(payload?.error || payload?.error_code || `hosted account login complete failed with HTTP ${response.status}`);
    }
    state.hostedAdmission = payload?.admission ? clone(payload.admission) : state.hostedAdmission;
    await installSession(state, {
      available: true,
      hostedAccountId: String(payload.account.hosted_account_id || "").trim() || null,
      playerId: String(payload.grant.player_id || "").trim(),
      loginChannel: String(payload.account.login_channel || "").trim() || null,
      maskedLoginHint: String(payload.account.masked_login_hint || "").trim() || null,
      deviceSessionId: String(payload.grant.device_session_id || "").trim()
        || String(payload.grant.release_token || "").trim()
        || null,
      publicKey: keypair.publicKey,
      privateKey: keypair.privateKey,
      releaseToken: String(payload.grant.release_token || "").trim() || null,
      registrationGrant: String(payload.grant.registration_grant || "").trim() || null,
      error: null,
      revokeReason: null,
      revokedBy: null,
      source: "hosted_browser_storage",
      registrationStatus: "issued",
      sessionEpoch: null,
      bindingEpoch: null,
      authorityEpoch: null,
      issuedAtUnixMs: payload?.grant?.issued_at_unix_ms == null ? Date.now() : Number(payload.grant.issued_at_unix_ms),
      recoveryErrorCode: null,
      recoveryErrorMessage: null,
      issueInFlight: false,
      syncInFlight: false,
      runtimeStatus: "issued",
      boundAgentId: null,
      controlLostAgentId: null,
      pendingRequestedAgentId: null,
      pendingForceRebind: false,
      rebindNotice: null,
    });
    persistHostedPlayerSession(state.auth);
    resetHostedLoginChallenge();
    state.hostedLogin.startInFlight = false;
    state.hostedLogin.error = null;
    render();
    return state.auth;
  } catch (error) {
    state.auth.issueInFlight = false;
    state.hostedLogin.completeInFlight = false;
    state.hostedLogin.error = String(error);
    state.auth.error = String(error);
    render();
    return state.auth;
  }
}
;
}
