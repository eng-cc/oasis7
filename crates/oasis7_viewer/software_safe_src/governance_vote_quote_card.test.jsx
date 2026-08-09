import { render, screen, within } from "@solidjs/testing-library";
import { describe, expect, it } from "vitest";
import { GovernanceVoteQuoteCard } from "./governance_vote_quote_card.jsx";

const tr = (locale, zh, en) => locale === "zh" ? zh : en;
const quote = {
  proposal_topic: "Keep the solar reserve",
  ticks_remaining: 12,
  current_quorum_weight: 0,
  required_quorum_weight: 3,
  current_pass_bps: 0,
  required_pass_bps: 6000,
  actor_vote_weight: 3,
  vote_swing_potential: 3,
  likely_outcome_after_action: "passed",
  world_change_if_passed: "Prioritize the solar reserve over an emergency drawdown.",
  cost_or_cooldown_if_failed: "No governance action cost or cooldown is defined for this proposal.",
  recommended_governance_action: "cast_vote",
  why_this_vote_matters: "This vote changes the likely outcome.",
};

describe("GovernanceVoteQuoteCard", () => {
  it("localizes the recommended governance action without leaking its canonical token", () => {
    const englishRender = render(() => <GovernanceVoteQuoteCard quote={quote} locale="en" tr={tr} />);
    const englishCard = screen.getByTestId("governance-vote-quote");

    expect(within(englishCard).getByText("Cast vote")).toBeInTheDocument();
    expect(within(englishCard).queryByText("cast_vote")).not.toBeInTheDocument();

    englishRender.unmount();
    render(() => <GovernanceVoteQuoteCard quote={quote} locale="zh" tr={tr} />);
    const chineseCard = screen.getByTestId("governance-vote-quote");

    expect(within(chineseCard).getByText("投票")).toBeInTheDocument();
    expect(within(chineseCard).queryByText("cast_vote")).not.toBeInTheDocument();
  });
});
