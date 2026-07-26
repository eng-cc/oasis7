import { createSignal } from "solid-js";

function raw(value) {
  return value == null || value === "" ? "-" : String(value);
}

function stageLabel(value, locale, tr) {
  const labels = {
    bootstrap: ["起步", "Bootstrap"],
    scale_out: ["规模扩展", "Scale out"],
    governance: ["治理", "Governance"],
  };
  const label = labels[String(value || "")];
  return label ? tr(locale, label[0], label[1]) : raw(value);
}

function roleLabel(value, locale, tr) {
  const labels = { explore: ["探索", "Explore"], scale: ["规模化", "Scale"], governance: ["治理", "Governance"], survival: ["生存", "Survival"] };
  const label = labels[String(value || "")];
  return label ? tr(locale, label[0], label[1]) : raw(value);
}

function actionLabel(value, locale, tr) {
  if (value === "advance_industry_stage") return tr(locale, "推进产业阶段", "Advance industry stage");
  if (value === "validate_product_with_module") return tr(locale, "验证产品", "Validate product");
  return raw(value);
}

function QuoteMetric(props) {
  return <div class="metric"><div class="metric__label">{props.label}</div><div class="metric__value">{props.value}</div></div>;
}

export function ProductValidationQuoteCard(props) {
  const quote = () => props.quote || {};
  const locale = () => props.locale;
  const tr = props.tr;
  const isAllowed = () => quote().submission_allowed === true;
  const hasPrerequisite = () => Boolean(String(quote().missing_prerequisite || "").trim());
  return (
    <section
      class="panel panel--nested"
      data-testid="product-validation-quote"
      data-quote-kind="preflight"
      data-product-id={raw(quote().product_id)}
      data-product-role={raw(quote().product_role)}
      data-stage-before={raw(quote().stage_before)}
      data-stage-after={raw(quote().stage_after)}
      data-submission-allowed={String(isAllowed())}
    >
      <div class="panel__header"><div class="stack stack--compact">
        <div class="panel__eyebrow">{tr(locale(), "提交前估价", "Before You Commit")}</div>
        <div class="panel__title">{tr(locale(), "产品验证预估", "Product Validation Quote")}</div>
        <div class="panel__meta-copy">{tr(locale(), "这是已签名的只读预估；不会提交产品验证、执行模块或生成回执。", "This is a signed read-only quote. It does not submit product validation, execute a module, or create a receipt.")}</div>
      </div></div>
      <div class="panel__body stack">
        <div class="badge-row">
          <span class="badge badge--accent">{tr(locale(), "预估", "quote")}</span>
          <span class="badge">{`${tr(locale(), "产品", "Product")}: ${raw(quote().product_id)}`}</span>
          <span class="badge">{`${tr(locale(), "角色", "Role")}: ${roleLabel(quote().product_role, locale(), tr)}`}</span>
          <span class={quote().tradable ? "badge badge--good" : "badge"}>{quote().tradable ? tr(locale(), "可交易", "Tradable") : tr(locale(), "不可交易", "Not tradable")}</span>
        </div>
        <div class="summary-grid">
          <QuoteMetric label={tr(locale(), "阶段", "Stage")} value={`${stageLabel(quote().stage_before, locale(), tr)} → ${stageLabel(quote().stage_after, locale(), tr)}`} />
          <QuoteMetric label={tr(locale(), "解锁 / 价值等级", "Unlock / value class")} value={stageLabel(quote().unlock_or_value_class, locale(), tr)} />
          <QuoteMetric label={tr(locale(), "提交状态", "Submission status")} value={isAllowed() ? tr(locale(), "运行时允许", "Allowed by runtime") : tr(locale(), "运行时阻止", "Blocked by runtime")} />
        </div>
        <div class="feedback-summary" data-testid="product-validation-quote-recommended-action">{`${tr(locale(), "建议", "Recommended")}: ${actionLabel(quote().recommended_action, locale(), tr)}`}</div>
        {hasPrerequisite() ? <div class="feedback-summary feedback-summary--warn" data-testid="product-validation-quote-advisory">{isAllowed()
          ? tr(locale(), "阶段前提尚未满足；这是建议，不会自行禁用运行时允许的提交。", "The stage prerequisite is not met; this is advisory and does not disable a runtime-allowed submission.")
          : tr(locale(), "运行时已阻止提交；请先完成所列前提。", "Runtime has blocked submission; complete the listed prerequisite first.")}</div> : null}
        {hasPrerequisite() ? <div class="feedback-detail" data-raw-missing-prerequisite={raw(quote().missing_prerequisite)}>{`${tr(locale(), "缺少前提", "Missing prerequisite")}: ${raw(quote().missing_prerequisite)}`}</div> : null}
        {quote().reachable_advance_or_recovery ? <div class="feedback-detail" data-raw-recovery={raw(quote().reachable_advance_or_recovery)}>{`${tr(locale(), "可达路径", "Reachable path")}: ${raw(quote().reachable_advance_or_recovery)}`}</div> : null}
      </div>
    </section>
  );
}

export function ProductValidationQuotePanel(props) {
  const [productId, setProductId] = createSignal("logistics_drone");
  const [amount, setAmount] = createSignal("1");
  const [requesting, setRequesting] = createSignal(false);
  const [localError, setLocalError] = createSignal("");
  const remote = () => props.requestState || {};
  const tr = props.tr;
  const locale = () => props.locale;
  const error = () => remote().status === "error" || localError()
    ? tr(locale(), "无法获取产品验证预估。请检查连接、玩家会话和产品输入后重试。", "Could not get the product validation quote. Check the connection, player session, and product input, then retry.")
    : "";
  async function requestQuote(event) {
    event.preventDefault(); setLocalError(""); setRequesting(true);
    try {
      const result = await props.requestProductValidationQuote(productId(), amount());
      if (!result?.ok) setLocalError(result?.reason || "quote failed");
    } catch (requestError) { setLocalError(String(requestError)); } finally { setRequesting(false); }
  }
  return <section class="panel panel--nested" data-testid="product-validation-quote-panel" data-quote-kind="preflight">
    <div class="panel__header"><div class="stack stack--compact"><div class="panel__eyebrow">{tr(locale(), "提交前估价", "Before You Commit")}</div><div class="panel__title">{tr(locale(), "产品验证预估", "Product Validation Quote")}</div></div></div>
    <div class="panel__body stack">
      <form class="stack stack--compact" data-testid="product-validation-quote-request-form" onSubmit={requestQuote}>
        <label><span>{tr(locale(), "产品 ID", "Product ID")}</span><input aria-label={tr(locale(), "产品 ID", "Product ID")} value={productId()} onInput={(event) => setProductId(event.currentTarget.value)} /></label>
        <label><span>{tr(locale(), "数量", "Amount")}</span><input aria-label={tr(locale(), "数量", "Amount")} type="number" min="1" step="1" inputmode="numeric" value={amount()} onInput={(event) => setAmount(event.currentTarget.value)} /></label>
        <button type="submit" class="button button--secondary" disabled={requesting()}>{requesting() ? tr(locale(), "正在请求预估…", "Requesting quote…") : tr(locale(), "请求预估", "Request quote")}</button>
      </form>
      {error() ? <div class="feedback-summary feedback-summary--error" role="alert">{error()}</div> : null}
      {remote().status === "received" ? <div class="feedback-summary" role="status">{tr(locale(), "预估已返回；请在确认前查看建议。", "Quote received; review the guidance before confirmation.")}</div> : null}
      {props.quote ? <ProductValidationQuoteCard quote={props.quote} locale={locale()} tr={tr} /> : null}
    </div>
  </section>;
}
