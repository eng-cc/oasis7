import { describe, expect, it, vi } from "vitest";
import { createViewerHostedSessionRefreshModule } from "./viewer_hosted_session_refresh_module.js";

describe("viewer hosted session refresh module", () => {
  it("rotates the single-use registration grant for the current browser key without exposing the bearer in the URL", async () => {
    const state = {
      auth: {
        available: true,
        playerId: "hosted-player-account-1",
        releaseToken: "release-token-1",
        publicKey: "public-key-2",
        deviceSessionId: "device-session-1",
      },
      hostedAdmission: null,
    };
    const fetchImpl = vi.fn(async () => ({
      ok: true,
      status: 200,
      json: async () => ({
        ok: true,
        admission: { active_player_sessions: 1 },
        registration_grant: "registration-grant-2",
        device_session_id: "device-session-2",
      }),
    }));
    const persistHostedPlayerSession = vi.fn();
    const { refreshHostedPlayerLease } = createViewerHostedSessionRefreshModule({
      clone: structuredClone,
      ensureHostedAuthSigningKey: async (auth) => auth,
      fetchImpl,
      legacyViewerAuthBootstrapSource: "viewer_auth_bootstrap",
      persistHostedPlayerSession,
      refreshRoute: "/api/public/player-session/refresh",
      state,
    });

    await refreshHostedPlayerLease();

    expect(fetchImpl).toHaveBeenCalledWith(
      "/api/public/player-session/refresh",
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify({
          player_id: "hosted-player-account-1",
          release_token: "release-token-1",
          public_key: "public-key-2",
        }),
      }),
    );
    expect(state.auth).toMatchObject({
      registrationGrant: "registration-grant-2",
      deviceSessionId: "device-session-2",
    });
    expect(persistHostedPlayerSession).toHaveBeenCalledWith(state.auth);
  });
});
