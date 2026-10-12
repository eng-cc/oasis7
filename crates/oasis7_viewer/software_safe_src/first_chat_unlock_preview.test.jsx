import { render, within } from "@solidjs/testing-library";
import { createSignal } from "solid-js";
import { describe, expect, it } from "vitest";
import { FirstChatUnlockPreview } from "./first_chat_unlock_preview.jsx";

const tr = (locale, zh, en) => locale === "zh" ? zh : en;
const gateExplanation = "First chat passes the starter OC gate with a positive liquid OC balance or an existing starter OC claim record. An existing starter OC record keeps this gate open even after the balance is spent.";
const zhGateExplanation = "可用 OC 大于零或已有初始 OC 领取记录，即可通过初始 OC 这道首聊门禁。已有领取记录时，即使余额用尽，这道门禁也保持通过。";
const existingClaimDefer = `Deferring leaves your existing economic claims and their current upkeep rules unchanged. ${gateExplanation}`;
const unknownClaimDefer = `Current economic claim obligations are not confirmed. Refresh authoritative state; deferring creates or clears no obligation. ${gateExplanation}`;
const zhUnknownClaimDefer = `当前经济认领义务尚未确认，请刷新权威状态；暂缓不会创建或清除义务。${zhGateExplanation}`;
const firstChatUnlockPreview = {
  chat_purpose: "Start a first conversation with the Agent you control.",
  immediate_playable_help: "Ask what the Agent can do next for the current gameplay goal.",
  first_question_or_action_hint: "Ask: What should we do first?",
  resource_boundary: `Starter OC grants initial liquid OC and a one-time first-chat unlock. ${gateExplanation} It is separate from economic slot-1 claim and upkeep funding.`,
  defer_effect: `Deferring leaves your Agent binding unchanged and creates no economic claim or upkeep obligation. ${gateExplanation}`,
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
    expect(within(preview).getByText("与当前有控制权的 Agent 开始第一次对话。")).toBeInTheDocument();
    expect(within(preview).getByText(`初始 OC 提供一次性首聊解锁和初始可用 OC。${zhGateExplanation}它与经济认领中第 1 个槽位的认领及维护资金分开。`)).toBeInTheDocument();
    expect(within(preview).getByText(`暂缓不改变当前 Agent 绑定，也不会新增经济认领或维护义务。${zhGateExplanation}`)).toBeInTheDocument();
    expect(within(preview).getByText("领取初始 OC")).toBeInTheDocument();
    expect(queryByText("claim_starter_oc")).not.toBeInTheDocument();
  });

  it("preserves actual economic upkeep rules in the Chinese projection", () => {
    const { getByTestId, queryByText } = render(() => (
      <FirstChatUnlockPreview
        preview={{ ...firstChatUnlockPreview, defer_effect: existingClaimDefer }}
        locale="zh"
      />
    ));
    expect(within(getByTestId("first-chat-unlock-preview")).getByText(
      `暂缓不改变已有经济认领，其现行维护规则继续适用。${zhGateExplanation}`,
    )).toBeInTheDocument();
    expect(queryByText(/不会新增经济认领或维护义务/)).not.toBeInTheDocument();
  });

  it.each(["en", "zh"])("treats legacy economic responsibility as unknown in %s", (locale) => {
    const legacyPreview = {
      ...firstChatUnlockPreview,
      chat_purpose: "Start a first conversation with your claimed Agent.",
      resource_boundary: "Starter OC unlocks first chat and initial liquid OC; it is separate from slot-1 claim and upkeep funding.",
      defer_effect: "Deferring keeps the completed claim and its upkeep responsibility, but first chat stays locked while liquid OC is zero and no starter OC claim exists.",
    };
    const { getByTestId, queryByText } = render(() => (
      <FirstChatUnlockPreview preview={legacyPreview} locale={locale} />
    ));
    const preview = within(getByTestId("first-chat-unlock-preview"));
    expect(preview.getByText(locale === "zh" ? zhUnknownClaimDefer : unknownClaimDefer)).toBeInTheDocument();
    expect(queryByText(legacyPreview.defer_effect)).not.toBeInTheDocument();
    expect(queryByText(/暂缓不会取消已完成的认领及其维护责任/)).not.toBeInTheDocument();
    expect(preview.getByText(locale === "zh"
      ? `初始 OC 提供一次性首聊解锁和初始可用 OC。${zhGateExplanation}它与经济认领中第 1 个槽位的认领及维护资金分开。`
      : firstChatUnlockPreview.resource_boundary)).toBeInTheDocument();
  });

  it.each([undefined, null, " "])("does not infer no economic claim from a missing defer value (%s)", (deferEffect) => {
    const { getByText, queryByText } = render(() => (
      <FirstChatUnlockPreview
        preview={{ ...firstChatUnlockPreview, defer_effect: deferEffect }}
        locale="en"
      />
    ));
    expect(getByText(unknownClaimDefer)).toBeInTheDocument();
    expect(queryByText(firstChatUnlockPreview.defer_effect)).not.toBeInTheDocument();
  });

  it("keeps an unfamiliar authoritative explanation readable when no translation exists", () => {
    const explanation = "Claim obligations await an authoritative refresh for this Agent.";
    const { getByText } = render(() => (
      <FirstChatUnlockPreview
        preview={{ ...firstChatUnlockPreview, defer_effect: explanation }}
        locale="zh"
      />
    ));
    expect(getByText(explanation)).toBeInTheDocument();
  });

  it("replaces the economic explanation when the supplied Agent preview changes", () => {
    const [preview, setPreview] = createSignal(firstChatUnlockPreview);
    const { getByText, queryByText } = render(() => (
      <FirstChatUnlockPreview preview={preview()} locale="en" />
    ));
    expect(getByText(firstChatUnlockPreview.defer_effect)).toBeInTheDocument();
    setPreview({ ...firstChatUnlockPreview, defer_effect: existingClaimDefer });
    expect(getByText(existingClaimDefer)).toBeInTheDocument();
    expect(queryByText(firstChatUnlockPreview.defer_effect)).not.toBeInTheDocument();
    setPreview({ ...firstChatUnlockPreview, defer_effect: undefined });
    expect(getByText(unknownClaimDefer)).toBeInTheDocument();
    expect(queryByText(existingClaimDefer)).not.toBeInTheDocument();
  });
});
