import { For } from "solid-js";

const CHAT_PURPOSE = "Start a first conversation with the Agent you control.";
const CHAT_GATE_EXPLANATION = "First chat passes the starter OC gate with a positive liquid OC balance or an existing starter OC claim record. An existing starter OC record keeps this gate open even after the balance is spent.";
const RESOURCE_BOUNDARY = `Starter OC grants initial liquid OC and a one-time first-chat unlock. ${CHAT_GATE_EXPLANATION} It is separate from economic slot-1 claim and upkeep funding.`;
const DEFER_WITHOUT_ECONOMIC_CLAIM = `Deferring leaves your Agent binding unchanged and creates no economic claim or upkeep obligation. ${CHAT_GATE_EXPLANATION}`;
const DEFER_WITH_ECONOMIC_CLAIMS = `Deferring leaves your existing economic claims and their current upkeep rules unchanged. ${CHAT_GATE_EXPLANATION}`;
const DEFER_WITH_UNKNOWN_CLAIMS = `Current economic claim obligations are not confirmed. Refresh authoritative state; deferring creates or clears no obligation. ${CHAT_GATE_EXPLANATION}`;
const LEGACY_CHAT_PURPOSE = "Start a first conversation with your claimed Agent.";
const LEGACY_RESOURCE_BOUNDARY = "Starter OC unlocks first chat and initial liquid OC; it is separate from slot-1 claim and upkeep funding.";
const LEGACY_DEFER_EFFECT = "Deferring keeps the completed claim and its upkeep responsibility, but first chat stays locked while liquid OC is zero and no starter OC claim exists.";
const ZH_CHAT_GATE_EXPLANATION = "可用 OC 大于零或已有初始 OC 领取记录，即可通过初始 OC 这道首聊门禁。已有领取记录时，即使余额用尽，这道门禁也保持通过。";

const ZH_VALUE_MAP = {
  chat_purpose: {
    [CHAT_PURPOSE]: "与当前有控制权的 Agent 开始第一次对话。",
  },
  immediate_playable_help: {
    "Ask what the Agent can do next for the current gameplay goal.": "询问 Agent 为当前玩法目标下一步能做什么。",
  },
  first_question_or_action_hint: {
    "Ask: What should we do first?": "试着问：我们第一步该做什么？",
  },
  resource_boundary: {
    [RESOURCE_BOUNDARY]: `初始 OC 提供一次性首聊解锁和初始可用 OC。${ZH_CHAT_GATE_EXPLANATION}它与经济认领中第 1 个槽位的认领及维护资金分开。`,
  },
  defer_effect: {
    [DEFER_WITHOUT_ECONOMIC_CLAIM]: `暂缓不改变当前 Agent 绑定，也不会新增经济认领或维护义务。${ZH_CHAT_GATE_EXPLANATION}`,
    [DEFER_WITH_ECONOMIC_CLAIMS]: `暂缓不改变已有经济认领，其现行维护规则继续适用。${ZH_CHAT_GATE_EXPLANATION}`,
    [DEFER_WITH_UNKNOWN_CLAIMS]: `当前经济认领义务尚未确认，请刷新权威状态；暂缓不会创建或清除义务。${ZH_CHAT_GATE_EXPLANATION}`,
  },
};

function canonicalPreviewValue(field, value) {
  const hasText = typeof value === "string" && value.trim().length > 0;
  if (field === "chat_purpose" && value === LEGACY_CHAT_PURPOSE) return CHAT_PURPOSE;
  if (field === "resource_boundary" && (!hasText || value === LEGACY_RESOURCE_BOUNDARY)) {
    return RESOURCE_BOUNDARY;
  }
  // Legacy copy did not distinguish binding from an economic claim. Missing
  // evidence must remain unknown; do not infer either responsibility or absence.
  if (field === "defer_effect" && (!hasText || value === LEGACY_DEFER_EFFECT)) {
    return DEFER_WITH_UNKNOWN_CLAIMS;
  }
  return value;
}

function previewValue(field, value, locale) {
  const canonicalValue = canonicalPreviewValue(field, value);
  return locale === "zh" ? ZH_VALUE_MAP[field]?.[canonicalValue] || canonicalValue : canonicalValue;
}

function recommendedActionValue(value, locale) {
  if (value === "claim_starter_oc") return locale === "zh" ? "领取初始 OC" : "Claim Starter OC";
  return value;
}

export function FirstChatUnlockPreview(props) {
  const locale = () => props.locale || "en";
  const tr = (zh, en) => props.tr?.(locale(), zh, en) || (locale() === "zh" ? zh : en);
  const fields = () => [
    ["chat_purpose", tr("目的", "Purpose")],
    ["immediate_playable_help", tr("即时帮助", "Immediate help")],
    ["first_question_or_action_hint", tr("先试试", "Try first")],
    ["resource_boundary", tr("资源边界", "Resource boundary")],
    ["defer_effect", tr("如果等待", "If you wait")],
    ["recommended_unlock_action", tr("建议操作", "Recommended action")],
  ];
  const value = (field) => field === "recommended_unlock_action"
    ? recommendedActionValue(props.preview?.[field], locale())
    : previewValue(field, props.preview?.[field], locale());

  return (
    <div class="stack stack--compact" data-testid="first-chat-unlock-preview">
      <For each={fields()}>{([field, label]) => (
        <div class="first-chat-unlock-preview__field" data-preview-field={field}>
          <div class="metric__label">{label}</div>
          <div class={field === "chat_purpose" ? "feedback-summary" : "feedback-detail"}>{value(field)}</div>
        </div>
      )}</For>
    </div>
  );
}
