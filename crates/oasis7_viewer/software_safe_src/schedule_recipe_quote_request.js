export function createScheduleRecipeQuoteRequestModule({ buildAuthEnvelope, clone, ensureHostedPlayerAuthAvailable, ensureRegisteredPlayerSession, getSocket, nextAuthNonce, sendJson, signAuthPayload, state }) {
  async function requestScheduleRecipeQuote(factoryId, recipeId, batches) {
    if (state.scheduleRecipeQuoteRequest?.status === "pending") return { ok: false, reason: "schedule recipe quote request already pending" };
    const factory = String(factoryId || "").trim(); const recipe = String(recipeId || "").trim(); const batchCount = Number(batches);
    if (!factory || !recipe || !Number.isSafeInteger(batchCount) || batchCount <= 0) {
      const reason = "schedule recipe quote requires a factory, recipe, and positive whole-number batches";
      state.scheduleRecipeQuoteRequest = { status: "error", error: reason }; return { ok: false, reason };
    }
    const socket = getSocket();
    if (!socket || socket.readyState !== WebSocket.OPEN) {
      const reason = "schedule recipe quote requires a connected viewer websocket";
      state.scheduleRecipeQuoteRequest = { status: "error", error: reason }; return { ok: false, reason };
    }
    try {
      await ensureHostedPlayerAuthAvailable();
      if (!state.auth.available) {
        const reason = state.auth.error || "schedule recipe quote requires an active player session";
        state.scheduleRecipeQuoteRequest = { status: "error", error: reason }; return { ok: false, reason };
      }
      const boundAgentId = String(state.auth.boundAgentId || "").trim();
      if (!boundAgentId) {
        const reason = "schedule recipe quote requires a bound player Agent";
        state.scheduleRecipeQuoteRequest = { status: "error", error: reason }; return { ok: false, reason };
      }
      await ensureRegisteredPlayerSession(boundAgentId);
      const request = { factory_id: factory, recipe_id: recipe, batches: batchCount, player_id: state.auth.playerId, public_key: state.auth.publicKey };
      const nonce = nextAuthNonce();
      const payload = buildAuthEnvelope({ operation: "gameplay_action", action_id: "quote_schedule_recipe", target_agent_id: `factory_id:${factory}|recipe_id:${recipe}|batches:${batchCount}`, player_id: state.auth.playerId, public_key: state.auth.publicKey, nonce });
      request.auth = { scheme: "ed25519", player_id: state.auth.playerId, public_key: state.auth.publicKey, nonce, signature: await signAuthPayload(payload, state.auth) };
      state.scheduleRecipeQuote = null;
      state.scheduleRecipeQuoteRequest = { status: "pending", error: null };
      sendJson({ type: "quote_schedule_recipe", request });
      return { ok: true, request: clone(request) };
    } catch (error) {
      const reason = `schedule recipe quote request failed: ${String(error)}`;
      state.scheduleRecipeQuoteRequest = { status: "error", error: reason }; return { ok: false, reason };
    }
  }
  return { requestScheduleRecipeQuote };
}
