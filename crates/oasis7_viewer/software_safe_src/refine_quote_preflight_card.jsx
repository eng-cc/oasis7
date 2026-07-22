function displayValue(value) {
  if (value === null || value === undefined || value === "") return "-";
  return String(value);
}

function classificationCopy(value, locale, tr) {
  switch (String(value || "")) {
    case "enough_to_advance":
      return tr(locale, "足以推进下一步", "Enough to advance");
    case "partial_progress":
      return tr(locale, "可获得部分进展", "Partial progress");
    default:
      return tr(locale, "电力投入不划算", "Poor power tradeoff");
  }
}

function linkageCopy(value, locale, tr) {
  switch (String(value || "")) {
    case "enables_factory_build_hardware_goal":
      return tr(locale, "本次产出可满足工厂硬件目标", "This output satisfies the factory hardware target");
    case "reduces_factory_build_hardware_shortfall":
      return tr(locale, "本次产出会缩小工厂硬件缺口", "This output reduces the factory hardware gap");
    default:
      return tr(locale, "本次产出不会缩小当前工厂硬件缺口", "This output does not reduce the current factory hardware gap");
  }
}

function QuoteMetric(props) {
  return (
    <div class="metric">
      <div class="metric__label">{props.label}</div>
      <div class="metric__value">{displayValue(props.value)}</div>
      {props.detail ? <div class="metric__detail">{props.detail}</div> : null}
    </div>
  );
}

export function RefineQuotePreflightCard(props) {
  const quote = () => props.quote || {};
  const locale = () => props.locale;
  const tr = props.tr;
  return (
    <section class="panel panel--nested" data-testid="refine-quote-preflight" data-quote-kind="preflight">
      <div class="panel__header">
        <div class="stack stack--compact">
          <div class="panel__eyebrow">{tr(locale(), "提交前估价", "Before You Commit")}</div>
          <div class="panel__title">{tr(locale(), "化合物精炼预估", "Compound Refining Quote")}</div>
          <div class="panel__meta-copy">
            {tr(
              locale(),
              "这是只读预估，不会提交精炼、扣除电力或生成回执。",
              "This is a read-only quote. It does not submit refining, spend electricity, or create a receipt.",
            )}
          </div>
        </div>
      </div>
      <div class="panel__body stack">
        <div class="badge-row">
          <span class="badge badge--accent">{tr(locale(), "预估", "quote")}</span>
          <span class="badge">{`${tr(locale(), "目标", "target")}: ${displayValue(quote().target_id)}`}</span>
          <span class="badge">{`${tr(locale(), "Agent", "Agent")}: ${displayValue(quote().owner_agent_id)}`}</span>
        </div>
        <div class="summary-grid">
          <QuoteMetric label={tr(locale(), "精炼量", "Refine amount")} value={`${displayValue(quote().compound_mass_g)} g`} />
          <QuoteMetric label={tr(locale(), "电力成本", "Electricity cost")} value={quote().electricity_cost} />
          <QuoteMetric label={tr(locale(), "剩余电力", "Electricity remaining")} value={quote().electricity_after} />
          <QuoteMetric label={tr(locale(), "硬件产出", "Hardware output")} value={quote().hardware_output} />
          <QuoteMetric label={tr(locale(), "目标缺口", "Target gap")} value={`${displayValue(quote().target_gap_before)} → ${displayValue(quote().target_gap_after)}`} />
          <QuoteMetric label={tr(locale(), "建议精炼量", "Recommended amount")} value={`${displayValue(quote().recommended_refine_amount)} g`} />
        </div>
        <div class="feedback-summary">
          {`${tr(locale(), "目标关联", "Target linkage")}: ${linkageCopy(quote().target_linkage, locale(), tr)}`}
        </div>
        <div class="feedback-summary">
          {`${tr(locale(), "价值判断", "Value assessment")}: ${classificationCopy(quote().value_classification, locale(), tr)}`}
        </div>
      </div>
    </section>
  );
}
