function known(value, labels, locale, tr) {
  const label = labels[String(value || "")];
  return label ? tr(locale, label[0], label[1]) : tr(locale, "状态暂不可用", "Status unavailable");
}

const settlementPath = { core_fallback: ["核心结算预估", "Core settlement quote"] };
const conflictStatus = {
  none: ["当前没有冲突阻塞", "No current conflict blocker"],
  active_conflict: ["当前冲突尚未结算", "An active conflict is not settled"],
  pending_conflict: ["已有宣战正在等待处理", "A war declaration is pending processing"],
};
const projectedOutcome = {
  aggressor_wins: ["攻击方预计获胜", "Aggressor projected to win"],
  defender_wins: ["防守方预计获胜", "Defender projected to win"],
};
const risk = {
  resource_and_reputation_change: ["结算会改变参与者资源和声望；不保证模块奖励。", "Settlement changes participant resources and reputation; module rewards are not guaranteed."],
  loss_resource_and_reputation: ["预计失利会改变失败方资源和声望。", "A projected loss changes losing members' resources and reputation."],
};
const action = {
  wait: ["等待当前冲突结算", "Wait for the current conflict to settle"],
  gather_resources: ["先收集动员资源", "Gather mobilization resources first"],
  recruit: ["先招募或提高强度", "Recruit or raise intensity first"],
  declare_war: ["可据此评估宣战；提交时仍会重新校验", "You may evaluate declaring war; submission will revalidate"],
  negotiate: ["先谈判", "Negotiate first"],
};

export function buildWarDeclarationQuoteDisplayModel(quote, locale, tr) {
  const q = quote || {};
  return {
    settlementPath: known(q.settlement_path, settlementPath, locale, tr),
    conflictStatus: known(q.conflict_status, conflictStatus, locale, tr),
    projectedOutcome: known(q.projected_outcome, projectedOutcome, locale, tr),
    risk: known(q.settlement_risk_code, risk, locale, tr),
    recommendedAction: known(q.recommended_war_action, action, locale, tr),
    alternativeAction: known(q.alternative_action, action, locale, tr),
    affordability: q.mobilization_affordable === true
      ? tr(locale, "动员资源充足", "Mobilization resources available")
      : q.mobilization_affordable === false
        ? tr(locale, "动员资源不足", "Mobilization resources missing")
        : tr(locale, "状态暂不可用", "Status unavailable"),
  };
}
