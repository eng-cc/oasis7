import { For, Show } from "solid-js";

function RecoveryMetric(props) {
  return (
    <div class="metric">
      <div class="metric__label">{props.label}</div>
      <div class="metric__value">{props.value}</div>
    </div>
  );
}

export function RecoveryOptionComparisonPanel(props) {
  const continuation = () => props.continuation || {};
  const options = () => continuation().recoveryOptionComparisons || [];
  const text = (zh, en) => props.tr(props.locale, zh, en);

  return (
    <Show
      when={options().length > 0}
      fallback={
        <RecoveryMetric
          label={text("恢复选项", "Recovery Options")}
          value={continuation().recoveryOptions || text("待发布", "not published")}
        />
      }
    >
      <div class="event-list" data-testid="viewer-recovery-options">
        <For each={options()}>
          {(option) => (
            <div class="event-card recovery-option-card">
              <div class="event-card__title"><span>{option.kind}</span></div>
              <div data-testid="viewer-recovery-option" data-recovery-kind={option.kind}>
                <div class="summary-grid">
                  <RecoveryMetric label={text("时间", "Time")} value={option.timeClass} />
                  <RecoveryMetric label={text("资源", "Resources")} value={option.resourceClass} />
                  <RecoveryMetric label={text("风险", "Risk")} value={option.riskClass} />
                  <RecoveryMetric label={text("保留收益", "Retains")} value={option.retainedBenefit} />
                  <RecoveryMetric label={text("推荐原因", "Why")} value={option.recommendationReason} />
                </div>
              </div>
            </div>
          )}
        </For>
      </div>
    </Show>
  );
}
