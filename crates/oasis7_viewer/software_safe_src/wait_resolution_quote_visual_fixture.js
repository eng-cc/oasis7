const waitResolutionQuoteFixture = Object.freeze({
  safe_to_wait: false,
  resolution_trigger: "committed runtime event applies the queued smelter",
  recheck_tick_or_event: "event 8",
  expected_change: "smelter construction becomes visible",
  unresolved_risk: "the action can still be blocked",
  alternative_unlock_condition: "refresh the snapshot and choose an enabled action",
});

export function installWaitResolutionQuoteVisualFixture(fixtures, {
  core,
  setFixturePlayerAuth,
  viewerFixtureBaseSnapshot,
}) {
  fixtures.wait_resolution_quote = () => {
    const snapshot = viewerFixtureBaseSnapshot();
    Object.assign(snapshot.player_gameplay, {
      stage_status: "accepted",
      execution_state: "accepted",
      fallback_tradeoff_preview: [],
      no_safe_fallback_reason: null,
      required_next_decision_action_id: null,
      required_next_decision_class: null,
      wait_resolution_quote: { ...waitResolutionQuoteFixture },
    });
    core.injectSnapshot(snapshot, { returnState: false });
    core.applySelection({ kind: "agent", id: "agent-0" });
    setFixturePlayerAuth();
  };
}
