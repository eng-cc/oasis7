import { beforeEach, describe, expect, it } from "vitest";
import { createViewerHostedAuthStateModule } from "./viewer_hosted_auth_state_module.js";

function createHostedAuthStateModule() {
  return createViewerHostedAuthStateModule({
    hostedPlayerSessionStoragePrefix: "oasis7:viewer:hosted-player-session",
    initialWsUrl: () => "ws://127.0.0.1:7777/ws",
    viewerAuthBootstrapObject: "__OASIS7_VIEWER_AUTH__",
    viewerAuthPrivateKey: "private_key",
    viewerAuthPublicKey: "public_key",
    viewerPlayerIdKey: "player_id",
    windowRef: window,
  });
}

function hostedSessionStorageKey() {
  return "oasis7:viewer:hosted-player-session:ws://127.0.0.1:7777/ws";
}

describe("viewer hosted auth state module", () => {
  beforeEach(() => {
    delete window.__OASIS7_VIEWER_AUTH__;
    window.localStorage.clear();
  });

  it("recovers snake_case hosted player session storage and migrates it to the canonical key shape", () => {
    window.localStorage.setItem(
      hostedSessionStorageKey(),
      JSON.stringify({
        hosted_account_id: "hosted-account-1",
        player_id: "hosted-player-1",
        login_channel: "email",
        masked_login_hint: "p***@example.test",
        device_session_id: "device-session-1",
        release_token: "release-token-1",
        registration_grant: "registration-grant-1",
        issued_at_unix_ms: 1234,
        session_epoch: 7,
      }),
    );

    const auth = createHostedAuthStateModule().resolveViewerAuthState();

    expect(auth).toMatchObject({
      available: true,
      hostedAccountId: "hosted-account-1",
      playerId: "hosted-player-1",
      loginChannel: "email",
      maskedLoginHint: "p***@example.test",
      deviceSessionId: "device-session-1",
      releaseToken: "release-token-1",
      registrationGrant: "registration-grant-1",
      source: "hosted_browser_storage",
      registrationStatus: "issued",
      sessionEpoch: 7,
      issuedAtUnixMs: 1234,
      runtimeStatus: "issued",
    });
    expect(JSON.parse(window.localStorage.getItem(hostedSessionStorageKey()))).toEqual({
      hostedAccountId: "hosted-account-1",
      playerId: "hosted-player-1",
      loginChannel: "email",
      maskedLoginHint: "p***@example.test",
      deviceSessionId: "device-session-1",
      releaseToken: "release-token-1",
      registrationGrant: "registration-grant-1",
      issuedAtUnixMs: 1234,
      sessionEpoch: 7,
    });
  });

  it("drops malformed numeric session metadata while preserving recoverable hosted auth", () => {
    window.localStorage.setItem(
      hostedSessionStorageKey(),
      JSON.stringify({
        playerId: "hosted-player-1",
        deviceSessionId: "device-session-1",
        releaseToken: "release-token-1",
        issuedAtUnixMs: "not-a-timestamp",
        sessionEpoch: "not-an-epoch",
      }),
    );

    const auth = createHostedAuthStateModule().resolveViewerAuthState();

    expect(auth).toMatchObject({
      available: true,
      playerId: "hosted-player-1",
      deviceSessionId: "device-session-1",
      releaseToken: "release-token-1",
      source: "hosted_browser_storage",
      registrationStatus: "issued",
      sessionEpoch: null,
      issuedAtUnixMs: null,
      runtimeStatus: "issued",
    });
    expect(JSON.parse(window.localStorage.getItem(hostedSessionStorageKey()))).toMatchObject({
      playerId: "hosted-player-1",
      deviceSessionId: "device-session-1",
      releaseToken: "release-token-1",
      issuedAtUnixMs: null,
      sessionEpoch: null,
    });
  });

  it("preserves zero-valued hosted session metadata when persisting auth state", () => {
    const module = createHostedAuthStateModule();

    module.persistHostedPlayerSession({
      available: true,
      hostedAccountId: "hosted-account-1",
      playerId: "hosted-player-1",
      loginChannel: "email",
      maskedLoginHint: "p***@example.test",
      deviceSessionId: "device-session-1",
      releaseToken: "release-token-1",
      issuedAtUnixMs: 0,
      sessionEpoch: 0,
      source: "hosted_runtime_issue",
    });

    expect(JSON.parse(window.localStorage.getItem(hostedSessionStorageKey()))).toMatchObject({
      issuedAtUnixMs: 0,
      sessionEpoch: 0,
    });
  });
});
