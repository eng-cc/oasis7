import { describe, expect, it, vi } from "vitest";
import { createViewerHostedLoginRegistrationBridge } from "./viewer_hosted_login_registration_bridge.js";
const deferred = () => { let resolve, reject; const promise = new Promise((yes, no) => { resolve = yes; reject = no; }); return { promise, resolve, reject }; };

describe("issued Hosted login registration bridge", () => {
  it.each(["hosted_test_login", "hosted_browser_storage"])("awaits %s issuer then actual registration, sharing duplicate clicks", async source => {
    const issued = deferred(), registered = deferred();
    const state = { auth: { available: false }, hostedLogin: {} };
    const register = vi.fn(async () => {
      await issued.promise; // Issuer flight finished: no recursive self-wait.
      await registered.promise;
      Object.assign(state.auth, { registrationStatus: "registered", runtimeStatus: "registered_unbound" });
    });
    const bridge = createViewerHostedLoginRegistrationBridge({ registerPlayerSession: register, render: vi.fn(), state });
    const issuer = vi.fn(async () => { await issued.promise; state.auth = { available: true, source, registrationStatus: "issued" }; return state.auth; });
    const login = bridge.wrapLogin(issuer, source), first = login();
    expect(login()).toBe(first);
    await Promise.resolve(); expect(register).not.toHaveBeenCalled();
    issued.resolve(); await vi.waitFor(() => expect(register).toHaveBeenCalledTimes(1));
    expect(register).toHaveBeenCalledWith(null, { forceRebind: false });
    let finished = false; first.then(() => { finished = true; });
    await Promise.resolve(); expect(finished).toBe(false);
    registered.resolve(); await expect(first).resolves.toMatchObject({ registrationStatus: "registered", runtimeStatus: "registered_unbound" });
    await bridge.wrapLogin(async () => state.auth, source)();
    expect(register).toHaveBeenCalledTimes(1); expect(issuer).toHaveBeenCalledTimes(1);
  });
  it("does not register failed issuance or an unrelated existing session", async () => {
    const state = { auth: { available: false }, hostedLogin: {} }, register = vi.fn();
    const bridge = createViewerHostedLoginRegistrationBridge({ registerPlayerSession: register, render: vi.fn(), state });
    await expect(bridge.wrapLogin(async () => ({ ok: false, reason: "issuer denied" }), "hosted_test_login")()).resolves.toEqual({ ok: false, reason: "issuer denied" });
    state.auth = { available: true, source: "legacy_bootstrap" };
    await bridge.wrapLogin(async () => state.auth, "hosted_test_login")();
    expect(register).not.toHaveBeenCalled();
  });
  it("does not register a replacement session when an old issuer finishes", async () => {
    const old = { available: true, source: "hosted_test_login", registrationStatus: "issued" };
    const state = { auth: { ...old, playerId: "replacement" }, hostedLogin: {} }, register = vi.fn();
    const bridge = createViewerHostedLoginRegistrationBridge({ registerPlayerSession: register, render: vi.fn(), state });
    await expect(bridge.wrapLogin(async () => old, "hosted_test_login")()).resolves.toEqual({ ok: false, reason: "issued login session was replaced before registration" });
    expect(register).not.toHaveBeenCalled();
  });
  it("shows registration rejection without changing binding or reissuing grants", async () => {
    const state = { auth: { available: true, source: "hosted_test_login", registrationStatus: "issued" }, hostedLogin: {} };
    const render = vi.fn(), issuer = vi.fn(async () => state.auth);
    const register = vi.fn(async () => { throw new Error("registration grant rejected"); });
    const bridge = createViewerHostedLoginRegistrationBridge({ registerPlayerSession: register, render, state });
    await expect(bridge.wrapLogin(issuer, "hosted_test_login")()).resolves.toEqual({ ok: false, reason: "Error: registration grant rejected" });
    expect(register).toHaveBeenCalledWith(null, { forceRebind: false }); expect(issuer).toHaveBeenCalledTimes(1);
    expect(state.auth.registrationStatus).toBe("issued"); expect(state.auth.runtimeStatus).toBe("error");
    expect(state.hostedLogin.error).toContain("registration grant rejected"); expect(render).toHaveBeenCalledTimes(1);
  });
  it("does not dispatch registration when the captured session changes before its microtask", async () => {
    const state = { auth: { available: true, source: "hosted_test_login", registrationStatus: "issued" }, hostedLogin: {} };
    const register = vi.fn(), render = vi.fn();
    const bridge = createViewerHostedLoginRegistrationBridge({ registerPlayerSession: register, render, state });
    const registration = bridge.registerIssuedSession(state.auth);
    state.auth = { available: true, playerId: "replacement" };
    await expect(registration).rejects.toThrow("replaced before registration dispatch");
    expect(register).not.toHaveBeenCalled(); expect(render).not.toHaveBeenCalled();
  });
  it("rejects an old login when successful registration completes after replacement", async () => {
    const registered = deferred(), started = deferred();
    const state = { auth: { available: true, source: "hosted_test_login", registrationStatus: "issued" }, hostedLogin: {} };
    const register = vi.fn(() => { started.resolve(); return registered.promise; });
    const bridge = createViewerHostedLoginRegistrationBridge({ registerPlayerSession: register, render: vi.fn(), state });
    const flow = bridge.wrapLogin(async () => state.auth, "hosted_test_login")();
    await started.promise; state.auth = { available: true, playerId: "replacement" }; registered.resolve();
    await expect(flow).resolves.toEqual({ ok: false, reason: "Error: issued login session was replaced before registration completed" });
    expect(state.auth.error).toBeUndefined();
  });
  it("does not write stale rejection into a replacement session", async () => {
    const failure = deferred(), started = deferred();
    const state = { auth: { available: true, source: "hosted_test_login", registrationStatus: "issued" }, hostedLogin: {} }, render = vi.fn();
    const bridge = createViewerHostedLoginRegistrationBridge({ registerPlayerSession: () => { started.resolve(); return failure.promise; }, render, state });
    const flow = bridge.wrapLogin(async () => state.auth, "hosted_test_login")();
    await started.promise; state.auth = { available: true, playerId: "replacement" };
    failure.reject(new Error("old session denied")); await flow;
    expect(state.auth.error).toBeUndefined(); expect(state.hostedLogin.error).toBeUndefined(); expect(render).not.toHaveBeenCalled();
  });
});
