const VISUAL_FIXTURE_NAME = "transfer_material_quote";

const visualFixtureQuote = Object.freeze({
  requester_agent_id: "agent-0", from_ledger: "site:source", to_ledger: "site:destination", kind: "iron_ingot", requested_amount: 20,
  submission_feasible: true, max_transferable_amount: 40, sent_amount: 20, distance_km: 200, loss_bps: 5, expected_loss_amount: 2,
  expected_received_amount: 18, source_amount_before: 40, source_amount_after: 20, destination_amount_before: 0, destination_expected_amount_after: 18,
  ticks_until_arrival: 2, ready_at: 3, effective_priority: "standard", priority_reason: "material_default_priority", inflight_before: 0,
  inflight_capacity: 2, recommendation: "submit_transfer", conditional: true,
});

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
