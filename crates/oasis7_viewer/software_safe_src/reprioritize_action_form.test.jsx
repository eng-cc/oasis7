import { fireEvent, render, screen } from "@solidjs/testing-library";
import { beforeEach, describe, expect, it, vi } from "vitest";

const core = vi.hoisted(() => ({
  state: { lastPromptFeedback: null, snapshot: { model: { agent_prompt_profiles: {} } } },
  sendGameplayAction: vi.fn(),
  sendPromptControl: vi.fn(() => ({ ok: true })),
  snapshotSemanticFeedback: (feedback) => feedback,
}));

vi.mock("./legacy_core.js", () => core);

import { ReprioritizeActionForm } from "./reprioritize_action_form.jsx";

const action = { actionId: "reprioritize", label: "Replace this Agent's short-term goal", targetAgentId: "agent-0" };
const tr = (_locale, _zh, en) => en;

describe("ReprioritizeActionForm", () => {
  beforeEach(() => {
    core.state.lastPromptFeedback = null;
    core.state.snapshot = { model: { agent_prompt_profiles: { "agent-0": { system_prompt_override: "system", long_term_goal_override: "long" } } } };
    core.sendGameplayAction.mockReset();
    core.sendPromptControl.mockReset();
    core.sendPromptControl.mockReturnValue({ ok: true });
  });

  it("opens, rejects whitespace locally, and applies only a short-term goal through prompt control", () => {
    render(() => <ReprioritizeActionForm action={action} locale="en" tr={tr} observeState={() => {}} />);
    fireEvent.click(screen.getByTestId("viewer-available-action-reprioritize"));
    const goal = screen.getByLabelText("Replacement short-term goal");
    fireEvent.input(goal, { target: { value: "  " } });
    fireEvent.submit(goal.closest("form"));
    expect(core.sendPromptControl).not.toHaveBeenCalled();
    expect(goal).toHaveFocus();
    fireEvent.input(goal, { target: { value: "Keep the forge queue moving" } });
    fireEvent.click(screen.getByRole("button", { name: "Apply new goal" }));
    expect(core.sendPromptControl).toHaveBeenCalledWith("apply", {
      agentId: "agent-0",
      shortTermGoal: "Keep the forge queue moving",
      systemPrompt: "system",
      longTermGoal: "long",
    });
    expect(core.sendGameplayAction).not.toHaveBeenCalled();
  });

  it("cancels locally without sending", () => {
    render(() => <ReprioritizeActionForm action={action} locale="en" tr={tr} observeState={() => {}} />);
    fireEvent.click(screen.getByTestId("viewer-available-action-reprioritize"));
    fireEvent.input(screen.getByLabelText("Replacement short-term goal"), { target: { value: "temporary" } });
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    expect(core.sendPromptControl).not.toHaveBeenCalled();
    expect(screen.queryByLabelText("Replacement short-term goal")).not.toBeInTheDocument();
  });
});
