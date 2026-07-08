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
});
