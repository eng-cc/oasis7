// Runtime messages report effects; they never install or repair signing identity.
export function validateRuntimeAckIdentity(ack, auth, registration = false, requestedAgentId = null) {
  const player = ack?.player_id;
  const publicKey = ack?.session_pubkey ?? ack?.public_key;
  if (registration && (!player || !publicKey)) return "runtime ACK is missing identity";
  if (player != null && player !== auth?.playerId) return "runtime ACK player identity conflict";
  if (publicKey != null && String(publicKey).toLowerCase() !== String(auth?.publicKey).toLowerCase()) return "runtime ACK signing identity conflict";
  if (registration && ack?.status !== "session_revoked" && requestedAgentId != null && ack?.agent_id !== requestedAgentId) return "runtime registration ACK target conflict";
  return null;
}
