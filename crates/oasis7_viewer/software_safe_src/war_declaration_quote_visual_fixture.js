const warDeclarationQuoteFixture = Object.freeze({
  actor_alliance_id: "alliance.red", target_alliance_id: "alliance.blue", intensity: 3,
  settlement_path: "core_fallback", conflict_status: "none",
  minimum_winning_intensity: 2, war_duration_ticks: 24,
  aggressor_score_estimate: 38, defender_score_estimate: 20, likely_winner_before_action: "alliance.red", projected_outcome: "aggressor_wins", victory_margin_estimate: 18,
  conflict_window_blocked_until: 36,
  reentry_cooldown_or_active_conflict_blocker: "none",
  settlement_risk: "core settlement changes participant resources and reputation", settlement_risk_code: "resource_and_reputation_change",
  alternative_action: "negotiate", recommended_war_action: "declare_war", why_this_war_is_worth_or_risky: "Proceed only if a 24-tick commitment fits the current resource runway.",
  mobilization_electricity_required: 24, mobilization_electricity_current: 40, mobilization_electricity_after: 16,
  mobilization_data_required: 17, mobilization_data_current: 35, mobilization_data_after: 18, mobilization_affordable: true,
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
