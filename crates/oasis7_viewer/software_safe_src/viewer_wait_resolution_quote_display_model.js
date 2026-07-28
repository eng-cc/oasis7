function displayableString(value) {
  return typeof value === "string" && value.trim() ? value.trim() : null;
}

export function buildWaitResolutionQuoteDisplayModel(rawQuote, locale, localeText) {
  if (!rawQuote || typeof rawQuote !== "object" || Array.isArray(rawQuote)) {
    return null;
  }

  const resolutionTrigger = displayableString(rawQuote.resolution_trigger ?? rawQuote.resolutionTrigger);
  const recheckTickOrEvent = displayableString(rawQuote.recheck_tick_or_event ?? rawQuote.recheckTickOrEvent);
  const expectedChange = displayableString(rawQuote.expected_change ?? rawQuote.expectedChange);
  const unresolvedRisk = displayableString(rawQuote.unresolved_risk ?? rawQuote.unresolvedRisk);
  const alternativeUnlockCondition = displayableString(
    rawQuote.alternative_unlock_condition ?? rawQuote.alternativeUnlockCondition,
  );
  if (![resolutionTrigger, recheckTickOrEvent, expectedChange, unresolvedRisk, alternativeUnlockCondition].some(Boolean)) {
    return null;
  }

  const safeToWait = rawQuote.safe_to_wait === true || rawQuote.safeToWait === true;
  return {
    safeToWait,
    resolutionTrigger,
    recheckTickOrEvent,
    expectedChange,
    unresolvedRisk,
    alternativeUnlockCondition,
    fallbackTradeoffOption: {
      valueClass: "safe_wait",
      available: safeToWait,
      reason: [
        `${localeText(locale, "触发条件", "Trigger")}: ${resolutionTrigger || "—"}`,
        `${localeText(locale, "未解决风险", "Unresolved risk")}: ${unresolvedRisk || "—"}`,
      ].join(" · "),
      progressKept: `${localeText(locale, "预期变化", "Expected change")}: ${expectedChange || "—"}`,
      cost: `${localeText(locale, "复查点", "Recheck")}: ${recheckTickOrEvent || "—"}`,
      opportunityCost: `${localeText(locale, "替代解锁条件", "Alternative unlock")}: ${alternativeUnlockCondition || "—"}`,
      recommended: safeToWait,
    },
  };
}
