// Public fixture projection; the App harness installs the key through the real
// session boundary before mounting or dispatching controls.
export function bindLocalTestAgent(core, agentId = "agent-0", playerId = "local-test-player-bound") {
  core.state.auth = {
    ...core.state.auth,
    available: true,
    playerId,
    publicKey: "ab".repeat(32),
    privateKey: "07".repeat(32),
    source: "local_test_api_ephemeral",
    registrationStatus: "registered",
    runtimeStatus: "registered", syncInFlight: false,
    boundAgentId: agentId,
  };
}
export function bindFirstSnapshotAgentForTest(core, snapshot) {
  const agentId = Object.keys(snapshot?.model?.agents || {})[0];
  const playerId = snapshot?.model?.agent_player_bindings?.[agentId];
  if (!agentId || !playerId) {
    return;
  }
  bindLocalTestAgent(core, agentId, playerId);
}

export function bindUnboundLocalPlayerForTest(core) {
  bindLocalTestAgent(core, null);
}
