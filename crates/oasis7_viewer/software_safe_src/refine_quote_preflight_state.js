const VISUAL_FIXTURE_NAME = "refine_quote_preflight";

const visualFixtureQuote = Object.freeze({
  owner_agent_id: "agent-0",
  compound_mass_g: 40,
  electricity_cost: 12,
  electricity_after: 88,
  hardware_output: 20,
  target_id: "factory_build_hardware",
  target_gap_before: 20,
  target_gap_after: 0,
  target_linkage: "enables_factory_build_hardware_goal",
  recommended_refine_amount: 40,
  value_classification: "enough_to_advance",
});

export function createRefineQuotePreflightStateModule({ clone, getSearchParams, isTestApiEnabled, render, state }) {
  function handleRefineQuotePreflight(quote) {
    if (!quote || typeof quote !== "object") return;
    // A quote is kept outside gameplay-action feedback so it cannot be presented
    // as an accepted action or receipt.
    state.refineQuotePreflight = clone(quote);
  }

  function injectRefineQuotePreflightForTest(quote) {
    if (!isTestApiEnabled()) {
      throw new Error("injectRefineQuotePreflightForTest requires test_api=1");
    }
    handleRefineQuotePreflight(quote);
    render();
    return clone(state.refineQuotePreflight);
  }

  function installRefineQuotePreflightVisualFixture() {
    if (!isTestApiEnabled() || getSearchParams().get("fixture") !== VISUAL_FIXTURE_NAME) return;
    handleRefineQuotePreflight(visualFixtureQuote);
  }

  return {
    handleRefineQuotePreflight,
    injectRefineQuotePreflightForTest,
    installRefineQuotePreflightVisualFixture,
  };
}
