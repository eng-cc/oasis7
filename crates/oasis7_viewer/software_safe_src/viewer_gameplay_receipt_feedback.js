export function projectGameplayReceiptFeedback(gameplay, localFeedback, intentTarget) {
    const runtimeRecentFeedback = gameplay.recent_feedback && typeof gameplay.recent_feedback === "object"
      ? {
          source: "runtime",
          agentId: intentTarget,
          action: gameplay.recent_feedback.action || null,
          stage: gameplay.recent_feedback.stage || null,
          effect: gameplay.recent_feedback.effect || null,
          reason: gameplay.recent_feedback.reason || null,
          hint: gameplay.recent_feedback.hint || null,
          deltaLogicalTime: Number(gameplay.recent_feedback.delta_logical_time || 0),
          deltaEventSeq: Number(gameplay.recent_feedback.delta_event_seq || 0),
        }
      : null;
    const localGameplayFeedback = localFeedback?.kind === "gameplay_action"
      ? {
          source: "local_gameplay_action",
          agentId: Object.hasOwn(localFeedback, "agentId") ? localFeedback.agentId : localFeedback.targetAgentId ?? null,
          action: localFeedback.action || null,
          stage: localFeedback.stage || null,
          effect: localFeedback.effect || null,
          reason: localFeedback.reason || null,
          hint: localFeedback.response?.hint || null,
          deltaLogicalTime: Number(localFeedback.deltaLogicalTime || 0),
          deltaEventSeq: Number(localFeedback.deltaEventSeq || 0),
        }
      : null;
    const recentFeedback = localGameplayFeedback || runtimeRecentFeedback;
  return {runtimeRecentFeedback,localGameplayFeedback,recentFeedback};
}
