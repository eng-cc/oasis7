import { describe, expect, it, vi } from "vitest";
import { createWarDeclarationQuoteIntegration } from "./war_declaration_quote_integration.js";

function createIntegration(state) {
  return createWarDeclarationQuoteIntegration({
    buildAuthEnvelope: vi.fn(), clone: (value) => JSON.parse(JSON.stringify(value)),
    ensureHostedPlayerAuthAvailable: vi.fn(), ensureRegisteredPlayerSession: vi.fn(),
    getSocket: vi.fn(), nextAuthNonce: vi.fn(), sendJson: vi.fn(), signAuthPayload: vi.fn(), state,
  });
}

describe("war declaration quote integration", () => {
  it("rejects a duplicate request before creating another signed websocket action", async () => {
    const state = { warDeclarationQuoteRequest: { status: "pending", requestKey: "alliance.red|alliance.blue|3" } };
    const integration = createIntegration(state);

    await expect(integration.requestWarDeclarationQuote("alliance.red", "alliance.blue", "3"))
      .resolves.toEqual({ ok: false, reason: "war quote request is already pending" });
  });

  it("accepts only the signed request's matching quote response", () => {
    const state = { warDeclarationQuoteRequest: { status: "pending", requestKey: "alliance.red|alliance.blue|3" }, warDeclarationQuote: null };
    const integration = createIntegration(state);

    expect(integration.handleWarDeclarationQuote({ actor_alliance_id: "alliance.red", target_alliance_id: "alliance.blue", intensity: 2 })).toBe(false);
    expect(integration.handleWarDeclarationQuote({ actor_alliance_id: "alliance.red", target_alliance_id: "alliance.blue", intensity: 3 })).toBe(true);
    expect(state.warDeclarationQuoteRequest).toEqual({ status: "received", error: null });
  });
});
