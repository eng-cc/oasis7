import assert from "node:assert/strict";

globalThis.window = {
  location: { search: "" },
  addEventListener() {},
};
globalThis.document = {
  createElement() {
    return {
      getContext() {
        return null;
      },
    };
  },
};

const core = await import("../software_safe_src/legacy_core.js");

{
  const display = core.describeSemanticFeedback({
    kind: "chat",
    stage: "error",
    effect: "llm_init_failed",
    reason: "llm init failed for agent-0: llm config error: missing env variable: OASIS7_LLM_MODEL",
    response: {
      code: "llm_init_failed",
      message: "llm init failed for agent-0: llm config error: missing env variable: OASIS7_LLM_MODEL",
    },
  });
  assert.equal(display.label, "LLM unavailable");
  assert.match(display.summary, /no usable LLM configuration/i);
  assert.match(display.detail, /config\.toml|OASIS7_LLM_/);
}

{
  const display = core.describeSemanticFeedback({
    kind: "prompt",
    stage: "rollback_ack",
    effect: "prompt rolled back via version=4 → target=2",
    response: {
      version: 4,
      rolled_back_to_version: 2,
      code: "should_not_surface_for_ack",
    },
  });
  assert.equal(display.code, null);
}

{
  core.state.promptDraft.currentVersion = 2;
  core.state.promptDraft.rollbackTargetVersion = 1;
  const versionState = core.describePromptVersionState({
    stage: "rollback_ack",
    response: {
      version: 2,
      rolled_back_to_version: 0,
    },
  });
  assert.equal(versionState.currentVersion, 2);
  assert.equal(versionState.restoredFromVersion, 0);
  assert.equal(versionState.nextRollbackTargetVersion, 1);
  assert.match(versionState.detail, /next target v1/i);
}

{
  core.state.snapshot = {
    player_gameplay: {
      stage_id: "post_onboarding",
      stage_status: "blocked",
      goal_id: "post_onboarding.recover_capability",
      goal_kind: "RecoverCapability",
      goal_title: "Recover sustainable capability",
      objective: "Recover the blocked line or capability chain instead of repeating one-off actions.",
      progress_detail: "Stage progress: the primary line is blocked.",
      progress_percent: 68,
      blocker_kind: "material_shortage",
      blocker_detail: "iron input exhausted at factory-0",
      next_step_hint: "Replenish upstream materials, then advance again to confirm the line resumes.",
      branch_hint: null,
      available_actions: [
        {
          action_id: "advance_step",
          label: "Advance 1 step",
          protocol_action: "live_control.step",
          target_agent_id: null,
          disabled_reason: null,
        },
      ],
      recent_feedback: {
        action: "step",
        stage: "blocked",
        effect: "gameplay blocked before requested advance completed: logicalTime +0, eventSeq +0",
        reason: "latest live control was blocked before runtime advance",
        hint: "inspect the blocker details, recover the line, then advance again to confirm progress.",
        delta_logical_time: 0,
        delta_event_seq: 0,
      },
      agent_claim: null,
    },
  };
  const gameplaySummary = core.buildGameplaySummary();
  assert.equal(gameplaySummary.stageId, "post_onboarding");
  assert.equal(gameplaySummary.stageStatus, "blocked");
  assert.equal(gameplaySummary.progressPercent, 68);
  assert.equal(gameplaySummary.availableActions[0].actionId, "advance_step");
  assert.match(gameplaySummary.assetGovernanceHandoff, /no main token transfer form/i);
  assert.equal(core.getState().gameplaySummary.goalTitle, "Recover sustainable capability");
}

console.log("software-safe feedback contract tests passed");
