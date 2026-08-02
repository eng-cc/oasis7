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
  it("binds the candidate target and exposes factual fallback without a compare recommendation", async () => {
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
    expect(within(stagePanel).getByText("Candidate rationale missing")).toBeInTheDocument();
    expect(within(stagePanel).getByText("Wait or fund first")).toBeInTheDocument();
    expect(within(stagePanel).queryByText("Compare candidates first")).not.toBeInTheDocument();
  });

  it("keeps published candidate facts visible at the session boundary without exposing claim controls", async () => {
    const app = await renderViewerApp(slot1CandidateClaimSnapshot(), { boundAgentId: null, runtimeStatus: "registered_unbound" });
    dispose = app.dispose;
    const stagePanel = app.container.querySelector("#viewer-stage-panel");

    expect(within(stagePanel).getByText("Current Account Has No Bound Agent")).toBeInTheDocument();
    expect(within(stagePanel).getByText("Slot-1 Candidate")).toBeInTheDocument();
    expect(within(stagePanel).getByText("agent-choice-target")).toBeInTheDocument();
    expect(within(stagePanel).getByText("120, -40, 5 cm")).toBeInTheDocument();
    expect(within(stagePanel).getByText("Candidate rationale missing")).toBeInTheDocument();
    expect(within(stagePanel).getByText("Wait or fund first")).toBeInTheDocument();
    expect(within(stagePanel).queryByLabelText("Target Agent")).not.toBeInTheDocument();
    expect(within(stagePanel).queryByRole("button", { name: "Claim Agent" })).not.toBeInTheDocument();
  });
});
