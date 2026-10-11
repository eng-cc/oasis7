import { canonicalAgentChatView } from "./viewer_canonical_owner_read_module.js";
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
    const canonical = canonicalAgentChatView(state);
    const required = state.viewerProtocol?.capabilities?.includes("canonical_agent_chat_v1");
    if (canonical != null || required) {
      const extension = canonical?.canonical_authority;
      if (!canonical || canonical.agent_id !== request.agent_id
        || canonical.player_id !== auth.playerId || canonical.public_key !== auth.publicKey
        || canonical.world_id !== authority.world_id || canonical.reorg_epoch !== authority.reorg_epoch
        || canonical.authority_scope !== AGENT_CHAT_AUTHORITY_SCOPE
        || typeof extension?.branch_id !== "string" || !extension.branch_id.trim()
        || (canonical.current_intent_id != null && (typeof canonical.current_intent_id !== "string" || !canonical.current_intent_id.trim()))
        || !Number.isSafeInteger(extension.agent_identity_generation) || extension.agent_identity_generation < 0) {
        throw new Error("agent_chat requires current authenticated canonical owner authority");
      }
      request.canonical_authority = { branch_id: extension.branch_id, agent_identity_generation: extension.agent_identity_generation };
      if (canonical.current_intent_id != null) request.replaces_intent_id = canonical.current_intent_id;
    }
    const nonce = nextAuthNonce();
    if (request.canonical_authority) {
      if (!Number.isSafeInteger(state.logicalTime) || state.logicalTime < 0) throw new Error("canonical goal requires current logical time");
      request.intent_seq = nonce;
      request.intent_tick = state.logicalTime;
    }
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
    if (request.canonical_authority != null) payload.canonical_authority = request.canonical_authority;
    const signingPayload = buildAuthEnvelope(payload);
    const signature = await signAuthPayload(signingPayload, auth);
    if (request.canonical_authority) {
      const current = canonicalAgentChatView(state);
      const runtime = currentRuntimeAuthority(state);
      if (current?.player_id !== auth.playerId || current?.public_key !== auth.publicKey
        || current?.agent_id !== request.agent_id || runtime.world_id !== request.world_id
        || runtime.reorg_epoch !== request.reorg_epoch
        || current?.canonical_authority?.branch_id !== request.canonical_authority.branch_id
        || current?.canonical_authority?.agent_identity_generation !== request.canonical_authority.agent_identity_generation
        || (current?.current_intent_id ?? null) !== (request.replaces_intent_id ?? null)) {
        throw new Error("canonical owner authority changed during signing");
      }
    }
    return {
      scheme: "ed25519",
      player_id: auth.playerId,
      public_key: auth.publicKey,
      nonce,
      signature,
    };
  }

  return { buildAuthProof };
}
