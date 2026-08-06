export function createPowerSaleQuoteRequestModule({
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
      action_id: "quote_power_sale",
      target_agent_id: `buyer_agent_id:${request.buyer_agent_id}|amount:${request.amount}|requested_price_per_pu:${request.requested_price_per_pu}`,
      player_id: auth.playerId,
      public_key: auth.publicKey,
      nonce,
    });
    return { scheme: "ed25519", player_id: auth.playerId, public_key: auth.publicKey, nonce, signature: await signAuthPayload(signingPayload, auth) };
  }

  async function requestPowerSaleQuote(buyerAgentId, amount, requestedPricePerPu) {
    if (state.powerSaleQuoteRequest?.status === "pending") return { ok: false, reason: "power sale quote request already pending" };
    const buyer = String(buyerAgentId || "").trim();
    const amountNumber = Number(amount);
    const priceNumber = Number(requestedPricePerPu);
    if (!buyer || !Number.isSafeInteger(amountNumber) || amountNumber <= 0 || !Number.isSafeInteger(priceNumber) || priceNumber < 0) {
      const reason = "power sale quote requires a buyer, positive whole-number amount, and non-negative whole-number price";
      state.powerSaleQuoteRequest = { status: "error", error: reason };
      return { ok: false, reason };
    }
    const socket = getSocket();
    if (!socket || socket.readyState !== WebSocket.OPEN) {
      const reason = "power sale quote requires a connected viewer websocket";
      state.powerSaleQuoteRequest = { status: "error", error: reason };
      return { ok: false, reason };
    }
    try {
      await ensureHostedPlayerAuthAvailable();
      if (!state.auth.available) {
        const reason = state.auth.error || "power sale quote requires an active player session";
        state.powerSaleQuoteRequest = { status: "error", error: reason };
        return { ok: false, reason };
      }
      const seller = String(state.auth.boundAgentId || "").trim();
      if (!seller) {
        const reason = "power sale quote requires a bound player Agent";
        state.powerSaleQuoteRequest = { status: "error", error: reason };
        return { ok: false, reason };
      }
      await ensureRegisteredPlayerSession(seller);
      const request = { buyer_agent_id: buyer, amount: amountNumber, requested_price_per_pu: priceNumber, player_id: state.auth.playerId, public_key: state.auth.publicKey };
      request.auth = await buildAuthProof(request, state.auth);
      state.powerSaleQuote = null;
      state.powerSaleQuoteRequest = { status: "pending", error: null };
      sendJson({ type: "quote_power_sale", request });
      return { ok: true, request: clone(request) };
    } catch (error) {
      const reason = `power sale quote request failed: ${String(error)}`;
      state.powerSaleQuoteRequest = { status: "error", error: reason };
      return { ok: false, reason };
    }
  }

  return { requestPowerSaleQuote };
}
