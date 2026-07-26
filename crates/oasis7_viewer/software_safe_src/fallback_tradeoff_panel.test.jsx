import { render, within } from "@solidjs/testing-library";
import { describe, expect, it } from "vitest";
import { FallbackTradeoffPanel } from "./fallback_tradeoff_panel.jsx";

const tr = (locale, zh, en) => locale === "zh" ? zh : en;

describe("fallback tradeoff panel", () => {
  it("keeps all published choices visible, localized, and non-actionable", () => {
    const options = [
      { valueClass: "safe_wait", available: false, reason: "No bounded trigger is published.", progressKept: "Keeps the current line.", cost: "One recheck.", opportunityCost: "Delays alternate output.", recommended: false },
      { valueClass: "repair_now", available: true, reason: "The blocker is repairable.", progressKept: "Keeps the current capability.", cost: "Spend repair materials.", opportunityCost: "Uses the repair reserve.", recommended: true },
      { valueClass: "reroute_now", available: false, reason: "No alternate route is available.", progressKept: "Keeps the goal.", cost: "Change route.", opportunityCost: "Loses near-term output.", recommended: false },
    ];
    const { getByTestId } = render(() => <FallbackTradeoffPanel options={options} locale="en" tr={tr} />);
    const panel = getByTestId("viewer-fallback-tradeoff");
    const cards = panel.querySelectorAll('[data-testid="viewer-fallback-tradeoff-option"]');

    expect(cards).toHaveLength(3);
    expect(within(cards[0]).getByText("Wait")).toBeInTheDocument();
    expect(within(cards[0]).getByText("Unavailable")).toBeInTheDocument();
    expect(within(cards[1]).getByText("Repair")).toBeInTheDocument();
    expect(within(cards[1]).getByText("Recommended")).toBeInTheDocument();
    expect(within(cards[2]).getByText("Reroute")).toBeInTheDocument();
    expect(panel).not.toHaveTextContent(/safe_wait|repair_now|reroute_now/);
    expect(panel.querySelectorAll("button")).toHaveLength(0);
  });

  it("renders a display-only handoff when runtime publishes no safe fallback", () => {
    const { getByTestId } = render(() => (
      <FallbackTradeoffPanel
        options={[]}
        noSafeFallbackHandoff={{
          reason: "Repair and reroute are unavailable.",
          requiredNextDecisionActionId: "select_new_goal",
          requiredNextDecisionClass: "goal_selection",
        }}
        locale="en"
        tr={tr}
      />
    ));
    const handoff = getByTestId("viewer-no-safe-fallback-handoff");

    expect(handoff).toHaveTextContent("No safe fallback");
    expect(handoff).toHaveTextContent("Repair and reroute are unavailable.");
    expect(handoff).toHaveTextContent("Required next decision");
    expect(handoff).toHaveTextContent("Select New Goal");
    expect(handoff).not.toHaveTextContent(/select_new_goal|goal_selection/);
    expect(handoff.querySelectorAll("button")).toHaveLength(0);
  });
});
