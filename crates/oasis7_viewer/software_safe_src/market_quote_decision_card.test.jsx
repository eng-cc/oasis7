import { render, screen, within } from "@solidjs/testing-library";
import { describe, expect, it, vi } from "vitest";
import { MarketQuoteDecisionCard, MarketQuoteDecisionPanel } from "./market_quote_decision_card.jsx";

const tr = (_locale, zh, en) => en;
const quote = { contributions: [{ material: "Iron ingot", requested_amount: 4, local_available_amount: 1, world_cover_amount: 2, shortfall_amount: 1, transit_loss_bps: 20, governance_tax_bps: 100, effective_cost_index_ppm: 1002000 }], total_shortfall_amount: 1, submission_allowed: false, conditional_notice: "This is a conditional preview.", recommendation: "Reduce the request or obtain more materials", rationale: "Available local and world materials do not cover this request.", next_action: "Reduce requested amounts or source the missing materials" };

describe("MarketQuoteDecision", () => {
  it("renders readable contribution, conditional rationale, and next action without runtime tokens", () => {
    render(() => <MarketQuoteDecisionCard quote={quote} locale="en" tr={tr} />);
    const card = screen.getByTestId("market-quote-decision");
    expect(within(card).getByTestId("market-quote-contribution")).toHaveTextContent("Iron ingot");
    expect(card).toHaveTextContent("Governance tax 100 bps");
    expect(card).toHaveTextContent("Cost index 1002000 ppm");
    expect(card).toHaveTextContent("Reduce requested amounts or source the missing materials");
    expect(card).not.toHaveTextContent("reduce_or_source_materials");
  });

  it("submits the bounded material input through the supplied request function", async () => {
    const requestMarketQuoteDecision = vi.fn(async () => ({ ok: true }));
    render(() => <MarketQuoteDecisionPanel quote={null} requestState={{ status: "idle" }} requestMarketQuoteDecision={requestMarketQuoteDecision} locale="en" tr={tr} />);
    await screen.getByRole("button", { name: "Request market preview" }).click();
    expect(requestMarketQuoteDecision).toHaveBeenCalledWith([{ material: "iron_ingot", amount: "4" }]);
  });

  it("shows a localized safe error rather than protocol diagnostics", () => {
    render(() => <MarketQuoteDecisionPanel quote={null} requestState={{ status: "error", error: "market_quote_rejected" }} requestMarketQuoteDecision={vi.fn()} locale="en" tr={tr} />);
    expect(screen.getByRole("alert")).toHaveTextContent("Could not get the market preview");
    expect(screen.getByRole("alert")).not.toHaveTextContent("market_quote_rejected");
  });
});
