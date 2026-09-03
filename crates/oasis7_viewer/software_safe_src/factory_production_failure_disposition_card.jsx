import { Show } from "solid-js";

function formatInputs(inputs, locale, localeText) {
  if (!Array.isArray(inputs) || inputs.length === 0) {
    return localeText(locale, "未记录", "None recorded");
  }
  return inputs.map((input) => {
    const kind = input?.kind || localeText(locale, "未知物料", "Unknown material");
    return input?.amount == null ? kind : `${kind} × ${input.amount}`;
  }).join(", ");
}

function formatPower(value, locale, localeText) {
  return value == null ? localeText(locale, "未记录", "Not recorded") : String(value);
}

export function FactoryProductionFailureDispositionCard(props) {
  const locale = () => props.locale;
  const disposition = () => props.disposition || null;
  const recoveryAction = () => disposition()?.recoveryAction || null;
  const text = (zh, en) => props.localeText(locale(), zh, en);

  return (
    <Show when={disposition()}>
      <div
        class="event-card event-card--factory-failure"
        data-testid="viewer-factory-production-failure-disposition"
        role="status"
        aria-live="polite"
      >
        <div class="event-card__title">
          <span>{text("生产结果未通过验证", "Production result failed validation")}</span>
          <span class="badge badge--warn">{disposition().dispositionKind || text("已记录", "Recorded")}</span>
        </div>
        <div class="event-card__meta">
          {`${text("工厂", "Factory")}: ${disposition().factoryId || text("未知", "unknown")} · ${text("配方", "Recipe")}: ${disposition().recipeId || text("未知", "unknown")}`}
        </div>
        <div class="feedback-summary">
          {`${text("阻塞", "Blocker")}: ${disposition().blockerKind || text("生产受阻", "Production blocked")}`}
        </div>
        <Show when={disposition().blockerDetail}>
          <div class="feedback-detail">{`${text("详情", "Detail")}: ${disposition().blockerDetail}`}</div>
        </Show>
        <Show when={disposition().actionId || disposition().requesterAgentId}>
          <div class="feedback-detail">
            {[
              disposition().actionId ? `${text("动作", "Action")}: ${disposition().actionId}` : null,
              disposition().requesterAgentId ? `${text("请求者", "Requester")}: ${disposition().requesterAgentId}` : null,
            ].filter(Boolean).join(" · ")}
          </div>
        </Show>
        <div class="summary-grid">
          {[
            ["已消费投入", "Consumed inputs", formatInputs(disposition().consumedInputs, locale(), props.localeText)],
            ["已损失投入", "Lost inputs", formatInputs(disposition().lostInputs, locale(), props.localeText)],
            ["已消费电力", "Consumed power", formatPower(disposition().consumedPower, locale(), props.localeText)],
            ["已损失电力", "Lost power", formatPower(disposition().lostPower, locale(), props.localeText)],
          ].map(([zh, en, value]) => (
            <div class="metric">
              <div class="metric__label">{text(zh, en)}</div>
              <div class="metric__value">{value}</div>
            </div>
          ))}
        </div>
        <div class="badge-row badge-row--spaced"><span class="badge badge--accent">{text("下一动作", "Next action")}</span></div>
        <div class="feedback-summary">{disposition().nextAction || text("按已发布的下一步处理", "Follow the published next step")}</div>
        <Show when={recoveryAction()}>
          {(action) => (
            <>
              <div class="feedback-detail" data-testid="factory-failure-recovery-action-id">
                {`${text("恢复动作", "Recovery action")}: ${action().actionId}`}
              </div>
              <Show when={action().disabledReason}>
                <div class="feedback-detail" data-testid="factory-failure-recovery-disabled-reason">
                  {`${text("暂不可用", "Unavailable")}: ${action().disabledReason}`}
                </div>
              </Show>
              <Show when={action().executeKind !== "none"}>
                <button
                  class="button button--secondary"
                  type="button"
                  data-testid="factory-failure-recovery-action"
                  onClick={() => props.onAction?.(action())}
                >
                  {action().label}
                </button>
              </Show>
            </>
          )}
        </Show>
        <div class="feedback-detail" data-testid="factory-failure-next-recheck">
          {`${text("下一次复查", "Next recheck")}: ${disposition().nextRecheckBoundary || text("下一次 committed 快照", "next committed snapshot")}`}
        </div>
      </div>
    </Show>
  );
}
