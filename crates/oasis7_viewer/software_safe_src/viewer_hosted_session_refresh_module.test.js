import { installSession, authCredentials, invalidateAuthConnection } from "./viewer_auth_session_module.js";
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
    await installSession(state, state.auth);
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
      deviceSessionId: "device-session-2",
    });
    expect(authCredentials(state.auth).registrationGrant).toBe("registration-grant-2");
    expect(persistHostedPlayerSession).toHaveBeenCalledWith(state.auth);
  });
});


describe("lease concurrency and stale responses", () => {
  it("shares one request and discards a response after logout", async () => {
    let finish;
    const response = new Promise(resolve => { finish = resolve; });
    const state = { auth: { available: true, playerId: "alice", publicKey: "key", releaseToken: "token" } };
    await installSession(state, state.auth);
    const persist = vi.fn();
    const fetchImpl = vi.fn(() => response);
    const module = createViewerHostedSessionRefreshModule({ clone: structuredClone, ensureHostedAuthSigningKey: async auth => auth,
      fetchImpl, persistHostedPlayerSession: persist, refreshRoute: "/refresh", state });
    const a = module.refreshHostedPlayerLease();
    const b = module.refreshHostedPlayerLease();
    expect(a).toBe(b);
    await Promise.resolve(); expect(fetchImpl).toHaveBeenCalledTimes(1);
    state.auth = { available: false };
    finish({ ok: true, json: async () => ({ ok: true, admission: { leaked: true }, registration_grant: "late" }) });
    expect(await a).toBeNull(); expect(persist).not.toHaveBeenCalled(); expect(state.hostedAdmission).toBeUndefined();
  });
});


describe("lease generation-only invalidation", () => {
  it("drops a response when the connection generation changes but the socket object is retained", async () => {
    let finish;
    const deferred = new Promise(resolve => { finish = resolve; });
    const state = { wsUrl: "wss://trusted.example/runtime", auth: { available: true, playerId: "alice", publicKey: "key", releaseToken: "token" } };
    await installSession(state, state.auth);
    const persist = vi.fn(); const fetchImpl = vi.fn(() => deferred); const socket = {};
    const module = createViewerHostedSessionRefreshModule({ clone: structuredClone, ensureHostedAuthSigningKey: async auth => auth, fetchImpl,
      persistHostedPlayerSession: persist, refreshRoute: "/refresh", state, captureConnection: () => socket });
    const pending = module.refreshHostedPlayerLease(); await Promise.resolve();
    invalidateAuthConnection();
    finish({ ok: true, json: async () => ({ ok: true, admission: { unexpected: true }, registration_grant: "stale" }) });
    expect(await pending).toBeNull(); expect(persist).not.toHaveBeenCalled(); expect(state.hostedAdmission).toBeUndefined();
    expect(authCredentials(state.auth).registrationGrant).toBeNull();
  });
});


it("rejects generation invalidation while signing-key readiness is pending", async () => {
  let release;
  const state = { wsUrl: "wss://trusted.example/runtime", auth: { available: true, playerId: "alice", publicKey: "key", releaseToken: "token" } };
  await installSession(state, state.auth);
  const auth = state.auth; const socket = {}; const fetchImpl = vi.fn(); const persist = vi.fn();
  const module = createViewerHostedSessionRefreshModule({ clone: structuredClone,
    ensureHostedAuthSigningKey: () => new Promise(resolve => { release = resolve; }), fetchImpl,
    persistHostedPlayerSession: persist, refreshRoute: "/refresh", state, captureConnection: () => socket });
  const pending = module.refreshHostedPlayerLease();
  invalidateAuthConnection(); release(auth);
  expect(await pending).toBeNull(); expect(fetchImpl).not.toHaveBeenCalled(); expect(persist).not.toHaveBeenCalled();
});
