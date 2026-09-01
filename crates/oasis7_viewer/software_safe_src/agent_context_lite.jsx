import { Show } from "solid-js";
import { AgentActivitySurface } from "./agent_activity_surface.jsx";
import { AgentIntentSurface } from "./agent_intent_surface.jsx";

function localeIsZh(locale) {
  return String(locale || "").toLowerCase().startsWith("zh");
}

function copy(locale, zh, en) {
  return localeIsZh(locale) ? zh : en;
}

function Field(props) {
  return (
    <Show when={props.value?.value}>
      <div class="agent-context-lite__field">
        <div class="metric__label">{props.label}</div>
        <div class="agent-context-lite__value">{props.value.value}</div>
      </div>
    </Show>
  );
}

export function AgentContextLite(props) {
  const model = () => props.model || {};
  const locale = () => props.locale || "en";
  const isAgent = () => model().kind === "agent";
  const intent = () => model().intent || { kind: "unavailable" };
  const receipt = () => model().receipt || { present: false, state: "none" };
  const identity = () => model().identity || {};
  const ariaLabel = () => isAgent()
    ? copy(locale(), "Agent 上下文", "Agent Context")
    : copy(locale(), "实体上下文", "Entity Context");

  return (
    <section
      class="stack agent-context-lite"
      role="region"
      aria-label={ariaLabel()}
      data-agent-context-kind={model().kind || "unknown"}
      data-agent-context-intent={intent().kind || "unavailable"}
      data-agent-context-receipt={receipt().present ? "present" : receipt().state || "none"}
    >
      <div class="agent-context-lite__heading panel__title">
        {ariaLabel()}
      </div>
      <div class="agent-context-lite__identity">
        <div class="metric__label">{copy(locale(), "身份", "Identity")}</div>
        <div class="agent-context-lite__value">{identity().name || identity().id || copy(locale(), "未知", "Unknown")}</div>
        <Show when={identity().label && identity().label !== identity().name}>
          <div class="feedback-detail">{identity().label}</div>
        </Show>
        <Show when={model().location?.name}>
          <div class="feedback-detail">
            {copy(locale(), "所在位置", "Location")}: {model().location.name}
          </div>
        </Show>
      </div>
      <Show
        when={isAgent()}
        fallback={
          <div class="empty agent-context-lite__unavailable" data-agent-context-unavailable="true">
            {model().unavailableReason || copy(locale(), "上下文不可用。", "Context unavailable.")}
          </div>
        }
      >
        <div class="badge-row agent-context-lite__state-row">
          <span class="badge">{model().state?.label}</span>
          <Show when={!(intent().kind === "matched" && intent().state === model().freshness?.kind)}>
            <span class="badge">{model().freshness?.label}</span>
          </Show>
        </div>
        <AgentActivitySurface
          activity={model().activity?.surface || null}
          locale={locale()}
        />
        <div class="agent-context-lite__gameplay">
          <Field label={copy(locale(), "目标", "Objective")} value={model().objective} />
          <Field label={copy(locale(), "下一步", "Next Move")} value={model().nextMove} />
          <Field label={copy(locale(), "阻塞", "Blocker")} value={model().blocker} />
          <Field label={copy(locale(), "玩家杠杆", "Player Leverage")} value={model().playerLeverage} />
        </div>
        <Show when={model().feedback?.kind === "status"}>
          <div class="feedback-detail" data-agent-context-feedback="status">
            <span class="metric__label">{model().feedback.label}</span>
            <Show when={model().feedback.value}> {model().feedback.value}</Show>
          </div>
        </Show>
        <AgentIntentSurface
          intent={intent().kind === "matched" ? intent().source : null}
          locale={locale()}
          connectionStatus={model().connectionStatus || "connected"}
          showReceiptConfirmation={false}
        />
        <Show when={model().unavailableReason}>
          <div class="empty agent-context-lite__unavailable">{model().unavailableReason}</div>
        </Show>
      </Show>
    </section>
  );
}
