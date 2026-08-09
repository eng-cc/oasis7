import { beforeEach, describe, expect, it, vi } from "vitest";
import { buildAuthEnvelope } from "./viewer_auth_crypto.js";

function installMockWebSocket() {
  const sentMessages = []; const sockets = [];
  class MockWebSocket { static OPEN = 1; static CONNECTING = 0; constructor() { this.readyState = MockWebSocket.CONNECTING; this.listeners = new Map(); sockets.push(this); } addEventListener(type, listener) { this.listeners.set(type, [...(this.listeners.get(type) || []), listener]); } send(payload) { sentMessages.push(JSON.parse(payload)); } close() { this.readyState = 3; } open() { this.readyState = MockWebSocket.OPEN; for (const listener of this.listeners.get("open") || []) listener({}); } }
  Object.defineProperty(window, "WebSocket", { configurable: true, value: MockWebSocket }); return { sentMessages, sockets };
}
function installTestCrypto() { Object.defineProperty(window, "crypto", { configurable: true, value: { subtle: { async importKey() { return {}; }, async sign() { return new Uint8Array(64).fill(7).buffer; } } } }); }

describe("requestScheduleRecipeQuote", () => {
  beforeEach(() => { vi.resetModules(); window.history.replaceState({}, "", "/software_safe.html?test_api=1&connect=1&hosted_bootstrap=0"); installTestCrypto(); });
  it("signs a read-only factory, recipe, and batch quote request", async () => {
    const signSpy = vi.spyOn(window.crypto.subtle, "sign"); const { sentMessages, sockets } = installMockWebSocket();
    const core = await import("./legacy_core.js"); core.initializeSoftwareSafeCore();
    expect(await core.requestScheduleRecipeQuote("", "assemble_hardware", 1)).toEqual(expect.objectContaining({ ok: false }));
    sockets[0].open();
    core.state.auth = { ...core.state.auth, available: true, playerId: "player-schedule-quote", publicKey: "09".repeat(32), privateKey: "07".repeat(32), registrationStatus: "registered", runtimeStatus: "registered", boundAgentId: "agent-0" };
    expect(await window.__AW_TEST__.requestScheduleRecipeQuote("factory-0", "assemble_hardware", 2)).toEqual(expect.objectContaining({ ok: true, request: expect.objectContaining({ factory_id: "factory-0", recipe_id: "assemble_hardware", batches: 2 }) }));
    expect(core.state.scheduleRecipeQuoteRequest.status).toBe("pending");
    const message = sentMessages.find((entry) => entry.type === "quote_schedule_recipe");
    const actualSigningPayload = new Uint8Array(signSpy.mock.calls.at(-1)[2]);
    const expectedSigningPayload = buildAuthEnvelope({ operation: "gameplay_action", action_id: "quote_schedule_recipe", target_agent_id: "factory_id:factory-0|recipe_id:assemble_hardware|batches:2", player_id: message.request.auth.player_id, public_key: message.request.auth.public_key, nonce: message.request.auth.nonce });
    expect(Array.from(actualSigningPayload)).toEqual(Array.from(expectedSigningPayload));
  });
});
