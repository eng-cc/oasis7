export function createPowerSurvivalQuoteRequestModule({
  buildAuthEnvelope,
  clone,
  ensureHostedPlayerAuthAvailable,
  ensureRegisteredPlayerSession,
  getSocket,
  nextAuthNonce,
  sendJson,
  signAuthPayload,
  state,
}) {
  async function buildAuthProof(request, auth) {
    const nonce = nextAuthNonce();
    const signingPayload = buildAuthEnvelope({
      operation: "gameplay_action",
      action_id: "quote_power_survival",
      target_agent_id: `seller_agent_id:${request.seller_agent_id}|amount:${request.amount}|requested_price_per_pu:${request.requested_price_per_pu}`,
      player_id: auth.playerId,
      public_key: auth.publicKey,
      nonce,
    });
    return { scheme: "ed25519", player_id: auth.playerId, public_key: auth.publicKey, nonce, signature: await signAuthPayload(signingPayload, auth) };
  }

  async function requestPowerSurvivalQuote(sellerAgentId, amount, requestedPricePerPu) {
    if (state.powerSurvivalQuoteRequest?.status === "pending") {
      return { ok: false, reason: "power survival quote request already pending" };
    }
    const seller = String(sellerAgentId || "").trim();
    const amountNumber = Number(amount);
    const priceNumber = Number(requestedPricePerPu);
    if (!seller || !Number.isSafeInteger(amountNumber) || amountNumber <= 0 || !Number.isSafeInteger(priceNumber) || priceNumber < 0) {
      const reason = "power survival quote requires a seller, positive whole-number amount, and non-negative whole-number price";
      state.powerSurvivalQuoteRequest = { status: "error", error: reason };
      return { ok: false, reason };
    }
    const socket = getSocket();
    if (!socket || socket.readyState !== WebSocket.OPEN) {
      const reason = "power survival quote requires a connected viewer websocket";
      state.powerSurvivalQuoteRequest = { status: "error", error: reason };
      return { ok: false, reason };
    }
    try {
      await ensureHostedPlayerAuthAvailable();
      if (!state.auth.available) {
        const reason = state.auth.error || "power survival quote requires an active player session";
        state.powerSurvivalQuoteRequest = { status: "error", error: reason };
        return { ok: false, reason };
      }
      const boundAgentId = String(state.auth.boundAgentId || "").trim();
      if (!boundAgentId) {
        const reason = "power survival quote requires a bound player Agent";
        state.powerSurvivalQuoteRequest = { status: "error", error: reason };
        return { ok: false, reason };
      }
      await ensureRegisteredPlayerSession(boundAgentId);
      const request = { seller_agent_id: seller, amount: amountNumber, requested_price_per_pu: priceNumber, player_id: state.auth.playerId, public_key: state.auth.publicKey };
      request.auth = await buildAuthProof(request, state.auth);
      state.powerSurvivalQuote = null;
      state.powerSurvivalQuoteRequest = { status: "pending", error: null };
      sendJson({ type: "quote_power_survival", request });
      return { ok: true, request: clone(request) };
    } catch (error) {
      const reason = `power survival quote request failed: ${String(error)}`;
      state.powerSurvivalQuoteRequest = { status: "error", error: reason };
      return { ok: false, reason };
    }
  }

  return { requestPowerSurvivalQuote };
}
