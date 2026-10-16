import { webcrypto } from "node:crypto";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const seam = vi.hoisted(() => ({
  sign: vi.fn(),
  client: {
    capabilities: vi.fn(), joinWorld: vi.fn(), refreshWorld: vi.fn(), releaseWorld: vi.fn(),
  },
}));
vi.mock("./viewer_passkey_account_module.js", async (original) => {
  const module = await original();
  return { ...module, createPasskeyAccountClient: () => seam.client };
});
vi.mock("./viewer_auth_crypto.js", async (original) => ({
  ...await original(),
  importSigningIdentity: vi.fn(async (publicKey) => ({ publicKey, identityId: `fixture:${publicKey}` })),
  signAuthPayload: (...args) => seam.sign(...args),
}));

import { authCredentials, installSession, clearSession } from "./viewer_auth_session_module.js";
import { createHostedWorldAccount } from "./viewer_hosted_world_account_module.js";

const account = { account_id: "account-a" };
const pair = (number) => ({ publicKey: String(number).repeat(64), privateKey: `fixture-secret-${number}` });
const grant = (number, epoch = number) => ({
  ok: true, world: { state: "registration_pending" },
  grant: { player_id: "hosted-player-a", device_session_id: `device-${number}`,
    world_session_epoch: epoch, registration_grant: `signed-fixture-grant-${number}` },
});
const deferred = () => {
  let resolve, reject;
  const promise = new Promise((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
};
const instances = [];
let storage;
function fixture(initial = { available: false, source: "guest_only" }) {
  const state = { auth: initial };
  const generateKeypair = vi.fn(async () => pair(1));
  const register = vi.fn(async () => {});
  const render = vi.fn();
  const clearLegacy = vi.fn();
  const module = createHostedWorldAccount({ state, generateKeypair, register, render, clearLegacy });
  instances.push(module);
  return { module, state, generateKeypair, register, render, clearLegacy };
}

beforeEach(() => {
  vi.useFakeTimers();
  vi.clearAllMocks();
  vi.stubGlobal("crypto", webcrypto);
  storage = { setItem: vi.fn(), getItem: vi.fn(), removeItem: vi.fn() };
  vi.stubGlobal("localStorage", storage);
  seam.client.capabilities.mockResolvedValue({ ok: true });
  seam.sign.mockResolvedValue("fixture-session-proof");
  seam.client.joinWorld.mockResolvedValue(grant(1));
  seam.client.refreshWorld.mockResolvedValue({ ...grant(1), world: { state: "bound" } });
  seam.client.releaseWorld.mockResolvedValue({ ok: true, world: { state: "released" } });
});
afterEach(() => {
  for (const module of instances.splice(0)) module.invalidate();
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

describe("hosted account world bridge contracts", () => {
  it("clears legacy bootstrap errors after v2 detection while preserving active v2 control errors", () => {
    const guest = fixture({ source: "guest_only", error: "viewer auth bootstrap is unavailable", rebindNotice: "legacy" });
    guest.module.disableLegacy();
    expect(guest.state.auth).toMatchObject({ error: null, rebindNotice: null });
    expect(guest.render).toHaveBeenCalledOnce();
    const active = fixture({ source: "hosted_account_cookie", error: "session_revoked" });
    active.module.disableLegacy();
    expect(active.state.auth.error).toBe("session_revoked");
  });
  it("does not turn an existing account login into an automatic world join", async () => {
    const { module, generateKeypair, register } = fixture({
      available: true, source: "hosted_account_cookie", hostedAccountId: account.account_id,
    });
    await vi.advanceTimersByTimeAsync(30_000);
    expect(await module.renew()).toBeNull();
    expect(generateKeypair).not.toHaveBeenCalled();
    expect(seam.client.joinWorld).not.toHaveBeenCalled();
    expect(register).not.toHaveBeenCalled();
  });

  it("creates a 43-character base64url attempt and shares duplicate join work", async () => {
    const waiting = deferred();
    seam.client.joinWorld.mockReturnValue(waiting.promise);
    const { module, generateKeypair, register } = fixture();
    const first = module.join(account), second = module.join(account);
    await vi.waitFor(() => expect(seam.client.joinWorld).toHaveBeenCalledTimes(1));
    const payload = seam.client.joinWorld.mock.calls[0][0];
    expect(payload.join_attempt_id).toMatch(/^[A-Za-z0-9_-]{43}$/);
    expect(payload).toMatchObject({ public_key: pair(1).publicKey, takeover: false });
    expect(payload).not.toHaveProperty("account_id");
    expect(payload).not.toHaveProperty("player_id");
    waiting.resolve(grant(1));
    await Promise.all([first, second]);
    expect(generateKeypair).toHaveBeenCalledTimes(1);
    expect(register).toHaveBeenCalledTimes(1);
    expect(register).toHaveBeenCalledWith(null, { forceRebind: false });
  });

  it("sends takeover and the explicit server epoch only after confirmation", async () => {
    seam.client.joinWorld.mockResolvedValue(grant(1, 8));
    const { module } = fixture();
    await module.join(account, { takeover: true, expectedEpoch: 7 });
    expect(seam.client.joinWorld).toHaveBeenCalledWith(expect.objectContaining({
      takeover: true, expected_epoch: 7,
    }));
  });

  it("renews a fresh signed grant and re-registers the same control key every ten seconds", async () => {
    const { module, state, generateKeypair, register } = fixture();
    await module.join(account);
    seam.client.refreshWorld.mockResolvedValue({
      ...grant(1), world: { state: "bound" },
      grant: { ...grant(1).grant, registration_grant: "fresh-signed-grant" },
    });
    await vi.advanceTimersByTimeAsync(9_999);
    expect(seam.client.refreshWorld).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(1);
    expect(seam.client.refreshWorld).toHaveBeenCalledExactlyOnceWith({
      public_key: pair(1).publicKey, device_session_id: "device-1", expected_epoch: 1,
    });
    expect(authCredentials(state.auth).registrationGrant).toBe("fresh-signed-grant");
    expect(register).toHaveBeenCalledTimes(2);
    expect(register).toHaveBeenLastCalledWith(null, { forceRebind: false, renewAuthorization: true });
    expect(generateKeypair).toHaveBeenCalledTimes(1);
  });

  it("retains the original attempt and key when the join result is unknown", async () => {
    seam.client.joinWorld.mockRejectedValueOnce(new Error("operation_pending"));
    const { module, generateKeypair, register } = fixture();
    await expect(module.join(account)).rejects.toThrow("operation_pending");
    const original = seam.client.joinWorld.mock.calls[0][0];
    expect(register).not.toHaveBeenCalled();
    await module.join(account);
    expect(seam.client.joinWorld.mock.calls[1][0]).toEqual(original);
    expect(generateKeypair).toHaveBeenCalledTimes(1);
  });

  it("restores the previous signing identity and its renewal when new binding is confirmed pending", async () => {
    seam.client.joinWorld.mockResolvedValueOnce(grant(1)).mockResolvedValueOnce(grant(2));
    const { module, state, generateKeypair, register } = fixture();
    generateKeypair.mockResolvedValueOnce(pair(1)).mockResolvedValueOnce(pair(2));
    await module.join(account);
    register.mockRejectedValueOnce(new Error("candidate bind rejected"));
    seam.client.refreshWorld.mockResolvedValueOnce({ world: { state: "registration_pending" } });
    await expect(module.join(account, { takeover: true, expectedEpoch: 1 })).rejects.toThrow("candidate bind rejected");
    expect(state.auth.publicKey).toBe(pair(1).publicKey);
    expect(authCredentials(state.auth).privateKey).toBe(pair(1).privateKey);
    seam.client.refreshWorld.mockResolvedValue({ ...grant(1), world: { state: "bound" } });
    await module.renew();
    expect(seam.client.refreshWorld).toHaveBeenLastCalledWith({
      public_key: pair(1).publicKey, device_session_id: "device-1", expected_epoch: 1,
    });
    expect(register).toHaveBeenCalledTimes(3);
    expect(register).toHaveBeenLastCalledWith(null, { forceRebind: false, renewAuthorization: true });
  });

  it("coalesces concurrent renewal so an older response cannot overwrite another grant", async () => {
    const { module, register } = fixture();
    await module.join(account);
    const response = deferred();
    seam.client.refreshWorld.mockReturnValueOnce(response.promise);
    const first = module.renew(), second = module.renew();
    expect(first).toBe(second);
    await vi.waitFor(() => expect(seam.client.refreshWorld).toHaveBeenCalledTimes(1));
    response.resolve({ ...grant(1), world: { state: "bound" } });
    await Promise.all([first, second]);
    expect(register).toHaveBeenCalledTimes(2);
  });

  it("forces the real registered-session consumer to dispatch renewed authorization", async () => {
    const priorLocation = window.location.href;
    window.history.replaceState({}, "", "/viewer.html?test_api=1&connect=0&hosted_bootstrap=0&hosted_access=%7B%22deployment_mode%22%3A%22hosted_public_join%22%7D");
    const core = await import("./legacy_core.js");
    await installSession(core.state, {
      available: true, source: "hosted_account_cookie", playerId: "hosted-player-a",
      ...pair(1), registrationGrant: "fresh-signed-grant", registrationStatus: "registered",
      runtimeStatus: "registered_unbound", deviceSessionId: "device-1",
    });
    try {
      await expect(core.registerPlayerSessionForTest(null, { forceRebind: false })).resolves.toMatchObject({ registrationStatus: "registered" });
      expect(seam.sign).not.toHaveBeenCalled();
      // The fixture has no socket. A renewal must reach signing/send instead of
      // resolving via the existing registered-session shortcut.
      await expect(core.registerPlayerSessionForTest(null, { forceRebind: false, renewAuthorization: true })).rejects.toThrow("viewer websocket is not connected");
      expect(seam.sign).toHaveBeenCalledTimes(1);
    } finally {
      clearSession(core.state);
      window.history.replaceState({}, "", priorLocation);
    }
  });

  it("keeps control proofs in memory and never exposes a release bearer in state, storage or requests", async () => {
    const { module, state } = fixture();
    await module.join(account);
    expect(state.auth).not.toHaveProperty("privateKey");
    expect(state.auth).not.toHaveProperty("registrationGrant");
    expect(state.auth).not.toHaveProperty("releaseToken");
    expect(authCredentials(state.auth).releaseToken).toBeNull();
    await module.renew();
    await module.release();
    for (const call of [seam.client.joinWorld, seam.client.refreshWorld, seam.client.releaseWorld]) {
      for (const [payload] of call.mock.calls) {
        expect(payload).not.toHaveProperty("release_token");
        expect(payload).not.toHaveProperty("private_key");
        expect(payload).not.toHaveProperty("account_token");
      }
    }
    expect(storage.setItem).not.toHaveBeenCalled();
    expect(state.auth.available).toBe(false);
  });

  it("does not revive an invalidated account while key generation is pending", async () => {
    const ready = deferred();
    const { module, state, generateKeypair, register } = fixture();
    generateKeypair.mockReturnValueOnce(ready.promise);
    const joining = module.join(account);
    module.invalidate();
    ready.resolve(pair(1));
    await expect(joining).rejects.toThrow(/state_changed|context changed/);
    expect(seam.client.joinWorld).not.toHaveBeenCalled();
    expect(register).not.toHaveBeenCalled();
    expect(state.auth.available).toBe(false);
  });
});
