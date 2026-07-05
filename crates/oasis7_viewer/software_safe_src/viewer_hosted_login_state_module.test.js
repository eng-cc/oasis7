import { describe, expect, it } from "vitest";
import {
  createInitialHostedLoginState,
  resetHostedLoginChallenge,
} from "./viewer_hosted_login_state_module.js";

describe("viewer hosted login state module", () => {
  it("resets challenge transient state while preserving the login handle", () => {
    const hostedLogin = createInitialHostedLoginState();
    Object.assign(hostedLogin, {
      handle: "player@example.com",
      challengeId: "old-challenge",
      maskedLoginHint: "p***@example.com",
      deliveryMode: "email",
      code: "123456",
      expiresAtUnixMs: 1234567890,
      retryAfterSeconds: 21,
      accountExists: true,
      startInFlight: true,
      completeInFlight: true,
      error: "rate limited",
    });

    resetHostedLoginChallenge(hostedLogin);

    expect(hostedLogin).toEqual({
      ...createInitialHostedLoginState(),
      handle: "player@example.com",
    });
  });
});
