const powerSurvivalQuoteFixture = Object.freeze({
  buyer_agent_id: "agent-0", seller_agent_id: "agent-1", current_power_level: 2,
  power_state_before: "critical", recovery_action: "buy_power", recovery_amount: 18,
  power_gain_estimate: 18, requested_price_per_pu: 3, price_per_pu: 3, price_or_time_cost: 54,
  power_state_after_recovery: "low_power", survival_runway_ticks: 20,
  next_action_affordability_after_recovery: "limited",
  shutdown_avoidance_reason: "recovery restores 20 runway ticks and lifts agent from critical to low_power; recommended action: buy_power_partial",
  recommended_power_action: "buy_power_partial",
});

export function installPowerSurvivalQuoteVisualFixture(fixtures, { core, setFixturePlayerAuth, viewerFixtureBaseSnapshot }) {
  fixtures.power_survival_quote = () => {
    core.injectSnapshot(viewerFixtureBaseSnapshot(), { returnState: false });
    core.applySelection({ kind: "agent", id: "agent-0" });
    setFixturePlayerAuth();
    core.injectPowerSurvivalQuoteForTest(powerSurvivalQuoteFixture);
  };
}
