import { createSignal } from "solid-js";

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

function targetCopy(value, locale, tr) {
  switch (String(value || "")) {
    case "factory_build_hardware":
      return tr(locale, "工厂硬件建造", "Factory hardware build");
    default:
      return tr(locale, "当前工业目标", "Current industrial target");
  }
}

function nextDecisionGuidance(value, locale, tr) {
  switch (String(value || "")) {
    case "enough_to_advance":
      return tr(
        locale,
        "这笔预估足以推进目标：确认后可按推荐量精炼；当前面板不会替你提交精炼。",
        "This quote can advance the target: after you decide, refine the recommended amount. This panel will not submit refining for you.",
      );
    case "partial_progress":
      return tr(
        locale,
        "这次只能缩小缺口：可先补电后按推荐量继续精炼，或采矿/等待以准备剩余材料。",
        "This only reduces the gap: recharge, then refine the recommended amount, or mine/wait to prepare the remaining material.",
      );
    default:
      return tr(
        locale,
        "这笔电力投入不划算：先补电、采矿或等待，并降低精炼量后再请求一份新预估。",
        "This power tradeoff is poor: recharge, mine, or wait, then reduce the refine amount and request a new quote.",
      );
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
          <span class="badge" data-target-id={displayValue(quote().target_id)}>{`${tr(locale(), "目标", "target")}: ${targetCopy(quote().target_id, locale(), tr)}`}</span>
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
        <div class="feedback-summary" data-testid="refine-quote-next-decision">
          {`${tr(locale(), "下一步建议", "Next decision")}: ${nextDecisionGuidance(quote().value_classification, locale(), tr)}`}
        </div>
      </div>
    </section>
  );
}

export function RefineQuotePreflightPanel(props) {
  const [compoundMassG, setCompoundMassG] = createSignal("40");
  const [requesting, setRequesting] = createSignal(false);
  const [requestError, setRequestError] = createSignal("");
  const [requestStatus, setRequestStatus] = createSignal("");
  const locale = () => props.locale;
  const tr = props.tr;
  const remoteRequestState = () => props.requestState || {};
  const visibleError = () => remoteRequestState().status === "error" ? remoteRequestState().error : requestError();
  const visibleStatus = () => remoteRequestState().status === "received"
    ? tr(locale(), "预估已返回，请查看报价结果。", "Quote received; review the estimate below.")
    : requestStatus();

  async function requestQuote(event) {
    event.preventDefault();
    setRequestError("");
    setRequestStatus("");
    setRequesting(true);
    try {
      const result = await props.requestRefineQuote(compoundMassG());
      if (!result?.ok) {
        setRequestError(result?.reason || tr(locale(), "无法请求预估，请稍后重试。", "Could not request a quote. Please try again."));
        return;
      }
      setRequestStatus(tr(locale(), "已请求只读预估，正在等待报价结果。", "Read-only quote requested; waiting for the quote result."));
    } catch (error) {
      setRequestError(`${tr(locale(), "请求预估失败", "Quote request failed")}: ${String(error)}`);
    } finally {
      setRequesting(false);
    }
  }

  return (
    <section id="viewer-refine-quote-panel" class="panel panel--nested" data-testid="refine-quote-panel" data-quote-kind="preflight">
      <div class="panel__header">
        <div class="stack stack--compact">
          <div class="panel__eyebrow">{tr(locale(), "提交前估价", "Before You Commit")}</div>
          <div class="panel__title">{tr(locale(), "化合物精炼预估", "Compound Refining Quote")}</div>
          <div class="panel__meta-copy">
            {tr(locale(), "请求预估不会提交精炼、扣除电力或生成回执。", "Requesting a quote does not submit refining, spend electricity, or create a receipt.")}
          </div>
        </div>
      </div>
      <div class="panel__body stack">
        <form class="stack stack--compact" onSubmit={requestQuote} data-testid="refine-quote-request-form">
          <label>
            <span>{tr(locale(), "精炼量（克）", "Refine amount (g)")}</span>
            <input
              aria-label={tr(locale(), "精炼量（克）", "Refine amount (g)")}
              type="number"
              min="1"
              step="1"
              inputmode="numeric"
              value={compoundMassG()}
              onInput={(event) => setCompoundMassG(event.currentTarget.value)}
            />
          </label>
          <button type="submit" class="button button--secondary" disabled={requesting()}>
            {requesting() ? tr(locale(), "正在请求预估…", "Requesting quote…") : tr(locale(), "请求预估", "Request quote")}
          </button>
        </form>
        {visibleError() ? <div class="feedback-summary feedback-summary--error" role="alert">{visibleError()}</div> : null}
        {visibleStatus() ? <div class="feedback-summary" role="status">{visibleStatus()}</div> : null}
        {props.quote ? <RefineQuotePreflightCard quote={props.quote} locale={locale()} tr={tr} /> : null}
      </div>
    </section>
  );
}
