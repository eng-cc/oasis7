import { beforeEach, describe, expect, it, vi } from "vitest";
import { buildAuthEnvelope } from "./viewer_auth_crypto.js";

function installMockWebSocket() {
  const sentMessages = []; const sockets = [];
  class MockWebSocket {
    static OPEN = 1; static CONNECTING = 0;
    constructor() { this.readyState = MockWebSocket.CONNECTING; this.listeners = new Map(); sockets.push(this); }
    addEventListener(type, listener) { this.listeners.set(type, [...(this.listeners.get(type) || []), listener]); }
    send(payload) { sentMessages.push(JSON.parse(payload)); }
    close() { this.readyState = 3; }
    open() { this.readyState = MockWebSocket.OPEN; for (const listener of this.listeners.get("open") || []) listener({}); }
  }
  Object.defineProperty(window, "WebSocket", { configurable: true, value: MockWebSocket }); return { sentMessages, sockets };
}
function installTestCrypto() { Object.defineProperty(window, "crypto", { configurable: true, value: { subtle: { async importKey() { return {}; }, async sign() { return new Uint8Array(64).fill(7).buffer; } } } }); }

describe("requestFragmentRefillPreview", () => {
  beforeEach(() => { vi.resetModules(); window.history.replaceState({}, "", "/software_safe.html?test_api=1&connect=1&hosted_bootstrap=0"); installTestCrypto(); });

  it("binds the exact chunk in a signed read-only preview request and rejects a duplicate request", async () => {
    const signSpy = vi.spyOn(window.crypto.subtle, "sign"); const { sentMessages, sockets } = installMockWebSocket();
    const core = await import("./legacy_core.js"); core.initializeSoftwareSafeCore(); sockets[0].open();
    core.state.auth = { ...core.state.auth, available: true, playerId: "player-fragment-preview", publicKey: "09".repeat(32), privateKey: "07".repeat(32), registrationStatus: "registered", runtimeStatus: "registered", boundAgentId: "agent-0" };
    expect(await core.requestFragmentRefillPreview("invalid", 0, 0)).toEqual(expect.objectContaining({ ok: false }));
    expect(await window.__AW_TEST__.requestFragmentRefillPreview(2, -1, 0)).toEqual(expect.objectContaining({ ok: true, request: expect.objectContaining({ chunk: { x: 2, y: -1, z: 0 } }) }));
    expect(await core.requestFragmentRefillPreview(2, -1, 0)).toEqual(expect.objectContaining({ ok: false, reason: expect.stringContaining("already pending") }));
    const message = sentMessages.find((entry) => entry.type === "preview_fragment_replenishment");
    expect(message).toEqual(expect.objectContaining({ request: expect.objectContaining({ chunk: { x: 2, y: -1, z: 0 } }) }));
    const actualSigningPayload = new Uint8Array(signSpy.mock.calls.at(-1)[2]);
    const expectedSigningPayload = buildAuthEnvelope({ operation: "gameplay_action", action_id: "preview_fragment_replenishment", target_agent_id: "chunk:2:-1:0", player_id: message.request.auth.player_id, public_key: message.request.auth.public_key, nonce: message.request.auth.nonce });
    expect(Array.from(actualSigningPayload)).toEqual(Array.from(expectedSigningPayload));
  });
});
