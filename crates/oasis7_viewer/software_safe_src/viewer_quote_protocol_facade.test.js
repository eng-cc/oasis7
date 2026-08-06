import { describe, expect, it, vi } from "vitest";
import { createViewerQuoteProtocolFacade } from "./viewer_quote_protocol_facade.js";

function integration() {
  return {
    handleMarketQuoteDecision: vi.fn(), handleMarketQuoteDecisionError: vi.fn(() => false),
    handlePowerSaleQuote: vi.fn(), handlePowerSaleQuoteError: vi.fn(() => false),
    handlePowerSurvivalQuote: vi.fn(), handlePowerSurvivalQuoteError: vi.fn(() => false),
    handleProductValidationQuote: vi.fn(), handleProductValidationQuoteError: vi.fn(() => false),
    handleFragmentRefillPreview: vi.fn(), handleFragmentRefillPreviewError: vi.fn(() => false),
    handleWarDeclarationQuote: vi.fn(), handleWarDeclarationQuoteError: vi.fn(() => false),
    invalidateWarDeclarationQuoteForAuthoritativeSnapshot: vi.fn(),
    invalidateFragmentRefillPreview: vi.fn(),
    invalidatePowerSurvivalQuote: vi.fn(),
  };
}

describe("viewer quote protocol facade", () => {
  it("keeps an in-flight war quote across an unrelated snapshot while clearing snapshot-bound quotes", () => {
    const state = {
      marketQuoteDecision: { value: "old" }, marketQuoteDecisionRequest: { status: "received" },
      warDeclarationQuote: { quoted_at_tick: 12 }, warDeclarationQuoteRequest: { status: "pending", requestKey: "red|blue|3" },
    };
    const powerSurvivalQuote = integration();
    const fragmentRefillPreview = integration();
    const facade = createViewerQuoteProtocolFacade({
      fragmentRefillPreview,
      handleRefineQuoteError: vi.fn(() => false), handleRefineQuotePreflight: vi.fn(),
      marketQuoteDecision: integration(), powerSurvivalQuote, productValidationQuote: integration(), state,
      warDeclarationQuote: integration(),
    });

    facade.invalidateSnapshotBoundQuotes();

    expect(powerSurvivalQuote.invalidatePowerSurvivalQuote).toHaveBeenCalledOnce();
    expect(fragmentRefillPreview.invalidateFragmentRefillPreview).toHaveBeenCalledOnce();
    expect(state.marketQuoteDecision).toBeNull();
    expect(state.marketQuoteDecisionRequest).toEqual({ status: "idle", error: null });
    expect(state.warDeclarationQuoteRequest).toEqual({ status: "pending", requestKey: "red|blue|3" });
    expect(state.warDeclarationQuote).toEqual({ quoted_at_tick: 12 });
  });

  it("routes war quote replies through the extracted protocol boundary", () => {
    const warDeclarationQuote = integration();
    const facade = createViewerQuoteProtocolFacade({
      fragmentRefillPreview: integration(),
      handleRefineQuoteError: vi.fn(() => false), handleRefineQuotePreflight: vi.fn(),
      marketQuoteDecision: integration(), powerSurvivalQuote: integration(), productValidationQuote: integration(),
      state: {}, warDeclarationQuote,
    });
    const quote = { actor_alliance_id: "alliance.red" };

    expect(facade.handleQuoteViewerMessage({ type: "war_declaration_quote_preflight", quote })).toBe(true);
    expect(warDeclarationQuote.handleWarDeclarationQuote).toHaveBeenCalledWith(quote);
  });

  it("routes a power sale preflight through the seller-side quote boundary", () => {
    const powerSaleQuote = integration();
    const facade = createViewerQuoteProtocolFacade({
      fragmentRefillPreview: integration(),
      handleRefineQuoteError: vi.fn(() => false), handleRefineQuotePreflight: vi.fn(),
      marketQuoteDecision: integration(), powerSaleQuote, powerSurvivalQuote: integration(), productValidationQuote: integration(),
      state: {}, warDeclarationQuote: integration(),
    });
    const quote = { seller_agent_id: "agent-seller", buyer_agent_id: "agent-buyer", production_interrupt_risk: true };

    expect(facade.handleQuoteViewerMessage({ type: "power_sale_quote_preflight", quote })).toBe(true);
    expect(powerSaleQuote.handlePowerSaleQuote).toHaveBeenCalledWith(quote);
  });

  it("invalidates a received quote when an authoritative snapshot changes at the same tick", () => {
    const state = {
      warDeclarationQuote: { quoted_at_tick: 12, state_fingerprint: "before" },
      warDeclarationQuoteRequest: { status: "received", error: null },
    };
    const warDeclarationQuote = integration();
    const fragmentRefillPreview = integration();
    const facade = createViewerQuoteProtocolFacade({
      fragmentRefillPreview,
      handleRefineQuoteError: vi.fn(() => false), handleRefineQuotePreflight: vi.fn(),
      marketQuoteDecision: integration(), powerSurvivalQuote: integration(), productValidationQuote: integration(), state,
      warDeclarationQuote,
    });

    facade.invalidateSnapshotBoundQuotes();

    expect(fragmentRefillPreview.invalidateFragmentRefillPreview).toHaveBeenCalledOnce();
    expect(warDeclarationQuote.invalidateWarDeclarationQuoteForAuthoritativeSnapshot).toHaveBeenCalledOnce();
  });
});
