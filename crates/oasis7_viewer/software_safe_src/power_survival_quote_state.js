import { VISUAL_FIXTURE_NAME, visualFixtureQuote } from "./power_survival_quote_state_visual_test_data.js";

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
