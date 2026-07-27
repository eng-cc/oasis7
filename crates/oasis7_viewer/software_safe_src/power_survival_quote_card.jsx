import { createSignal } from "solid-js";

function display(value) { return value == null || value === "" ? "—" : String(value); }
function powerState(value, locale, tr) {
  return ({ normal: tr(locale, "正常", "Normal"), low_power: tr(locale, "低电量", "Low power"), critical: tr(locale, "临界电量", "Critical power"), shutdown: tr(locale, "已停机", "Shutdown") })[String(value || "")] || tr(locale, "状态暂不可用", "State unavailable");
}
function affordability(value, locale, tr) {
  return ({ healthy: tr(locale, "下一步可负担", "Next action affordable"), limited: tr(locale, "下一步受限", "Next action limited"), blocked: tr(locale, "下一步仍不可负担", "Next action still blocked") })[String(value || "")] || tr(locale, "可负担性暂不可用", "Affordability unavailable");
}
function recommendation(value, locale, tr) {
  return ({ buy_power: tr(locale, "按此补电后继续", "Buy this power, then continue"), buy_power_partial: tr(locale, "继续补电后再行动", "Buy more power before acting"), buy_more_power: tr(locale, "先补充更多电力", "Buy more power first") })[String(value || "")] || tr(locale, "重新请求预估后再决定", "Request a fresh quote before deciding");
}
function Metric(props) { return <div class="metric"><div class="metric__label">{props.label}</div><div class="metric__value">{props.value}</div></div>; }

export function PowerSurvivalQuoteCard(props) {
  const quote = () => props.quote || {}; const locale = () => props.locale; const tr = props.tr;
  return <section class="panel panel--nested" data-testid="power-survival-quote" data-quote-kind="preflight" data-seller-agent-id={display(quote().seller_agent_id)} data-amount={display(quote().recovery_amount)} data-requested-price-per-pu={display(quote().requested_price_per_pu)}>
    <div class="panel__header"><div class="stack stack--compact"><div class="panel__eyebrow">{tr(locale(), "提交前估价", "Before You Commit")}</div><div class="panel__title">{tr(locale(), "补电生存预估", "Power Recovery Quote")}</div><div class="panel__meta-copy">{tr(locale(), "这是已签名的只读预估，不会购买电力、扣除成本、推进时间或生成回执。", "This is a signed read-only quote. It does not buy power, charge a cost, advance time, or create a receipt.")}</div></div></div>
    <div class="panel__body stack">
      <div class="badge-row"><span class="badge badge--accent">{tr(locale(), "预估", "quote")}</span><span class="badge">{`${tr(locale(), "卖方", "Seller")}: ${display(quote().seller_agent_id)}`}</span><span class="badge">{`${tr(locale(), "补电量", "Power amount")}: ${display(quote().recovery_amount)}`}</span><span class="badge">{`${tr(locale(), "报价", "Quoted price")}: ${display(quote().price_per_pu)}`}</span></div>
      <div class="summary-grid"><Metric label={tr(locale(), "预计补电", "Expected gain")} value={display(quote().power_gain_estimate)} /><Metric label={tr(locale(), "预计成本", "Estimated cost")} value={display(quote().price_or_time_cost)} /><Metric label={tr(locale(), "电力状态", "Power state")} value={`${powerState(quote().power_state_before, locale(), tr)} → ${powerState(quote().power_state_after_recovery, locale(), tr)}`} /><Metric label={tr(locale(), "可行动时长", "Action runway")} value={`${display(quote().survival_runway_ticks)} ${tr(locale(), "tick", "ticks")}`} /><Metric label={tr(locale(), "下一步可负担性", "Next-action affordability")} value={affordability(quote().next_action_affordability_after_recovery, locale(), tr)} /></div>
      <div class="feedback-summary" data-testid="power-survival-shutdown-avoidance">{`${tr(locale(), "防停机判断", "Shutdown avoidance")}: ${quote().power_state_before === "shutdown" && quote().power_state_after_recovery !== "shutdown" ? tr(locale(), "本次补电可让 Agent 脱离停机状态。", "This purchase can bring the Agent out of shutdown.") : quote().power_state_after_recovery === "shutdown" ? tr(locale(), "本次补电后仍会停机；请先补充更多电力。", "This amount still leaves the Agent shut down; buy more power first.") : tr(locale(), "本次补电保留了可行动的电力状态。", "This purchase keeps the Agent in an actionable power state.")}`}</div>
      <div class="feedback-summary" data-testid="power-survival-recommendation">{`${tr(locale(), "建议", "Recommended")}: ${recommendation(quote().recommended_power_action, locale(), tr)}`}</div>
    </div>
  </section>;
}

export function PowerSurvivalQuotePanel(props) {
  const [seller, setSeller] = createSignal("agent-1"); const [amount, setAmount] = createSignal("18"); const [price, setPrice] = createSignal("0"); const [requesting, setRequesting] = createSignal(false); const [localError, setLocalError] = createSignal("");
  const locale = () => props.locale; const tr = props.tr; const remote = () => props.requestState || {};
  const stale = () => Boolean(props.quote) && (String(props.quote.seller_agent_id) !== seller().trim() || String(props.quote.recovery_amount) !== amount().trim() || String(props.quote.requested_price_per_pu) !== price().trim());
  const error = () => remote().status === "error" || localError() ? tr(locale(), "无法获取补电生存预估。请检查连接、玩家会话和输入后重试。", "Could not get the power recovery quote. Check the connection, player session, and inputs, then retry.") : "";
  async function requestQuote(event) { event.preventDefault(); setLocalError(""); setRequesting(true); try { const result = await props.requestPowerSurvivalQuote(seller(), amount(), price()); if (!result?.ok) setLocalError(result?.reason || "quote failed"); } catch (requestError) { setLocalError(String(requestError)); } finally { setRequesting(false); } }
  return <section id="viewer-power-survival-quote-panel" class="panel panel--nested" data-testid="power-survival-quote-panel" data-quote-kind="preflight">
    <div class="panel__header"><div class="stack stack--compact"><div class="panel__eyebrow">{tr(locale(), "提交前估价", "Before You Commit")}</div><div class="panel__title">{tr(locale(), "补电生存预估", "Power Recovery Quote")}</div></div></div>
    <div class="panel__body stack"><form class="stack stack--compact" data-testid="power-survival-quote-request-form" onSubmit={requestQuote}><label><span>{tr(locale(), "卖方 Agent", "Seller Agent")}</span><input aria-label={tr(locale(), "卖方 Agent", "Seller Agent")} value={seller()} onInput={(event) => setSeller(event.currentTarget.value)} /></label><label><span>{tr(locale(), "补电量", "Power amount")}</span><input aria-label={tr(locale(), "补电量", "Power amount")} type="number" min="1" step="1" inputmode="numeric" value={amount()} onInput={(event) => setAmount(event.currentTarget.value)} /></label><label><span>{tr(locale(), "每单位报价", "Price per unit")}</span><input aria-label={tr(locale(), "每单位报价", "Price per unit")} type="number" min="0" step="1" inputmode="numeric" value={price()} onInput={(event) => setPrice(event.currentTarget.value)} /></label><button type="submit" class="button button--secondary" disabled={requesting()}>{requesting() ? tr(locale(), "正在请求预估…", "Requesting quote…") : tr(locale(), "请求补电预估", "Request power quote")}</button></form>
      {error() ? <div class="feedback-summary feedback-summary--error" role="alert">{error()}</div> : null}
      {remote().status === "received" && !stale() ? <div class="feedback-summary" role="status">{tr(locale(), "预估已返回；确认前请查看建议。", "Quote received; review the guidance before confirmation.")}</div> : null}
      {stale() ? <div class="feedback-summary feedback-summary--warn" role="status" data-testid="power-survival-quote-stale">{tr(locale(), "输入已变更；当前预估已过期。请重新请求预估后再购买电力。", "Inputs changed; this quote is stale. Request a new quote before buying power.")}</div> : null}
      {props.quote ? <PowerSurvivalQuoteCard quote={props.quote} locale={locale()} tr={tr} /> : null}
    </div>
  </section>;
}
