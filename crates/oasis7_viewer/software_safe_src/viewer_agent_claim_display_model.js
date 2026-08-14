export function createViewerAgentClaimDisplayModel({ state, tr }) {
  function normalizedId(value) {
    return String(value || "").trim();
  }

  function slot1ClaimChoiceQuote(agentClaim) {
    const quote = agentClaim?.next_claim_quote || agentClaim?.nextClaimQuote || {};
    return quote?.slot_1_claim_choice_quote || quote?.slot1ClaimChoiceQuote || null;
  }

  function publishedClaimChoiceCandidates(snapshot, agentClaim) {
    const choiceQuote = slot1ClaimChoiceQuote(agentClaim);
    const publishedCandidates = Array.isArray(choiceQuote?.candidates) ? choiceQuote.candidates : [];
    const agents = snapshot?.model?.agents || {};
    return publishedCandidates
      .map((candidate) => {
        const id = normalizedId(candidate?.agent_id || candidate?.agentId || candidate?.id);
        if (!id) return null;
        return {
          ...candidate,
          id,
          name: candidate?.name || agents[id]?.name || id,
          isClaimer: id === normalizedId(agentClaim?.claimer_agent_id || agentClaim?.claimerAgentId),
        };
      })
      .filter(Boolean);
  }

  function buildAgentClaimTargets(snapshot, agentClaim) {
    const agents = snapshot?.model?.agents || {};
    const publishedCandidates = publishedClaimChoiceCandidates(snapshot, agentClaim);
    if (publishedCandidates.length > 0) {
      return publishedCandidates;
    }
    const ownedTargets = new Set(
      Array.isArray(agentClaim?.owned_claims)
        ? agentClaim.owned_claims.map((claim) => String(claim?.target_agent_id || "").trim()).filter(Boolean)
        : [],
    );
    const claimerAgentId = String(agentClaim?.claimer_agent_id || "").trim();
    const candidates = Object.keys(agents)
      .filter((agentId) => !ownedTargets.has(agentId))
      .map((agentId) => ({
        id: agentId,
        name: agents[agentId]?.name || agentId,
        isClaimer: agentId === claimerAgentId,
      }));
    const unclaimedNonActor = candidates.filter((candidate) => !candidate.isClaimer);
    return unclaimedNonActor.length > 0 ? unclaimedNonActor : candidates;
  }

  function agentBindingForId(agentId, snapshot = state.snapshot) {
    const id = normalizedId(agentId);
    if (!id) {
      return { playerId: null, publicKey: null };
    }
    return {
      playerId: snapshot?.model?.agent_player_bindings?.[id] || null,
      publicKey: snapshot?.model?.agent_player_public_key_bindings?.[id] || null,
    };
  }

  function describeAgentSessionStatus(agentId, locale, snapshot = state.snapshot) {
    const id = normalizedId(agentId);
    const boundAgentId = normalizedId(state.auth.boundAgentId);
    const playerId = normalizedId(state.auth.playerId);
    const binding = agentBindingForId(id, snapshot);
    const boundPlayerId = normalizedId(binding.playerId);
    const isCurrentBoundAgent = Boolean(id && boundAgentId && id === boundAgentId);
    const isBoundToCurrentPlayer = Boolean(boundPlayerId && playerId && boundPlayerId === playerId);

    if (isCurrentBoundAgent) {
      return {
        kind: "current", isCurrentSessionAgent: true,
        badge: tr(locale, "我的 Agent", "My Agent"),
        detail: tr(locale, "当前会话绑定，可执行聊天和指挥。", "Bound to the current session; chat and command controls are available."),
        badgeClass: "badge badge--good", binding,
      };
    }
    if (isBoundToCurrentPlayer) {
      return {
        kind: "current_player_binding_pending", isCurrentSessionAgent: false,
        badge: tr(locale, "绑定待同步", "Binding Pending"),
        detail: tr(locale, "快照显示这个 Agent 绑定到当前玩家，但当前会话还没有同步 boundAgent；聊天和指挥暂不开放。", "The snapshot shows this Agent bound to the current player, but this session has not synced boundAgent yet; chat and command stay unavailable."),
        badgeClass: "badge badge--accent", binding,
      };
    }
    if (boundPlayerId) {
      return {
        kind: "other_bound", isCurrentSessionAgent: false,
        badge: tr(locale, "已隐藏", "Hidden"),
        detail: tr(locale, "这个 Agent 已绑定到其他账号，默认不在当前账号的 Agent 列表中展示。", "This Agent is bound to another account and is hidden from the current account's Agent list by default."),
        badgeClass: "badge badge--warn", binding,
      };
    }
    return {
      kind: "unbound_agent_hidden", isCurrentSessionAgent: false,
      badge: tr(locale, "未绑定", "Unbound"),
      detail: tr(locale, "这个 Agent 没有账号绑定，默认不在玩家 Agent 列表中展示。", "This Agent has no account binding and is hidden from the player Agent list by default."),
      badgeClass: "badge badge--warn", binding,
    };
  }

  function agentClaimUsesCurrentBoundAgent(agentClaim) {
    const claimerAgentId = normalizedId(agentClaim?.claimer_agent_id);
    const boundAgentId = normalizedId(state.auth.boundAgentId);
    return Boolean(claimerAgentId && boundAgentId && claimerAgentId === boundAgentId);
  }

  function slot1ClaimChoiceNeedsDefer(agentClaim) {
    const choiceQuote = slot1ClaimChoiceQuote(agentClaim);
    const status = normalizedId(choiceQuote?.status || choiceQuote?.choiceStatus);
    const fallbackReason = normalizedId(choiceQuote?.fallback_reason || choiceQuote?.fallbackReason);
    const choiceClass = normalizedId(
      choiceQuote?.claim_choice_class
        || choiceQuote?.claimChoiceClass
        || choiceQuote?.recommended_claim_action
        || choiceQuote?.recommendedClaimAction,
    );
    const rationaleMissing = status === "candidate_rationale_missing"
      || fallbackReason === "candidate_rationale_missing";
    return rationaleMissing && choiceClass === "wait_or_fund_first";
  }

  function buildAgentClaimAction(agentClaim, targetAgentId) {
    const claimerAgentId = String(agentClaim?.claimer_agent_id || "").trim();
    const boundAgentId = normalizedId(state.auth.boundAgentId);
    const target = String(targetAgentId || "").trim();
    const blockedReason = String(agentClaim?.next_claim_quote?.blocked_reason || "").trim();
    if (!claimerAgentId || !target || blockedReason || !boundAgentId || claimerAgentId !== boundAgentId) return null;
    const disabledReason = slot1ClaimChoiceNeedsDefer(agentClaim) ? "candidate_rationale_missing" : null;
    return {
      actionId: "claim_agent", action_id: "claim_agent", label: "Claim Agent",
      protocolAction: "gameplay_action.submit", protocol_action: "gameplay_action.submit",
      executeKind: "claim_agent", targetAgentId: target, target_agent_id: target,
      actorAgentId: claimerAgentId, actor_agent_id: claimerAgentId,
      disabledReason, disabled_reason: disabledReason,
    };
  }

  function hasExecutableAgentClaim(snapshot, agentClaim) {
    if (!agentClaim || !agentClaimUsesCurrentBoundAgent(agentClaim) || String(agentClaim?.next_claim_quote?.blocked_reason || "").trim()) return false;
    const targets = buildAgentClaimTargets(snapshot, agentClaim);
    return targets.length > 0 && Boolean(buildAgentClaimAction(agentClaim, targets[0]?.id));
  }

  function hasAgentClaimSessionBoundary(agentClaim) {
    return Boolean(agentClaim?.next_claim_quote) && !agentClaimUsesCurrentBoundAgent(agentClaim);
  }

  return { agentBindingForId, agentClaimUsesCurrentBoundAgent, buildAgentClaimAction, buildAgentClaimTargets, describeAgentSessionStatus, hasAgentClaimSessionBoundary, hasExecutableAgentClaim, normalizedId, publishedClaimChoiceCandidates, slot1ClaimChoiceNeedsDefer, slot1ClaimChoiceQuote };
}
