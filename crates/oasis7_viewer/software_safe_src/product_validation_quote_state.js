const VISUAL_FIXTURE_NAME = "product_validation_quote";

const visualFixtureQuote = Object.freeze({
  product_id: "logistics_drone",
  product_role: "explore",
  tradable: true,
  stage_before: "bootstrap",
  stage_after: "bootstrap",
  unlock_or_value_class: "scale_out",
  recommended_action: "advance_industry_stage",
  submission_allowed: true,
  missing_prerequisite: "industry_stage=scale_out",
  reachable_advance_or_recovery: "complete_reachable_industry_progress",
});

export function createProductValidationQuoteStateModule({ clone, getSearchParams, isTestApiEnabled, render, state }) {
  function handleProductValidationQuote(quote) {
    if (!quote || typeof quote !== "object") return;
    state.productValidationQuote = clone(quote);
    state.productValidationQuoteRequest = { status: "received", error: null };
  }

  function handleProductValidationQuoteError(error) {
    if (String(error?.action_id || "").trim() !== "quote_validate_product") return false;
    state.productValidationQuoteRequest = {
      status: "error",
      error: String(error?.message || error?.code || "product validation quote request failed"),
    };
    return true;
  }

  function injectProductValidationQuoteForTest(quote) {
    if (!isTestApiEnabled()) {
      throw new Error("injectProductValidationQuoteForTest requires test_api=1");
    }
    handleProductValidationQuote(quote);
    render();
    return clone(state.productValidationQuote);
  }

  function installProductValidationQuoteVisualFixture() {
    if (!isTestApiEnabled() || getSearchParams().get("fixture") !== VISUAL_FIXTURE_NAME) return;
    handleProductValidationQuote(visualFixtureQuote);
  }

  return {
    handleProductValidationQuote,
    handleProductValidationQuoteError,
    injectProductValidationQuoteForTest,
    installProductValidationQuoteVisualFixture,
  };
}
