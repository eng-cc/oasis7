import { render, within } from "@solidjs/testing-library";
import { describe, expect, it } from "vitest";
import { RecoveryOptionComparisonPanel } from "./recovery_option_comparison_panel.jsx";
import { recoveryOptionVisualFixture } from "./viewer_recovery_option_fixture.js";

const tr = (_locale, _zh, en) => en;

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
    const { getByTestId } = render(() => (
      <RecoveryOptionComparisonPanel continuation={{ recoveryOptionComparisons: options }} locale="en" tr={tr} />
    ));
    const list = getByTestId("viewer-recovery-options");
    const rebuild = list.querySelector('[data-recovery-kind="rebuild"]');

    expect(list.querySelectorAll('[data-testid="viewer-recovery-option"]')).toHaveLength(3);
    expect(within(rebuild).getByText("Time")).toBeInTheDocument();
    expect(within(rebuild).getByText("Resources")).toBeInTheDocument();
    expect(within(rebuild).getByText("broader_local_reinvestment")).toBeInTheDocument();
    expect(within(rebuild).getByText("Risk")).toBeInTheDocument();
    expect(within(rebuild).getByText("Retains")).toBeInTheDocument();
    expect(within(rebuild).getByText("Why")).toBeInTheDocument();
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
