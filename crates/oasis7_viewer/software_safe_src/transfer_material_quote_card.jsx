import { createSignal } from "solid-js";

function display(value) { return value == null || value === "" ? "—" : String(value); }
const MATERIAL_LABELS = {
  iron_ingot: ["铁锭", "Iron ingot"],
};

function humanizeMaterialKey(value) {
  const words = String(value || "").trim().replace(/[_-]+/g, " ").replace(/\s+/g, " ");
  if (!words) return "—";
  return words.replace(/\b\w/g, (letter) => letter.toUpperCase());
}

export function materialLabel(value, locale, tr) {
  const key = String(value || "").trim();
  const labels = MATERIAL_LABELS[key];
  if (labels) return tr(locale, labels[0], labels[1]);
  const humanized = humanizeMaterialKey(key);
  return tr(locale, `未知物料：${humanized}`, `Unknown material: ${humanized}`);
}

function materialKeyFromInput(value) {
  const input = String(value || "").trim();
  if (input === "铁锭" || input.toLowerCase() === "iron ingot" || input === "iron_ingot") {
    return "iron_ingot";
  }
  return input;
}

function priorityCopy(value, locale, tr) { return ({ urgent: tr(locale, "紧急优先", "Urgent priority"), standard: tr(locale, "标准优先", "Standard priority") })[String(value || "")] || tr(locale, "优先级待确认", "Priority pending"); }
function recommendationCopy(value, locale, tr) {
  return ({ submit_transfer: tr(locale, "可以提交转运", "Submit the transfer"), submit_immediate_transfer: tr(locale, "可以立即转运", "Submit the immediate transfer"), wait_for_transit_capacity: tr(locale, "等待在途容量释放", "Wait for transit capacity"), reduce_amount_or_source_materials: tr(locale, "减少数量或补足来源库存", "Reduce amount or add source material"), restore_power_or_use_lower_tariff_route: tr(locale, "恢复电力或改走低资费路线", "Restore power or use a lower-tariff route"), route_unavailable: tr(locale, "当前路线不可用，请选择其他可用路线", "Requested route unavailable; choose another available route"), path_unavailable: tr(locale, "当前路径不可用，请选择其他可用路线", "Requested path unavailable; choose another available route"), choose_another_route: tr(locale, "选择其他可用路线", "Choose another available route") })[String(value || "")] || tr(locale, "重新请求预估后再决定", "Request a fresh quote before deciding");
}
function pathIdentity(value, locale, tr) { return value == null || String(value).trim() === "" ? tr(locale, "路径不可用", "Path unavailable") : String(value); }
function routeIdsCopy(value, locale, tr) { const routeIds = Array.isArray(value) ? value.map((routeId) => String(routeId || "").trim()).filter(Boolean) : []; return routeIds.length ? routeIds.join(" → ") : tr(locale, "未选择路线", "No route selected"); }
function Metric(props) { return <div class="metric"><div class="metric__label">{props.label}</div><div class="metric__value">{props.value}</div></div>; }

export function TransferMaterialQuoteCard(props) {
  const quote = () => props.quote || {}; const locale = () => props.locale; const tr = props.tr;
  const feasible = () => quote().submission_feasible === true;
  return <section class="panel panel--nested" data-testid="transfer-material-quote" data-quote-kind="preflight">
    <div class="panel__header"><div class="stack stack--compact"><div class="panel__eyebrow">{tr(locale(), "提交前估价", "Before You Commit")}</div><div class="panel__title">{tr(locale(), "物料转运预估", "Transfer Material Quote")}</div><div class="panel__meta-copy">{tr(locale(), "这是只读预估，不会扣除库存、占用在途容量、推进时间或生成回执。", "This is a read-only quote. It does not spend inventory, reserve transit capacity, advance time, or create a receipt.")}</div></div></div>
    <div class="panel__body stack"><div class="badge-row"><span class="badge badge--accent">{tr(locale(), "预估", "quote")}</span><span class="badge">{`${tr(locale(), "物料", "Material")}: ${materialLabel(quote().kind, locale(), tr)}`}</span><span class="badge">{`${tr(locale(), "距离", "Distance")}: ${display(quote().distance_km)} km`}</span></div>
      <div class={feasible() ? "feedback-summary" : "feedback-summary feedback-summary--warn"} data-testid="transfer-material-quote-recommendation">{`${tr(locale(), "建议", "Recommended")}: ${recommendationCopy(quote().recommendation, locale(), tr)}`}</div>
      <div class="summary-grid"><Metric label={tr(locale(), "路径标识", "Path identity")} value={pathIdentity(quote().path_id, locale(), tr)} /><Metric label={tr(locale(), "路线", "Routes")} value={routeIdsCopy(quote().route_ids, locale(), tr)} /><Metric label={tr(locale(), "资费电力", "Tariff electricity")} value={display(quote().tariff_electricity_total)} /><Metric label={tr(locale(), "改道次数", "Reroute count")} value={display(quote().reroute_count)} /><Metric label={tr(locale(), "预计收到", "Expected received")} value={display(quote().expected_received_amount)} /><Metric label={tr(locale(), "预计损失", "Expected loss")} value={display(quote().expected_loss_amount)} /><Metric label={tr(locale(), "到达时间", "Arrival") } value={`${display(quote().ticks_until_arrival)} ${tr(locale(), "步后", "ticks")}`} /><Metric label={tr(locale(), "预计就绪", "Ready at")} value={tr(locale(), `第 ${display(quote().ready_at)} 步`, `Tick ${display(quote().ready_at)}`)} /><Metric label={tr(locale(), "优先级", "Priority")} value={priorityCopy(quote().effective_priority, locale(), tr)} /><Metric label={tr(locale(), "在途容量", "Transit capacity")} value={`${display(quote().inflight_before)} / ${display(quote().inflight_capacity)}`} /><Metric label={tr(locale(), "来源库存", "Source after")} value={`${display(quote().source_amount_before)} → ${display(quote().source_amount_after)}`} /><Metric label={tr(locale(), "目的地库存", "Destination after")} value={`${display(quote().destination_amount_before)} → ${display(quote().destination_expected_amount_after)}`} /></div>
      <div class="feedback-summary">{`${tr(locale(), "优先级原因", "Priority reason")}: ${quote().priority_reason === "explicit_priority" ? tr(locale(), "玩家指定", "Player selected") : tr(locale(), "物料默认优先级", "Material default")}`}</div>
      <div class="feedback-summary">{quote().conditional === true ? tr(locale(), "条件性预估：提交时会再次校验库存与在途容量。", "Conditional quote: inventory and transit capacity are checked again on submit.") : tr(locale(), "请重新请求预估后再提交。", "Request a fresh quote before submitting.")}</div>
    </div>
  </section>;
}

export function TransferMaterialQuotePanel(props) {
  const [fromLedger, setFromLedger] = createSignal("site:source"); const [toLedger, setToLedger] = createSignal("site:destination"); const [kind, setKind] = createSignal("iron_ingot"); const [kindInput, setKindInput] = createSignal(null); const [amount, setAmount] = createSignal("20"); const [distance, setDistance] = createSignal("200"); const [priority, setPriority] = createSignal(""); const [routeIdsInput, setRouteIdsInput] = createSignal(""); const [autoReroute, setAutoReroute] = createSignal(false); const [requesting, setRequesting] = createSignal(false); const [localError, setLocalError] = createSignal(""); const locale = () => props.locale; const tr = props.tr; const remote = () => props.requestState || {};
  const displayedKind = () => kindInput() ?? materialLabel(kind(), locale(), tr);
  function updateKindInput(value) { setKind(materialKeyFromInput(value)); setKindInput(String(value || "")); }
  async function requestQuote(event) { event.preventDefault(); setLocalError(""); setRequesting(true); try { const args = [props.requesterAgentId || "", fromLedger(), toLedger(), kind(), amount(), distance(), priority()]; if (routeIdsInput().trim() || autoReroute()) args.push(routeIdsInput(), autoReroute()); const result = await props.requestTransferMaterialQuote(...args); if (!result?.ok) setLocalError(result?.reason || "quote failed"); } catch (error) { setLocalError(String(error)); } finally { setRequesting(false); } }
  return <section id="viewer-transfer-material-quote-panel" class="panel panel--nested" data-testid="transfer-material-quote-panel" data-quote-kind="preflight"><div class="panel__header"><div class="stack stack--compact"><div class="panel__eyebrow">{tr(locale(), "提交前估价", "Before You Commit")}</div><div class="panel__title">{tr(locale(), "物料转运预估", "Transfer Material Quote")}</div></div></div><div class="panel__body stack"><form class="stack stack--compact" data-testid="transfer-material-quote-request-form" onSubmit={requestQuote}><label><span>{tr(locale(), "来源账本", "Source ledger")}</span><input aria-label={tr(locale(), "来源账本", "Source ledger")} value={fromLedger()} onInput={(event) => setFromLedger(event.currentTarget.value)} /></label><label><span>{tr(locale(), "目的地账本", "Destination ledger")}</span><input aria-label={tr(locale(), "目的地账本", "Destination ledger")} value={toLedger()} onInput={(event) => setToLedger(event.currentTarget.value)} /></label><label><span>{tr(locale(), "物料", "Material")}</span><input aria-label={tr(locale(), "物料", "Material")} value={displayedKind()} onInput={(event) => updateKindInput(event.currentTarget.value)} /></label><label><span>{tr(locale(), "数量", "Amount")}</span><input aria-label={tr(locale(), "数量", "Amount")} type="number" min="1" step="1" inputmode="numeric" value={amount()} onInput={(event) => setAmount(event.currentTarget.value)} /></label><label><span>{tr(locale(), "距离（公里）", "Distance (km)")}</span><input aria-label={tr(locale(), "距离（公里）", "Distance (km)")} type="number" min="0" step="1" inputmode="numeric" value={distance()} onInput={(event) => setDistance(event.currentTarget.value)} /></label><label><span>{tr(locale(), "优先级（可选）", "Priority (optional)")}</span><select aria-label={tr(locale(), "优先级（可选）", "Priority (optional)")} value={priority()} onChange={(event) => setPriority(event.currentTarget.value)}><option value="">{tr(locale(), "使用物料默认", "Use material default")}</option><option value="standard">{tr(locale(), "标准", "Standard")}</option><option value="urgent">{tr(locale(), "紧急", "Urgent")}</option></select></label><label><span>{tr(locale(), "路线 ID（可选，按顺序填写）", "Route IDs (optional, ordered)")}</span><textarea aria-label={tr(locale(), "路线 ID（可选，按顺序填写）", "Route IDs (optional, ordered")} value={routeIdsInput()} onInput={(event) => setRouteIdsInput(event.currentTarget.value)} /></label><label><input type="checkbox" aria-label={tr(locale(), "路线不可用时自动改道", "Auto-reroute when route is unavailable")} checked={autoReroute()} onChange={(event) => setAutoReroute(event.currentTarget.checked)} /> <span>{tr(locale(), "路线不可用时自动改道", "Auto-reroute when route is unavailable")}</span></label><button type="submit" class="button button--secondary" disabled={requesting() || remote().status === "pending"}>{requesting() || remote().status === "pending" ? tr(locale(), "正在请求预估…", "Requesting quote…") : tr(locale(), "请求转运预估", "Request transfer quote")}</button></form>{localError() || remote().status === "error" ? <div class="feedback-summary feedback-summary--error" role="alert">{tr(locale(), "无法获取转运预估。请检查连接、玩家会话和输入后重试。", "Could not get the transfer quote. Check the connection, player session, and inputs, then retry.")}</div> : null}{remote().status === "pending" ? <div class="feedback-summary" role="status">{tr(locale(), "正在刷新预估…", "Requesting quote…")}</div> : null}{remote().status === "received" ? <div class="feedback-summary" role="status">{tr(locale(), "预估已返回；提交时仍会重新校验。", "Quote received; submission will re-check the current state.")}</div> : null}{props.quote && remote().status !== "pending" ? <TransferMaterialQuoteCard quote={props.quote} locale={locale()} tr={tr} /> : null}</div></section>;
}
