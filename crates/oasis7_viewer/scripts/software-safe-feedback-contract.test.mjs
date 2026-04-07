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

console.log("software-safe feedback contract tests passed");
