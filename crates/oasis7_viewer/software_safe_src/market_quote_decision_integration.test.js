import { describe, expect, it, vi } from "vitest";
import { createMarketQuoteDecisionIntegration } from "./market_quote_decision_integration.js";

function createSubject() {
  const state = { auth: { available: true, playerId: "player-1", publicKey: "pk", boundAgentId: "agent-0" }, marketQuoteDecision: null, marketQuoteDecisionRequest: { status: "idle", error: null } };
  const sendJson = vi.fn();
  const subject = createMarketQuoteDecisionIntegration({ buildAuthEnvelope: (payload) => payload, clone: (value) => structuredClone(value), ensureHostedPlayerAuthAvailable: vi.fn(), ensureRegisteredPlayerSession: vi.fn(), getSocket: () => ({ readyState: WebSocket.OPEN }), nextAuthNonce: () => 9, sendJson, signAuthPayload: vi.fn(async () => "signature"), state });
  return { state, sendJson, subject };
}

describe("market quote decision integration", () => {
  it("binds each material contribution in a signed read-only request", async () => {
    const { state, sendJson, subject } = createSubject();
    await expect(subject.requestMarketQuoteDecision([{ material: "iron_ingot", amount: "4" }])).resolves.toMatchObject({ ok: true, request: { consume: [{ material: "iron_ingot", amount: 4 }] } });
    expect(sendJson).toHaveBeenCalledWith(expect.objectContaining({ type: "quote_market_decision", request: expect.objectContaining({ auth: expect.objectContaining({ nonce: 9 }) }) }));
    expect(state.marketQuoteDecisionRequest.status).toBe("pending");
  });

  it("stores only recognized preflight responses and exposes a player-safe error state", () => {
    const { state, subject } = createSubject();
    expect(subject.handleMarketQuoteDecision({ recommendation: "Submit with local materials", contributions: [] })).toBe(true);
    expect(state.marketQuoteDecisionRequest.status).toBe("received");
    expect(subject.handleMarketQuoteDecisionError({ action_id: "other", message: "ignored" })).toBe(false);
    expect(subject.handleMarketQuoteDecisionError({ action_id: "quote_market_decision", message: "request rejected" })).toBe(true);
    expect(state.marketQuoteDecisionRequest).toEqual({ status: "error", error: "request rejected" });
  });
});
