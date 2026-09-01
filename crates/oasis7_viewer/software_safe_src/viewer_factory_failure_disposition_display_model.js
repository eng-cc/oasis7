function isRecord(value) {
  return value != null && typeof value === "object" && !Array.isArray(value);
}

function displayableString(value) {
  if (typeof value === "string" && value.trim()) {
    return value.trim();
  }
  if (typeof value === "number" && Number.isFinite(value)) {
    return String(value);
  }
  return null;
}

function finiteNumber(value) {
  if (value == null || value === "") {
    return null;
  }
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : null;
}

function normalizedCode(value) {
  return displayableString(value)?.toLowerCase() || null;
}

function normalizedActionToken(value) {
  return normalizedCode(value)?.replace(/[^a-z0-9]+/g, "_").replace(/^_+|_+$/g, "") || null;
}

function actionField(action, snakeKey, camelKey) {
  return action?.[snakeKey] ?? action?.[camelKey] ?? null;
}

function recoveryActionForDisposition(value, availableActions, locale, localeText) {
  const actions = Array.isArray(availableActions) ? availableActions : [];
  const recipeToken = normalizedActionToken(value.recipe_id ?? value.recipeId);
  const recipeSuffix = recipeToken?.startsWith("recipe_")
    ? recipeToken.slice("recipe_".length)
    : recipeToken;
  const isExecutable = (action) => ["gameplay_action", "request_snapshot", "step", "play"]
    .includes(normalizedCode(actionField(action, "execute_kind", "executeKind")));
  const matchingScheduleAction = actions.find((action) => {
    if (!isExecutable(action)) return false;
    const actionId = normalizedActionToken(actionField(action, "action_id", "actionId"));
    return actionId?.startsWith("schedule_recipe_")
      && recipeSuffix
      && (actionId.endsWith(`_${recipeSuffix}`) || actionId.endsWith(`_${recipeToken}`));
  });
  const snapshotAction = actions.find((action) => {
    const actionId = normalizedActionToken(actionField(action, "action_id", "actionId"));
    const protocolAction = normalizedActionToken(actionField(action, "protocol_action", "protocolAction"));
    return isExecutable(action) && (actionId === "request_snapshot" || protocolAction === "request_snapshot");
  });
  const selected = matchingScheduleAction || snapshotAction || {
    actionId: "request_snapshot",
    label: localeText(locale, "刷新玩法快照", "Refresh gameplay snapshot"),
    protocolAction: "request_snapshot",
    targetAgentId: null,
    disabledReason: null,
    executeKind: "request_snapshot",
  };
  const actionId = displayableString(actionField(selected, "action_id", "actionId")) || "request_snapshot";
  const label = displayableString(selected.label)
    || localeText(locale, "刷新玩法快照", "Refresh gameplay snapshot");
  return {
    actionId,
    label,
    protocolAction: displayableString(actionField(selected, "protocol_action", "protocolAction")) || "request_snapshot",
    targetAgentId: displayableString(actionField(selected, "target_agent_id", "targetAgentId")),
    disabledReason: displayableString(actionField(selected, "disabled_reason", "disabledReason")),
    executeKind: displayableString(actionField(selected, "execute_kind", "executeKind")) || "request_snapshot",
  };
}

function localizedCodeLabel(code, locale, localeText, labels, fallbackZh, fallbackEn) {
  const label = labels[code];
  return label
    ? localeText(locale, label[0], label[1])
    : localeText(locale, fallbackZh, fallbackEn);
}

function normalizeInputs(value) {
  if (!Array.isArray(value)) {
    return [];
  }
  return value
    .filter(isRecord)
    .map((stack) => {
      const kind = displayableString(stack.kind ?? stack.material_kind ?? stack.materialKind);
      if (!kind) {
        return null;
      }
      return {
        kind,
        amount: finiteNumber(stack.amount),
      };
    })
    .filter(Boolean);
}

const BLOCKER_LABELS = {
  product_validation: ["产品验证失败", "Product validation failed"],
  product_validation_rejected: ["产品验证失败", "Product validation failed"],
};

const DISPOSITION_LABELS = {
  consumed_lost: ["投入已消费且损失", "Inputs consumed and lost"],
};

export function normalizeFactoryProductionFailureDisposition(
  value,
  locale,
  localeText,
  availableActions = [],
) {
  if (!isRecord(value)) {
    return null;
  }

  const blockerCode = normalizedCode(value.blocker_kind ?? value.blockerKind);
  const dispositionCode = normalizedCode(value.disposition_kind ?? value.dispositionKind);
  const recoveryAction = recoveryActionForDisposition(value, availableActions, locale, localeText);
  const nextRecheck = finiteNumber(value.next_recheck ?? value.nextRecheck);

  return {
    actionId: displayableString(value.action_id ?? value.actionId),
    requesterAgentId: displayableString(value.requester_agent_id ?? value.requesterAgentId),
    factoryId: displayableString(value.factory_id ?? value.factoryId),
    recipeId: displayableString(value.recipe_id ?? value.recipeId),
    blockerKind: localizedCodeLabel(
      blockerCode,
      locale,
      localeText,
      BLOCKER_LABELS,
      "生产受阻",
      "Production blocked",
    ),
    blockerDetail: displayableString(value.blocker_detail ?? value.blockerDetail),
    dispositionKind: localizedCodeLabel(
      dispositionCode,
      locale,
      localeText,
      DISPOSITION_LABELS,
      "处置已记录",
      "Disposition recorded",
    ),
    consumedInputs: normalizeInputs(value.consumed_inputs ?? value.consumedInputs),
    lostInputs: normalizeInputs(value.lost_inputs ?? value.lostInputs),
    consumedPower: finiteNumber(value.consumed_power ?? value.consumedPower),
    lostPower: finiteNumber(value.lost_power ?? value.lostPower),
    nextAction: recoveryAction.label,
    recoveryAction,
    recoveryActionId: recoveryAction.actionId,
    recoveryActionLabel: recoveryAction.label,
    recoveryActionDisabledReason: recoveryAction.disabledReason,
    nextRecheck,
    nextRecheckBoundary: nextRecheck == null
      ? localeText(locale, "下一次 committed 快照", "next committed snapshot")
      : localeText(locale, `世界时刻 ${nextRecheck}`, `world tick ${nextRecheck}`),
  };
}
