const warDeclarationQuoteFixture = Object.freeze({
  actor_alliance_id: "alliance.red", target_alliance_id: "alliance.blue", intensity: 3,
  minimum_winning_intensity: 2, war_duration_ticks: 24,
  likely_winner_before_action: "alliance.red", victory_margin_estimate: 18,
  reentry_cooldown_or_active_conflict_blocker: "none",
  settlement_risk: "medium: projected margin is positive but narrow",
  why_this_war_is_worth_or_risky: "Proceed only if a 24-tick commitment fits the current resource runway.",
  quoted_at_tick: 12, state_fingerprint: "sha256:war-declaration-quote-visual-fixture",
});

export function installWarDeclarationQuoteVisualFixture(fixtures, { core, setFixturePlayerAuth, viewerFixtureBaseSnapshot }) {
  fixtures.war_declaration_quote = () => {
    core.injectSnapshot(viewerFixtureBaseSnapshot(), { returnState: false });
    core.applySelection({ kind: "agent", id: "agent-0" });
    setFixturePlayerAuth();
    core.injectWarDeclarationQuoteForTest(warDeclarationQuoteFixture);
  };
}
