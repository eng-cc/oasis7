import { VISUAL_FIXTURE_NAME, visualFixtureQuote } from "./schedule_recipe_quote_state_visual_test_data.js";

export function createScheduleRecipeQuoteStateModule({ clone, getSearchParams, isTestApiEnabled, render, state }) {
  function handleScheduleRecipeQuote(quote, acceptUnsolicited = false) {
    if (!quote || typeof quote !== "object" || (!acceptUnsolicited && state.scheduleRecipeQuoteRequest?.status !== "pending")) return false;
    state.scheduleRecipeQuote = clone(quote);
    state.scheduleRecipeQuoteRequest = { status: "received", error: null };
    return true;
  }
  function handleScheduleRecipeQuoteError(error) {
    if (String(error?.action_id || "").trim() !== "quote_schedule_recipe") return false;
    if (state.scheduleRecipeQuoteRequest?.status === "pending") state.scheduleRecipeQuoteRequest = { status: "error", error: String(error?.message || error?.code || "schedule recipe quote request failed") };
    return true;
  }
  function injectScheduleRecipeQuoteForTest(quote) {
    if (!isTestApiEnabled()) throw new Error("injectScheduleRecipeQuoteForTest requires test_api=1");
    handleScheduleRecipeQuote(quote, true); render(); return clone(state.scheduleRecipeQuote);
  }
  function invalidateScheduleRecipeQuote() {
    state.scheduleRecipeQuote = null;
    state.scheduleRecipeQuoteRequest = { status: "idle", error: null };
  }
  function installScheduleRecipeQuoteVisualFixture() {
    if (!isTestApiEnabled() || getSearchParams().get("fixture") !== VISUAL_FIXTURE_NAME) return;
    handleScheduleRecipeQuote(visualFixtureQuote, true);
  }
  return { handleScheduleRecipeQuote, handleScheduleRecipeQuoteError, injectScheduleRecipeQuoteForTest, invalidateScheduleRecipeQuote, installScheduleRecipeQuoteVisualFixture };
}
