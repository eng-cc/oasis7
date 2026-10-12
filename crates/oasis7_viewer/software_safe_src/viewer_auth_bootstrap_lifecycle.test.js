import { webcrypto } from "node:crypto";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

const reconnect = vi.hoisted(() => ({ sync: vi.fn(async () => ({ ok: true })) }));
vi.mock("./viewer_hosted_session_reconnect_module.js", () => ({
  createViewerHostedSessionReconnectModule: () => ({ syncHostedPlayerSessionOnConnect: reconnect.sync }),
}));

const originalCrypto = Object.getOwnPropertyDescriptor(window, "crypto");
let sockets;

function deferred() {
  let resolve;
  let reject;
  const promise = new Promise((onResolve, onReject) => { resolve = onResolve; reject = onReject; });
  return { promise, resolve, reject };
}

function pausePairingProof() {
  const started = deferred();
  const resume = deferred();
  const sign = webcrypto.subtle.sign.bind(webcrypto.subtle);
  const spy = vi.spyOn(webcrypto.subtle, "sign").mockImplementationOnce(async (...args) => {
    started.resolve();
    await resume.promise;
    return sign(...args);
  });
  return { started, resume, spy };
}

beforeEach(() => {
  vi.resetModules();
  reconnect.sync.mockReset().mockResolvedValue({ ok: true });
  Object.defineProperty(window, "crypto", { configurable: true, value: webcrypto });
  window.history.replaceState({}, "", "/viewer.html?test_api=1&connect=1&hosted_bootstrap=1&locale=en");
  window.localStorage.clear();
  sockets = [];
  class MockWebSocket {
    static OPEN = 1;
    constructor() { this.readyState = 1; this.listeners = new Map(); sockets.push(this); }
    addEventListener(type, listener) { this.listeners.set(type, [...(this.listeners.get(type) || []), listener]); }
    send() {}
    close() { this.readyState = 3; }
    receive(message) { for (const listener of this.listeners.get("message") || []) listener({ data: JSON.stringify(message) }); }
  }
  vi.stubGlobal("WebSocket", MockWebSocket);
  vi.stubGlobal("fetch", vi.fn(() => { throw new Error("local identity bootstrap must not fetch"); }));
});

afterEach(() => {
  if (vi.isFakeTimers()) vi.clearAllTimers();
  vi.useRealTimers();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  Object.defineProperty(window, "crypto", originalCrypto);
});

it("waits for the local identity proof while starting the socket immediately", async () => {
  const proof = pausePairingProof();
  const core = await import("./legacy_core.js");
  let settled = false;
  const initialized = core.initializeSoftwareSafeCore();
  void initialized.then(() => { settled = true; }, () => { settled = true; });
  try {
    await proof.started.promise;
    expect(sockets).toHaveLength(1);
    expect(settled).toBe(false);
    expect(core.state.auth.available).toBe(false);
    expect(fetch).not.toHaveBeenCalled();
  } finally {
    // Drain the real crypto task even when the pre-fix assertion fails.
    proof.resume.resolve();
    await initialized;
    await vi.waitFor(() => expect(core.state.auth.available).toBe(true));
  }
  const { hasSigningIdentity } = await import("./viewer_auth_session_module.js");
  expect(hasSigningIdentity(core.state.auth)).toBe(true);
  expect(core.state.auth.source).toBe("local_test_api_ephemeral");
}, 30000);

it("shares an in-flight local issuance with hello acknowledgement authentication", async () => {
  const proof = pausePairingProof();
  const generated = vi.spyOn(webcrypto.subtle, "generateKey");
  const reconnected = deferred();
  reconnect.sync.mockImplementation(async () => { reconnected.resolve(); return { ok: true }; });
  const core = await import("./legacy_core.js");
  const initialized = core.initializeSoftwareSafeCore();
  try {
    await proof.started.promise;
    vi.useFakeTimers();
    sockets[0].receive({ type: "hello_ack", server: "test", world_id: "test-world" });
    expect(generated).toHaveBeenCalledTimes(1);
    expect(reconnect.sync).not.toHaveBeenCalled();
  } finally {
    proof.resume.resolve();
    await initialized;
  }
  await reconnected.promise;
  expect(proof.spy).toHaveBeenCalledTimes(1);
  expect(core.state.auth.available).toBe(true);
  expect(fetch).not.toHaveBeenCalled();
}, 30000);

it("reports a failed identity proof and lets a later hello acknowledgement retry issuance", async () => {
  const proof = pausePairingProof();
  const generated = vi.spyOn(webcrypto.subtle, "generateKey");
  const core = await import("./legacy_core.js");
  const initialized = core.initializeSoftwareSafeCore();
  const failure = expect(initialized).rejects.toThrow("pairing proof failed");
  await proof.started.promise;
  vi.useFakeTimers();
  sockets[0].receive({ type: "hello_ack", server: "test", world_id: "test-world" });
  proof.resume.reject(new Error("pairing proof failed"));
  await failure;
  expect(core.state.auth.available).toBe(false);
  expect(core.state.auth.error).toContain("pairing proof failed");
  expect(core.state.lastError).toContain("pairing proof failed");
  expect(reconnect.sync).not.toHaveBeenCalled();

  const reconnected = deferred();
  reconnect.sync.mockImplementation(async () => { reconnected.resolve(); return { ok: true }; });
  sockets[0].receive({ type: "hello_ack", server: "test", world_id: "test-world" });
  await reconnected.promise;
  expect(generated).toHaveBeenCalledTimes(2);
  expect(core.state.auth.available).toBe(true);
  expect(core.state.auth.error).toBeNull();
  expect(fetch).not.toHaveBeenCalled();
}, 30000);

it("does not install a local identity after the original guest session is cleared during key generation", async () => {
  const started = deferred();
  const resume = deferred();
  const generate = webcrypto.subtle.generateKey.bind(webcrypto.subtle);
  vi.spyOn(webcrypto.subtle, "generateKey").mockImplementationOnce(async (...args) => {
    started.resolve();
    await resume.promise;
    return generate(...args);
  });
  const sign = vi.spyOn(webcrypto.subtle, "sign");
  const core = await import("./legacy_core.js");
  const { clearSession, hasSigningIdentity } = await import("./viewer_auth_session_module.js");
  const initialized = core.initializeSoftwareSafeCore();
  await started.promise;
  clearSession(core.state, { source: "guest_only" });
  const replacement = core.state.auth;
  resume.resolve();
  await initialized;
  expect(core.state.auth).toBe(replacement);
  expect(core.state.auth.available).toBe(false);
  expect(hasSigningIdentity(core.state.auth)).toBe(false);
  expect(sign).not.toHaveBeenCalled();
}, 30000);
