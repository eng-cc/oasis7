export function createTransferMaterialQuoteRequestModule({ buildAuthEnvelope, clone, ensureHostedPlayerAuthAvailable, ensureRegisteredPlayerSession, getSocket, nextAuthNonce, sendJson, signAuthPayload, state }) {
  async function requestTransferMaterialQuote(requesterAgentId, fromLedger, toLedger, kind, amount, distanceKm, requestedPriority = null) {
    if (state.transferMaterialQuoteRequest?.status === "pending") return { ok: false, reason: "transfer material quote request already pending" };
    const agent = String(requesterAgentId || "").trim(); const from = String(fromLedger || "").trim(); const to = String(toLedger || "").trim(); const material = String(kind || "").trim(); const quantity = Number(amount); const distance = Number(distanceKm); const priority = requestedPriority == null || requestedPriority === "" ? null : String(requestedPriority).trim().toLowerCase();
    if (!agent || !from || !to || !material || !Number.isSafeInteger(quantity) || quantity <= 0 || !Number.isSafeInteger(distance) || distance < 0 || (priority !== null && !["urgent", "standard"].includes(priority))) {
      const reason = "transfer material quote requires source, destination, material, positive amount, non-negative distance, and a known priority";
      state.transferMaterialQuoteRequest = { status: "error", error: reason }; return { ok: false, reason };
    }
    const socket = getSocket();
    if (!socket || socket.readyState !== WebSocket.OPEN) {
      const reason = "transfer material quote requires a connected viewer websocket";
      state.transferMaterialQuoteRequest = { status: "error", error: reason }; return { ok: false, reason };
    }
    try {
      await ensureHostedPlayerAuthAvailable();
      if (!state.auth.available) {
        const reason = state.auth.error || "transfer material quote requires an active player session";
        state.transferMaterialQuoteRequest = { status: "error", error: reason }; return { ok: false, reason };
      }
      const boundAgentId = String(state.auth.boundAgentId || "").trim();
      if (!boundAgentId || boundAgentId !== agent) {
        const reason = "transfer material quote requires the requested Agent to be bound to this player session";
        state.transferMaterialQuoteRequest = { status: "error", error: reason }; return { ok: false, reason };
      }
      await ensureRegisteredPlayerSession(boundAgentId);
      const request = { requester_agent_id: agent, from_ledger: from, to_ledger: to, kind: material, amount: quantity, distance_km: distance, requested_priority: priority, player_id: state.auth.playerId, public_key: state.auth.publicKey };
      const nonce = nextAuthNonce();
      const payload = buildAuthEnvelope({ operation: "gameplay_action", action_id: "quote_transfer_material", target_agent_id: JSON.stringify({ amount: quantity, distance_km: distance, from_ledger: from, kind: material, requested_priority: priority, requester_agent_id: agent, to_ledger: to }), player_id: state.auth.playerId, public_key: state.auth.publicKey, nonce });
      request.auth = { scheme: "ed25519", player_id: state.auth.playerId, public_key: state.auth.publicKey, nonce, signature: await signAuthPayload(payload, state.auth) };
      state.transferMaterialQuote = null;
      state.transferMaterialQuoteRequest = { status: "pending", error: null };
      sendJson({ type: "quote_transfer_material", request });
      return { ok: true, request: clone(request) };
    } catch (error) {
      const reason = `transfer material quote request failed: ${String(error)}`;
      state.transferMaterialQuoteRequest = { status: "error", error: reason }; return { ok: false, reason };
    }
  }
  return { requestTransferMaterialQuote };
}
