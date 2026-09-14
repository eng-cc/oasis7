import { describe, expect, it, vi } from "vitest";
import { createViewerHostedTestLoginModule } from "./viewer_hosted_test_login_module.js";

function deferred() {
  let resolve;
  const promise = new Promise((nextResolve) => {
    resolve = nextResolve;
  });
  return { promise, resolve };
}

describe("viewer hosted test login module", () => {
  it("lets registration readiness await an in-flight issuer response", async () => {
    const response = deferred();
    const state = {
      auth: {
        available: false,
        issueInFlight: false,
      },
      hostedAccess: { deployment_mode: "hosted_public_join" },
      hostedLogin: {
        channel: "email",
        startInFlight: false,
        error: null,
      },
      hostedAdmission: null,
    };
    const module = createViewerHostedTestLoginModule({
      clone: structuredClone,
      fetchImpl: vi.fn(async () => response.promise),
      generateEphemeralEd25519Keypair: vi.fn(async () => ({
        publicKey: "test-public-key",
        privateKey: "test-private-key",
      })),
      getSearchParams: () => new URLSearchParams("hosted_test_login=1"),
      isHostedPublicJoinDeploymentMode: (mode) => mode === "hosted_public_join",
      persistHostedPlayerSession: vi.fn(),
      render: vi.fn(),
      resetHostedLoginChallenge: vi.fn(),
      route: "/api/public/hosted-account/test-login",
      state,
    });

    const loginPromise = module.start();
    const readyPromise = module.waitForStart();
    let ready = false;
    void readyPromise.then(() => {
      ready = true;
    });
    await Promise.resolve();
    expect(ready).toBe(false);

    response.resolve({
      ok: true,
      status: 200,
      async json() {
        return {
          ok: true,
          grant: {
            player_id: "hosted-player-test",
            device_session_id: "device-session-test",
            release_token: "release-token-test",
            registration_grant: "registration-grant-test",
          },
        };
      },
    });

    await expect(loginPromise).resolves.toMatchObject({
      available: true,
      playerId: "hosted-player-test",
    });
    await expect(readyPromise).resolves.toMatchObject({
      available: true,
      playerId: "hosted-player-test",
    });
    expect(ready).toBe(true);
  });
});
