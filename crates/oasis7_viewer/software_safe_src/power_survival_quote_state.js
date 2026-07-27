const VISUAL_FIXTURE_NAME = "power_survival_quote";

const visualFixtureQuote = Object.freeze({
  buyer_agent_id: "agent-0", seller_agent_id: "agent-1", current_power_level: 2,
  power_state_before: "critical", recovery_action: "buy_power", recovery_amount: 18,
  power_gain_estimate: 18, requested_price_per_pu: 3, price_per_pu: 3, price_or_time_cost: 54,
  power_state_after_recovery: "low_power", survival_runway_ticks: 20,
  next_action_affordability_after_recovery: "limited",
  shutdown_avoidance_reason: "recovery restores 20 runway ticks and lifts agent from critical to low_power; recommended action: buy_power_partial",
  recommended_power_action: "buy_power_partial",
});

export function createPowerSurvivalQuoteStateModule({ clone, getSearchParams, isTestApiEnabled, render, state }) {
  function handlePowerSurvivalQuote(quote, acceptUnsolicited = false) {
    if (!quote || typeof quote !== "object" || (!acceptUnsolicited && state.powerSurvivalQuoteRequest?.status !== "pending")) return false;
    state.powerSurvivalQuote = clone(quote);
    state.powerSurvivalQuoteRequest = { status: "received", error: null };
    return true;
  }
  function handlePowerSurvivalQuoteError(error) {
    if (String(error?.action_id || "").trim() !== "quote_power_survival") return false;
    if (state.powerSurvivalQuoteRequest?.status === "pending") {
      state.powerSurvivalQuoteRequest = { status: "error", error: String(error?.message || error?.code || "power survival quote request failed") };
    }
    return true;
  }
  function injectPowerSurvivalQuoteForTest(quote) {
    if (!isTestApiEnabled()) throw new Error("injectPowerSurvivalQuoteForTest requires test_api=1");
    handlePowerSurvivalQuote(quote, true); render(); return clone(state.powerSurvivalQuote);
  }
  function invalidatePowerSurvivalQuote() {
    state.powerSurvivalQuote = null;
    state.powerSurvivalQuoteRequest = { status: "idle", error: null };
  }
  function installPowerSurvivalQuoteVisualFixture() {
    if (!isTestApiEnabled() || getSearchParams().get("fixture") !== VISUAL_FIXTURE_NAME) return;
    handlePowerSurvivalQuote(visualFixtureQuote, true);
  }
  return { handlePowerSurvivalQuote, handlePowerSurvivalQuoteError, injectPowerSurvivalQuoteForTest, installPowerSurvivalQuoteVisualFixture, invalidatePowerSurvivalQuote };
}
