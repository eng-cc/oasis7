import { createSignal, For } from "solid-js";

const display = (value) => value == null || value === "" ? "—" : String(value);
const Metric = (props) => <div class="metric"><div class="metric__label">{props.label}</div><div class="metric__value">{props.value}</div></div>;

export function MarketQuoteDecisionCard(props) {
  const quote = () => props.quote || {}; const locale = () => props.locale; const tr = props.tr;
  return <section class="panel panel--nested" data-testid="market-quote-decision" data-quote-kind="preflight" data-submission-allowed={String(quote().submission_allowed === true)}>
    <div class="panel__header"><div class="stack stack--compact"><div class="panel__eyebrow">{tr(locale(), "提交前估价", "Before You Commit")}</div><div class="panel__title">{tr(locale(), "市场材料预估", "Market Material Preview")}</div><div class="panel__meta-copy">{tr(locale(), "这是已签名的只读预估，不会预留材料、扣除成本或提交配方。", "This is a signed read-only preview. It does not reserve materials, charge costs, or submit a recipe.")}</div></div></div>
    <div class="panel__body stack">
      <div class={quote().submission_allowed ? "feedback-summary" : "feedback-summary feedback-summary--warn"} data-testid="market-quote-recommendation">{`${tr(locale(), "建议", "Recommended")}: ${display(quote().recommendation)}`}</div>
      <div class="summary-grid"><Metric label={tr(locale(), "总缺口", "Total shortfall")} value={display(quote().total_shortfall_amount)} /><Metric label={tr(locale(), "提交条件", "Submission") } value={quote().submission_allowed ? tr(locale(), "当前可提交", "Currently covered") : tr(locale(), "材料不足", "Materials missing")} /></div>
      <For each={quote().contributions || []}>{(item) => <div class="feedback-detail" data-testid="market-quote-contribution"><strong>{display(item.material)}</strong>{`: ${tr(locale(), "请求", "Requested")} ${display(item.requested_amount)} · ${tr(locale(), "本地", "Local")} ${display(item.local_available_amount)} · ${tr(locale(), "世界补足", "World cover")} ${display(item.world_cover_amount)} · ${tr(locale(), "缺口", "Shortfall")} ${display(item.shortfall_amount)} · ${tr(locale(), "运输损耗", "Transit loss")} ${display(item.transit_loss_bps)} bps · ${tr(locale(), "治理税", "Governance tax")} ${display(item.governance_tax_bps)} bps · ${tr(locale(), "成本指数", "Cost index")} ${display(item.effective_cost_index_ppm)} ppm`}</div>}</For>
      <div class="feedback-detail" data-testid="market-quote-rationale">{`${tr(locale(), "原因", "Why")}: ${display(quote().rationale)}`}</div>
      <div class="feedback-detail" data-testid="market-quote-next-action">{`${tr(locale(), "下一步", "Next step")}: ${display(quote().next_action)}`}</div>
      <div class="feedback-summary feedback-summary--warn" data-testid="market-quote-conditional">{display(quote().conditional_notice)}</div>
    </div>
  </section>;
}

export function MarketQuoteDecisionPanel(props) {
  const [material, setMaterial] = createSignal("iron_ingot"); const [amount, setAmount] = createSignal("4"); const [requesting, setRequesting] = createSignal(false); const [localError, setLocalError] = createSignal(""); const locale = () => props.locale; const tr = props.tr; const remote = () => props.requestState || {};
  async function requestQuote(event) { event.preventDefault(); setLocalError(""); setRequesting(true); try { const result = await props.requestMarketQuoteDecision([{ material: material(), amount: amount() }]); if (!result?.ok) setLocalError(result?.reason || "quote failed"); } catch (error) { setLocalError(String(error)); } finally { setRequesting(false); } }
  return <section class="panel panel--nested" data-testid="market-quote-decision-panel"><div class="panel__header"><div class="panel__title">{tr(locale(), "市场材料预估", "Market Material Preview")}</div></div><div class="panel__body stack"><form class="stack stack--compact" data-testid="market-quote-decision-request-form" onSubmit={requestQuote}><label><span>{tr(locale(), "材料", "Material")}</span><input aria-label={tr(locale(), "材料", "Material")} value={material()} onInput={(event) => setMaterial(event.currentTarget.value)} /></label><label><span>{tr(locale(), "数量", "Amount")}</span><input aria-label={tr(locale(), "数量", "Amount")} type="number" min="1" step="1" value={amount()} onInput={(event) => setAmount(event.currentTarget.value)} /></label><button class="button button--secondary" type="submit" disabled={requesting() || remote().status === "pending"}>{requesting() || remote().status === "pending" ? tr(locale(), "正在请求预估…", "Requesting preview…") : tr(locale(), "请求市场预估", "Request market preview")}</button></form>{localError() || remote().status === "error" ? <div class="feedback-summary feedback-summary--error" role="alert">{tr(locale(), "无法获取市场预估。请检查连接、玩家会话和输入后重试。", "Could not get the market preview. Check connection, player session, and inputs, then retry.")}</div> : null}{props.quote && remote().status !== "pending" ? <MarketQuoteDecisionCard quote={props.quote} locale={locale()} tr={tr} /> : null}</div></section>;
}
