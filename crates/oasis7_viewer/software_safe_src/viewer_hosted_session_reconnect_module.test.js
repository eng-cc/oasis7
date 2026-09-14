import { describe, expect, it, vi } from "vitest";
import { createViewerHostedSessionReconnectModule } from "./viewer_hosted_session_reconnect_module.js";

function createModule(auth, overrides = {}) {
  const state = { auth: { ...auth } };
  const calls = [];
  const module = createViewerHostedSessionReconnectModule({
    authHasSigningKeyMaterial: (value) => !!value.publicKey && !!value.privateKey,
    legacyViewerAuthBootstrapSource: "legacy_viewer_auth_bootstrap",
    onRefreshFailure: vi.fn(() => calls.push("refresh-failure")),
    refreshHostedPlayerLease: vi.fn(async () => {
      calls.push("refresh");
      return { ok: true };
    }),
    registerHostedPlayerSession: vi.fn(async () => calls.push("register")),
    sendReconnectSync: vi.fn(async () => calls.push("reconnect")),
    state,
    ...overrides,
  });
  return { calls, module, state };
}

describe("viewer hosted session reconnect module", () => {
  it("refreshes the device lease before registering a new in-memory browser key after reload", async () => {
    const { calls, module } = createModule({
      available: true,
      source: "hosted_browser_storage",
      playerId: "hosted-player-1",
      releaseToken: "release-token-1",
      publicKey: null,
      privateKey: null,
      syncInFlight: false,
    });

    await expect(module.syncHostedPlayerSessionOnConnect()).resolves.toEqual({
      ok: true,
      mode: "registration",
    });
    expect(calls).toEqual(["refresh", "register"]);
  });

  it("keeps the existing reconnect path when the current browser key is present", async () => {
    const { calls, module } = createModule({
      available: true,
      source: "hosted_test_login",
      playerId: "hosted-player-1",
      releaseToken: "release-token-1",
      publicKey: "public-key-1",
      privateKey: "private-key-1",
      syncInFlight: false,
    });

    await expect(module.syncHostedPlayerSessionOnConnect()).resolves.toEqual({
      ok: true,
      mode: "reconnect",
    });
    expect(calls).toEqual(["reconnect"]);
  });

  it("does not send a new key when lease refresh fails", async () => {
    const onRefreshFailure = vi.fn();
    const refreshHostedPlayerLease = vi.fn(async () => ({ ok: false }));
    const registerHostedPlayerSession = vi.fn();
    const sendReconnectSync = vi.fn();
    const { module } = createModule(
      {
        available: true,
        source: "hosted_browser_storage",
        playerId: "hosted-player-1",
        releaseToken: "release-token-1",
        publicKey: null,
        privateKey: null,
        syncInFlight: false,
      },
      { onRefreshFailure, refreshHostedPlayerLease, registerHostedPlayerSession, sendReconnectSync },
    );

    await expect(module.syncHostedPlayerSessionOnConnect()).resolves.toEqual({
      ok: false,
      reason: "session_refresh_failed",
    });
    expect(onRefreshFailure).toHaveBeenCalledOnce();
    expect(registerHostedPlayerSession).not.toHaveBeenCalled();
    expect(sendReconnectSync).not.toHaveBeenCalled();
  });
});
