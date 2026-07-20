import { render, within } from "@solidjs/testing-library";
import { describe, expect, it } from "vitest";
import { RecoveryOptionComparisonPanel, recoveryOptionDisplayLabel } from "./recovery_option_comparison_panel.jsx";
import { recoveryOptionVisualFixture } from "./viewer_recovery_option_fixture.js";

const tr = (locale, zh, en) => locale === "zh" ? zh : en;

describe("recovery option comparison panel", () => {
  it("renders the deterministic fixture as labeled recovery cards", () => {
    const options = recoveryOptionVisualFixture().map((option) => ({
      kind: option.kind,
      timeClass: option.estimated_time_class,
      resourceClass: option.estimated_resource_class,
      riskClass: option.risk_class,
      retainedBenefit: option.retained_benefit,
      recommendationReason: option.recommendation_reason,
    }));
    const { getByTestId, getByText } = render(() => (
      <RecoveryOptionComparisonPanel continuation={{ recoveryOptionComparisons: options }} locale="en" tr={tr} />
    ));
    const list = getByTestId("viewer-recovery-options");
    const rebuild = list.querySelector('[data-recovery-kind="rebuild"]');

    expect(list.querySelectorAll('[data-testid="viewer-recovery-option"]')).toHaveLength(3);
    expect(within(rebuild).getByText("Time")).toBeInTheDocument();
    expect(within(rebuild).getByText("Resources")).toBeInTheDocument();
    expect(getByText("Rebuild")).toBeInTheDocument();
    expect(within(rebuild).getByText("Broader local reinvestment")).toBeInTheDocument();
    expect(within(rebuild).getByText("Risk")).toBeInTheDocument();
    expect(within(rebuild).getByText("Retains")).toBeInTheDocument();
    expect(within(rebuild).getByText("Why")).toBeInTheDocument();
  });

  it("localizes recovery enum values without changing their raw data selectors", () => {
    const option = {
      kind: "pivot",
      timeClass: "medium",
      resourceClass: "redirected_local_commitment",
      riskClass: "tradeoff",
      retainedBenefit: "保留独立进度。",
      recommendationReason: "转向本地路径。",
    };
    const { getByTestId, getByText } = render(() => (
      <RecoveryOptionComparisonPanel continuation={{ recoveryOptionComparisons: [option] }} locale="zh" tr={tr} />
    ));
    const pivot = getByTestId("viewer-recovery-option");

    expect(pivot).toHaveAttribute("data-recovery-kind", "pivot");
    expect(getByText("转向")).toBeInTheDocument();
    expect(within(pivot).getByText("中期")).toBeInTheDocument();
    expect(within(pivot).getByText("转向本地投入")).toBeInTheDocument();
    expect(within(pivot).getByText("权衡")).toBeInTheDocument();
  });

  it("humanizes unknown enum values instead of exposing raw snake case", () => {
    expect(recoveryOptionDisplayLabel("resource", "new_local_path", "en", tr)).toBe("New Local Path");
    expect(recoveryOptionDisplayLabel("risk", "future-risk", "zh", tr)).toBe("未知：Future Risk");
  });

  it("keeps the legacy recovery-options fallback when comparisons are absent", () => {
    const { getByText, queryByTestId } = render(() => (
      <RecoveryOptionComparisonPanel
        continuation={{ recoveryOptionComparisons: [], recoveryOptions: "repair: available" }}
        locale="en"
        tr={tr}
      />
    ));

    expect(getByText("Recovery Options")).toBeInTheDocument();
    expect(getByText("repair: available")).toBeInTheDocument();
    expect(queryByTestId("viewer-recovery-options")).toBeNull();
  });
});
