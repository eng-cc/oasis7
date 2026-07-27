import { fireEvent, render, screen, within } from "@solidjs/testing-library";
import { describe, expect, it, vi } from "vitest";
import { PowerSurvivalQuoteCard, PowerSurvivalQuotePanel } from "./power_survival_quote_card.jsx";

const quote = {
  buyer_agent_id: "agent-0", seller_agent_id: "agent-1", current_power_level: 0,
  power_state_before: "shutdown", recovery_action: "buy_power", recovery_amount: 18,
  power_gain_estimate: 18, requested_price_per_pu: 3, price_per_pu: 3, price_or_time_cost: 54,
  power_state_after_recovery: "low_power", survival_runway_ticks: 18,
  next_action_affordability_after_recovery: "limited", shutdown_avoidance_reason: "internal-only",
  recommended_power_action: "buy_power_partial",
};
const tr = (locale, zh, en) => locale === "zh" ? zh : en;

describe("PowerSurvivalQuote", () => {
  it("renders a localized, seller/amount/price-bound read-only survival quote without raw enums", () => {
    render(() => <PowerSurvivalQuoteCard quote={quote} locale="en" tr={tr} />);
    const card = screen.getByTestId("power-survival-quote");
    expect(card).toHaveAttribute("data-seller-agent-id", "agent-1");
    expect(card).toHaveAttribute("data-amount", "18");
    expect(card).toHaveAttribute("data-requested-price-per-pu", "3");
    expect(within(card).getByText("Power Recovery Quote")).toBeInTheDocument();
    expect(within(card).getByText("Expected gain")).toBeInTheDocument();
    expect(within(card).getByText("Estimated cost")).toBeInTheDocument();
    expect(within(card).getByText("Shutdown → Low power")).toBeInTheDocument();
    expect(within(card).getByText("Next action limited")).toBeInTheDocument();
    expect(within(card).getByTestId("power-survival-shutdown-avoidance")).toHaveTextContent(/bring the Agent out of shutdown/i);
    expect(within(card).getByTestId("power-survival-recommendation")).toHaveTextContent(/Buy more power before acting/i);
    expect(within(card).queryByText("buy_power_partial")).not.toBeInTheDocument();
    expect(within(card).queryByText("low_power")).not.toBeInTheDocument();
    expect(within(card).queryByText("internal-only")).not.toBeInTheDocument();
    expect(within(card).queryByRole("button", { name: /buy|submit|commit/i })).not.toBeInTheDocument();
  });

  it("requests the signed quote with all three bound inputs and marks an old quote stale", async () => {
    const requestPowerSurvivalQuote = vi.fn(async () => ({ ok: true }));
    const firstRender = render(() => <PowerSurvivalQuotePanel quote={null} requestPowerSurvivalQuote={requestPowerSurvivalQuote} locale="en" tr={tr} />);
    const panel = screen.getByTestId("power-survival-quote-panel");
    fireEvent.input(within(panel).getByRole("textbox", { name: "Seller Agent" }), { target: { value: "agent-9" } });
    fireEvent.input(within(panel).getByRole("spinbutton", { name: "Power amount" }), { target: { value: "24" } });
    fireEvent.input(within(panel).getByRole("spinbutton", { name: "Price per unit" }), { target: { value: "4" } });
    fireEvent.submit(within(panel).getByTestId("power-survival-quote-request-form"));
    await vi.waitFor(() => expect(requestPowerSurvivalQuote).toHaveBeenCalledWith("agent-9", "24", "4"));
    expect(within(panel).queryByTestId("power-survival-quote")).not.toBeInTheDocument();

    firstRender.unmount();
    render(() => <PowerSurvivalQuotePanel quote={{ ...quote, requested_price_per_pu: 0 }} requestState={{ status: "received" }} requestPowerSurvivalQuote={requestPowerSurvivalQuote} locale="en" tr={tr} />);
    expect(screen.getByRole("status")).toHaveTextContent(/Quote received/i);
    fireEvent.input(screen.getByRole("spinbutton", { name: "Power amount" }), { target: { value: "25" } });
    expect(screen.getByTestId("power-survival-quote-stale")).toHaveTextContent(/quote is stale/i);
  });
});
