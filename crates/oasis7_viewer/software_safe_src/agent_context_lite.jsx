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
  const field = () => props.value || {};
  const state = () => field().state === "published" && field().value ? "published" : "unavailable";
  const unavailableCopy = () => copy(
    props.locale,
    "不可用",
    "Unavailable",
  );
  return (
    <div
      class="agent-context-lite__field"
      data-agent-context-field={props.name}
      data-agent-context-field-state={state()}
      aria-label={`${props.label}: ${state() === "published" ? field().value : unavailableCopy()}`}
    >
      <div class="metric__label">{props.label}</div>
      <div class="agent-context-lite__value">
        {state() === "published" ? field().value : unavailableCopy()}
      </div>
    </div>
  );
}

export function AgentContextLite(props) {
  const model = () => props.model || {};
  const locale = () => props.locale || "en";
  const isAgent = () => model().kind === "agent";
  const intent = () => model().intent || { kind: "unavailable" };
  const receipt = () => model().receipt || { present: false, state: "none" };
  const identity = () => model().identity || {};
  const decisionFields = () => [model().objective, model().nextMove, model().blocker, model().playerLeverage];
  const decisionUnavailable = () => decisionFields().some((field) => field?.state !== "published" || !field?.value);
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
      data-agent-context-fixture={props.fixtureMetadata?.mode}
      data-agent-context-fixture-state={props.fixtureMetadata?.state}
      data-agent-context-fixture-copy={props.fixtureMetadata?.copy}
      data-agent-context-measurement={props.fixtureMetadata?.measurement}
      data-agent-context-fixture-schema={props.fixtureMetadata?.schema}
    >
      <div class="agent-context-lite__heading panel__title">
        {ariaLabel()}
      </div>
      <div
        class="agent-context-lite__identity agent-context-lite__group"
        role="group"
        aria-labelledby="agent-context-lite-identity-label"
        data-agent-context-group="identity"
      >
        <div id="agent-context-lite-identity-label" class="agent-context-lite__section-label metric__label">
          {copy(locale(), "身份", "Identity")}
        </div>
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
        <div
          class="agent-context-lite__group agent-context-lite__truth"
          role="group"
          aria-labelledby="agent-context-lite-truth-label"
          data-agent-context-group="truth"
          data-agent-context-activity="status-summary"
        >
          <div id="agent-context-lite-truth-label" class="agent-context-lite__section-label metric__label">
            {copy(locale(), "事实", "Truth")}
          </div>
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
        </div>
        <div
          class="agent-context-lite__group agent-context-lite__decision"
          role="group"
          aria-labelledby="agent-context-lite-decision-label"
          data-agent-context-group="decision"
        >
          <div id="agent-context-lite-decision-label" class="agent-context-lite__section-label metric__label">
            {copy(locale(), "决策", "Decision")}
          </div>
          <div class="agent-context-lite__gameplay">
            <Field name="objective" label={copy(locale(), "目标", "Objective")} value={model().objective} locale={locale()} />
            <Field name="next-move" label={copy(locale(), "下一步", "Next Move")} value={model().nextMove} locale={locale()} />
            <Field name="blocker" label={copy(locale(), "阻塞", "Blocker")} value={model().blocker} locale={locale()} />
            <Field name="player-leverage" label={copy(locale(), "玩家杠杆", "Player Leverage")} value={model().playerLeverage} locale={locale()} />
          </div>
          <Show when={decisionUnavailable() || model().unavailableReason}>
            <div class="empty agent-context-lite__unavailable" data-agent-context-unavailable="true">
              {model().unavailableReason || copy(
                locale(),
                "部分决策信息不可用，等待权威 Agent 投影。",
                "Some decision details are unavailable while waiting for the authoritative Agent projection.",
              )}
            </div>
          </Show>
          <Show when={model().feedback?.kind === "status"}>
            <div class="feedback-detail" data-agent-context-feedback="status">
              <span class="metric__label">{model().feedback.label}</span>
              <Show when={model().feedback.value}> {model().feedback.value}</Show>
            </div>
          </Show>
        </div>
        <div
          class="agent-context-lite__group agent-context-lite__intent"
          role="group"
          aria-labelledby="agent-context-lite-intent-label"
          data-agent-context-group="intent"
        >
          <div id="agent-context-lite-intent-label" class="agent-context-lite__section-label metric__label">
            {copy(locale(), "意图", "Intent")}
          </div>
          <AgentIntentSurface
            intent={intent().kind === "matched" ? intent().source : null}
            locale={locale()}
            connectionStatus={model().connectionStatus || "connected"}
            showReceiptConfirmation={false}
          />
        </div>
      </Show>
    </section>
  );
}
