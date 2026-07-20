import { For, Show } from "solid-js";

const RECOVERY_OPTION_LABELS = {
  kind: {
    repair: ["修复", "Repair"],
    rebuild: ["重建", "Rebuild"],
    pivot: ["转向", "Pivot"],
  },
  time: {
    short: ["短期", "Short"],
    medium: ["中期", "Medium"],
  },
  resource: {
    focused_local_input: ["集中本地投入", "Focused local input"],
    broader_local_reinvestment: ["更广泛的本地再投入", "Broader local reinvestment"],
    redirected_local_commitment: ["转向本地投入", "Redirected local commitment"],
  },
  risk: {
    low: ["低", "Low"],
    moderate: ["中等", "Moderate"],
    tradeoff: ["权衡", "Trade-off"],
  },
};

function humanizeRecoveryOptionValue(value) {
  const words = String(value || "").trim().replace(/[_-]+/g, " ").replace(/\s+/g, " ");
  if (!words) return "—";
  return words.replace(/\b\w/g, (letter) => letter.toUpperCase());
}

export function recoveryOptionDisplayLabel(category, value, locale, tr) {
  const labels = RECOVERY_OPTION_LABELS[category]?.[value];
  if (labels) return tr(locale, labels[0], labels[1]);
  const humanized = humanizeRecoveryOptionValue(value);
  return tr(locale, `未知：${humanized}`, humanized);
}

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
              <div class="event-card__title"><span>{recoveryOptionDisplayLabel("kind", option.kind, props.locale, props.tr)}</span></div>
              <div data-testid="viewer-recovery-option" data-recovery-kind={option.kind}>
                <div class="summary-grid">
                  <RecoveryMetric label={text("时间", "Time")} value={recoveryOptionDisplayLabel("time", option.timeClass, props.locale, props.tr)} />
                  <RecoveryMetric label={text("资源", "Resources")} value={recoveryOptionDisplayLabel("resource", option.resourceClass, props.locale, props.tr)} />
                  <RecoveryMetric label={text("风险", "Risk")} value={recoveryOptionDisplayLabel("risk", option.riskClass, props.locale, props.tr)} />
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
