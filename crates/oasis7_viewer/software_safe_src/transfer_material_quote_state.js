import { VISUAL_FIXTURE_NAME, visualFixtureQuote } from "./transfer_material_quote_state_visual_test_data.js";

export function createTransferMaterialQuoteStateModule({ clone, getSearchParams, isTestApiEnabled, render, state }) {
  function handleTransferMaterialQuote(quote, acceptUnsolicited = false) {
    if (!quote || typeof quote !== "object" || (!acceptUnsolicited && state.transferMaterialQuoteRequest?.status !== "pending")) return false;
    state.transferMaterialQuote = clone(quote);
    state.transferMaterialQuoteRequest = { status: "received", error: null };
    return true;
  }
  function handleTransferMaterialQuoteError(error) {
    if (String(error?.action_id || "").trim() !== "quote_transfer_material") return false;
    if (state.transferMaterialQuoteRequest?.status === "pending") state.transferMaterialQuoteRequest = { status: "error", error: String(error?.message || error?.code || "transfer material quote request failed") };
    return true;
  }
  function injectTransferMaterialQuoteForTest(quote) {
    if (!isTestApiEnabled()) throw new Error("injectTransferMaterialQuoteForTest requires test_api=1");
    handleTransferMaterialQuote(quote, true); render(); return clone(state.transferMaterialQuote);
  }
  function invalidateTransferMaterialQuote() {
    state.transferMaterialQuote = null;
    state.transferMaterialQuoteRequest = { status: "idle", error: null };
  }
  function installTransferMaterialQuoteVisualFixture() {
    if (!isTestApiEnabled() || getSearchParams().get("fixture") !== VISUAL_FIXTURE_NAME) return;
    handleTransferMaterialQuote(visualFixtureQuote, true);
  }
  return { handleTransferMaterialQuote, handleTransferMaterialQuoteError, injectTransferMaterialQuoteForTest, invalidateTransferMaterialQuote, installTransferMaterialQuoteVisualFixture };
}
