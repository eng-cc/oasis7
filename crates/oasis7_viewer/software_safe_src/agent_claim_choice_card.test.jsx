import { afterEach, describe, expect, it, vi } from "vitest";
import { within } from "@solidjs/testing-library";
import { renderViewerApp, slot1CandidateClaimSnapshot } from "./test_support/viewer_app_fixture.jsx";

vi.mock("./pixel_world_host.jsx", () => ({
  PixelWorldHost: () => <div data-testid="pixel-world-host" />,
}));

let dispose = null;

afterEach(() => {
  dispose?.();
  dispose = null;
});

describe("slot-1 claim choice card", () => {
  it("renders the localized English defer warning for exact-upfront zero-runway candidate facts", async () => {
    const app = await renderViewerApp(slot1CandidateClaimSnapshot());
    dispose = app.dispose;
    const stagePanel = app.container.querySelector("#viewer-stage-panel");

    expect(within(stagePanel).getByLabelText("Target Agent")).toHaveValue("agent-choice-target");
    expect(within(stagePanel).getByText("Slot-1 Candidate")).toBeInTheDocument();
    expect(within(stagePanel).getByText("agent-choice-target")).toBeInTheDocument();
    expect(within(stagePanel).getByText("120, -40, 5 cm")).toBeInTheDocument();
    expect(within(stagePanel).getByText("industrial_worker")).toBeInTheDocument();
    expect(within(stagePanel).getByText("light_frame")).toBeInTheDocument();
    expect(within(stagePanel).getByText("drill · scanner")).toBeInTheDocument();
    expect(within(stagePanel).getByText("Wait before confirming")).toBeInTheDocument();
    expect(within(stagePanel).getByText(/The 325 upfront cost is payable now, but confirmation leaves 0 full upkeep epochs/)).toBeInTheDocument();
    expect(within(stagePanel).getByText(/No canonical route rationale is published, so no candidate is recommended/)).toBeInTheDocument();
    expect(stagePanel.textContent).not.toContain("candidate_rationale_missing");
    expect(stagePanel.textContent).not.toContain("wait_or_fund_first");
  });

  it("renders the localized Chinese defer warning at the session boundary without exposing claim controls", async () => {
    const app = await renderViewerApp(slot1CandidateClaimSnapshot(), { boundAgentId: null, runtimeStatus: "registered_unbound" }, "zh");
    dispose = app.dispose;
    const stagePanel = app.container.querySelector("#viewer-stage-panel");

    expect(within(stagePanel).getByText("当前账号尚未绑定 Agent")).toBeInTheDocument();
    expect(within(stagePanel).getByText("首个候选 Agent")).toBeInTheDocument();
    expect(within(stagePanel).getByText("agent-choice-target")).toBeInTheDocument();
    expect(within(stagePanel).getByText("120, -40, 5 cm")).toBeInTheDocument();
    expect(within(stagePanel).getByText("暂不确认")).toBeInTheDocument();
    expect(within(stagePanel).getByText(/当前可支付 325 upfront，但确认后只能维持 0 个完整 upkeep epoch/)).toBeInTheDocument();
    expect(within(stagePanel).getByText(/尚未发布 canonical 路线理由，因此不推荐任何候选/)).toBeInTheDocument();
    expect(stagePanel.textContent).not.toContain("candidate_rationale_missing");
    expect(stagePanel.textContent).not.toContain("wait_or_fund_first");
    expect(within(stagePanel).queryByLabelText("Target Agent")).not.toBeInTheDocument();
    expect(within(stagePanel).queryByRole("button", { name: "Claim Agent" })).not.toBeInTheDocument();
  });
});
