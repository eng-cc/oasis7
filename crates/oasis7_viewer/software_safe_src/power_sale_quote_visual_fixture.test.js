import { describe, expect, it, vi } from "vitest";
import { installPowerSaleQuoteVisualFixture } from "./power_sale_quote_visual_fixture.js";

describe("installPowerSaleQuoteVisualFixture", () => {
  it("registers and hydrates the deterministic dangerous-sale fixture", () => {
    const fixtures = {};
    const core = { injectSnapshot: vi.fn(), applySelection: vi.fn(), injectPowerSaleQuoteForTest: vi.fn() };
    const setFixturePlayerAuth = vi.fn();
    installPowerSaleQuoteVisualFixture(fixtures, { core, setFixturePlayerAuth, viewerFixtureBaseSnapshot: vi.fn(() => ({ player_gameplay: {} })) });

    expect(fixtures.power_sale_quote).toEqual(expect.any(Function));
    fixtures.power_sale_quote();

    expect(core.injectSnapshot).toHaveBeenCalledWith({ player_gameplay: {} }, { returnState: false });
    expect(core.applySelection).toHaveBeenCalledWith({ kind: "agent", id: "agent-0" });
    expect(setFixturePlayerAuth).toHaveBeenCalledOnce();
    expect(core.injectPowerSaleQuoteForTest).toHaveBeenCalledWith(expect.objectContaining({
      seller_agent_id: "agent-0", buyer_agent_id: "agent-buyer", current_power_level: 15,
      sale_amount: 10, price_per_pu: 3, expected_revenue: 30, power_state_after_sale: "critical",
      remaining_runway_ticks: 5, next_action_affordability_after_sale: "limited",
      production_interrupt_risk: true, recommended_sale_action: "defer_sale",
    }));
  });
});
