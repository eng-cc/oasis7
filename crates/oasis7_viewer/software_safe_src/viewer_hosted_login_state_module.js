export function createInitialHostedLoginState() {
  return {
    channel: "email",
    handle: "",
    challengeId: null,
    maskedLoginHint: null,
    deliveryMode: null,
    code: "",
    expiresAtUnixMs: null,
    retryAfterSeconds: null,
    accountExists: false,
    startInFlight: false,
    completeInFlight: false,
    error: null,
  };
}

export function resetHostedLoginChallenge(hostedLogin) {
  if (!hostedLogin) {
    return;
  }
  hostedLogin.channel = "email";
  hostedLogin.challengeId = null;
  hostedLogin.maskedLoginHint = null;
  hostedLogin.deliveryMode = null;
  hostedLogin.code = "";
  hostedLogin.expiresAtUnixMs = null;
  hostedLogin.retryAfterSeconds = null;
  hostedLogin.accountExists = false;
  hostedLogin.startInFlight = false;
  hostedLogin.completeInFlight = false;
  hostedLogin.error = null;
}
