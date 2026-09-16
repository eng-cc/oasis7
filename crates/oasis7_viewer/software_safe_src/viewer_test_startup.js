export async function settleLocalTestAuthStartup(core, sentMessages) {
  for (let attempt = 0; attempt < 20; attempt += 1) {
    if (
      core.state.auth.source === "local_test_api_ephemeral"
      && sentMessages.some((message) => (
        message.type === "authoritative_recovery"
        && message.command?.mode === "reconnect_sync"
      ))
    ) {
      return;
    }
    await Promise.resolve();
  }
  throw new Error("local test auth startup did not complete its reconnect handshake");
}
