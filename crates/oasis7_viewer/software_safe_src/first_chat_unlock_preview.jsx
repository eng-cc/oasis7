import { For } from "solid-js";

const ZH_VALUE_MAP = {
  chat_purpose: {
    "Start a first conversation with your claimed Agent.": "与已认领的 Agent 开始第一次对话。",
  },
  immediate_playable_help: {
    "Ask what the Agent can do next for the current gameplay goal.": "询问 Agent 为当前玩法目标下一步能做什么。",
  },
  first_question_or_action_hint: {
    "Ask: What should we do first?": "试着问：我们第一步该做什么？",
  },
  resource_boundary: {
    "Starter OC unlocks first chat and initial liquid OC; it is separate from slot-1 claim and upkeep funding.": "初始 OC 会解锁首次聊天和初始可用 OC；它不同于第 1 个槽位的认领及维护资金。",
  },
  defer_effect: {
    "Deferring keeps the completed claim and its upkeep responsibility, but first chat stays locked while liquid OC is zero and no starter OC claim exists.": "暂缓不会取消已完成的认领及其维护责任；但在可用 OC 为零且尚未领取初始 OC 时，首次聊天仍会锁定。",
  },
};

function previewValue(field, value, locale) {
  return locale === "zh" ? ZH_VALUE_MAP[field]?.[value] || value : value;
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
    ? recommendedActionValue(props.preview[field], locale())
    : previewValue(field, props.preview[field], locale());

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
