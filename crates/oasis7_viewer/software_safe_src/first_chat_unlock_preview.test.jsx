import { render, within } from "@solidjs/testing-library";
import { describe, expect, it } from "vitest";
import { FirstChatUnlockPreview } from "./first_chat_unlock_preview.jsx";

const tr = (locale, zh, en) => locale === "zh" ? zh : en;
const firstChatUnlockPreview = {
  chat_purpose: "Start a first conversation with your claimed Agent.",
  immediate_playable_help: "Ask what the Agent can do next for the current gameplay goal.",
  first_question_or_action_hint: "Ask: What should we do first?",
  resource_boundary: "Starter OC unlocks first chat and initial liquid OC; it is separate from slot-1 claim and upkeep funding.",
  defer_effect: "Deferring keeps the completed claim and its upkeep responsibility, but first chat stays locked while liquid OC is zero and no starter OC claim exists.",
  recommended_unlock_action: "claim_starter_oc",
};

describe("first chat unlock preview", () => {
  it("renders all canonical English values in labeled semantic blocks without the raw action id", () => {
    const { getByTestId, queryByText } = render(() => (
      <FirstChatUnlockPreview preview={firstChatUnlockPreview} locale="en" tr={tr} />
    ));
    const preview = getByTestId("first-chat-unlock-preview");

    ["Purpose", "Immediate help", "Try first", "Resource boundary", "If you wait", "Recommended action"].forEach((label) => {
      expect(within(preview).getByText(label)).toBeInTheDocument();
    });
    Object.entries(firstChatUnlockPreview).forEach(([field, value]) => {
      if (field !== "recommended_unlock_action") expect(within(preview).getByText(value)).toBeInTheDocument();
    });
    expect(within(preview).getByText("Claim Starter OC")).toBeInTheDocument();
    expect(queryByText("claim_starter_oc")).not.toBeInTheDocument();
  });

  it("localizes labels and known values in Chinese without leaking the raw action id", () => {
    const { getByTestId, queryByText } = render(() => (
      <FirstChatUnlockPreview preview={firstChatUnlockPreview} locale="zh" tr={tr} />
    ));
    const preview = getByTestId("first-chat-unlock-preview");

    ["目的", "即时帮助", "先试试", "资源边界", "如果等待", "建议操作"].forEach((label) => {
      expect(within(preview).getByText(label)).toBeInTheDocument();
    });
    expect(within(preview).getByText("与已认领的 Agent 开始第一次对话。")).toBeInTheDocument();
    expect(within(preview).getByText("领取初始 OC")).toBeInTheDocument();
    expect(queryByText("claim_starter_oc")).not.toBeInTheDocument();
  });
});
