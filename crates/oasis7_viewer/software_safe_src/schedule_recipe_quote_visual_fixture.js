const normalQuote = Object.freeze({
  owner_agent_id: "agent-0", factory_id: "factory-0", recipe_id: "assemble_hardware", batches: 2,
  base_duration_ticks: 6, electricity_cost: 12, electricity_after: 88, hardware_cost: 4,
  data_output: 8, finished_product_id: "hardware", finished_product_units: 2,
  local_shortage_delay_ticks: 0, shortage_reason: "none", recommended_pre_step: "schedule_now",
  runway_before_ticks: 40, runway_after_ticks: 40, downtime_threshold_ppm: 250000,
  continue_production_risk: "normal", maintenance_pressure_delta: "unchanged", recommended_maintenance_action: "none",
});

export function installScheduleRecipeQuoteVisualFixture(fixtures, { core, setFixturePlayerAuth, viewerFixtureBaseSnapshot }) {
  function install(quote) {
    core.injectSnapshot(viewerFixtureBaseSnapshot(), { returnState: false });
    core.applySelection({ kind: "agent", id: "agent-0" });
    setFixturePlayerAuth();
    core.injectScheduleRecipeQuoteForTest(quote);
  }
  fixtures.schedule_recipe_quote = () => install(normalQuote);
  fixtures.schedule_recipe_quote_critical = () => install({ ...normalQuote, continue_production_risk: "critical", recommended_pre_step: "restore_power", runway_before_ticks: 3, runway_after_ticks: 3, local_shortage_delay_ticks: 5, shortage_reason: "local_hardware_shortage" });
}
