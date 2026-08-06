import { fireEvent, render, screen, within } from "@solidjs/testing-library";
import { describe, expect, it, vi } from "vitest";
import { PowerSaleQuoteCard, PowerSaleQuotePanel } from "./power_sale_quote_card.jsx";

const quote = {
  seller_agent_id: "agent-seller", buyer_agent_id: "agent-buyer", current_power_level: 15,
  power_state_before: "low_power", sale_amount: 10, price_per_pu: 3, expected_revenue: 30,
  power_state_after_sale: "critical", remaining_runway_ticks: 5,
  next_action_affordability_after_sale: "limited", production_interrupt_risk: true,
  recommended_sale_action: "defer_sale", why_sale_is_safe_or_risky: "critical power runway",
};
const tr = (locale, zh, en) => locale === "zh" ? zh : en;

describe("PowerSaleQuote", () => {
  it("renders the dangerous sale as a localized read-only preflight without raw protocol enums", () => {
    const englishRender = render(() => <PowerSaleQuoteCard quote={quote} locale="en" tr={tr} />);
    const card = screen.getByTestId("power-sale-quote");
    expect(card).toHaveAttribute("data-seller-agent-id", "agent-seller");
    expect(card).toHaveAttribute("data-buyer-agent-id", "agent-buyer");
    expect(card).toHaveAttribute("data-amount", "10");
    expect(card).toHaveAttribute("data-requested-price-per-pu", "3");
    expect(within(card).getByText("Power Sale Quote")).toBeInTheDocument();
    expect(within(card).getByText("Expected revenue")).toBeInTheDocument();
    expect(within(card).getByText("Low power → Critical power")).toBeInTheDocument();
    expect(within(card).getByText("5 ticks")).toBeInTheDocument();
    expect(within(card).getByText("Next action limited")).toBeInTheDocument();
    expect(within(card).getByTestId("power-sale-production-risk")).toHaveTextContent(/interrupt production/i);
    expect(within(card).getByTestId("power-sale-recommendation")).toHaveTextContent(/defer this sale/i);
    expect(within(card).queryByText("low_power")).not.toBeInTheDocument();
    expect(within(card).queryByText("critical")).not.toBeInTheDocument();
    expect(within(card).queryByText("defer_sale")).not.toBeInTheDocument();
    expect(within(card).queryByText(quote.why_sale_is_safe_or_risky)).not.toBeInTheDocument();
    expect(within(card).queryByRole("button", { name: /sell|submit|commit/i })).not.toBeInTheDocument();

    englishRender.unmount();
    render(() => <PowerSaleQuoteCard quote={quote} locale="zh" tr={tr} />);
    expect(screen.getByText("预计收入")).toBeInTheDocument();
    expect(screen.queryByText("Expected revenue")).not.toBeInTheDocument();
  });

  it("binds the buyer, amount, and price before requesting the read-only sale quote", async () => {
    const requestPowerSaleQuote = vi.fn(async () => ({ ok: true }));
    render(() => <PowerSaleQuotePanel quote={null} requestPowerSaleQuote={requestPowerSaleQuote} locale="en" tr={tr} />);
    const panel = screen.getByTestId("power-sale-quote-panel");
    fireEvent.input(within(panel).getByRole("textbox", { name: "Buyer Agent" }), { target: { value: "agent-buyer" } });
    fireEvent.input(within(panel).getByRole("spinbutton", { name: "Power amount" }), { target: { value: "10" } });
    fireEvent.input(within(panel).getByRole("spinbutton", { name: "Price per unit" }), { target: { value: "3" } });
    fireEvent.submit(within(panel).getByTestId("power-sale-quote-request-form"));
    await vi.waitFor(() => expect(requestPowerSaleQuote).toHaveBeenCalledWith("agent-buyer", "10", "3"));
    expect(within(panel).queryByTestId("power-sale-quote")).not.toBeInTheDocument();
  });
});
