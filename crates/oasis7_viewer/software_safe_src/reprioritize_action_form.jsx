import { createEffect, createSignal, Show } from "solid-js";

import * as core from "./legacy_core.js";

export function ReprioritizeActionForm(props) {
  const [open, setOpen] = createSignal(false);
  const [draft, setDraft] = createSignal("");
  const [localError, setLocalError] = createSignal("");
  const [submitted, setSubmitted] = createSignal(false);
  let textarea;
  let errorNode;
  const feedback = () => {
    props.observeState();
    return core.snapshotSemanticFeedback(core.state.lastPromptFeedback);
  };
  const inFlight = () => submitted() && ["registering", "signing", "sent"].includes(String(feedback()?.stage || ""));
  const cancel = () => {
    setDraft("");
    setLocalError("");
    setSubmitted(false);
    setOpen(false);
  };
  createEffect(() => {
    const current = feedback();
    if (!submitted() || !current) return;
    if (current.stage === "apply_ack") {
      cancel();
      core.sendGameplayAction({ protocol_action: "request_snapshot", action_id: "request_snapshot" });
    } else if (current.stage === "error") {
      setLocalError(current.reason || current.effect || props.tr(props.locale, "目标替换失败，请检查后重试。", "Goal replacement failed; review the error and retry."));
      queueMicrotask(() => errorNode?.focus());
    }
  });
  const submit = (event) => {
    event.preventDefault();
    const shortTermGoal = draft().trim();
    if (!shortTermGoal) {
      setLocalError(props.tr(props.locale, "请输入替代短期目标。", "Enter a replacement short-term goal."));
      textarea?.focus();
      return;
    }
    const agentId = props.action.targetAgentId;
    const profile = core.state.snapshot?.model?.agent_prompt_profiles?.[agentId] || {};
    setLocalError("");
    const result = core.sendPromptControl("apply", {
      agentId,
      shortTermGoal,
      // Never inherit dirty Advanced Prompt Settings drafts into this narrow action.
      systemPrompt: profile.system_prompt_override || "",
      longTermGoal: profile.long_term_goal_override || "",
    });
    if (result?.ok === false) {
      setLocalError(result.reason);
      queueMicrotask(() => errorNode?.focus());
      return;
    }
    setSubmitted(true);
  };
  return <div class="toolbar" data-testid="viewer-reprioritize-action">
    <Show when={!open()} fallback={
      <form onSubmit={submit} onKeyDown={(event) => {
        if (event.key === "Escape" && !inFlight()) {
          event.preventDefault();
          cancel();
        }
      }}>
        <label for="viewer-reprioritize-goal">{props.tr(props.locale, "替代短期目标", "Replacement short-term goal")}</label>
        <textarea id="viewer-reprioritize-goal" ref={textarea} rows="3" value={draft()} aria-describedby="viewer-reprioritize-help viewer-reprioritize-status" onInput={(event) => { setDraft(event.currentTarget.value); setLocalError(""); }} />
        <div id="viewer-reprioritize-help" class="feedback-detail">{props.tr(props.locale, "这会替换短期目标，不保证 Agent 下一步必定执行。", "This replaces the short-term goal; it does not guarantee the Agent's next action.")}</div>
        <Show when={localError()}><div ref={errorNode} id="viewer-reprioritize-status" role="alert" tabindex="-1" class="feedback-detail">{localError()}</div></Show>
        <Show when={!localError() && inFlight()}><div id="viewer-reprioritize-status" aria-live="polite" class="feedback-detail">{props.tr(props.locale, "正在认证并提交新目标…", "Authenticating and submitting the new goal…")}</div></Show>
        <div class="toolbar"><button type="button" onClick={cancel} disabled={inFlight()}>{props.tr(props.locale, "取消", "Cancel")}</button><button type="submit" disabled={!draft().trim() || inFlight()}>{props.tr(props.locale, "应用新目标", "Apply new goal")}</button></div>
      </form>
    }>
      <button data-testid="viewer-available-action-reprioritize" aria-label={props.action.label} onClick={() => { setOpen(true); queueMicrotask(() => textarea?.focus()); }}>{props.action.label}</button>
    </Show>
  </div>;
}
