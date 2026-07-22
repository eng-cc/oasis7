import { render, screen, within } from "@solidjs/testing-library";
import { describe, expect, it } from "vitest";
import { RefineQuotePreflightCard } from "./refine_quote_preflight_card.jsx";

const quote = {
  owner_agent_id: "agent-0",
  compound_mass_g: 40,
  electricity_cost: 12,
  electricity_after: 88,
  hardware_output: 20,
  target_id: "factory_build_hardware",
  target_gap_before: 20,
  target_gap_after: 0,
  target_linkage: "enables_factory_build_hardware_goal",
  recommended_refine_amount: 40,
  value_classification: "enough_to_advance",
};

function tr(locale, zh, en) {
  return locale === "zh" ? zh : en;
}

describe("RefineQuotePreflightCard", () => {
  it("renders a localized read-only quote without presenting a receipt or submit action", () => {
    const { unmount } = render(() => <RefineQuotePreflightCard quote={quote} locale="en" tr={tr} />);
    const card = screen.getByTestId("refine-quote-preflight");

    expect(card).toHaveAttribute("data-quote-kind", "preflight");
    expect(within(card).getByText("Compound Refining Quote")).toBeInTheDocument();
    expect(within(card).getByText(/read-only quote\. It does not submit refining/i)).toBeInTheDocument();
    expect(within(card).getByText("Electricity cost")).toBeInTheDocument();
    expect(within(card).getByText("Electricity remaining")).toBeInTheDocument();
    expect(within(card).getByText("Hardware output")).toBeInTheDocument();
    expect(within(card).getByText("20 → 0")).toBeInTheDocument();
    expect(within(card).getByText(/This output satisfies the factory hardware target/i)).toBeInTheDocument();
    expect(within(card).getByText(/Value assessment: Enough to advance/i)).toBeInTheDocument();
    expect(within(card).queryByRole("button", { name: /refin|submit|commit/i })).not.toBeInTheDocument();

    unmount();
    render(() => <RefineQuotePreflightCard quote={quote} locale="zh" tr={tr} />);
    const zhCard = screen.getByTestId("refine-quote-preflight");
    expect(within(zhCard).getByText("化合物精炼预估")).toBeInTheDocument();
    expect(within(zhCard).getByText(/这是只读预估，不会提交精炼、扣除电力或生成回执/)).toBeInTheDocument();
    expect(within(zhCard).getByText("电力成本")).toBeInTheDocument();
    expect(within(zhCard).getByText(/目标关联: 本次产出可满足工厂硬件目标/)).toBeInTheDocument();
    expect(within(zhCard).getByText(/价值判断: 足以推进下一步/)).toBeInTheDocument();
    expect(within(zhCard).queryByText("enough_to_advance")).not.toBeInTheDocument();
    expect(within(zhCard).queryByText("enables_factory_build_hardware_goal")).not.toBeInTheDocument();
  });
});
