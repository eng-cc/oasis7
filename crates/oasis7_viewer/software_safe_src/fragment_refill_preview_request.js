export function createFragmentRefillPreviewRequestModule({
  buildAuthEnvelope, clone, ensureHostedPlayerAuthAvailable, ensureRegisteredPlayerSession,
  getSocket, nextAuthNonce, sendJson, signAuthPayload, state,
}) {
  async function buildAuthProof(request, auth) {
    const nonce = nextAuthNonce();
    const signingPayload = buildAuthEnvelope({
      operation: "gameplay_action",
      action_id: "preview_fragment_replenishment",
      target_agent_id: `chunk:${request.chunk.x}:${request.chunk.y}:${request.chunk.z}`,
      player_id: auth.playerId,
      public_key: auth.publicKey,
      nonce,
    });
    return { scheme: "ed25519", player_id: auth.playerId, public_key: auth.publicKey, nonce, signature: await signAuthPayload(signingPayload, auth) };
  }

  async function requestFragmentRefillPreview(x, y, z) {
    if (state.fragmentRefillPreviewRequest?.status === "pending") return { ok: false, reason: "fragment refill preview request already pending" };
    const chunk = { x: Number(x), y: Number(y), z: Number(z) };
    if (!Object.values(chunk).every(Number.isSafeInteger)) {
      const reason = "fragment refill preview requires whole-number chunk coordinates";
      state.fragmentRefillPreviewRequest = { status: "error", error: reason };
      return { ok: false, reason };
    }
    const socket = getSocket();
    if (!socket || socket.readyState !== WebSocket.OPEN) {
      const reason = "fragment refill preview requires a connected viewer websocket";
      state.fragmentRefillPreviewRequest = { status: "error", error: reason };
      return { ok: false, reason };
    }
    try {
      await ensureHostedPlayerAuthAvailable();
      if (!state.auth.available) {
        const reason = state.auth.error || "fragment refill preview requires an active player session";
        state.fragmentRefillPreviewRequest = { status: "error", error: reason };
        return { ok: false, reason };
      }
      const boundAgentId = String(state.auth.boundAgentId || "").trim();
      if (!boundAgentId) {
        const reason = "fragment refill preview requires a bound player Agent";
        state.fragmentRefillPreviewRequest = { status: "error", error: reason };
        return { ok: false, reason };
      }
      await ensureRegisteredPlayerSession(boundAgentId);
      const request = { chunk, player_id: state.auth.playerId, public_key: state.auth.publicKey };
      request.auth = await buildAuthProof(request, state.auth);
      state.fragmentRefillPreview = null;
      state.fragmentRefillPreviewRequest = { status: "pending", error: null, requestKey: `${chunk.x}:${chunk.y}:${chunk.z}` };
      sendJson({ type: "preview_fragment_replenishment", request });
      return { ok: true, request: clone(request) };
    } catch (error) {
      const reason = `fragment refill preview request failed: ${String(error)}`;
      state.fragmentRefillPreviewRequest = { status: "error", error: reason };
      return { ok: false, reason };
    }
  }

  return { requestFragmentRefillPreview };
}
