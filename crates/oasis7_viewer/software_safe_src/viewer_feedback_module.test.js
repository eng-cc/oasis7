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

  it("projects the canonical product-validation preview without inventing an unlock", () => {
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
      tradable: false,
      requiredStage: "bootstrap",
      currentStage: "bootstrap",
      stageStatus: "available",
      valueSummary: "Validated scale product; trading disabled.",
      nextStepHint: "Use this product in its scale role.",
    });
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
