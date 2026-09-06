import { Show } from "solid-js";
import { agentIntentCopy, describeAgentIntent } from "./agent_intent_display_model.js";

export {
  AGENT_INTENT_SUMMARIES,
  INTENT_STATUS_LABELS,
  PLAYER_SAFE_COPY_SCHEMA_VERSION,
  boundedSafeText,
  describeAgentIntent,
  hasReceiptReference,
} from "./agent_intent_display_model.js";

export function AgentIntentSurface(props) {
  const locale = () => props.locale || "en";
  const model = () => describeAgentIntent(props.intent, locale(), props.connectionStatus);
  const hidden = () => ["control_lost", "read_only", "unauthorized", "unavailable"].includes(model().kind);
  const showReceiptConfirmation = () => props.showReceiptConfirmation !== false;
  return (
    <section class="agent-intent" data-agent-intent-state={model().kind} data-agent-intent-receipt-state={model().receiptState || "not_applicable"} aria-live="polite">
      <div class="agent-intent__heading metric__label">{agentIntentCopy(locale(), "heading")}</div>
      <div class="agent-intent__state">{model().label}</div>
      <Show when={!hidden()}>
        <div class="agent-intent__status-row">
          <Show when={model().statusLabel}><span class="badge badge--accent">{model().statusLabel}</span></Show>
        </div>
        <Show when={model().message}><div class="agent-intent__summary">{model().message}</div></Show>
        <Show when={showReceiptConfirmation() && model().receiptLabel}><div class="agent-intent__detail agent-intent__receipt"><span class="metric__label">{model().receiptLabel}</span></div></Show>
        <Show when={model().reasonLabel || model().reasonSummary}><div class="agent-intent__detail"><span class="metric__label">{agentIntentCopy(locale(), "reason")}</span><span class="agent-intent__summary">{model().reasonSummary}</span></div></Show>
        <Show when={model().lifecycleNote}><div class="agent-intent__detail agent-intent__lifecycle">{model().lifecycleNote}</div></Show>
      </Show>
      <Show when={showReceiptConfirmation() && hidden() && model().receiptLabel}><div class="agent-intent__detail agent-intent__receipt"><span class="metric__label">{model().receiptLabel}</span></div></Show>
      <Show when={model().nextStep}><div class="agent-intent__detail agent-intent__next-step"><span class="metric__label">{agentIntentCopy(locale(), "nextStep")}</span><span class="agent-intent__summary">{model().nextStep}</span></div></Show>
    </section>
  );
}
