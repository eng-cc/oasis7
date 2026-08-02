import { render, screen, within } from "@solidjs/testing-library";
import { describe, expect, it } from "vitest";
import { WaitResolutionQuoteCard } from "./wait_resolution_quote_card.jsx";

const quote = {
  safe_to_wait: false,
  resolution_trigger: "A committed runtime event applies the queued smelter.",
  recheck_tick_or_event: "Recheck at event 8.",
  expected_change: "Smelter construction becomes visible.",
  unresolved_risk: "The action can still be blocked.",
  alternative_unlock_condition: "Refresh the snapshot and choose an enabled action.",
};

const tr = (_locale, _zh, en) => en;

describe("WaitResolutionQuoteCard", () => {
  it("renders an explicit unsafe wait verdict with five readable decision details and no action control", () => {
    render(() => <WaitResolutionQuoteCard quote={quote} locale="en" tr={tr} />);

    const card = screen.getByTestId("wait-resolution-quote");
    expect(within(card).getByText("Do not wait")).toBeInTheDocument();
    expect(within(card).getByText("Trigger")).toBeInTheDocument();
    expect(within(card).getByText("Recheck")).toBeInTheDocument();
    expect(within(card).getByText("Expected change")).toBeInTheDocument();
    expect(within(card).getByText("Unresolved risk")).toBeInTheDocument();
    expect(within(card).getByText("Alternative unlock")).toBeInTheDocument();
    expect(within(card).getByText(quote.resolution_trigger)).toBeInTheDocument();
    expect(within(card).getByText(quote.recheck_tick_or_event)).toBeInTheDocument();
    expect(within(card).getByText(quote.expected_change)).toBeInTheDocument();
    expect(within(card).getByText(quote.unresolved_risk)).toBeInTheDocument();
    expect(within(card).getByText(quote.alternative_unlock_condition)).toBeInTheDocument();
    expect(within(card).queryByRole("button")).not.toBeInTheDocument();
    expect(within(card).queryByText(/safe_to_wait|safe_wait|wait_for_resolution/i)).not.toBeInTheDocument();
  });

  it("does not render when the runtime supplied no wait-resolution quote", () => {
    render(() => <WaitResolutionQuoteCard quote={null} locale="en" tr={tr} />);

    expect(screen.queryByTestId("wait-resolution-quote")).not.toBeInTheDocument();
  });
});
