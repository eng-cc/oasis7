import { fireEvent, render, screen, within } from "@solidjs/testing-library";
import { describe, expect, it, vi } from "vitest";
import { WarDeclarationQuoteCard, WarDeclarationQuotePanel } from "./war_declaration_quote_card.jsx";

const quote = { actor_alliance_id: "alliance.red", target_alliance_id: "alliance.blue", intensity: 3, settlement_path: "core_fallback", conflict_status: "active_conflict", minimum_winning_intensity: 3, war_duration_ticks: 12, aggressor_score_estimate: 32, defender_score_estimate: 30, likely_winner_before_action: "alliance.red", projected_outcome: "aggressor_wins", victory_margin_estimate: 2, conflict_window_blocked_until: 40, reentry_cooldown_or_active_conflict_blocker: "active war w-1 blocks either alliance until tick 40", settlement_risk: "loss can change resources and reputation", settlement_risk_code: "loss_resource_and_reputation", alternative_action: "wait", recommended_war_action: "wait", why_this_war_is_worth_or_risky: "wait for the active conflict to conclude", mobilization_electricity_required: 24, mobilization_electricity_current: 30, mobilization_electricity_after: 6, mobilization_data_required: 17, mobilization_data_current: 20, mobilization_data_after: 3, mobilization_affordable: true, quoted_at_tick: 12, state_fingerprint: "fingerprint-12" };
const tr = (locale, zh, en) => locale === "zh" ? zh : en;

describe("WarDeclarationQuote", () => {
  it("renders the read-only quote and never enables a declaration submit action", () => {
    render(() => <WarDeclarationQuoteCard quote={quote} locale="en" tr={tr} />);
    const card = screen.getByTestId("war-declaration-quote");
    expect(card).toHaveAttribute("data-state-fingerprint", "fingerprint-12");
    expect(card).toHaveAttribute("data-quote-fingerprint", "fingerprint-12");
    expect(within(card).getByText("War Outcome Quote")).toBeInTheDocument();
    expect(within(card).getByTestId("war-declaration-blocker")).toHaveTextContent(/active conflict is not settled/i);
    expect(within(card).getByTestId("war-declaration-blocker")).toHaveTextContent(/retry at 40 ticks/i);
    expect(within(card).getByTestId("war-declaration-submit-disabled")).toBeDisabled();
    expect(within(card).getByTestId("war-declaration-mobilization-electricity")).toHaveTextContent(/required 24/i);
  });
  it("maps stable protocol codes to Chinese player copy", () => {
    render(() => <WarDeclarationQuoteCard quote={{ ...quote, conflict_status: "pending_conflict", recommended_war_action: "gather_resources", mobilization_affordable: false }} locale="zh" tr={tr} />);
    expect(screen.getByTestId("war-declaration-blocker")).toHaveTextContent("已有宣战正在等待处理");
    expect(screen.getByTestId("war-declaration-blocker")).not.toHaveTextContent("可于");
    expect(screen.getByTestId("war-declaration-blocker")).not.toHaveTextContent("40");
    expect(screen.getByTestId("war-declaration-recommendation")).toHaveTextContent("先收集动员资源");
    expect(screen.queryByText("pending_conflict")).not.toBeInTheDocument();
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
    expect(screen.getByTestId("war-declaration-unavailable")).toHaveTextContent(/current settlement path/i);
  });
});
