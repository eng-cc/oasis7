const CURRENT_WORLD_FEED_STATUSES = new Set(["ready", "replay", "empty"]);
const AGENT_CHAT_AUTHORITY_SCOPE = "player_agent_chat";

function currentRuntimeAuthority(state) {
  const feed = state?.worldFeed;
  const worldId = String(feed?.worldId || "").trim();
  const reorgEpoch = Number(feed?.reorgEpoch);
  if (
    !CURRENT_WORLD_FEED_STATUSES.has(feed?.status)
    || feed?.stale === true
    || !worldId
    || !Number.isSafeInteger(reorgEpoch)
    || reorgEpoch < 0
  ) {
    throw new Error("agent_chat requires the current runtime world authority");
  }
  return {
    world_id: worldId,
    reorg_epoch: reorgEpoch,
    authority_scope: AGENT_CHAT_AUTHORITY_SCOPE,
  };
}

export function createViewerAgentChatAuthModule({
  buildAuthEnvelope,
  nextAuthNonce,
  signAuthPayload,
  state,
}) {
  async function buildAuthProof(request, auth) {
    const authority = currentRuntimeAuthority(state);
    const nonce = nextAuthNonce();
    request.world_id = authority.world_id;
    request.reorg_epoch = authority.reorg_epoch;
    request.authority_scope = authority.authority_scope;
    const payload = {
      operation: "agent_chat",
      agent_id: request.agent_id,
      player_id: auth.playerId,
      public_key: auth.publicKey,
      nonce,
      message: request.message,
    };
    if (request.intent_tick != null) {
      payload.intent_tick = request.intent_tick;
    }
    if (request.intent_seq != null) {
      payload.intent_seq = request.intent_seq;
    }
    payload.world_id = authority.world_id;
    payload.reorg_epoch = authority.reorg_epoch;
    payload.authority_scope = authority.authority_scope;
    if (request.replaces_intent_id != null) {
      payload.replaces_intent_id = request.replaces_intent_id;
    }
    const signingPayload = buildAuthEnvelope(payload);
    return {
      scheme: "ed25519",
      player_id: auth.playerId,
      public_key: auth.publicKey,
      nonce,
      signature: await signAuthPayload(signingPayload, auth),
    };
  }

  return { buildAuthProof };
}
