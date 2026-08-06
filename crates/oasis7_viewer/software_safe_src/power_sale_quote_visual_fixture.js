const powerSaleQuoteFixture = Object.freeze({
  seller_agent_id: "agent-0", buyer_agent_id: "agent-buyer", current_power_level: 15,
  power_state_before: "low_power", sale_amount: 10, price_per_pu: 3, expected_revenue: 30,
  power_state_after_sale: "critical", remaining_runway_ticks: 5,
  next_action_affordability_after_sale: "limited", production_interrupt_risk: true,
  recommended_sale_action: "defer_sale", why_sale_is_safe_or_risky: "critical power runway",
});

export function installPowerSaleQuoteVisualFixture(fixtures, { core, setFixturePlayerAuth, viewerFixtureBaseSnapshot }) {
  fixtures.power_sale_quote = () => {
    core.injectSnapshot(viewerFixtureBaseSnapshot(), { returnState: false });
    core.applySelection({ kind: "agent", id: "agent-0" });
    setFixturePlayerAuth();
    core.injectPowerSaleQuoteForTest(powerSaleQuoteFixture);
  };
}
