import { beforeEach, describe, expect, it, vi } from "vitest";
import { buildAuthEnvelope } from "./viewer_auth_crypto.js";

function installMockWebSocket() {
  const sentMessages = []; const sockets = [];
  class MockWebSocket { static OPEN = 1; static CONNECTING = 0; constructor() { this.readyState = MockWebSocket.CONNECTING; this.listeners = new Map(); sockets.push(this); } addEventListener(type, listener) { this.listeners.set(type, [...(this.listeners.get(type) || []), listener]); } send(payload) { sentMessages.push(JSON.parse(payload)); } close() { this.readyState = 3; } open() { this.readyState = MockWebSocket.OPEN; for (const listener of this.listeners.get("open") || []) listener({}); } }
  Object.defineProperty(window, "WebSocket", { configurable: true, value: MockWebSocket }); return { sentMessages, sockets };
}
function installTestCrypto() { Object.defineProperty(window, "crypto", { configurable: true, value: { subtle: { async importKey() { return {}; }, async sign() { return new Uint8Array(64).fill(7).buffer; } } } }); }

describe("requestTransferMaterialQuote", () => {
  beforeEach(() => { vi.resetModules(); window.history.replaceState({}, "", "/software_safe.html?test_api=1&connect=1&hosted_bootstrap=0"); installTestCrypto(); });
  it("binds every logistics parameter in the signed read-only request", async () => {
    const signSpy = vi.spyOn(window.crypto.subtle, "sign"); const { sentMessages, sockets } = installMockWebSocket();
    const core = await import("./legacy_core.js"); core.initializeSoftwareSafeCore(); sockets[0].open();
    core.state.auth = { ...core.state.auth, available: true, playerId: "player-transfer-quote", publicKey: "09".repeat(32), privateKey: "07".repeat(32), registrationStatus: "registered", runtimeStatus: "registered", boundAgentId: "agent-0" };
    expect(await window.__AW_TEST__.requestTransferMaterialQuote("agent-0", "site:source", "site:destination", "iron_ingot", 20, 200, "standard")).toEqual(expect.objectContaining({ ok: true }));
    const message = sentMessages.find((entry) => entry.type === "quote_transfer_material");
    expect(message.request.requested_priority).toBe("standard");
    const actualSigningPayload = new Uint8Array(signSpy.mock.calls.at(-1)[2]);
    const expectedSigningPayload = buildAuthEnvelope({ operation: "gameplay_action", action_id: "quote_transfer_material", target_agent_id: JSON.stringify({ amount: 20, distance_km: 200, from_ledger: "site:source", kind: "iron_ingot", requested_priority: "standard", requester_agent_id: "agent-0", to_ledger: "site:destination" }), player_id: message.request.auth.player_id, public_key: message.request.auth.public_key, nonce: message.request.auth.nonce });
    expect(Array.from(actualSigningPayload)).toEqual(Array.from(expectedSigningPayload));
  });

  it("binds optional ordered route IDs and auto-reroute in the signed request and payload", async () => {
    const signSpy = vi.spyOn(window.crypto.subtle, "sign"); const { sentMessages, sockets } = installMockWebSocket();
    const core = await import("./legacy_core.js"); core.initializeSoftwareSafeCore(); sockets[0].open();
    core.state.auth = { ...core.state.auth, available: true, playerId: "player-transfer-route-quote", publicKey: "09".repeat(32), privateKey: "07".repeat(32), registrationStatus: "registered", runtimeStatus: "registered", boundAgentId: "agent-0" };
    expect(await window.__AW_TEST__.requestTransferMaterialQuote("agent-0", "site:source", "site:destination", "iron_ingot", 20, 200, "standard", ["route:source-relay", "route:relay-destination"], true)).toEqual(expect.objectContaining({ ok: true }));
    const message = sentMessages.find((entry) => entry.type === "quote_transfer_material");
    expect(message.request.route_ids).toEqual(["route:source-relay", "route:relay-destination"]);
    expect(message.request.auto_reroute).toBe(true);
    const actualSigningPayload = new Uint8Array(signSpy.mock.calls.at(-1)[2]);
    const expectedSigningPayload = buildAuthEnvelope({ operation: "gameplay_action", action_id: "quote_transfer_material", target_agent_id: JSON.stringify({ amount: 20, auto_reroute: true, distance_km: 200, from_ledger: "site:source", kind: "iron_ingot", requested_priority: "standard", requester_agent_id: "agent-0", route_ids: ["route:source-relay", "route:relay-destination"], to_ledger: "site:destination" }), player_id: message.request.auth.player_id, public_key: message.request.auth.public_key, nonce: message.request.auth.nonce });
    expect(Array.from(actualSigningPayload)).toEqual(Array.from(expectedSigningPayload));
  });
});
