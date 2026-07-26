import { For, Show } from "solid-js";

const FALLBACK_LABELS = {
  safe_wait: ["等待", "Wait"],
  repair_now: ["修复", "Repair"],
  reroute_now: ["改道", "Reroute"],
};

function humanize(value) {
  return String(value || "")
    .trim()
    .replace(/[_-]+/g, " ")
    .replace(/\b\w/g, (letter) => letter.toUpperCase()) || "—";
}

export function fallbackTradeoffLabel(valueClass, locale, tr) {
  const labels = FALLBACK_LABELS[valueClass];
  return labels ? tr(locale, labels[0], labels[1]) : humanize(valueClass);
}

function Detail(props) {
  return (
    <div class="fallback-tradeoff__detail">
      <dt>{props.label}</dt>
      <dd>{props.value || "—"}</dd>
    </div>
  );
}

export function FallbackTradeoffPanel(props) {
  const options = () => props.options || [];
  const handoff = () => props.noSafeFallbackHandoff || null;
  const text = (zh, en) => props.tr(props.locale, zh, en);
  const requiredNextDecision = () => {
    const value = handoff()?.requiredNextDecisionActionId || handoff()?.requiredNextDecisionClass;
    return value ? humanize(value) : null;
  };

  return (
    <Show when={options().length > 0 || handoff()}>
      <section class="fallback-tradeoff" aria-labelledby="fallback-tradeoff-heading" data-testid="viewer-fallback-tradeoff">
        <div class="fallback-tradeoff__heading">
          <h3 id="fallback-tradeoff-heading">{text("恢复选项", "Recovery choices")}</h3>
          <span>{text("比较后再执行推荐动作", "Compare before using the recommended action")}</span>
        </div>
        <div class="summary-grid" role="list">
          <For each={options()}>
            {(option) => (
              <article
                class={`event-card recovery-option-card${option.recommended ? " metric--claim-primary" : ""}`}
                data-testid="viewer-fallback-tradeoff-option"
                data-fallback-value-class={option.valueClass || "unknown"}
                role="listitem"
              >
                <div class="event-card__title">
                  <h4>{fallbackTradeoffLabel(option.valueClass, props.locale, props.tr)}</h4>
                  <div class="badge-row">
                    <span class={option.available ? "badge badge--good" : "badge badge--warn"}>
                      {option.available ? text("可用", "Available") : text("不可用", "Unavailable")}
                    </span>
                    <Show when={option.recommended}>
                      <span class="badge badge--accent">{text("推荐", "Recommended")}</span>
                    </Show>
                  </div>
                </div>
                <dl class="fallback-tradeoff__details">
                  <Detail label={text("原因", "Reason")} value={option.reason} />
                  <Detail label={text("保留", "Keeps")} value={option.progressKept} />
                  <Detail label={text("成本", "Cost")} value={option.cost} />
                  <Detail label={text("机会成本", "Opportunity cost")} value={option.opportunityCost} />
                </dl>
              </article>
            )}
          </For>
        </div>
        <Show when={handoff()}>
          <aside class="event-card fallback-tradeoff__handoff" data-testid="viewer-no-safe-fallback-handoff">
            <div class="event-card__title">
              <h4>{text("没有安全恢复选项", "No safe fallback")}</h4>
              <span class="badge badge--warn">{text("需要新的决定", "New decision required")}</span>
            </div>
            <dl class="fallback-tradeoff__details">
              <Detail label={text("原因", "Reason")} value={handoff().reason} />
              <Show when={requiredNextDecision()}>
                <Detail label={text("所需下一决定", "Required next decision")} value={requiredNextDecision()} />
              </Show>
            </dl>
          </aside>
        </Show>
      </section>
    </Show>
  );
}
