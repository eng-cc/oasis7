function isRecord(value) {
  return value != null && typeof value === "object" && !Array.isArray(value);
}

function stageLabel(stage) {
  return ({
    bootstrap: "起步",
    scale_out: "规模扩展",
    governance: "治理",
    none: "无要求",
    unknown: "未知",
  })[stage] || stage;
}

export function buildValidationUnlockPreviewDisplayModel(rawPreview, locale, isLocaleZh) {
  if (!isRecord(rawPreview)) {
    return null;
  }
  const productId = rawPreview.product_id || rawPreview.productId || null;
  const roleTag = rawPreview.role_tag || rawPreview.roleTag || "unknown";
  const tradable = typeof rawPreview.tradable === "boolean" ? rawPreview.tradable : null;
  const requiredStage = rawPreview.required_stage || rawPreview.requiredStage || "unknown";
  const currentStage = rawPreview.current_stage || rawPreview.currentStage || "unknown";
  const stageStatus = rawPreview.stage_status || rawPreview.stageStatus || "unknown";
  const valueSummary = rawPreview.value_summary || rawPreview.valueSummary || null;
  const nextStepHint = rawPreview.next_step_hint || rawPreview.nextStepHint || null;
  if (!isLocaleZh(locale)) {
    return {
      productId, roleTag, roleLabel: roleTag, tradable,
      requiredStage, requiredStageLabel: requiredStage,
      currentStage, currentStageLabel: currentStage,
      stageStatus, stageStatusLabel: stageStatus,
      valueSummary, localizedValueSummary: valueSummary,
      nextStepHint, localizedNextStepHint: nextStepHint,
    };
  }
  const roleLabel = ({ bootstrap: "启动", scale: "规模化", governance: "治理", unknown: "未知" })[roleTag] || roleTag;
  const requiredStageLabel = stageLabel(requiredStage);
  const currentStageLabel = stageLabel(currentStage);
  const stageStatusLabel = ({ available: "可用", denied: "未满足", unknown: "未知" })[stageStatus] || stageStatus;
  const localizedValueSummary = stageStatus === "available"
    ? `已验证${roleLabel}产品；${tradable ? "已启用交易" : "未启用交易"}。`
    : stageStatus === "denied"
      ? `已验证${roleLabel}产品仍受阶段 ${requiredStageLabel} 限制。`
      : `已验证${roleLabel}产品的阶段要求未知。`;
  const localizedNextStepHint = stageStatus === "available"
    ? `将此产品用于${roleLabel}角色；验证不会解锁新能力。`
    : stageStatus === "denied"
      ? `将产业从${currentStageLabel}推进至${requiredStageLabel}；验证不会解锁新能力。`
      : "请先查看受治理的产品档案，再依赖此验证。";
  return {
    productId, roleTag, roleLabel, tradable,
    requiredStage, requiredStageLabel,
    currentStage, currentStageLabel,
    stageStatus, stageStatusLabel,
    valueSummary, localizedValueSummary,
    nextStepHint, localizedNextStepHint,
  };
}
