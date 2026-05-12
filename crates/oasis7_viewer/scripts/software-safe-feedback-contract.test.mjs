import assert from "node:assert/strict";

globalThis.window = {
  location: { search: "?test_api=1", href: "http://127.0.0.1:4173/software_safe.html?ws=ws://127.0.0.1:5011&test_api=1", pathname: "/software_safe.html" },
  history: { replaceState() {} },
  localStorage: { getItem() { return null; }, setItem() {}, removeItem() {} },
  addEventListener() {},
};
globalThis.document = {
  documentElement: { lang: "en" },
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
    kind: "chat",
    stage: "error",
    effect: "llm_init_failed",
    reason: "llm init failed",
    response: {
      code: "llm_init_failed",
      message: "llm init failed",
    },
  }, "zh");
  assert.equal(display.label, "LLM 不可用");
  assert.match(display.summary, /没有可用的 LLM 配置/);
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
    model: {
      agents: { "agent-0": { id: "agent-0" } },
      locations: { "loc-0": { id: "loc-0" } },
    },
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
        {
          action_id: "request_snapshot",
          label: "Request snapshot",
          protocol_action: "world.request_snapshot",
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
  assert.deepEqual(
    gameplaySummary.availableActions.map((action) => action.actionId),
    ["advance_step", "request_snapshot"],
  );
  assert.match(gameplaySummary.assetGovernanceHandoff, /no main token transfer form/i);
  assert.equal(core.getState().gameplaySummary.goalTitle, "Recover sustainable capability");
}

{
  const gameplaySummary = core.buildGameplaySummary("zh");
  assert.match(gameplaySummary.assetGovernanceHandoff, /资产 \/ 治理动作/);
}

{
  const injectedState = core.injectSnapshot({
    time: 12,
    config: {
      space: {
        width_cm: 10_000_000,
        depth_cm: 5_000_000,
        height_cm: 1_000_000,
      },
    },
    model: {
      agents: { "agent-0": { id: "agent-0", location_id: "loc-0", resources: {} } },
      locations: {
        "loc-0": {
          id: "loc-0",
          name: "Loc 0",
          pos: { x_cm: 0, y_cm: 0, z_cm: 0 },
          profile: { radius_cm: 25_000, radiation_emission_per_tick: 0, material: "silicate" },
          resources: {},
        },
        "loc-1": {
          id: "loc-1",
          name: "Loc 1",
          pos: { x_cm: 100_000, y_cm: 0, z_cm: 0 },
          profile: { radius_cm: 10_000, radiation_emission_per_tick: 0, material: "silicate" },
          resources: {},
        },
        "loc-bad": {
          id: "loc-bad",
          name: "Broken Location",
          pos: { x_cm: null, y_cm: 0, z_cm: 0 },
          profile: { radius_cm: 5_000, radiation_emission_per_tick: 0, material: "silicate" },
          resources: {},
        },
      },
    },
    player_gameplay: {
      stage_id: "first_session_loop",
      stage_status: "active",
      goal_id: "first_session_loop.create_first_world_feedback",
      goal_kind: "CreateFirstWorldFeedback",
      goal_title: "Claim your first agent slot",
      objective: "Select a target and submit your first slot-1 claim.",
      progress_detail: "The dedicated starter pool can auto-fund the first slot-1 claim when you confirm it.",
      progress_percent: 24,
      blocker_kind: null,
      blocker_detail: null,
      next_step_hint: "Pick an unclaimed target, review the canonical quote, then confirm ClaimAgent.",
      branch_hint: null,
      available_actions: [],
      recent_feedback: null,
      agent_claim: {
        claimer_agent_id: "agent-0",
        current_epoch: 3,
        reputation_tier: 0,
        claim_cap: 1,
        owned_claim_count: 0,
        liquid_main_token_balance: 0,
        restricted_starter_claim_balance: 0,
        slot_1_auto_restricted_starter_claim_amount: 325,
        slot_1_eligible_claim_balance: 325,
        next_claim_quote: {
          slot_index: 1,
          reputation_tier: 0,
          claim_cap: 1,
          owned_claim_count: 0,
          activation_fee_amount: 100,
          claim_bond_amount: 200,
          upkeep_per_epoch: 25,
          total_upfront_amount: 325,
          transferable_liquid_balance: 0,
          restricted_starter_claim_balance: 0,
          auto_restricted_starter_claim_amount: 325,
          eligible_claim_balance: 325,
          release_cooldown_epochs: 3,
          grace_epochs: 2,
          idle_warning_epochs: 7,
          forced_idle_reclaim_epochs: 10,
          forced_reclaim_penalty_bps: 2000,
          blocked_reason: null,
        },
        owned_claims: [],
      },
    },
  });
  assert.equal(injectedState.logicalTime, 12);
  assert.equal(injectedState.gameplaySummary.agentClaim.next_claim_quote.auto_restricted_starter_claim_amount, 325);
  assert.equal(injectedState.gameplaySummary.agentClaim.slot_1_auto_restricted_starter_claim_amount, 325);
  core.select({ kind: "location", id: "loc-0" });
  const worldScale = core.buildWorldScaleSurface();
  assert.equal(worldScale.physicalTruth.canonicalUnitLabel, "1 cm");
  assert.match(worldScale.physicalTruth.worldBoundsLabel, /100 km × 50 km × 10 km/);
  assert.equal(worldScale.physicalTruth.anchor.id, "loc-0");
  assert.equal(worldScale.physicalTruth.anchor.radiusLabel, "250 m");
  assert.equal(worldScale.physicalTruth.nearestLocations[0].id, "loc-1");
  assert.equal(worldScale.physicalTruth.nearestLocations[0].distanceLabel, "1 km");
  assert.equal(
    worldScale.physicalTruth.nearestLocations.some((location) => location.id === "loc-bad"),
    false,
  );
  assert.match(worldScale.presentationScale.markerTruthNote, /do not read on-screen diameter/i);
  assert.match(worldScale.presentationScale.zoomTruthNote, /zoom tiers/i);
}

{
  assert.equal(core.setSoftwareSafeLocale("zh"), "zh");
  assert.equal(core.state.uiLocale, "zh");
  assert.equal(globalThis.document.documentElement.lang, "zh-CN");
}

{
  core.state.snapshot = {
    model: {
      agents: { "agent-0": { id: "agent-0" } },
      locations: { "loc-0": { id: "loc-0" } },
    },
    player_gameplay: {
      next_step_hint: "Enable LLM access before retrying world controls.",
      available_actions: [
        {
          action_id: "advance_step",
          label: "Advance 1 step",
          protocol_action: "live_control.step",
          target_agent_id: null,
          disabled_reason: "missing env variable: OASIS7_LLM_MODEL",
        },
        {
          action_id: "resume_play",
          label: "Resume live play",
          protocol_action: "live_control.play",
          target_agent_id: null,
          disabled_reason: "missing env variable: OASIS7_LLM_MODEL",
        },
        {
          action_id: "request_snapshot",
          label: "Request snapshot",
          protocol_action: "world.request_snapshot",
          target_agent_id: null,
          disabled_reason: null,
        },
      ],
    },
  };
  const gameplaySummary = core.buildGameplaySummary();
  assert.deepEqual(
    gameplaySummary.availableActions.map((action) => action.actionId),
    ["advance_step", "resume_play", "request_snapshot"],
  );
  core.state.controlProfile = "live";
  const feedback = core.sendControl("step");
  assert.equal(feedback.accepted, false);
  assert.equal(feedback.stage, "blocked");
  assert.equal(feedback.reason, "missing env variable: OASIS7_LLM_MODEL");
  assert.match(feedback.effect, /control blocked by gameplay gate/i);
  assert.equal(feedback.hint, "Enable LLM access before retrying world controls.");
}

{
  core.state.snapshot = {
    model: {
      agents: {},
      locations: {},
    },
    player_gameplay: {
      stage_id: "post_onboarding",
      stage_status: "active",
      goal_id: "post_onboarding.establish_first_capability",
      goal_kind: "EstablishFirstCapability",
      goal_title: "Establish your first sustainable capability",
      objective: "Do not stall after onboarding.",
      progress_detail: "Progress exists, but entities are missing from the viewer snapshot.",
      progress_percent: 20,
      blocker_kind: null,
      blocker_detail: null,
      next_step_hint: "this will be replaced by the empty-entity guard",
      branch_hint: null,
      available_actions: [
        {
          action_id: "advance_step",
          label: "Advance 1 step",
          protocol_action: "live_control.step",
          target_agent_id: null,
          disabled_reason: null,
        },
        {
          action_id: "request_snapshot",
          label: "Request snapshot",
          protocol_action: "request_snapshot",
          target_agent_id: null,
          disabled_reason: null,
        },
      ],
      recent_feedback: null,
      agent_claim: null,
    },
  };
  const gameplaySummary = core.buildGameplaySummary();
  assert.equal(gameplaySummary.stageStatus, "blocked");
  assert.equal(gameplaySummary.blockerKind, "runtime_snapshot_empty_entities");
  assert.match(gameplaySummary.blockerDetail, /no agents\/locations|没有 Agent \/ 地点/i);
  assert.match(gameplaySummary.nextStepHint, /fresh snapshot|刷新快照/i);
  assert.equal(gameplaySummary.availableActions[0].disabledReason !== null, true);
  assert.equal(gameplaySummary.availableActions[1].disabledReason, null);
}

{
  core.state.snapshot = {
    player_gameplay: {
      next_step_hint: "Recover the lane before retrying world controls.",
      available_actions: [],
    },
  };
  core.state.lastControlFeedback = {
    id: 41,
    action: "step",
    accepted: true,
    stage: "queued",
    reason: null,
    hint: null,
    effect: "queued",
    baselineLogicalTime: 7,
    baselineEventSeq: 3,
    deltaLogicalTime: 0,
    deltaEventSeq: 0,
    requestId: 41,
  };
  core.handleControlCompletionAck({
    request_id: 41,
    status: "blocked",
    delta_logical_time: 0,
    delta_event_seq: 0,
    error_code: "llm_init_failed",
    error_message: "gameplay requires a configured and reachable LLM provider",
  });
  assert.equal(core.state.lastControlFeedback.stage, "blocked");
  assert.equal(
    core.state.lastControlFeedback.reason,
    "gameplay requires a configured and reachable LLM provider",
  );
  assert.equal(
    core.state.lastControlFeedback.hint,
    "Recover the lane before retrying world controls.",
  );
  assert.match(core.state.lastControlFeedback.effect, /blocked before requested advance/i);
}

{
  core.state.lastControlFeedback = {
    id: 42,
    action: "step",
    accepted: true,
    stage: "queued",
    reason: null,
    hint: null,
    effect: "queued",
    baselineLogicalTime: 7,
    baselineEventSeq: 3,
    deltaLogicalTime: 0,
    deltaEventSeq: 0,
    requestId: 42,
  };
  core.handleControlCompletionAck({
    request_id: 42,
    status: "timeout_no_progress",
    delta_logical_time: 0,
    delta_event_seq: 0,
  });
  assert.equal(core.state.lastControlFeedback.stage, "completed_no_progress");
  assert.equal(core.state.lastControlFeedback.reason, "timeout_no_progress");
  assert.match(core.state.lastControlFeedback.effect, /no visible world delta/i);
}

console.log("software-safe feedback contract tests passed");
