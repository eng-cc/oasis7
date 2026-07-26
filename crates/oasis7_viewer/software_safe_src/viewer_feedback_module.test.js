import { describe, expect, it } from "vitest";
import { createViewerFeedbackModule } from "./viewer_feedback_module.js";

function createFeedbackModule(state) {
  return createViewerFeedbackModule({
    clone: (value) => (value == null ? value : JSON.parse(JSON.stringify(value))),
    feedbackBadgeClass: () => "feedback-badge",
    hostedActionPolicy: () => null,
    isAgentVisibleToCurrentSession: () => true,
    isLocaleZh: (locale) => locale === "zh",
    localeText: (locale, zh, en) => (locale === "zh" ? zh : en),
    state,
  });
}

describe("viewer feedback module", () => {
  it("preserves camelCase runtime available actions in the gameplay summary", () => {
    const state = {
      lastGameplayActionFeedback: null,
      snapshot: {
        model: {
          agents: {
            "agent-0": { id: "agent-0" },
          },
          locations: {
            base: { id: "base" },
          },
        },
        player_gameplay: {
          goal_kind: "create_first_world_feedback",
          progress_percent: 5,
          availableActions: [
            {
              actionId: "build_factory_smelter_mk1",
              label: "Build smelter",
              protocolAction: "gameplay_action.submit",
              targetAgentId: "agent-0",
              disabledReason: null,
            },
          ],
        },
      },
      uiLocale: "en",
    };

    const summary = createFeedbackModule(state).buildGameplaySummary();

    expect(summary.availableActions).toEqual([
      expect.objectContaining({
        actionId: "build_factory_smelter_mk1",
        label: "Build smelter",
        protocolAction: "gameplay_action.submit",
        targetAgentId: "agent-0",
        disabledReason: null,
        executeKind: "gameplay_action",
      }),
    ]);
    expect(summary.recommendedAction).toEqual(
      expect.objectContaining({
        actionId: "build_factory_smelter_mk1",
        executeKind: "gameplay_action",
      }),
    );
  });

  it("retains every published fallback tradeoff instead of collapsing the summary to one CTA", () => {
    const state = {
      lastGameplayActionFeedback: null,
      snapshot: {
        model: { agents: { "agent-0": { id: "agent-0" } }, locations: { base: { id: "base" } } },
        player_gameplay: {
          stage_status: "blocked",
          fallback_tradeoff_preview: [
            { value_class: "safe_wait", available: true, cost: "one bounded recheck", progress_kept: "keeps the current line", opportunity_cost: "delays alternate output", reason: "a confirmed event can clear the blocker", recommended: true },
            { value_class: "repair_now", available: true, cost: "spend repair materials", progress_kept: "keeps the current capability", opportunity_cost: "uses the repair reserve", reason: "the local blocker is repairable", recommended: false },
            { value_class: "reroute_now", available: false, cost: "move to an alternate route", progress_kept: "keeps the goal but changes the route", opportunity_cost: "abandons the current route's near-term output", reason: "no alternate route is currently available", recommended: false },
          ],
        },
      },
      uiLocale: "en",
    };

    expect(createFeedbackModule(state).buildGameplaySummary().fallbackTradeoffPreview).toEqual([
      { valueClass: "safe_wait", available: true, cost: "one bounded recheck", progressKept: "keeps the current line", opportunityCost: "delays alternate output", reason: "a confirmed event can clear the blocker", recommended: true },
      { valueClass: "repair_now", available: true, cost: "spend repair materials", progressKept: "keeps the current capability", opportunityCost: "uses the repair reserve", reason: "the local blocker is repairable", recommended: false },
      { valueClass: "reroute_now", available: false, cost: "move to an alternate route", progressKept: "keeps the goal but changes the route", opportunityCost: "abandons the current route's near-term output", reason: "no alternate route is currently available", recommended: false },
    ]);
    expect(
      createFeedbackModule(state).buildGameplaySummary().fallbackTradeoffPreview
        .filter((option) => option.recommended),
    ).toHaveLength(1);
  });

  it("maps the no-safe-fallback handoff without inventing an executable action", () => {
    const state = {
      lastGameplayActionFeedback: null,
      snapshot: {
        model: { agents: { "agent-0": { id: "agent-0" } }, locations: { base: { id: "base" } } },
        player_gameplay: {
          no_safe_fallback_reason: "Repair and reroute are unavailable.",
          required_next_decision_action_id: "select_new_goal",
          required_next_decision_class: "goal_selection",
        },
      },
      uiLocale: "en",
    };

    expect(createFeedbackModule(state).buildGameplaySummary().noSafeFallbackHandoff).toEqual({
      reason: "Repair and reroute are unavailable.",
      requiredNextDecisionActionId: "select_new_goal",
      requiredNextDecisionClass: "goal_selection",
    });
  });

  it("preserves English product-validation preview values and labels", () => {
    const state = {
      lastGameplayActionFeedback: null,
      snapshot: {
        model: { agents: { "agent-0": { id: "agent-0" } }, locations: { base: { id: "base" } } },
        player_gameplay: {
          validation_unlock_preview: {
            product_id: "iron_ingot",
            role_tag: "scale",
            tradable: false,
            required_stage: "bootstrap",
            current_stage: "bootstrap",
            stage_status: "available",
            value_summary: "Validated scale product; trading disabled.",
            next_step_hint: "Use this product in its scale role.",
          },
        },
      },
      uiLocale: "en",
    };

    expect(createFeedbackModule(state).buildGameplaySummary().validationUnlockPreview).toEqual({
      productId: "iron_ingot",
      roleTag: "scale",
      roleLabel: "scale",
      tradable: false,
      requiredStage: "bootstrap",
      requiredStageLabel: "bootstrap",
      currentStage: "bootstrap",
      currentStageLabel: "bootstrap",
      stageStatus: "available",
      stageStatusLabel: "available",
      valueSummary: "Validated scale product; trading disabled.",
      localizedValueSummary: "Validated scale product; trading disabled.",
      nextStepHint: "Use this product in its scale role.",
      localizedNextStepHint: "Use this product in its scale role.",
    });
  });

  it("localizes product-validation preview display text while preserving raw DTO values", () => {
    const state = {
      lastGameplayActionFeedback: null,
      snapshot: {
        model: { agents: { "agent-0": { id: "agent-0" } }, locations: { base: { id: "base" } } },
        player_gameplay: {
          validation_unlock_preview: {
            product_id: "iron_ingot",
            role_tag: "scale",
            tradable: false,
            required_stage: "bootstrap",
            current_stage: "bootstrap",
            stage_status: "available",
            value_summary: "Validated scale product; trading disabled.",
            next_step_hint: "Use this product in its scale role.",
          },
        },
      },
      uiLocale: "zh",
    };

    expect(createFeedbackModule(state).buildGameplaySummary().validationUnlockPreview).toEqual(expect.objectContaining({
      roleTag: "scale",
      roleLabel: "规模化",
      requiredStage: "bootstrap",
      requiredStageLabel: "起步",
      currentStage: "bootstrap",
      currentStageLabel: "起步",
      stageStatus: "available",
      stageStatusLabel: "可用",
      valueSummary: "Validated scale product; trading disabled.",
      localizedValueSummary: "已验证规模化产品；未启用交易。",
      nextStepHint: "Use this product in its scale role.",
      localizedNextStepHint: "将此产品用于规模化角色；验证不会解锁新能力。",
    }));
  });

  it("localizes a blank-gate stage as no requirement while preserving its raw value", () => {
    const state = {
      lastGameplayActionFeedback: null,
      snapshot: {
        model: { agents: { "agent-0": { id: "agent-0" } }, locations: { base: { id: "base" } } },
        player_gameplay: {
          validation_unlock_preview: {
            product_id: "iron_ingot",
            role_tag: "scale",
            tradable: true,
            required_stage: "none",
            current_stage: "bootstrap",
            stage_status: "available",
            value_summary: "Validated scale product; trading enabled.",
            next_step_hint: "Use this product in its scale role.",
          },
        },
      },
      uiLocale: "zh",
    };

    expect(createFeedbackModule(state).buildGameplaySummary().validationUnlockPreview).toEqual(expect.objectContaining({
      requiredStage: "none",
      requiredStageLabel: "无要求",
      localizedValueSummary: "已验证规模化产品；已启用交易。",
    }));
  });

  it("projects canonical micro depot facilities without inventing quote or ROI fields", () => {
    const state = {
      lastGameplayActionFeedback: null,
      snapshot: {
        model: { agents: { "agent-0": { id: "agent-0" } }, locations: { base: { id: "base" } } },
        player_gameplay: {
          micro_depot_facilities: [{
            facility_id: "depot-1",
            owner_claim_id: "claim-1",
            status: "active",
            location_id: "base",
            available_units_by_kind: { data: 8 },
            module_id: "micro_depot.eval.v2",
            module_version: "v2",
            wasm_hash: "wasm-hash",
            last_receipt_id: "receipt-1",
            last_proposal_hash: "proposal-1",
            available_actions: ["service_micro_depot_repair"],
          }],
        },
      },
      uiLocale: "en",
    };

    expect(createFeedbackModule(state).buildGameplaySummary().microDepotFacilities).toEqual([
      expect.objectContaining({
        facilityId: "depot-1",
        availableUnitsByKind: { data: 8 },
        moduleId: "micro_depot.eval.v2",
        lastReceiptId: "receipt-1",
        lastProposalHash: "proposal-1",
        availableActions: ["service_micro_depot_repair"],
      }),
    ]);
  });

  it("drops malformed micro depot facility entries and normalizes display collections", () => {
    const state = {
      lastGameplayActionFeedback: null,
      snapshot: {
        model: { agents: {}, locations: {} },
        player_gameplay: {
          micro_depot_facilities: [
            null,
            "not-a-facility",
            {
              facility_id: "depot-safe",
              available_units_by_kind: "not-an-inventory-record",
              supported_resource_kinds: "data",
              available_actions: [" service_micro_depot_repair ", 7, "", { action: "unsafe" }],
            },
          ],
        },
      },
      uiLocale: "en",
    };

    expect(createFeedbackModule(state).buildGameplaySummary().microDepotFacilities).toEqual([
      expect.objectContaining({
        facilityId: "depot-safe",
        availableUnitsByKind: {},
        supportedResourceKinds: [],
        availableActions: ["service_micro_depot_repair"],
      }),
    ]);
  });

  it("uses an empty facility list when the optional snapshot field is absent or wrong-typed", () => {
    for (const microDepotFacilities of [undefined, null, {}, "depot-1"]) {
      const state = {
        lastGameplayActionFeedback: null,
        snapshot: {
          model: { agents: {}, locations: {} },
          player_gameplay: { micro_depot_facilities: microDepotFacilities },
        },
        uiLocale: "en",
      };

      expect(createFeedbackModule(state).buildGameplaySummary().microDepotFacilities).toEqual([]);
    }
  });

  it("renders published recovery options as escaped-by-the-view comparison details", () => {
    const state = {
      lastGameplayActionFeedback: null,
      snapshot: {
        model: { agents: {}, locations: {} },
        player_gameplay: {
          repair_available: true,
          rebuild_available: true,
          pivot_available: true,
          recovery_options: [
            {
              kind: "repair",
              estimated_time_class: "short",
              estimated_resource_class: "low",
              risk_class: "low",
              retained_benefit: "<existing throughput>",
              recommendation_reason: "Fastest safe recovery",
            },
            {
              kind: "rebuild",
              estimated_time_class: "medium",
              estimated_resource_class: "high",
              risk_class: "medium",
              retained_benefit: "new capacity",
              recommendation_reason: "Restores a stronger line",
            },
          ],
        },
      },
      uiLocale: "en",
    };

    const continuation = createFeedbackModule(state).buildGameplaySummary().matureWorldContinuation;

    expect(continuation.recoveryOptionComparisons).toEqual([
      expect.objectContaining({
        kind: "repair",
        timeClass: "short",
        resourceClass: "low",
        riskClass: "low",
        retainedBenefit: "<existing throughput>",
        recommendationReason: "Fastest safe recovery",
      }),
      expect.objectContaining({ kind: "rebuild", timeClass: "medium", resourceClass: "high" }),
    ]);
    expect(continuation.recoveryOptions).toBe(
      "repair: time=short · resources=low · risk=low · retains=<existing throughput> · why=Fastest safe recovery / rebuild: time=medium · resources=high · risk=medium · retains=new capacity · why=Restores a stronger line",
    );
  });

  it("keeps boolean recovery summaries when runtime recovery records are absent or invalid", () => {
    const state = {
      lastGameplayActionFeedback: null,
      snapshot: {
        model: { agents: {}, locations: {} },
        player_gameplay: {
          repair_available: true,
          rebuild_available: false,
          recovery_options: [{ estimated_time_class: "short" }, "repair"],
        },
      },
      uiLocale: "en",
    };

    const continuation = createFeedbackModule(state).buildGameplaySummary().matureWorldContinuation;

    expect(continuation.recoveryOptionComparisons).toEqual([]);
    expect(continuation.recoveryOptions).toBe("repair: available / rebuild: unavailable");
  });
});
