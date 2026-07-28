import { describe, expect, it, vi } from "vitest";
import { installWaitResolutionQuoteVisualFixture } from "./wait_resolution_quote_visual_fixture.js";

describe("installWaitResolutionQuoteVisualFixture", () => {
  it("injects a recommended repair card alongside every unavailable/non-recommended Wait detail for test_api screenshots", () => {
    const fixtures = {};
    const core = { injectSnapshot: vi.fn(), applySelection: vi.fn() };
    const setFixturePlayerAuth = vi.fn();
    installWaitResolutionQuoteVisualFixture(fixtures, {
      core,
      setFixturePlayerAuth,
      viewerFixtureBaseSnapshot: vi.fn(() => ({ player_gameplay: {} })),
    });

    fixtures.wait_resolution_quote();

    expect(core.injectSnapshot).toHaveBeenCalledWith(expect.objectContaining({
      player_gameplay: expect.objectContaining({
        stage_status: "accepted",
        execution_state: "accepted",
        fallback_tradeoff_preview: [{
          value_class: "repair_now",
          available: true,
          cost: "spend repair materials",
          progress_kept: "keeps the current capability",
          opportunity_cost: "uses the repair reserve",
          reason: "the local blocker is repairable",
          recommended: true,
        }],
        wait_resolution_quote: {
          safe_to_wait: false,
          resolution_trigger: "committed runtime event applies the queued smelter",
          recheck_tick_or_event: "event 8",
          expected_change: "smelter construction becomes visible",
          unresolved_risk: "the action can still be blocked",
          alternative_unlock_condition: "refresh the snapshot and choose an enabled action",
        },
      }),
    }), { returnState: false });
    expect(core.applySelection).toHaveBeenCalledWith({ kind: "agent", id: "agent-0" });
    expect(setFixturePlayerAuth).toHaveBeenCalledOnce();
  });
});
