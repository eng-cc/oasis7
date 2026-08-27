export function installAgentIntentV2VisualFixture(
  fixtures,
  { core, setFixturePlayerAuth, viewerFixtureBaseSnapshot },
) {
  fixtures.agent_intent_v2 = () => {
    const requestedStatus = String(
      new URLSearchParams(window.location.search || "").get("intent_status") || "accepted",
    ).trim().toLowerCase();
    const status = ["accepted", "blocked", "completed"].includes(requestedStatus)
      ? requestedStatus
      : "accepted";
    const intentId = "agent-intent-v2:headed-acceptance";
    const worldId = "live-formal-release-default";
    core.injectSnapshot(viewerFixtureBaseSnapshot({
      player_gameplay: {
        primary_intent: {
          schema_version: 2,
          intent_id: intentId,
          status,
          message: status === "completed"
            ? "Agent guidance completed with a confirmed world receipt."
            : status === "blocked"
              ? "Agent guidance is blocked pending a runtime recheck."
              : "Agent guidance accepted; the Agent will evaluate its next world action.",
          resume_required: status === "blocked",
          source_class: "runtime_projection",
          freshness: "current",
          control_state: "controllable",
          agent_id: "agent-0",
          world_id: worldId,
          reorg_epoch: 0,
          logical_time: 7,
          updated_at: 7,
          event_seq: "42",
          effect_intent_id: status === "completed" ? "effect-intent-v2:headed-acceptance" : null,
          receipt_ref: status === "completed" ? {
            intent_id: intentId,
            world_id: worldId,
            reorg_epoch: 0,
            logical_time: 7,
            event_seq: "42",
            receipt_id: "world-event:43",
          } : null,
          reason_summary: status === "blocked" ? "World prerequisites changed before execution." : null,
          next_step: status === "blocked" ? "Review the world state, then resume when ready." : null,
        },
      },
    }), { returnState: false });
    core.applySelection({ kind: "agent", id: "agent-0" });
    setFixturePlayerAuth();
  };
}
