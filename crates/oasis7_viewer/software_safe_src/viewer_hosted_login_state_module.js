export function createInitialHostedLoginState() {
  return {
    channel: "email",
    handle: "",
    challengeId: null,
    maskedLoginHint: null,
    deliveryMode: null,
    previewCode: null,
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
  hostedLogin.previewCode = null;
  hostedLogin.code = "";
  hostedLogin.expiresAtUnixMs = null;
  hostedLogin.accountExists = false;
  hostedLogin.completeInFlight = false;
}
