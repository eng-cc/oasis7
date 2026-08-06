export function createPowerSaleQuoteStateModule({ clone, isTestApiEnabled, render, state }) {
  function handlePowerSaleQuote(quote, acceptUnsolicited = false) {
    if (!quote || typeof quote !== "object" || (!acceptUnsolicited && state.powerSaleQuoteRequest?.status !== "pending")) return false;
    state.powerSaleQuote = clone(quote);
    state.powerSaleQuoteRequest = { status: "received", error: null };
    render();
    return true;
  }
  function handlePowerSaleQuoteError(error) {
    if (String(error?.action_id || "").trim() !== "quote_power_sale") return false;
    if (state.powerSaleQuoteRequest?.status === "pending") state.powerSaleQuoteRequest = { status: "error", error: String(error?.message || error?.code || "power sale quote request failed") };
    render();
    return true;
  }
  function invalidatePowerSaleQuote() {
    state.powerSaleQuote = null;
    state.powerSaleQuoteRequest = { status: "idle", error: null };
  }
  function injectPowerSaleQuoteForTest(quote) {
    if (!isTestApiEnabled()) throw new Error("injectPowerSaleQuoteForTest requires test_api=1");
    handlePowerSaleQuote(quote, true);
    return clone(state.powerSaleQuote);
  }
  return { handlePowerSaleQuote, handlePowerSaleQuoteError, injectPowerSaleQuoteForTest, invalidatePowerSaleQuote };
}
