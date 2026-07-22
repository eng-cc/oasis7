const refineQuotePreflightFixture = Object.freeze({
  owner_agent_id: "agent-0",
  compound_mass_g: 40,
  electricity_cost: 12,
  electricity_after: 88,
  hardware_output: 20,
  target_id: "factory_build_hardware",
  target_gap_before: 20,
  target_gap_after: 0,
  target_linkage: "enables_factory_build_hardware_goal",
  recommended_refine_amount: 40,
  value_classification: "enough_to_advance",
});

export function installRefineQuotePreflightVisualFixture(fixtures, { core, setFixturePlayerAuth, viewerFixtureBaseSnapshot }) {
  fixtures.refine_quote_preflight = () => {
    core.injectSnapshot(viewerFixtureBaseSnapshot(), { returnState: false });
    core.applySelection({ kind: "agent", id: "agent-0" });
    setFixturePlayerAuth();
    core.injectRefineQuotePreflightForTest(refineQuotePreflightFixture);
  };
}
