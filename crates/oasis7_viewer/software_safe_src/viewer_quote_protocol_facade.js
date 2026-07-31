export function createViewerQuoteProtocolFacade({
  handleRefineQuoteError,
  handleRefineQuotePreflight,
  marketQuoteDecision,
  powerSurvivalQuote,
  productValidationQuote,
  state,
  warDeclarationQuote,
}) {
  function handleQuoteGameplayActionError(error) {
    return handleRefineQuoteError(error)
      || productValidationQuote.handleProductValidationQuoteError(error)
      || powerSurvivalQuote.handlePowerSurvivalQuoteError(error)
      || warDeclarationQuote.handleWarDeclarationQuoteError(error)
      || marketQuoteDecision.handleMarketQuoteDecisionError(error);
  }

  function handleQuoteViewerMessage(message) {
    switch (message?.type) {
      case "market_quote_decision_preflight":
        marketQuoteDecision.handleMarketQuoteDecision(message.quote);
        return true;
      case "refine_quote_preflight":
        handleRefineQuotePreflight(message.quote);
        return true;
      case "product_validation_quote_preflight":
        productValidationQuote.handleProductValidationQuote(message.quote);
        return true;
      case "power_survival_quote_preflight":
        powerSurvivalQuote.handlePowerSurvivalQuote(message.quote);
        return true;
      case "war_declaration_quote_preflight":
        warDeclarationQuote.handleWarDeclarationQuote(message.quote);
        return true;
      default:
        return false;
    }
  }

  function invalidateSnapshotBoundQuotes() {
    powerSurvivalQuote.invalidatePowerSurvivalQuote();
    state.marketQuoteDecision = null;
    state.marketQuoteDecisionRequest = { status: "idle", error: null };
    // A war quote retains its signed request identity across snapshots. Its
    // quoted tick/input comparison drives the visible stale state instead.
  }

  return { handleQuoteGameplayActionError, handleQuoteViewerMessage, invalidateSnapshotBoundQuotes };
}
