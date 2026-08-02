import { createSignal, For } from "solid-js";

function display(value) { return value == null || value === "" ? "—" : String(value); }
function Metric(props) { return <div class="metric"><div class="metric__label">{props.label}</div><div class="metric__value">{props.value}</div></div>; }
function forecastState(quote, locale, tr) {
  if (!quote.replenishment_enabled) return tr(locale, "不可用", "Unavailable");
  return quote.replenishment_due ? tr(locale, "现已到期", "Due now") : tr(locale, "已排期", "Scheduled");
}
function uncertaintyCopy(quote, locale, tr) {
  if (!quote.replenishment_enabled) return tr(locale, "此预估中补充已禁用；不承诺会发生材料更新。", "Replenishment is disabled for this forecast; no renewal is promised.");
  if (!quote.replenishment_due || Number(quote.estimated_replenished_frag_count) <= 0) return tr(locale, "下一次材料更新前没有可用的碎片数量预估。", "No fragment-count estimate is available until the next replenishment.");
  return tr(locale, "这是运行时的只读估算；实际材料以未来世界快照为准。", "This is a read-only runtime estimate; a future world snapshot remains authoritative.");
}

export function FragmentRefillPreviewPanel(props) {
  const [x, setX] = createSignal(String(props.quote?.chunk?.x ?? 0));
  const [y, setY] = createSignal(String(props.quote?.chunk?.y ?? 0));
  const [z, setZ] = createSignal(String(props.quote?.chunk?.z ?? 0));
  const [requesting, setRequesting] = createSignal(false);
  const [localError, setLocalError] = createSignal("");
  const remote = () => props.requestState || {};
  const quote = () => props.quote || null;
  const locale = () => props.locale;
  const tr = props.tr;
  const stale = () => Boolean(quote()) && ["x", "y", "z"].some((axis) => String(quote().chunk?.[axis]) !== ({ x: x(), y: y(), z: z() })[axis].trim());
  const error = () => remote().status === "error" || localError() ? tr(locale(), "无法获取材料更新预估。请检查连接、玩家会话和区块坐标后重试。", "Could not get the material renewal forecast. Check the connection, player session, and chunk coordinates, then retry.") : "";
  async function requestPreview(event) {
    event.preventDefault(); setLocalError(""); setRequesting(true);
    try { const result = await props.requestFragmentRefillPreview(x(), y(), z()); if (!result?.ok) setLocalError(result?.reason || "preview failed"); } catch (requestError) { setLocalError(String(requestError)); } finally { setRequesting(false); }
  }
  return <section class="panel panel--nested" data-testid="fragment-refill-preview-panel" data-quote-kind="preflight">
    <div class="panel__header"><div class="stack stack--compact"><div class="panel__eyebrow">{tr(locale(), "材料更新预估", "Material renewal forecast")}</div><div class="panel__title">{tr(locale(), "区块材料更新", "Chunk material renewal")}</div><div class="panel__meta-copy">{tr(locale(), "这是已签名的只读预估；不会补充碎片、推进时间或生成回执。", "This is a signed read-only forecast. It does not replenish fragments, advance time, or create a receipt.")}</div></div></div>
    <div class="panel__body stack"><form class="stack stack--compact" data-testid="fragment-refill-preview-request-form" onSubmit={requestPreview}>
      <label><span>{tr(locale(), "区块 X", "Chunk X")}</span><input aria-label={tr(locale(), "区块 X", "Chunk X")} type="number" step="1" inputmode="numeric" value={x()} onInput={(event) => setX(event.currentTarget.value)} /></label>
      <label><span>{tr(locale(), "区块 Y", "Chunk Y")}</span><input aria-label={tr(locale(), "区块 Y", "Chunk Y")} type="number" step="1" inputmode="numeric" value={y()} onInput={(event) => setY(event.currentTarget.value)} /></label>
      <label><span>{tr(locale(), "区块 Z", "Chunk Z")}</span><input aria-label={tr(locale(), "区块 Z", "Chunk Z")} type="number" step="1" inputmode="numeric" value={z()} onInput={(event) => setZ(event.currentTarget.value)} /></label>
      <button type="submit" class="button button--secondary" disabled={requesting() || remote().status === "pending"}>{requesting() || remote().status === "pending" ? tr(locale(), "正在刷新预估…", "Refreshing forecast…") : tr(locale(), "刷新材料预估", "Refresh material forecast")}</button>
    </form>
    {error() ? <div class="feedback-summary feedback-summary--error" role="alert">{error()}</div> : null}
    {remote().status === "pending" ? <div class="feedback-summary" role="status">{tr(locale(), "正在刷新预估；旧预估已失效。", "Refreshing the forecast; the previous forecast is no longer current.")}</div> : null}
    {stale() && remote().status !== "pending" ? <div class="feedback-summary feedback-summary--warn" role="status" data-testid="fragment-refill-preview-stale">{tr(locale(), "区块坐标已变更；当前预估已过期。请重新刷新。", "Chunk coordinates changed; this forecast is stale. Refresh it.")}</div> : null}
    {quote() && remote().status !== "pending" ? <div class="stack" data-testid="fragment-refill-preview-card"><div class="badge-row"><span class="badge badge--accent">{forecastState(quote(), locale(), tr)}</span><span class="badge">{`${tr(locale(), "区块", "Chunk")}: ${display(quote().chunk?.x)}, ${display(quote().chunk?.y)}, ${display(quote().chunk?.z)}`}</span></div><div class="summary-grid"><Metric label={tr(locale(), "预计补充碎片", "Estimated replenished fragments")} value={display(quote().estimated_replenished_frag_count)} /><Metric label={tr(locale(), "等待成本", "Wait cost")} value={`${display(quote().wait_cost_ticks)} ${tr(locale(), "步", "ticks")}`} /><Metric label={tr(locale(), "区块余量", "Chunk remaining")} value={display(quote().chunk_remaining_summary)} /></div><div class="feedback-summary">{uncertaintyCopy(quote(), locale(), tr)}</div><div class="feedback-detail">{`${tr(locale(), "资源提示", "Resource hint")}: ${display(quote().estimated_replenished_resource_hint)}`}</div><For each={quote().remaining_by_element_g || []}>{(entry) => <div class="feedback-detail">{`${display(entry.element)}: ${display(entry.remaining_g)}g`}</div>}</For></div> : null}
    </div>
  </section>;
}
