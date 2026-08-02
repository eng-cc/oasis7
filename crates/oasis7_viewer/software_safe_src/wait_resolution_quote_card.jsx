function quoteField(quote, snakeCase, camelCase) {
  const value = quote?.[snakeCase] ?? quote?.[camelCase];
  return typeof value === "string" && value.trim() ? value.trim() : "—";
}

function Detail(props) {
  return (
    <div class="metric">
      <div class="metric__label">{props.label}</div>
      <div class="metric__value">{props.value}</div>
    </div>
  );
}

export function WaitResolutionQuoteCard(props) {
  if (!props.quote) return null;

  const locale = () => props.locale;
  const text = (zh, en) => props.tr(locale(), zh, en);
  const safeToWait = props.quote.safe_to_wait === true || props.quote.safeToWait === true;

  return (
    <section class="event-card" data-testid="wait-resolution-quote">
      <div class="event-card__title">
        <h3>{text("等待结果说明", "Wait resolution")}</h3>
        <span class={safeToWait ? "badge badge--good" : "badge badge--warn"}>
          {safeToWait ? text("可以等待", "Safe to wait") : text("不要等待", "Do not wait")}
        </span>
      </div>
      <div class="feedback-summary">
        {safeToWait
          ? text("运行时说明等待是安全的；在复查点确认预期变化。", "The runtime says waiting is safe; confirm the expected change at the recheck point.")
          : text("运行时说明等待并不安全；请比较恢复选项后再决定。", "The runtime says waiting is not safe; compare the recovery choices before deciding.")}
      </div>
      <div class="summary-grid">
        <Detail label={text("触发条件", "Trigger")} value={quoteField(props.quote, "resolution_trigger", "resolutionTrigger")} />
        <Detail label={text("复查点", "Recheck")} value={quoteField(props.quote, "recheck_tick_or_event", "recheckTickOrEvent")} />
        <Detail label={text("预期变化", "Expected change")} value={quoteField(props.quote, "expected_change", "expectedChange")} />
        <Detail label={text("未解决风险", "Unresolved risk")} value={quoteField(props.quote, "unresolved_risk", "unresolvedRisk")} />
        <Detail label={text("替代解锁条件", "Alternative unlock")} value={quoteField(props.quote, "alternative_unlock_condition", "alternativeUnlockCondition")} />
      </div>
    </section>
  );
}
