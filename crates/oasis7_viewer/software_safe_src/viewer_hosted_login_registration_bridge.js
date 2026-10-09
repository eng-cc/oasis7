export function createViewerHostedLoginRegistrationBridge({ registerPlayerSession, render, state }) {
  const registrations = new WeakMap();

  function registerIssuedSession(auth) {
    if (auth.registrationStatus === "registered" && ["registered", "registered_unbound"].includes(auth.runtimeStatus)) {
      return Promise.resolve(auth);
    }
    if (registrations.has(auth)) return registrations.get(auth);
    const registration = Promise.resolve()
      .then(() => {
        if (state.auth !== auth) throw new Error("issued login session was replaced before registration dispatch");
        return registerPlayerSession(null, { forceRebind: false });
      })
      .then(() => {
        if (state.auth !== auth) throw new Error("issued login session was replaced before registration completed");
        return auth;
      })
      .catch(error => {
        if (state.auth === auth) {
          state.hostedLogin.error = String(error);
          auth.error = String(error);
          auth.runtimeStatus = "error";
          auth.recoveryErrorCode ||= "session_register_failed";
          auth.recoveryErrorMessage ||= String(error);
          render();
        }
        throw error;
      })
      .finally(() => registrations.delete(auth));
    registrations.set(auth, registration);
    return registration;
  }

  function wrapLogin(issueLogin, expectedSource) {
    let inFlight = null;
    return function completeLogin() {
      if (inFlight) return inFlight;
      inFlight = Promise.resolve()
        .then(() => issueLogin())
        .then(result => {
          if (result?.ok === false || !state.auth.available || state.auth.source !== expectedSource) return result;
          if (result !== state.auth) return { ok: false, reason: "issued login session was replaced before registration" };
          return registerIssuedSession(state.auth);
        })
        .catch(error => ({ ok: false, reason: String(error) }))
        .finally(() => { inFlight = null; });
      return inFlight;
    };
  }

  return { wrapLogin, registerIssuedSession };
}
