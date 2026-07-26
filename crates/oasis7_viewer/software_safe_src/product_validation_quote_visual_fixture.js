const productValidationQuoteFixture = Object.freeze({
  product_id: "logistics_drone",
  product_role: "explore",
  tradable: true,
  stage_before: "bootstrap",
  stage_after: "bootstrap",
  unlock_or_value_class: "scale_out",
  recommended_action: "advance_industry_stage",
  submission_allowed: true,
  missing_prerequisite: "industry_stage=scale_out",
  reachable_advance_or_recovery: "complete_reachable_industry_progress",
});

export function installProductValidationQuoteVisualFixture(fixtures, { core, setFixturePlayerAuth, viewerFixtureBaseSnapshot }) {
  fixtures.product_validation_quote = () => {
    core.injectSnapshot(viewerFixtureBaseSnapshot(), { returnState: false });
    core.applySelection({ kind: "agent", id: "agent-0" });
    setFixturePlayerAuth();
    core.injectProductValidationQuoteForTest(productValidationQuoteFixture);
  };
}
