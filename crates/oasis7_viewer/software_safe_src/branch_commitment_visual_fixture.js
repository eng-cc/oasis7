export function installBranchCommitmentVisualFixture(fixtures, { core, setFixturePlayerAuth, viewerFixtureBaseSnapshot }) {
  fixtures.expansion_tradeoff_two_beats = () => {
    const snapshot = viewerFixtureBaseSnapshot();
    snapshot.player_gameplay = {
      ...snapshot.player_gameplay,
      goal_kind: "ChooseFirstExpansionTradeoff",
      goal_title: "Choose the first expansion tradeoff",
      branch_hint: "Compare the published consequences before committing.",
      branch_recommendations: [{ action_id: "build_alloy_factory", route_label: "Scale alloy throughput", immediate_gain: "Adds a second alloy production lane", future_beats: ["The next expansion starts with spare capacity", "New throughput requires a steadier structural-frame supply"], risk_or_lockin: "Consumes the current structural-frame reserve", next_session_hook: "Return to route the first bulk alloy order", first_delivery_preview: { local_need: "Regional fabricators need dependable alloy plates", expected_output: "Two alloy plates from the first smelter batch", required_inputs: ["iron_ingot × 2", "copper_wire × 2"], value_timing: "After one smelter run completes", leverage_class_unlocked: "regional_material_supplier", return_visit_hook: "Return to fulfill the next regional alloy order" } }],
      available_actions: [{ action_id: "build_alloy_factory", label: "Build alloy factory core", protocol_action: "gameplay_action.submit", disabled_reason: "missing structural frames" }],
    };
    core.injectSnapshot(snapshot, { returnState: false });
    core.applySelection({ kind: "agent", id: "agent-0" });
    setFixturePlayerAuth();
  };
}
