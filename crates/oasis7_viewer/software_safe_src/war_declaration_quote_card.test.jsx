import { fireEvent, render, screen, within } from "@solidjs/testing-library";
import { describe, expect, it, vi } from "vitest";
import { WarDeclarationQuoteCard, WarDeclarationQuotePanel } from "./war_declaration_quote_card.jsx";

const quote = { actor_alliance_id: "alliance.red", target_alliance_id: "alliance.blue", intensity: 3, minimum_winning_intensity: 3, war_duration_ticks: 12, likely_winner_before_action: "alliance.red", victory_margin_estimate: 2, reentry_cooldown_or_active_conflict_blocker: "active war w-1 blocks either alliance until tick 40", settlement_risk: "loss can change resources and reputation", why_this_war_is_worth_or_risky: "wait for the active conflict to conclude", quoted_at_tick: 12, state_fingerprint: "fingerprint-12" };
const tr = (locale, zh, en) => locale === "zh" ? zh : en;

describe("WarDeclarationQuote", () => {
  it("renders the read-only quote and never enables a declaration submit action", () => {
    render(() => <WarDeclarationQuoteCard quote={quote} locale="en" tr={tr} />);
    const card = screen.getByTestId("war-declaration-quote");
    expect(card).toHaveAttribute("data-state-fingerprint", "fingerprint-12");
    expect(within(card).getByText("War Outcome Quote")).toBeInTheDocument();
    expect(within(card).getByTestId("war-declaration-blocker")).toHaveTextContent(/active war w-1/i);
    expect(within(card).getByTestId("war-declaration-submit-disabled")).toBeDisabled();
  });
  it("binds all inputs and warns when world time makes the fingerprinted quote stale", async () => {
    const request = vi.fn(async () => ({ ok: true }));
    render(() => <WarDeclarationQuotePanel quote={quote} requestState={{ status: "received" }} requestWarDeclarationQuote={request} logicalTime={13} locale="en" tr={tr} />);
    const panel = screen.getByTestId("war-declaration-quote-panel");
    expect(within(panel).getByTestId("war-declaration-quote-stale")).toHaveTextContent(/quote is stale/i);
    fireEvent.submit(within(panel).getByTestId("war-declaration-quote-request-form"));
    await vi.waitFor(() => expect(request).toHaveBeenCalledWith("alliance.red", "alliance.blue", "3"));
  });
  it("explains the M5-unavailable blocker", () => {
    render(() => <WarDeclarationQuotePanel quote={null} requestState={{ status: "error", error: "war_declaration_quote_unavailable" }} requestWarDeclarationQuote={vi.fn()} logicalTime={0} locale="en" tr={tr} />);
    expect(screen.getByTestId("war-declaration-unavailable")).toHaveTextContent(/M5 settlement path/i);
  });
});
