import { describe, expect, it, vi } from "vitest";
import { installPowerSurvivalQuoteVisualFixture } from "./power_survival_quote_visual_fixture.js";

describe("installPowerSurvivalQuoteVisualFixture", () => {
  it("hydrates a player-readable critical-recovery quote fixture", () => {
    const fixtures = {}; const core = { injectSnapshot: vi.fn(), applySelection: vi.fn(), injectPowerSurvivalQuoteForTest: vi.fn() };
    installPowerSurvivalQuoteVisualFixture(fixtures, { core, setFixturePlayerAuth: vi.fn(), viewerFixtureBaseSnapshot: vi.fn(() => ({ player_gameplay: {} })) });
    fixtures.power_survival_quote();
    expect(core.injectPowerSurvivalQuoteForTest).toHaveBeenCalledWith(expect.objectContaining({ seller_agent_id: "agent-1", power_state_before: "critical", recommended_power_action: "buy_power_partial" }));
  });
});
