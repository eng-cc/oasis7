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

const NEXT_ACTION_LABELS = {
  inspect_product_validation_and_reschedule: ["检查产品验证并重新排程", "Inspect product validation and reschedule"],
};

export function normalizeFactoryProductionFailureDisposition(value, locale, localeText) {
  if (!isRecord(value)) {
    return null;
  }

  const blockerCode = normalizedCode(value.blocker_kind ?? value.blockerKind);
  const dispositionCode = normalizedCode(value.disposition_kind ?? value.dispositionKind);
  const nextActionCode = normalizedCode(value.next_action ?? value.nextAction);

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
    nextAction: localizedCodeLabel(
      nextActionCode,
      locale,
      localeText,
      NEXT_ACTION_LABELS,
      "按已发布的下一步处理",
      "Follow the published next step",
    ),
    nextRecheck: finiteNumber(value.next_recheck ?? value.nextRecheck),
  };
}
